// 비동기 I/O 기반 로그 파서 구현

use crate::models::{Block, UFS, UFSCUSTOM};
// StreamExt는 현재 사용되지 않으므로 제거
use lazy_static::lazy_static;
use memmap2::MmapOptions;
use rand::random;
use rayon::prelude::*;
use regex::Regex;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Read;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::fs::File as TokioFile;
use tokio::io::{AsyncBufReadExt, BufReader as TokioBufReader};
use tokio::sync::mpsc;
use tokio::task;

// 정규식 패턴 정의
lazy_static! {
    static ref UFS_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)").unwrap();    
    static ref BLOCK_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$").unwrap();
    static ref UFSCUSTOM_RE: Regex = Regex::new(r"^(?P<opcode>0x[0-9a-f]+),(?P<lba>\d+),(?P<size>\d+),(?P<start_time>\d+\.\d+),(?P<end_time>\d+\.\d+)$").unwrap();    
}

/// Asynchronously read a single line from the given reader and return it as a
/// UTF-8 `String`. Invalid UTF-8 sequences will be replaced with the Unicode
/// replacement character. Returns `Ok(None)` when EOF is reached.
async fn read_line_lossy_async<R: tokio::io::AsyncBufRead + Unpin>(
    reader: &mut R,
    buffer: &mut Vec<u8>,
) -> io::Result<Option<String>> {
    buffer.clear();
    let bytes_read = reader.read_until(b'\n', buffer).await?;
    if bytes_read == 0 {
        return Ok(None);
    }
    if buffer.ends_with(b"\n") {
        buffer.pop();
        if buffer.ends_with(b"\r") {
            buffer.pop();
        }
    }
    Ok(Some(String::from_utf8_lossy(buffer).to_string()))
}


// 임시 파일 생성 함수 (비동기)
#[allow(dead_code)]
async fn create_temp_file_async(prefix: &str) -> io::Result<(std::fs::File, String)> {
    let temp_path = format!("/tmp/{}_{}.tmp", prefix, random::<u64>());
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;
    Ok((file, temp_path))
}

// 각 라인 처리 함수
fn process_line(line: &str) -> Option<(Option<UFS>, Option<Block>, Option<UFSCUSTOM>)> {
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

        return Some((Some(ufs), None, None));
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

        return Some((None, Some(block), None));
    }
    // UFSCUSTOM 패턴 매칭
    else if let Some(caps) = UFSCUSTOM_RE.captures(line) {
        // 헤더 라인이면 건너뜀
        if line.starts_with("opcode,lba,size,start_time,end_time") {
            return None;
        }

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

        return Some((None, None, Some(ufscustom)));
    }

    None
}

// 비동기 로그 파싱 함수 (메인)
pub async fn parse_log_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let parse_start_time = Instant::now();
    println!("비동기 로그 파싱 시작: {}", filepath);

    let file_size = fs::metadata(filepath)?.len();
    println!("파일 크기: {:.2} GB", file_size as f64 / 1_073_741_824.0);

    let result = if file_size > 1_073_741_824 {
        parse_large_file_async(filepath).await?
    } else {
        parse_small_file_async(filepath).await?
    };

    println!(
        "비동기 로그 파싱 완료, 소요 시간: {:.2} 초",
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(result)
}

