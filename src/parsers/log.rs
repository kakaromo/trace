use crate::models::{Block, UFS, UFSCUSTOM};
use lazy_static::lazy_static;
use memmap2::MmapOptions;
use rand::random;
use rayon::prelude::*;
use regex::Regex;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::str;
use std::sync::mpsc;
use std::time::Instant;

// Use static regex to avoid repeated compilation
lazy_static! {
    static ref UFS_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)").unwrap();    
    static ref BLOCK_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$").unwrap();
    static ref UFSCUSTOM_RE: Regex = Regex::new(r"^(?P<opcode>0x[0-9a-f]+),(?P<lba>\d+),(?P<size>\d+),(?P<start_time>\d+\.\d+),(?P<end_time>\d+\.\d+)$").unwrap();    
}

// Create temporary file path
fn create_temp_file(prefix: &str) -> io::Result<(File, String)> {
    let temp_path = format!("/tmp/{}_{}.tmp", prefix, random::<u64>());
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;
    Ok((file, temp_path))
}

// Process log chunks and save to temporary files, including UFSCUSTOM
fn process_chunk_streaming(
    chunk: &[String],
    ufs_writer: &mut BufWriter<&File>,
    block_writer: &mut BufWriter<&File>,
    ufscustom_writer: &mut BufWriter<&File>,
) -> (usize, usize, usize) {
    let mut ufs_count = 0;
    let mut block_count = 0;
    let mut ufscustom_count = 0;

    for line in chunk {
        // UFS 패턴 매칭
        if let Some(caps) = UFS_RE.captures(line) {
            let raw_lba: u64 = caps["lba"].parse().unwrap_or(0);
            let raw_size: i64 = caps["size"].parse::<i64>().unwrap_or(0).unsigned_abs() as i64;

            // Convert bytes to 4KB units
            let lba_in_4kb = raw_lba / 4096;
            let size_in_4kb = (raw_size as f64 / 4096.0).ceil() as u32;

            let ufs = UFS {
                time: caps["time"].parse().unwrap_or(0.0),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap_or(0),
                action: caps["command"].to_string(),
                tag: caps["tag"].parse().unwrap_or(0),
                opcode: caps["opcode"].to_string(),
                lba: lba_in_4kb,
                size: size_in_4kb,
                groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap_or(0),
                hwqid: caps["hwq_id"].parse().unwrap_or(0),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };

            // Serialize to Bincode format
            bincode::encode_into_std_write(&ufs, &mut *ufs_writer, bincode::config::standard())
                .unwrap_or_else(|_| panic!("UFS bincode 직렬화 실패"));
            ufs_count += 1;
        }
        // Block IO 패턴 매칭
        else if let Some(caps) = BLOCK_RE.captures(line) {
            let block = Block {
                time: caps["time"].parse().unwrap_or(0.0),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap_or(0),
                flags: caps["flags"].to_string(),
                action: caps["action"].to_string(),
                devmajor: caps["devmajor"].parse().unwrap_or(0),
                devminor: caps["devminor"].parse().unwrap_or(0),
                io_type: caps["io_type"].to_string(),
                extra: caps
                    .name("extra")
                    .map_or(0, |m| m.as_str().parse().unwrap_or(0)),
                // If sector is == 18446744073709551615 (u64 max value), set to 0
                sector: match caps["sector"].parse::<u64>() {
                    Ok(18446744073709551615) => 0,
                    Ok(s) => s,
                    Err(_) => 0,
                },
                size: caps["size"].parse().unwrap_or(0),
                comm: caps["comm"].to_string(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };

            // Serialize to Bincode format
            bincode::encode_into_std_write(&block, &mut *block_writer, bincode::config::standard())
                .unwrap_or_else(|_| panic!("Block bincode 직렬화 실패"));
            block_count += 1;
        }
        // UFSCUSTOM 패턴 매칭
        else if let Some(caps) = UFSCUSTOM_RE.captures(line) {
            let opcode = caps["opcode"].to_string();
            let lba: u64 = caps["lba"].parse().unwrap_or(0);
            let size: u32 = caps["size"].parse().unwrap_or(0);
            let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
            let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

            // dtoc 계산 (밀리초 단위)
            let dtoc = (end_time - start_time) * 1000.0;

            let ufscustom = UFSCUSTOM {
                opcode,
                lba,
                size,
                start_time,
                end_time,
                dtoc,
            };

            // Serialize to Bincode format
            bincode::encode_into_std_write(
                &ufscustom,
                &mut *ufscustom_writer,
                bincode::config::standard(),
            )
            .unwrap_or_else(|_| panic!("UFSCUSTOM bincode 직렬화 실패"));
            ufscustom_count += 1;
        }
    }

    (ufs_count, block_count, ufscustom_count)
}

// Process chunks in parallel and return UFS and Block I/O and UFSCUSTOM items
fn process_chunk_parallel(chunks: Vec<Vec<String>>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    // Rayon의 par_iter를 사용하여 더 간결하고 효율적인 병렬 처리
    let results: Vec<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> =
        chunks.into_par_iter().map(process_lines).collect();

    // 결과 수집
    let mut ufs_items = Vec::new();
    let mut block_items = Vec::new();
    let mut ufscustom_items = Vec::new();

    for (mut ufs, mut block, mut ufscustom) in results {
        ufs_items.append(&mut ufs);
        block_items.append(&mut block);
        ufscustom_items.append(&mut ufscustom);
    }

    (ufs_items, block_items, ufscustom_items)
}

// Process lines and extract UFS, Block, and UFSCUSTOM items
fn process_lines(lines: Vec<String>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    let mut ufs_items = Vec::new();
    let mut block_items = Vec::new();
    let mut ufscustom_items = Vec::new();

    for line in lines {
        // Try to extract UFS activity from the line
        if let Ok(item) = parse_ufs_event_inline(&line) {
            ufs_items.push(item);
            continue;
        }

        // Try to extract Block I/O activity from the line
        if let Ok(item) = parse_block_io_event_inline(&line) {
            block_items.push(item);
            continue;
        }

        // Try to extract UFSCUSTOM activity from the line
        if let Ok(item) = parse_ufscustom_event_inline(&line) {
            ufscustom_items.push(item);
        }
    }

    (ufs_items, block_items, ufscustom_items)
}

// Parse a UFS event from a line
fn parse_ufs_event_inline(line: &str) -> Result<UFS, &'static str> {
    if let Some(caps) = UFS_RE.captures(line) {
        let raw_lba: u64 = caps["lba"].parse().unwrap_or(0);
        let raw_size: i64 = caps["size"].parse::<i64>().unwrap_or(0).unsigned_abs() as i64;

        // Convert bytes to 4KB units
        let lba_in_4kb = raw_lba / 4096;
        let size_in_4kb = (raw_size as f64 / 4096.0).ceil() as u32;

        let ufs = UFS {
            time: caps["time"].parse().unwrap_or(0.0),
            process: caps["process"].to_string(),
            cpu: caps["cpu"].parse().unwrap_or(0),
            action: caps["command"].to_string(),
            tag: caps["tag"].parse().unwrap_or(0),
            opcode: caps["opcode"].to_string(),
            lba: lba_in_4kb,
            size: size_in_4kb,
            groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap_or(0),
            hwqid: caps["hwq_id"].parse().unwrap_or(0),
            qd: 0,
            dtoc: 0.0,
            ctoc: 0.0,
            ctod: 0.0,
            continuous: false,
        };

        Ok(ufs)
    } else {
        Err("라인이 UFS 패턴과 일치하지 않습니다")
    }
}

