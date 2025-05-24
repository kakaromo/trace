// Asynchronous I/O based log parser implementation

use crate::models::{Block, UFS, UFSCUSTOM};
use crate::parsers::log_common;
use crate::parsers::log_common::UFSCUSTOM_RE;
use memmap2::MmapOptions;
use rayon::prelude::*;
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
use rand::random;

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


// Create temporary file function (async) - using standard function
#[allow(dead_code)]
async fn create_temp_file_async(prefix: &str) -> io::Result<(std::fs::File, String)> {
    // Using the synchronous function from common module - file creation generally doesn't impact performance significantly
    log_common::create_temp_file(prefix)
}

// Using common module's process_line function
use crate::parsers::log_common::process_line;

// Async log parsing function (main)
pub async fn parse_log_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let parse_start_time = Instant::now();
    println!("Starting async log parsing: {}", filepath);

    let file_size = fs::metadata(filepath)?.len();
    println!("File size: {:.2} GB", file_size as f64 / 1_073_741_824.0);

    let result = if file_size > 1_073_741_824 {
        parse_large_file_async(filepath).await?
    } else {
        parse_small_file_async(filepath).await?
    };

    println!(
        "Async log parsing completed, time taken: {:.2} seconds",
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(result)
}

// Async small file processing (in-memory)
async fn parse_small_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();
    let (mut ufs_traces, mut block_traces, mut ufscustom_traces) =
        (Vec::new(), Vec::new(), Vec::new());

    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    // Set queue size
    const CHANNEL_SIZE: usize = 1000;
    let (sender, mut receiver) = mpsc::channel(CHANNEL_SIZE);

    // Line reading task
    let read_task = task::spawn(async move {
        let mut count = 0;

        // Async line reading
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
                println!("Processing: {} million lines", count / 1_000_000);
            }
        }

        Ok(count)
    });

    // Collect processed results
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

    // Wait for reading task to complete
    let total_lines = match read_task.await {
        Ok(Ok(count)) => count,
        _ => 0,
    };

    println!(
        "Parsing completed: Total {} lines (UFS: {}, Block: {}, UFSCUSTOM: {}), time taken: {:.2} seconds",
        total_lines,
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Async large file processing (streaming)
async fn parse_large_file_async(
    filepath: &str,
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();

    // Create temporary file paths only
    let ufs_temp_path = format!("/tmp/ufs_{}.tmp", random::<u64>());
    let block_temp_path = format!("/tmp/block_{}.tmp", random::<u64>());
    let ufscustom_temp_path = format!("/tmp/ufscustom_{}.tmp", random::<u64>());

    // Buffer size
    const BUFFER_SIZE: usize = 32 * 1024 * 1024; // 32MB

    // Share file paths via Arc (String type)
    let ufs_temp_path = Arc::new(ufs_temp_path);
    let block_temp_path = Arc::new(block_temp_path);
    let ufscustom_temp_path = Arc::new(ufscustom_temp_path);

    // Each thread opens and writes to files
    // Changed to opening and closing per thread

    // Counter initialization (adding _ prefix for currently unused variables)
    let _ufs_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let _block_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let _ufscustom_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    // Open file and process line by line
    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    // Set chunk size and thread count
    const CHUNK_SIZE: usize = 100_000;
    let cpu_count = num_cpus::get();
    let worker_count = std::cmp::max(1, cpu_count - 1); // One for main thread

    // Create work channels
    let (result_sender, mut result_receiver) =
        mpsc::channel::<(usize, usize, usize)>(worker_count * 2);

    // Create dedicated sender/receiver pairs for each worker
    let mut senders = Vec::with_capacity(worker_count);

    // Create worker tasks
    let mut worker_handles = Vec::with_capacity(worker_count);

    for worker_id in 0..worker_count {
        // Create dedicated channel for each worker
        let (tx, mut rx) = mpsc::channel::<Vec<String>>(2);
        senders.push(tx);

        let worker_sender = result_sender.clone();

        // Clone individual paths for each worker
        let worker_ufs_path = Arc::clone(&ufs_temp_path);
        let worker_block_path = Arc::clone(&block_temp_path);
        let worker_ufscustom_path = Arc::clone(&ufscustom_temp_path);

        let worker_handle = task::spawn(async move {
            println!("Worker {} started", worker_id);

            // Open files for worker
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

            // Create buffered writers
            let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, ufs_file);
            let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, block_file);
            let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, ufscustom_file);

            let mut local_ufs_count = 0;
            let mut local_block_count = 0;
            let mut local_ufscustom_count = 0;

            while let Some(chunk) = rx.recv().await {
                for line in &chunk {
                    if let Some((maybe_ufs, maybe_block, maybe_ufscustom)) = process_line(line) {
                        // Process UFS
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

                // Periodically flush buffer
                if local_ufs_count % 10000 == 0 {
                    let _ = ufs_writer.flush();
                }
                if local_block_count % 10000 == 0 {
                    let _ = block_writer.flush();
                }
                if local_ufscustom_count % 10000 == 0 {
                    let _ = ufscustom_writer.flush();
                }

                // Report results
                let _ = worker_sender
                    .send((local_ufs_count, local_block_count, local_ufscustom_count))
                    .await;

                // Reset counters
                local_ufs_count = 0;
                local_block_count = 0;
                local_ufscustom_count = 0;
            }

            // Flush buffers before worker completes
            let _ = ufs_writer.flush();
            let _ = block_writer.flush();
            let _ = ufscustom_writer.flush();

            println!("Worker {} completed", worker_id);
        });

        worker_handles.push(worker_handle);
    }

    // Read lines and distribute work
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
                        // Distribute work to workers using round-robin
                        let worker_index = (total_lines / CHUNK_SIZE) % senders.len();
                        if senders[worker_index].send(current_chunk).await.is_err() {
                            break;
                        }

                        total_lines += CHUNK_SIZE;

                        // Report progress every 5 seconds
                        let now = Instant::now();
                        if now.duration_since(last_report_time).as_secs() >= 5 {
                            println!(
                                "Processing: {} million lines, elapsed time: {:.2}s",
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

        // Send remaining chunk
        if !chunk.is_empty() {
            // Send last chunk to the first worker
            let chunk_len = chunk.len();
            let _ = senders[0].send(chunk).await;
            total_lines += chunk_len;
        }

        // Send termination signal to all workers
        for sender in senders.iter() {
            let _ = sender.closed().await;
        }

        Ok(total_lines)
    });

    // Collect results
    let mut total_ufs = 0;
    let mut total_block = 0;
    let mut total_ufscustom = 0;

    // Result collection task
    let result_task = task::spawn(async move {
        while let Some((ufs, block, ufscustom)) = result_receiver.recv().await {
            total_ufs += ufs;
            total_block += block;
            total_ufscustom += ufscustom;
        }

        (total_ufs, total_block, total_ufscustom)
    });

    // Wait for line reading task to complete
    let total_lines = match read_task.await {
        Ok(Ok(count)) => count,
        _ => 0,
    };

    // Close the result channel after all data has been sent
    drop(result_sender);

    // Wait for all workers to complete
    for (i, handle) in worker_handles.into_iter().enumerate() {
        if let Err(e) = handle.await {
            println!("Worker {} error: {:?}", i, e);
        }
    }

    // Wait for result collection
    let (ufs_total, block_total, ufscustom_total) = result_task.await?;

    // Buffer flushing is automatically done when each worker closes its connection

    println!(
        "First pass complete: {} lines processed (UFS: {}, Block: {}, UFSCUSTOM: {}), elapsed time: {:.2}s",
        total_lines, ufs_total, block_total, ufscustom_total,
        start_time.elapsed().as_secs_f64()
    );

    // Second stage: Reading data from temporary files
    let loading_start_time = Instant::now();
    println!("Starting to load data from temporary files...");

    // UFS 데이터 로드 (비동기적으로 변환)
    let ufs_load = task::spawn_blocking(move || {
        let file = File::open(ufs_temp_path.as_str()).unwrap();
        let file_size = file.metadata().unwrap().len();
        println!("UFS data file size: {} MB", file_size / 1_048_576);

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
                            println!("Loaded {} million UFS items", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("UFS bincode deserialization error: {:?}", e);
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
        println!("Block data file size: {} MB", file_size / 1_048_576);

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
                            println!("Loaded {} million Block items", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("Block bincode deserialization error: {:?}", e);
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
        println!("UFSCUSTOM data file size: {} MB", file_size / 1_048_576);

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
                            println!("Loaded {} million UFSCUSTOM items", traces.len() / 1_000_000);
                        }
                    }
                    Err(e) => {
                        println!("UFSCUSTOM bincode deserialization error: {:?}", e);
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
        "Data loading complete: UFS={}, Block={}, UFSCUSTOM={}, loading time: {:.2}s",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        loading_start_time.elapsed().as_secs_f64()
    );

    println!(
        "Total processing complete: time taken: {:.2}s",
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Async parsing of UFSCUSTOM file
pub async fn parse_ufscustom_file_async(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();
    println!("Starting async UFSCUSTOM file parsing: {}", filepath);

    // Check if file exists
    if !Path::new(filepath).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {}", filepath),
        ));
    }

    let file = TokioFile::open(filepath).await?;
    let mut reader = TokioBufReader::new(file);
    let mut buf = Vec::new();

    let mut ufscustom_traces = Vec::new();
    let mut header_found = false;
    let mut parsed_lines = 0;
    let mut skipped_lines = 0;

    // Set chunk size
    const CHUNK_SIZE: usize = 100_000;
    let mut chunk = Vec::with_capacity(CHUNK_SIZE);

    // Line reading
    while let Some(line) = read_line_lossy_async(&mut reader, &mut buf).await? {
        // Process header line
        if line.starts_with("opcode,lba,size,start_time,end_time") {
            header_found = true;
            skipped_lines += 1;
            continue;
        }

        // Skip comments or empty lines
        if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
            skipped_lines += 1;
            continue;
        }

        chunk.push(line);

        // Process in parallel when enough lines are collected
        if chunk.len() >= CHUNK_SIZE {
            let to_process = std::mem::replace(&mut chunk, Vec::with_capacity(CHUNK_SIZE));

            // Process chunk (using tokio blocking task)
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

                            // Calculate dtoc (in milliseconds)
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

            // Display progress
            if ufscustom_traces.len() % 1_000_000 == 0 {
                println!(
                    "Processed {} million UFSCUSTOM items",
                    ufscustom_traces.len() / 1_000_000
                );
            }
        }
    }

    // Process remaining chunk
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

                        // Calculate dtoc (in milliseconds)
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

    // Sort by dtoc
    println!("Sorting results by dtoc...");
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

    println!("Sorting completed: {:.2} seconds", sort_start.elapsed().as_secs_f64());

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

        println!("UFSCustom dtoc statistics (milliseconds):");
        println!("  Min: {:.3} ms", min_dtoc);
        println!("  Max: {:.3} ms", max_dtoc);
        println!("  Avg: {:.3} ms", avg_dtoc);
    }

    println!(
        "UFSCustom file parsing completed: {} items (Header: {}, Skipped: {}), time taken: {:.2}s",
        parsed_lines,
        if header_found { "found" } else { "not found" },
        skipped_lines,
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(sorted)
}