// 비동기 작은 파일 처리 (메모리 내에서 처리)
async fn parse_small_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();
    let (mut ufs_traces, mut block_traces, mut ufscustom_traces) =
        (Vec::new(), Vec::new(), Vec::new());

    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    // 대기열 크기 설정
    const CHANNEL_SIZE: usize = 1000;
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SIZE);

    // 라인 읽기 작업
    let read_task = task::spawn(async move {
        let mut count = 0;

        // 라인 비동기 읽기
        while let Some(line) = match read_line_lossy_async(&mut reader, &mut buf).await {
            Ok(result) => result,
            Err(e) => return Err(e),
        } {
            if let Some(result) = process_line(&line) {
                if sender.send(result).await.is_err() {
                    break;
                }
            }

            count += 1;
            if count % 1_000_000 == 0 {
                println!("처리 중: {}백만 라인", count / 1_000_000);
            }
        }

        Ok(count)
    });

    // 처리된 결과 수집
    while let Some((maybe_ufs, maybe_block, maybe_ufscustom)) = receiver.recv().await {
        if let Some(ufs) = maybe_ufs {
            ufs_traces.push(ufs);
        }
        if let Some(block) = maybe_block {
            block_traces.push(block);
        }
        if let Some(ufscustom) = maybe_ufscustom {
            ufscustom_traces.push(ufscustom);
        }
    }

    // 읽기 작업 완료 대기
    let total_lines = match read_task.await {
        Ok(Ok(count)) => count,
        _ => 0,
    };

    println!(
        "파싱 완료: 총 {} 라인 (UFS: {}, Block: {}, UFSCUSTOM: {}), 소요 시간: {:.2}초",
        total_lines,
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// 비동기 대용량 파일 처리 (스트리밍)
async fn parse_large_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();

    // 임시 파일 경로만 생성
    let ufs_temp_path = format!("/tmp/ufs_{}.tmp", random::<u64>());
    let block_temp_path = format!("/tmp/block_{}.tmp", random::<u64>());
    let ufscustom_temp_path = format!("/tmp/ufscustom_{}.tmp", random::<u64>());

    // 버퍼 사이즈
    const BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB

    // 파일 경로만 Arc로 공유 (String 타입)
    let ufs_temp_path = Arc::new(ufs_temp_path);
    let block_temp_path = Arc::new(block_temp_path);
    let ufscustom_temp_path = Arc::new(ufscustom_temp_path);

    // 각 쓰레드에서 파일을 열고 쓰는 방식
    // 쓰레드별로 열고 닫는 방식으로 변경

    // 카운터 초기화 (현재 사용되지 않는 변수들이므로 _ 접두사 추가)
    let _ufs_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let _block_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let _ufscustom_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // 파일 열기 및 라인 단위 처리
    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    // 청크 크기 및 쓰레드 수 설정
    const CHUNK_SIZE: usize = 100_000;
    let cpu_count = num_cpus::get();
    let worker_count = std::cmp::max(1, cpu_count - 1); // 하나는 메인 쓰레드

    // 작업 채널들 생성
    let (result_sender, mut result_receiver) =
        mpsc::channel::<(usize, usize, usize)>(worker_count * 2);

    // 각 작업자에 대한 전용 송신자/수신자 쌍 생성
    let mut senders = Vec::with_capacity(worker_count);

    // 작업자 태스크들 생성
    let mut worker_handles = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        // 각 작업자에 전용 채널 생성
        let (tx, mut rx) = mpsc::channel::<Vec<String>>(2);
        senders.push(tx);

        let worker_sender = result_sender.clone();

        // 각 작업자마다 개별 경로 복제
        let worker_ufs_path = Arc::clone(&ufs_temp_path);
        let worker_block_path = Arc::clone(&block_temp_path);
        let worker_ufscustom_path = Arc::clone(&ufscustom_temp_path);

        let worker_handle = task::spawn(async move {
            println!("작업자 {} 시작", worker_id);

            // 작업자별로 파일 열기
            let ufs_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(worker_ufs_path.as_str())
                .unwrap();

            let block_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(worker_block_path.as_str())
                .unwrap();

            let ufscustom_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(worker_ufscustom_path.as_str())
                .unwrap();

            // 버퍼 라이터 생성
            let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, ufs_file);
            let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, block_file);
            let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, ufscustom_file);

            let mut local_ufs_count = 0;
            let mut local_block_count = 0;
            let mut local_ufscustom_count = 0;

            while let Some(chunk) = rx.recv().await {
                for line in &chunk {
                    if let Some((maybe_ufs, maybe_block, maybe_ufscustom)) = process_line(line) {
                        // UFS 처리
                        if let Some(ufs) = maybe_ufs {
                            if bincode::encode_into_std_write(
                                &ufs,
                                &mut ufs_writer,
                                bincode::config::standard(),
                            )
                            .is_ok()
                            {
                                local_ufs_count += 1;
                            }
                        }

                        // Block 처리
                        if let Some(block) = maybe_block {
                            if bincode::encode_into_std_write(
                                &block,
                                &mut block_writer,
                                bincode::config::standard(),
                            )
                            .is_ok()
                            {
                                local_block_count += 1;
                            }
                        }

                        // UFSCUSTOM 처리
                        if let Some(ufscustom) = maybe_ufscustom {
                            if bincode::encode_into_std_write(
                                &ufscustom,
                                &mut ufscustom_writer,
                                bincode::config::standard(),
                            )
                            .is_ok()
                            {
                                local_ufscustom_count += 1;
                            }
                        }
                    }
                }

                // 주기적으로 버퍼 플러시
                if local_ufs_count % 10000 == 0 {
                    let _ = ufs_writer.flush();
                }
                if local_block_count % 10000 == 0 {
                    let _ = block_writer.flush();
                }
                if local_ufscustom_count % 10000 == 0 {
                    let _ = ufscustom_writer.flush();
                }

                // 결과 보고
                let _ = worker_sender
                    .send((local_ufs_count, local_block_count, local_ufscustom_count))
                    .await;

                // 카운터 초기화
                local_ufs_count = 0;
                local_block_count = 0;
                local_ufscustom_count = 0;
            }

            // 작업자 종료 전에 버퍼 플러시
            let _ = ufs_writer.flush();
            let _ = block_writer.flush();
            let _ = ufscustom_writer.flush();

            println!("작업자 {} 종료", worker_id);
        });

        worker_handles.push(worker_handle);
    }

    // 라인 읽기 및 작업 분배
    let read_task = task::spawn(async move {
        let mut total_lines = 0;
        let mut chunk = Vec::with_capacity(CHUNK_SIZE);
        let mut last_report_time = Instant::now();

        loop {
            match read_line_lossy_async(&mut reader, &mut buf).await {
                Ok(Some(line)) => {
                    chunk.push(line);

                    if chunk.len() >= CHUNK_SIZE {
                        let current_chunk = std::mem::replace(&mut chunk, Vec::with_capacity(CHUNK_SIZE));
                        // 라운드 로빈 방식으로 작업자에게 분배
                        let worker_index = (total_lines / CHUNK_SIZE) % senders.len();
                        if senders[worker_index].send(current_chunk).await.is_err() {
                            break;
                        }

                        total_lines += CHUNK_SIZE;

                        // 5초마다 진행 상황 보고
                        let now = Instant::now();
                        if now.duration_since(last_report_time).as_secs() >= 5 {
                            println!(
                                "처리 중: {} 백만 라인, 경과 시간: {:.2}초",
                                total_lines / 1_000_000,
                                start_time.elapsed().as_secs_f64()
                            );
                            last_report_time = now;
                        }
                    }
                },
                Ok(None) => break, // End of file
                Err(e) => return Err(e), // Propagate the error
            }
        }

        // 남은 청크 보내기
        if !chunk.is_empty() {
            // 마지막 청크는 첫 번째 작업자에게 전송
            let chunk_len = chunk.len();
            let _ = senders[0].send(chunk).await;
            total_lines += chunk_len;
        }

        // 모든 작업자에게 종료 신호 전송
        for sender in senders.iter() {
            let _ = sender.closed().await;
        }

        Ok(total_lines)
    });

    // 결과 수집
    let mut total_ufs = 0;
    let mut total_block = 0;
    let mut total_ufscustom = 0;

    // 결과 수집 태스크
    let result_task = task::spawn(async move {
        while let Some((ufs, block, ufscustom)) = result_receiver.recv().await {
            total_ufs += ufs;
            total_block += block;
            total_ufscustom += ufscustom;
        }

        (total_ufs, total_block, total_ufscustom)
    });

    // 라인 읽기 완료 기다림
    let total_lines = match read_task.await {
        Ok(Ok(count)) => count,
        _ => 0,
    };

    // 모든 데이터 전송 완료 후 결과 채널 닫기
    drop(result_sender);

    // 모든 작업자 완료 대기
    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.await {
            println!("작업자 {} 오류: {:?}", i, e);
        }
    }

    // 결과 수집 대기
    let (ufs_total, block_total, ufscustom_total) = result_task.await?;

    // 버퍼 플러시는 각 작업자가 연결을 닫을 때 자동으로 수행됩니다.

    println!(
        "첫 번째 패스 완료: 총 {} 라인 (UFS: {}, Block: {}, UFSCUSTOM: {}) 처리됨, 경과 시간: {:.2}초",
        total_lines, ufs_total, block_total, ufscustom_total,
        start_time.elapsed().as_secs_f64()
    );

    // 두 번째 단계: 임시 파일에서 데이터 읽기
    let loading_start_time = Instant::now();
    println!("임시 파일에서 데이터 로드 시작...");

    // UFS 데이터 로드 (비동기적으로 변환)
    let ufs_load = task::spawn_blocking(move || {
        let file = File::open(ufs_temp_path.as_str()).unwrap();
        let file_size = file.metadata().unwrap().len();
        println!("UFS 데이터 파일 크기: {} MB", file_size / 1_048_576);

        let mut traces = Vec::with_capacity(ufs_total);

        // memmap 사용하여 로드
        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFS, _>(reader, config) {
                    Ok((ufs, size)) => {
                        traces.push(ufs);
                        reader = &reader[size..];

                        // 진행 상황 표시 (100만 항목마다)
                        if traces.len() % 1_000_000 == 0 && !traces.is_empty() {
                            println!("UFS 항목 {} 백만 개 로드됨", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("UFS bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // memmap 실패 시 일반 방식으로 로드
            let mut buffer = Vec::new();
            let mut std_file = std::fs::File::open(ufs_temp_path.as_str()).unwrap();
            std_file.read_to_end(&mut buffer).unwrap();

            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFS, _>(reader, config) {
                    Ok((ufs, size)) => {
                        traces.push(ufs);
                        reader = &reader[size..];
                    }
                    Err(_) => break,
                }
            }
        }

        // 임시 파일 삭제
        let _ = std::fs::remove_file(ufs_temp_path.as_str());

        traces
    });

    // Block 데이터 로드 (비동기적으로 변환)
    let block_load = task::spawn_blocking(move || {
        let file = File::open(block_temp_path.as_str()).unwrap();
        let file_size = file.metadata().unwrap().len();
        println!("Block 데이터 파일 크기: {} MB", file_size / 1_048_576);

        let mut traces = Vec::with_capacity(block_total);

        // memmap 사용하여 로드
        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<Block, _>(reader, config) {
                    Ok((block, size)) => {
                        traces.push(block);
                        reader = &reader[size..];

                        // 진행 상황 표시 (100만 항목마다)
                        if traces.len() % 1_000_000 == 0 && !traces.is_empty() {
                            println!("Block 항목 {} 백만 개 로드됨", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("Block bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // memmap 실패 시 일반 방식으로 로드
            let mut buffer = Vec::new();
            let mut std_file = std::fs::File::open(block_temp_path.as_str()).unwrap();
            std_file.read_to_end(&mut buffer).unwrap();

            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<Block, _>(reader, config) {
                    Ok((block, size)) => {
                        traces.push(block);
                        reader = &reader[size..];
                    }
                    Err(_) => break,
                }
            }
        }

        // 임시 파일 삭제
        let _ = std::fs::remove_file(block_temp_path.as_str());

        traces
    });

    // UFSCUSTOM 데이터 로드 (비동기적으로 변환)
    let ufscustom_load = task::spawn_blocking(move || {
        let file = File::open(ufscustom_temp_path.as_str()).unwrap();
        let file_size = file.metadata().unwrap().len();
        println!("UFSCUSTOM 데이터 파일 크기: {} MB", file_size / 1_048_576);

        let mut traces = Vec::with_capacity(ufscustom_total);

        // memmap 사용하여 로드
        if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
            let mut reader = &mmap[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFSCUSTOM, _>(reader, config) {
                    Ok((ufscustom, size)) => {
                        traces.push(ufscustom);
                        reader = &reader[size..];

                        // 진행 상황 표시 (10만 항목마다)
                        if traces.len() % 100_000 == 0 && !traces.is_empty() {
                            println!("UFSCUSTOM 항목 {} 백만 개 로드됨", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("UFSCUSTOM bincode 역직렬화 오류: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // memmap 실패 시 일반 방식으로 로드
            let mut buffer = Vec::new();
            let mut std_file = std::fs::File::open(ufscustom_temp_path.as_str()).unwrap();
            std_file.read_to_end(&mut buffer).unwrap();

            let mut reader = &buffer[..];
            let config = bincode::config::standard();

            while !reader.is_empty() {
                match bincode::decode_from_slice::<UFSCUSTOM, _>(reader, config) {
                    Ok((ufscustom, size)) => {
                        traces.push(ufscustom);
                        reader = &reader[size..];
                    }
                    Err(_) => break,
                }
            }
        }

        // 임시 파일 삭제
        let _ = std::fs::remove_file(ufscustom_temp_path.as_str());

        traces
    });

    // 비동기 작업 완료 대기
    let ufs_traces = ufs_load.await?;
    let block_traces = block_load.await?;
    let ufscustom_traces = ufscustom_load.await?;

    println!(
        "데이터 로드 완료: UFS={}, Block={}, UFSCUSTOM={}, 로드 시간: {:.2}초",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        loading_start_time.elapsed().as_secs_f64()
    );

    println!(
        "총 처리 완료: 소요 시간: {:.2}초",
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// UFSCUSTOM 파일 비동기 파싱
pub async fn parse_ufscustom_file_async(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();
    println!("비동기 UFSCustom 파일 파싱 시작: {}", filepath);

    // 파일이 존재하는지 확인
    if !Path::new(filepath).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("파일을 찾을 수 없습니다: {}", filepath),
        ));
    }

    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    let mut ufscustom_traces = Vec::new();
    let mut header_found = false;
    let mut parsed_lines = 0;
    let mut skipped_lines = 0;

    // 청크 크기 설정
    const CHUNK_SIZE: usize = 100_000;
    let mut chunk = Vec::with_capacity(CHUNK_SIZE);

    // 라인 읽기
    while let Some(line) = read_line_lossy_async(&mut reader, &mut buf).await? {
        // 헤더 라인 처리
        if line.starts_with("opcode,lba,size,start_time,end_time") {
            header_found = true;
            skipped_lines += 1;
            continue;
        }

        // 주석이나 빈 라인은 건너뜁니다
        if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
            skipped_lines += 1;
            continue;
        }

        chunk.push(line);

        // 충분한 라인을 모으면 병렬 처리
        if chunk.len() >= CHUNK_SIZE {
            let to_process = std::mem::replace(&mut chunk, Vec::with_capacity(CHUNK_SIZE));

            // 청크 처리 (tokio 블로킹 태스크로 처리)
            let processed_records = task::spawn_blocking(move || {
                to_process
                    .par_iter()
                    .filter_map(|line| {
                        if let Some(caps) = UFSCUSTOM_RE.captures(line) {
                            let opcode = caps["opcode"].to_string();
                            let lba: u64 = caps["lba"].parse().unwrap_or(0);
                            let size: u32 = caps["size"].parse().unwrap_or(0);
                            let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
                            let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

                            // dtoc 계산 (밀리초 단위)
                            let dtoc = (end_time - start_time) * 1000.0;

                            Some(UFSCUSTOM {
                                opcode,
                                lba,
                                size,
                                start_time,
                                end_time,
                                dtoc,
                            })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<UFSCUSTOM>>()
            })
            .await?;

            parsed_lines += processed_records.len();
            ufscustom_traces.extend(processed_records);

            // 진행 상황 표시
            if ufscustom_traces.len() % 1_000_000 == 0 {
                println!(
                    "UFSCustom 항목 {}백만 개 처리됨",
                    ufscustom_traces.len() / 1_000_000
                );
            }
        }
    }

    // 남은 청크 처리
    if !chunk.is_empty() {
        let processed_records = task::spawn_blocking(move || {
            chunk
                .par_iter()
                .filter_map(|line| {
                    if let Some(caps) = UFSCUSTOM_RE.captures(line) {
                        let opcode = caps["opcode"].to_string();
                        let lba: u64 = caps["lba"].parse().unwrap_or(0);
                        let size: u32 = caps["size"].parse().unwrap_or(0);
                        let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
                        let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

                        // dtoc 계산 (밀리초 단위)
                        let dtoc = (end_time - start_time) * 1000.0;

                        Some(UFSCUSTOM {
                            opcode,
                            lba,
                            size,
                            start_time,
                            end_time,
                            dtoc,
                        })
                    } else {
                        None
                    }
                })
                .collect::<Vec<UFSCUSTOM>>()
        })
        .await?;

        parsed_lines += processed_records.len();
        ufscustom_traces.extend(processed_records);
    }

    // dtoc 기준으로 정렬
    println!("dtoc 기준으로 결과 정렬 중...");
    let sort_start = Instant::now();

    // 병렬 정렬 수행
    let sorted = task::spawn_blocking(move || {
        let mut traces = ufscustom_traces;
        traces.par_sort_by(|a, b| {
            a.dtoc
                .partial_cmp(&b.dtoc)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        traces
    })
    .await?;

    println!("정렬 완료: {:.2} 초", sort_start.elapsed().as_secs_f64());

    // 기본 통계 출력
    if !sorted.is_empty() {
        let first = sorted.first().unwrap();
        let last = sorted.last().unwrap();
        let min_dtoc = first.dtoc;
        let max_dtoc = last.dtoc;
        let sorted_len = sorted.len();

        // 병렬로 평균 계산 (sorted를 클론하지 않고 immutable 참조로 사용)
        let sum: f64 = sorted.par_iter().map(|u| u.dtoc).sum();
        let avg_dtoc = sum / sorted_len as f64;

        println!("UFSCustom dtoc 통계 (밀리초):");
        println!("  최소: {:.3} ms", min_dtoc);
        println!("  최대: {:.3} ms", max_dtoc);
        println!("  평균: {:.3} ms", avg_dtoc);
    }

    println!(
        "UFSCustom 파일 파싱 완료: {} 항목 (헤더: {}, 건너뜀: {}), 소요 시간: {:.2}초",
        parsed_lines,
        if header_found { "발견" } else { "없음" },
        skipped_lines,
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(sorted)
}