// Parse a Block IO event from a line
fn parse_block_io_event_inline(line: &str) -> Result<Block, &'static str> {
    if let Some(caps) = BLOCK_RE.captures(line) {
        let block = Block {
            time: caps["time"].parse().unwrap_or(0.0),
            process: caps["process"].to_string(),
            cpu: caps["cpu"].parse().unwrap_or(0),
            flags: caps["flags"].to_string(),
            action: caps["action"].to_string(),
            devmajor: caps["devmajor"].parse().unwrap_or(0),
            devminor: caps["devminor"].parse().unwrap_or(0),
            io_type: caps["io_type"].to_string(),
            extra: caps
                .name("extra")
                .map_or(0, |m| m.as_str().parse().unwrap_or(0)),
            sector: match caps["sector"].parse::<u64>() {
                Ok(18446744073709551615) => 0,
                Ok(s) => s,
                Err(_) => 0,
            },
            size: caps["size"].parse().unwrap_or(0),
            comm: caps["comm"].to_string(),
            qd: 0,
            dtoc: 0.0,
            ctoc: 0.0,
            ctod: 0.0,
            continuous: false,
        };

        Ok(block)
    } else {
        Err("라인이 Block IO 패턴과 일치하지 않습니다")
    }
}

// Parse a UFSCUSTOM event from a line
fn parse_ufscustom_event_inline(line: &str) -> Result<UFSCUSTOM, &'static str> {
    // 헤더 라인은 건너뜁니다
    if line.starts_with("opcode,lba,size,start_time,end_time") {
        return Err("헤더 라인입니다");
    }

    // 주석이나 빈 라인은 건너뜁니다
    if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
        return Err("주석이나 빈 라인입니다");
    }

    if let Some(caps) = UFSCUSTOM_RE.captures(line) {
        // 문자열 참조 사용으로 복사 줄이기
        let opcode = caps["opcode"].to_string();
        let lba: u64 = caps["lba"].parse().unwrap_or(0);
        let size: u32 = caps["size"].parse().unwrap_or(0);
        let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
        let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

        // dtoc 계산 (밀리초 단위)
        let dtoc = (end_time - start_time) * 1000.0;

        let ufscustom = UFSCUSTOM {
            opcode,
            lba,
            size,
            start_time,
            end_time,
            dtoc,
        };

        Ok(ufscustom)
    } else {
        Err("라인이 UFSCUSTOM 패턴과 일치하지 않습니다")
    }
}

