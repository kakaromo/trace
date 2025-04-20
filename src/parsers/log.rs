use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use regex::Regex;
use crate::models::{UFS, Block};
use std::time::Instant;
use lazy_static::lazy_static;
use std::fs::{self, OpenOptions};
use rand::random;

// 정적 정규식을 사용하여 반복적인 컴파일을 방지
lazy_static! {
    static ref UFS_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)").unwrap();
    
    static ref BLOCK_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$").unwrap();
}

// 임시 파일 경로 생성 함수
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

// 청크 단위로 로그를 파싱하여 임시 파일에 저장
fn process_chunk(chunk: &[String], ufs_writer: &mut BufWriter<&File>, block_writer: &mut BufWriter<&File>) -> (usize, usize) {
    let mut ufs_count = 0;
    let mut block_count = 0;
    
    for line in chunk {
        if let Some(caps) = UFS_RE.captures(line) {
            let ufs = UFS {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                action: caps["command"].to_string(),
                tag: caps["tag"].parse().unwrap(),
                opcode: caps["opcode"].to_string(),
                lba: caps["lba"].parse().unwrap(),
                size: caps["size"].parse::<i64>().unwrap().abs() as u32,
                groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap(),
                hwqid: caps["hwq_id"].parse().unwrap(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };
            
            // 바이너리 형식으로 직렬화하여 저장 (serde 형식으로 변환)
            serde_json::to_writer(&mut *ufs_writer, &ufs).unwrap();
            // 레코드 구분자 추가
            ufs_writer.write_all(b"\n").unwrap();
            ufs_count += 1;
        } 
        else if let Some(caps) = BLOCK_RE.captures(line) {
            let block = Block {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                flags: caps["flags"].to_string(),
                action: caps["action"].to_string(),
                devmajor: caps["devmajor"].parse().unwrap(),
                devminor: caps["devminor"].parse().unwrap(),
                io_type: caps["io_type"].to_string(),
                extra: caps.name("extra").map_or(0, |m| m.as_str().parse().unwrap()),
                // sector가 18446744073709551615(u64 최대값)이상인 경우 0으로 설정
                sector: match caps["sector"].parse::<u64>() {
                    Ok(s) if s >= 18446744073709551615 => 0,
                    Ok(s) => s,
                    Err(_) => 0,
                },
                size: caps["size"].parse().unwrap(),
                comm: caps["comm"].to_string(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };
            
            // 바이너리 형식으로 직렬화하여 저장
            serde_json::to_writer(&mut *block_writer, &block).unwrap();
            // 레코드 구분자 추가
            block_writer.write_all(b"\n").unwrap();
            block_count += 1;
        }
    }
    
    (ufs_count, block_count)
}

// 병렬 처리를 위한 청크 단위 파싱 함수
fn process_chunk_parallel(chunk: Vec<String>) -> (Vec<UFS>, Vec<Block>) {
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    
    for line in &chunk {
        if let Some(caps) = UFS_RE.captures(line) {
            let ufs = UFS {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                action: caps["command"].to_string(),
                tag: caps["tag"].parse().unwrap(),
                opcode: caps["opcode"].to_string(),
                lba: caps["lba"].parse().unwrap(),
                size: caps["size"].parse::<i64>().unwrap().abs() as u32,
                groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap(),
                hwqid: caps["hwq_id"].parse().unwrap(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };
            ufs_traces.push(ufs);
        } 
        else if let Some(caps) = BLOCK_RE.captures(line) {
            let block = Block {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                flags: caps["flags"].to_string(),
                action: caps["action"].to_string(),
                devmajor: caps["devmajor"].parse().unwrap(),
                devminor: caps["devminor"].parse().unwrap(),
                io_type: caps["io_type"].to_string(),
                extra: caps.name("extra").map_or(0, |m| m.as_str().parse().unwrap()),
                // sector가 18446744073709551615(u64 최대값)인 경우 0으로 설정
                sector: match caps["sector"].parse::<u64>() {
                    Ok(s) if s == 18446744073709551615 => 0,
                    Ok(s) => s,
                    Err(_) => 0,
                },
                size: caps["size"].parse().unwrap(),
                comm: caps["comm"].to_string(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };
            block_traces.push(block);
        }
    }
    
    (ufs_traces, block_traces)
}

// 메인 파싱 함수 - 스트리밍 방식으로 처리
pub fn parse_log_file(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
    let parse_start_time = Instant::now();
    println!("로그 파일 파싱 시작: {}", filepath);

    // 파일 크기 확인
    let file_size = fs::metadata(filepath)?.len();
    println!("파일 크기: {:.2} GB", file_size as f64 / 1_073_741_824.0);

    // 파일 크기에 따라 처리 방식 결정
    let result = if file_size > 1_073_741_824 { // 1GB 이상일 경우 스트리밍 처리
        parse_log_file_streaming(filepath)?
    } else {
        // 작은 파일은 기존 방식으로 처리
        parse_log_file_in_memory(filepath)?
    };
    
    println!("전체 파싱 완료, 소요 시간: {:.2}초", parse_start_time.elapsed().as_secs_f64());
    
    Ok(result)
}

// 작은 파일용 인메모리 처리 방식
fn parse_log_file_in_memory(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    
    // 병렬 처리를 위한 청크 크기
    const CHUNK_SIZE: usize = 100_000;
    let mut lines_chunk = Vec::with_capacity(CHUNK_SIZE);
    let mut total_lines = 0;
    
    for line_result in reader.lines() {
        let line = line_result?;
        lines_chunk.push(line);
        
        if lines_chunk.len() >= CHUNK_SIZE {
            let chunks_to_process = std::mem::replace(&mut lines_chunk, Vec::with_capacity(CHUNK_SIZE));
            total_lines += chunks_to_process.len();
            
            // 청크 병렬 처리
            let (mut chunk_ufs, mut chunk_blocks) = process_chunk_parallel(chunks_to_process);
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);
            
            println!("처리된 라인 수: {}", total_lines);
        }
    }
    
    // 남은 청크 처리
    if !lines_chunk.is_empty() {
        total_lines += lines_chunk.len();
        let (mut chunk_ufs, mut chunk_blocks) = process_chunk_parallel(lines_chunk);
        ufs_traces.append(&mut chunk_ufs);
        block_traces.append(&mut chunk_blocks);
    }
    
    println!("총 처리된 라인 수: {}", total_lines);
    println!("처리된 UFS 이벤트: {}, Block 이벤트: {}", ufs_traces.len(), block_traces.len());
    
    Ok((ufs_traces, block_traces))
}

// 대용량 파일용 스트리밍 처리 방식
fn parse_log_file_streaming(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
    // 임시 파일 생성
    let (ufs_temp_file, ufs_temp_path) = create_temp_file("ufs")?;
    let (block_temp_file, block_temp_path) = create_temp_file("block")?;
    
    let start_time = Instant::now();
    let mut total_ufs = 0;
    let mut total_blocks = 0;
    
    // 병렬 처리를 위한 스레드 풀 생성
    let num_threads = num_cpus::get();
    println!("{}개의 스레드로 처리합니다", num_threads);
    
    // 파일 라인 스트리밍 처리
    {
        let file = File::open(filepath)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8MB 버퍼 사용

        let mut ufs_writer = BufWriter::new(&ufs_temp_file);
        let mut block_writer = BufWriter::new(&block_temp_file);
        
        // 청크 크기 설정
        const LINES_PER_CHUNK: usize = 500_000;
        let mut lines_chunk = Vec::with_capacity(LINES_PER_CHUNK);
        let mut processed_lines = 0;
        let mut last_report_time = Instant::now();
        
        for line_result in reader.lines() {
            let line = line_result?;
            lines_chunk.push(line);
            
            if lines_chunk.len() >= LINES_PER_CHUNK {
                let chunks_to_process = std::mem::replace(&mut lines_chunk, Vec::with_capacity(LINES_PER_CHUNK));
                processed_lines += chunks_to_process.len();
                
                // 청크 처리
                let (ufs_count, block_count) = process_chunk(&chunks_to_process, &mut ufs_writer, &mut block_writer);
                total_ufs += ufs_count;
                total_blocks += block_count;
                
                // 5초마다 진행 상황 보고
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "처리 중... {}백만 라인 (UFS: {}, Block: {}), 경과 시간: {:.2}초", 
                        processed_lines / 1_000_000, 
                        total_ufs, 
                        total_blocks, 
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report_time = now;
                }
            }
        }
        
        // 남은 청크 처리
        if !lines_chunk.is_empty() {
            let remaining_lines = lines_chunk.len();
            let (ufs_count, block_count) = process_chunk(&lines_chunk, &mut ufs_writer, &mut block_writer);
            total_ufs += ufs_count;
            total_blocks += block_count;
            processed_lines += remaining_lines;
            
            // 최종 진행 상황 보고
            println!(
                "처리 완료: 총 {}백만 라인 (UFS: {}, Block: {})", 
                processed_lines / 1_000_000, 
                total_ufs, 
                total_blocks
            );
        }
        
        // 버퍼 플러시
        ufs_writer.flush()?;
        block_writer.flush()?;
    }
    
    println!("1차 처리 완료: UFS={}, Block={}, 경과 시간: {:.2}초", 
        total_ufs, total_blocks, start_time.elapsed().as_secs_f64());

    // 임시 파일에서 데이터 로드
    let mut ufs_traces = Vec::with_capacity(total_ufs);
    let mut block_traces = Vec::with_capacity(total_blocks);
    
    // UFS 데이터 로드
    {
        let file = File::open(&ufs_temp_path)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);
        
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if let Ok(ufs) = serde_json::from_str::<UFS>(&line_str) {
                    ufs_traces.push(ufs);
                } else {
                    println!("UFS 역직렬화 오류: {}", line_str);
                }
            }
        }
    }
    
    // Block 데이터 로드
    {
        let file = File::open(&block_temp_path)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);
        
        for line in reader.lines() {
            if let Ok(line_str) = line {
                if let Ok(block) = serde_json::from_str::<Block>(&line_str) {
                    block_traces.push(block);
                } else {
                    println!("Block 역직렬화 오류: {}", line_str);
                }
            }
        }
    }
    
    // 임시 파일 삭제
    let _ = fs::remove_file(ufs_temp_path);
    let _ = fs::remove_file(block_temp_path);
    
    println!("로그 파일 파싱 완료: UFS={}, Block={}, 총 소요 시간: {:.2}초", 
        ufs_traces.len(), block_traces.len(), start_time.elapsed().as_secs_f64());
    
    Ok((ufs_traces, block_traces))
}