// Main log file parsing function
pub fn parse_log_file(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let parse_start_time = Instant::now();
    println!("Starting log file parsing: {}", filepath);

    // Check file size
    let file_size = fs::metadata(filepath)?.len();
    println!("File size: {:.2} GB", file_size as f64 / 1_073_741_824.0);

    // Choose processing method based on file size
    let result = if file_size > 1_073_741_824 {
        // If larger than 1GB, use streaming
        parse_log_file_streaming(filepath)?
    } else {
        // Process smaller files in memory
        parse_log_file_in_memory(filepath)?
    };

    println!(
        "All parsing completed, time taken: {:.2} seconds",
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(result)
}

// Parse log file in memory (for small files)
fn parse_log_file_in_memory(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();

    // 메모리 매핑 시도
    let file = File::open(filepath)?;
    let file_size = file.metadata()?.len();

    println!("파일 크기: {}MB", file_size / 1_048_576);

    // 병렬 처리를 위한 스레드 풀 설정
    let num_threads = num_cpus::get();
    println!("{}개 스레드로 처리 중", num_threads);

    // 메모리 매핑 시도
    if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
        println!("메모리 매핑 성공, 병렬 처리 시작");

        // 파일 내용을 라인으로 분할
        let content = match std::str::from_utf8(&mmap[..]) {
            Ok(content) => content.to_string(),
            Err(e) => {
                println!("메모리 매핑된 파일을 UTF-8로 변환할 수 없음: {}", e);
                // 대체 방법으로 일반 파일 읽기 사용
                let mut reader = BufReader::with_capacity(16 * 1024 * 1024, file);
                let mut content = String::new();
                reader.read_to_string(&mut content)?;
                content
            }
        };

        // 라인으로 분할
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        // 청크로 나누어 병렬 처리
        const CHUNK_SIZE: usize = 100_000;
        let chunks: Vec<_> = lines
            .par_chunks(CHUNK_SIZE)
            .map(|chunk| {
                chunk
                    .iter()
                    .map(|&s| s.to_string())
                    .collect::<Vec<String>>()
            })
            .collect();

        println!("총 {}개 라인, {}개 청크로 분할", total_lines, chunks.len());

        // 청크 병렬 처리
        let results: Vec<_> = chunks.into_par_iter().map(process_lines).collect();

        // 결과 수집
        for (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) in results {
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);
            ufscustom_traces.append(&mut chunk_ufscustom);
        }
    } else {
        // 메모리 매핑 실패 시 기존 방식 사용
        println!("메모리 매핑 실패, 일반 파일 읽기로 처리");

        let reader = BufReader::with_capacity(16 * 1024 * 1024, file);

        // 청크 크기 설정
        const CHUNK_SIZE: usize = 100_000;
        let mut lines_chunk = Vec::with_capacity(CHUNK_SIZE);
        let mut total_lines = 0;
        let mut last_report_time = Instant::now();

        for line_result in reader.lines() {
            let line = line_result?;
            lines_chunk.push(line);

            if lines_chunk.len() >= CHUNK_SIZE {
                let chunks_to_process = vec![std::mem::replace(
                    &mut lines_chunk,
                    Vec::with_capacity(CHUNK_SIZE),
                )];
                total_lines += chunks_to_process[0].len();

                // 청크 병렬 처리
                let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) =
                    process_chunk_parallel(chunks_to_process);
                ufs_traces.append(&mut chunk_ufs);
                block_traces.append(&mut chunk_blocks);
                ufscustom_traces.append(&mut chunk_ufscustom);

                // 5초마다 진행상황 보고
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "처리됨: {} 라인, 경과 시간: {:.2}초",
                        total_lines,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report_time = now;
                }
            }
        }

        // 남은 청크 처리
        if !lines_chunk.is_empty() {
            // total_lines 변수가 사용되지 않으므로 아래 줄 제거
            let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) =
                process_chunk_parallel(vec![lines_chunk]);
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);
            ufscustom_traces.append(&mut chunk_ufscustom);
        }
    }

    println!(
        "총 처리된 이벤트: UFS={}, Block={}, UFSCUSTOM={}, 소요 시간: {:.2}초",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Parse large log files with streaming (for large files)
fn parse_log_file_streaming(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    // Create temporary files
    let (ufs_temp_file, ufs_temp_path) = create_temp_file("ufs")?;
    let (block_temp_file, block_temp_path) = create_temp_file("block")?;
    let (ufscustom_temp_file, ufscustom_temp_path) = create_temp_file("ufscustom")?;

    let start_time = Instant::now();
    let mut total_ufs = 0;
    let mut total_blocks = 0;
    let mut total_ufscustom = 0;

    // 병렬 처리를 위한 스레드 수 설정
    let num_threads = num_cpus::get();
    println!("{}개 스레드로 처리 중", num_threads);

    // rayon은 전역 스레드 풀을 사용하므로 별도 풀 생성 불필요

    // 메모리 매핑된 파일 사용 시도
    let file = File::open(filepath)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    // 더 큰 버퍼 사이즈 설정 (32MB)
    const BUFFER_SIZE: usize = 32 * 1024 * 1024;

    // 멀티스레딩을 위한 청크 설정
    const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB 청크
    let total_chunks = (file_size as usize).div_ceil(CHUNK_SIZE);

    println!(
        "파일 크기: {}MB, 총 {}개 청크로 처리",
        file_size / 1_048_576,
        total_chunks
    );

    let (sender, receiver) = mpsc::channel();

    // 메모리 매핑 시도
    if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
        println!("메모리 매핑 성공, 병렬 처리 시작");

        let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufs_temp_file);
        let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, &block_temp_file);
        let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufscustom_temp_file);

        // 청크별로 병렬 처리
        (0..total_chunks)
            .into_par_iter()
            .for_each_with(sender.clone(), |s, chunk_idx| {
                let start = chunk_idx * CHUNK_SIZE;
                let end = std::cmp::min((chunk_idx + 1) * CHUNK_SIZE, mmap.len());

                // 청크 데이터 추출
                let chunk_bytes = &mmap[start..end];

                // 청크를 라인으로 분할 (라인의 끝을 찾기 위한 처리)
                let mut lines = Vec::new();
                let mut line_start = 0;

                for i in 0..chunk_bytes.len() {
                    if chunk_bytes[i] == b'\n' {
                        if let Ok(line) = str::from_utf8(&chunk_bytes[line_start..i]) {
                            lines.push(line.to_string());
                        }
                        line_start = i + 1;
                    }
                }

                // 마지막 라인 처리
                if line_start < chunk_bytes.len() {
                    if let Ok(line) = str::from_utf8(&chunk_bytes[line_start..]) {
                        if !line.is_empty() {
                            lines.push(line.to_string());
                        }
                    }
                }

                // 결과 전송
                s.send((chunk_idx, lines)).expect("결과 전송 실패");
            });

        drop(sender); // 모든 스레드 작업 완료 표시

        // 각 청크별 순서대로 처리
        let mut processed_lines = 0;
        let mut last_report_time = Instant::now();

        for (_, lines) in receiver.iter().collect::<Vec<_>>().into_iter() {
            processed_lines += lines.len();

            // 라인 처리
            let (ufs_count, block_count, ufscustom_count) = process_chunk_streaming(
                &lines,
                &mut ufs_writer,
                &mut block_writer,
                &mut ufscustom_writer,
            );

            total_ufs += ufs_count;
            total_blocks += block_count;
            total_ufscustom += ufscustom_count;

            // 진행상황 보고
            let now = Instant::now();
            if now.duration_since(last_report_time).as_secs() >= 5 {
                println!(
                    "처리 중... {} 백만 라인 (UFS: {}, Block: {}, UFSCUSTOM: {}), 경과 시간: {:.2} 초",
                    processed_lines / 1_000_000,
                    total_ufs,
                    total_blocks,
                    total_ufscustom,
                    start_time.elapsed().as_secs_f64()
                );
                last_report_time = now;
            }
        }

        // 버퍼 플러시
        ufs_writer.flush()?;
        block_writer.flush()?;
        ufscustom_writer.flush()?;
    } else {
        // 메모리 매핑 실패 시 일반 스트리밍 처리 (더 큰 버퍼 사용)
        println!("메모리 매핑 실패, 일반 스트리밍으로 처리");

        let reader = BufReader::with_capacity(BUFFER_SIZE, file);

        let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufs_temp_file);
        let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, &block_temp_file);
        let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufscustom_temp_file);

        // Set chunk size
        const LINES_PER_CHUNK: usize = 500_000;
        let mut lines_chunk = Vec::with_capacity(LINES_PER_CHUNK);
        let mut processed_lines = 0;
        let mut last_report_time = Instant::now();

        for line_result in reader.lines() {
            let line = line_result?;
            lines_chunk.push(line);

            if lines_chunk.len() >= LINES_PER_CHUNK {
                let chunks_to_process =
                    std::mem::replace(&mut lines_chunk, Vec::with_capacity(LINES_PER_CHUNK));
                processed_lines += chunks_to_process.len();

                // Process chunk
                let (ufs_count, block_count, ufscustom_count) = process_chunk_streaming(
                    &chunks_to_process,
                    &mut ufs_writer,
                    &mut block_writer,
                    &mut ufscustom_writer,
                );
                total_ufs += ufs_count;
                total_blocks += block_count;
                total_ufscustom += ufscustom_count;

                // Report progress every 5 seconds
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "처리 중... {} 백만 라인 (UFS: {}, Block: {}, UFSCUSTOM: {}), 경과 시간: {:.2} 초", 
                        processed_lines / 1_000_000,
                        total_ufs,
                        total_blocks,
                        total_ufscustom,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report_time = now;
                }
            }
        }

        // Process remaining chunk
        if !lines_chunk.is_empty() {
            let remaining_lines = lines_chunk.len();
            let (ufs_count, block_count, ufscustom_count) = process_chunk_streaming(
                &lines_chunk,
                &mut ufs_writer,
                &mut block_writer,
                &mut ufscustom_writer,
            );
            total_ufs += ufs_count;
            total_blocks += block_count;
            total_ufscustom += ufscustom_count;
            processed_lines += remaining_lines;

            // Final progress report
            println!(
                "처리 완료: 총 {} 백만 라인 (UFS: {}, Block: {}, UFSCUSTOM: {})",
                processed_lines / 1_000_000,
                total_ufs,
                total_blocks,
                total_ufscustom
            );
        }

        // Flush buffers
        ufs_writer.flush()?;
        block_writer.flush()?;
        ufscustom_writer.flush()?;
    }

    println!(
        "첫 번째 패스 완료: UFS={}, Block={}, UFSCUSTOM={}, 경과 시간: {:.2} 초",
        total_ufs,
        total_blocks,
        total_ufscustom,
        start_time.elapsed().as_secs_f64()
    );

    // 임시 파일에서 데이터 로드
    let mut ufs_traces = Vec::with_capacity(total_ufs);
    let mut block_traces = Vec::with_capacity(total_blocks);
    let mut ufscustom_traces = Vec::with_capacity(total_ufscustom);

    // 병렬 처리 설정
    let loading_start_time = Instant::now();

    println!("bincode에서 데이터 역직렬화 시작...");

    // UFS 데이터 로드 (메모리 매핑 사용)
    {
        let file = File::open(&ufs_temp_path)?;
        let file_size = file.metadata()?.len();

        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            println!(
                "UFS 데이터를 위한 메모리 매핑 성공 ({} MB)",
                file_size / 1_048_576
            );

            // 메모리 매핑된 파일을 라인별로 읽기
            let _content = match std::str::from_utf8(&mmap[..]) {
                Ok(content) => content,
                Err(e) => {
                    println!("메모리 매핑된 파일을 UTF-8로 변환할 수 없음: {}", e);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UTF-8 변환 실패",
                    ));
                }
            };

            // Bincode 데이터로 처리
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFS, _>(reader, config) {
                    Ok((ufs, size)) => {
                        ufs_traces.push(ufs);
                        reader = &reader[size..];

                        // 진행 상황 표시 (100만 항목마다)
                        if ufs_traces.len() % 1_000_000 == 0 && !ufs_traces.is_empty() {
                            println!("UFS 항목 {} 백만 개 로드됨", ufs_traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("UFS bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // 메모리 매핑 실패 시 일반 파일 읽기 사용 (대용량 버퍼)
            const BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB 버퍼
            let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            // Bincode 데이터로 처리
            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFS, _>(reader, config) {
                    Ok((ufs, size)) => {
                        ufs_traces.push(ufs);
                        reader = &reader[size..];
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("UFS bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    // Block 데이터 로드 (메모리 매핑 사용)
    {
        let file = File::open(&block_temp_path)?;
        let file_size = file.metadata()?.len();

        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            println!(
                "Block 데이터를 위한 메모리 매핑 성공 ({} MB)",
                file_size / 1_048_576
            );

            // bincode 설정 생성은 중복으로 아래에서 사용함

            // 메모리 매핑된 파일을 라인별로 읽기
            let _content = match std::str::from_utf8(&mmap[..]) {
                Ok(content) => content,
                Err(e) => {
                    println!("메모리 매핑된 파일을 UTF-8로 변환할 수 없음: {}", e);
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "UTF-8 변환 실패",
                    ));
                }
            };

            // Bincode 데이터로 처리
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<Block, _>(reader, config) {
                    Ok((block, size)) => {
                        block_traces.push(block);
                        reader = &reader[size..];

                        // 진행 상황 표시 (100만 항목마다)
                        if block_traces.len() % 1_000_000 == 0 && !block_traces.is_empty() {
                            println!(
                                "Block 항목 {} 백만 개 로드됨",
                                block_traces.len() / 1_000_000
                            );
                        }
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("Block bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // 메모리 매핑 실패 시 일반 파일 읽기 사용
            const BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB 버퍼
            let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            // Bincode 데이터로 처리
            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<Block, _>(reader, config) {
                    Ok((block, size)) => {
                        block_traces.push(block);
                        reader = &reader[size..];
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("Block bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    // UFSCUSTOM 데이터 로드 (메모리 매핑 사용)
    {
        let file = File::open(&ufscustom_temp_path)?;
        let file_size = file.metadata()?.len();

        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            println!(
                "UFSCUSTOM 데이터를 위한 메모리 매핑 성공 ({} MB)",
                file_size / 1_048_576
            );

            // Bincode 데이터로 처리
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFSCUSTOM, _>(reader, config) {
                    Ok((ufscustom, size)) => {
                        ufscustom_traces.push(ufscustom);
                        reader = &reader[size..];

                        // 진행 상황 표시 (10만 항목마다)
                        if ufscustom_traces.len() % 100_000 == 0 && !ufscustom_traces.is_empty() {
                            println!(
                                "UFSCUSTOM 항목 {} 백만 개 로드됨",
                                ufscustom_traces.len() / 1_000_000
                            );
                        }
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("UFSCUSTOM bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // 메모리 매핑 실패 시 일반 파일 읽기 사용
            const BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB 버퍼
            let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
            let mut buffer = Vec::new();
            reader.read_to_end(&mut buffer)?;

            // Bincode 데이터로 처리
            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            // Bincode로 디코딩
            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFSCUSTOM, _>(reader, config) {
                    Ok((ufscustom, size)) => {
                        ufscustom_traces.push(ufscustom);
                        reader = &reader[size..];
                    }
                    Err(e) => {
                        // 역직렬화 오류 발생 시 남은 데이터 건너뛰기
                        println!("UFSCUSTOM bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        }
    }

    println!(
        "데이터 로드 완료 (소요 시간: {:.2} 초): UFS={}, Block={}, UFSCUSTOM={}",
        loading_start_time.elapsed().as_secs_f64(),
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len()
    );

    // Remove temporary files
    let _ = fs::remove_file(ufs_temp_path);
    let _ = fs::remove_file(block_temp_path);
    let _ = fs::remove_file(ufscustom_temp_path);

    println!(
        "Log file parsing completed: UFS={}, Block={}, UFSCUSTOM={}, Total time: {:.2} seconds",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Parse UFSCustom log file
pub fn parse_ufscustom_log(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let _parse_start_time = Instant::now();
    println!("시작: UFSCustom 로그 파싱 - {}", filepath);

    // 파일 크기 확인
    let file = File::open(filepath)?;
    let file_size = file.metadata()?.len();
    println!("파일 크기: {}MB", file_size / 1_048_576);

    // 병렬 처리를 위한 스레드 풀 설정
    let num_threads = num_cpus::get();
    println!("{}개 스레드로 처리 중", num_threads);

    // 메모리 매핑 시도
    if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
        println!("메모리 매핑 성공, 병렬 처리 시작");

        // 파일 내용을 라인으로 분할
        let content = match std::str::from_utf8(&mmap[..]) {
            Ok(content) => content,
            Err(e) => {
                println!("메모리 매핑된 파일을 UTF-8로 변환할 수 없음: {}", e);
                // 대체 방법으로 일반 파일 읽기 사용
                let mut reader = BufReader::with_capacity(16 * 1024 * 1024, file);
                let mut content = String::new();
                reader.read_to_string(&mut content)?;
                return process_ufscustom_content(&content);
            }
        };

        // 병렬 처리 없이 한번에 처리
        process_ufscustom_content(content)
    } else {
        println!("메모리 매핑 실패, 일반 파일 읽기로 처리");

        // 더 큰 버퍼 사용 (16MB)
        let mut reader = BufReader::with_capacity(16 * 1024 * 1024, file);

        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        process_ufscustom_content(&content)
    }
}

// UFSCustom 콘텐츠 처리 헬퍼 함수
fn process_ufscustom_content(content: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();

    // 라인으로 분할하고 병렬 처리
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    println!("총 {}개 라인 파싱 시작", total_lines);

    let mut ufscustom_traces: Vec<UFSCUSTOM> = Vec::new();
    let mut parsed_lines = 0;
    let mut skipped_lines = 0;
    let mut _header_found = false;

    // 라인을 청크로 분할하여 병렬 처리
    const CHUNK_SIZE: usize = 100_000;

    // 첫번째 라인 확인 (헤더인지)
    if !lines.is_empty() && lines[0].starts_with("opcode,lba,size,start_time,end_time") {
        _header_found = true;
        println!("헤더 발견: {}", lines[0]);
        skipped_lines += 1;
    }

    // 병렬 처리
    let results: Vec<_> = lines
        .par_chunks(CHUNK_SIZE)
        .map(|chunk| {
            let mut chunk_results = Vec::with_capacity(chunk.len());
            let mut chunk_parsed = 0;
            let mut chunk_skipped = 0;

            for &line in chunk {
                // 헤더 라인은 건너뜁니다 (이미 첫 라인 확인했으므로 추가 확인 필요 없음)
                if line.starts_with("opcode,lba,size,start_time,end_time") {
                    chunk_skipped += 1;
                    continue;
                }

                // 주석이나 빈 라인은 건너뜁니다
                if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
                    chunk_skipped += 1;
                    continue;
                }

                // 정규식으로 라인 파싱
                if let Some(caps) = UFSCUSTOM_RE.captures(line) {
                    let opcode = caps["opcode"].to_string();
                    let lba: u64 = caps["lba"].parse().unwrap_or(0);
                    let size: u32 = caps["size"].parse().unwrap_or(0);
                    let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
                    let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

                    // dtoc 계산 (밀리초 단위)
                    let dtoc = (end_time - start_time) * 1000.0;

                    let ufscustom = UFSCUSTOM {
                        opcode,
                        lba,
                        size,
                        start_time,
                        end_time,
                        dtoc,
                    };

                    chunk_results.push(ufscustom);
                    chunk_parsed += 1;
                } else {
                    // 파싱 실패한 라인은 스킵
                    chunk_skipped += 1;
                }
            }

            (chunk_results, chunk_parsed, chunk_skipped)
        })
        .collect();

    // 결과 병합
    for (mut chunk_results, chunk_parsed, chunk_skipped) in results {
        ufscustom_traces.append(&mut chunk_results);
        parsed_lines += chunk_parsed;
        skipped_lines += chunk_skipped;
    }

    // 통계 출력
    println!(
        "UFSCustom 로그 파싱 완료: 총 {} 라인 (파싱 성공: {}, 건너뜀: {})",
        total_lines, parsed_lines, skipped_lines
    );
    println!(
        "파싱 소요 시간: {:.2} 초",
        parse_start_time.elapsed().as_secs_f64()
    );

    // dtoc 기준으로 정렬
    println!("dtoc 기준으로 결과 정렬 중...");
    let sort_start = Instant::now();
    ufscustom_traces.par_sort_by(|a, b| {
        a.dtoc
            .partial_cmp(&b.dtoc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!("정렬 완료: {:.2} 초", sort_start.elapsed().as_secs_f64());

    // 기본 통계 출력
    if !ufscustom_traces.is_empty() {
        let min_dtoc = ufscustom_traces.first().unwrap().dtoc;
        let max_dtoc = ufscustom_traces.last().unwrap().dtoc;

        // 병렬 처리로 평균 계산
        let sum_dtoc = ufscustom_traces.par_iter().map(|u| u.dtoc).sum::<f64>();
        let avg_dtoc = sum_dtoc / ufscustom_traces.len() as f64;

        println!("UFSCustom dtoc 통계 (밀리초):");
        println!("  최소: {:.3} ms", min_dtoc);
        println!("  최대: {:.3} ms", max_dtoc);
        println!("  평균: {:.3} ms", avg_dtoc);
    }

    Ok(ufscustom_traces)
}

// Parse UFSCustom CSV file for dtoc calculation
pub fn parse_ufscustom_file(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();
    println!("UFSCustom 파일 파싱 시작: {}", filepath);

    // 파일이 존재하는지 확인
    if !std::path::Path::new(filepath).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("파일을 찾을 수 없습니다: {}", filepath),
        ));
    }

    // 이미 구현된 parse_ufscustom_log 함수를 사용하여 파싱
    let ufscustom_traces = parse_ufscustom_log(filepath)?;

    println!(
        "UFSCustom 파일 파싱 완료: {} 항목, 소요 시간: {:.2}초",
        ufscustom_traces.len(),
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(ufscustom_traces)
}
