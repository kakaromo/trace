use crate::models::{Block, UFS, UFSCUSTOM};
use crate::parsers::log_common;
use crate::parsers::log_common::{create_temp_file, UFSCUSTOM_RE};
use crate::utils::encoding::{decode_bytes_auto, open_encoded_reader};
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::fs;
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Seek, Write};
use std::str;
use std::sync::mpsc;
use std::time::Instant;

// Using read_line_lossy from log_common module
use crate::parsers::log_common::read_line_lossy;

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
        match log_common::process_line_optimized(line) {
            Some((Some(ufs), None, None)) => {
                // Serialize to Bincode format
                bincode::encode_into_std_write(&ufs, &mut *ufs_writer, bincode::config::standard())
                    .unwrap_or_else(|e| panic!("UFS bincode serialization failed: {e}"));
                ufs_count += 1;
            }
            Some((None, Some(block), None)) => {
                // Serialize to Bincode format
                bincode::encode_into_std_write(
                    &block,
                    &mut *block_writer,
                    bincode::config::standard(),
                )
                .unwrap_or_else(|e| panic!("Block bincode serialization failed: {e}"));
                block_count += 1;
            }
            Some((None, None, Some(ufscustom))) => {
                // Serialize to Bincode format
                bincode::encode_into_std_write(
                    &ufscustom,
                    &mut *ufscustom_writer,
                    bincode::config::standard(),
                )
                .unwrap_or_else(|e| panic!("UFSCUSTOM bincode serialization failed: {e}"));
                ufscustom_count += 1;
            }
            _ => continue,
        }
    }

    (ufs_count, block_count, ufscustom_count)
}

// Process chunks in parallel and return UFS and Block I/O and UFSCUSTOM items
fn process_chunk_parallel(chunks: Vec<Vec<String>>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    // Use the common function for parallel processing with progress reporting
    log_common::process_chunks_with_progress(chunks, process_lines, "Parallel chunk processing")
}

// Process lines and extract UFS, Block, and UFSCUSTOM items
fn process_lines(lines: Vec<String>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    let mut ufs_items = Vec::new();
    let mut block_items = Vec::new();
    let mut ufscustom_items = Vec::new();

    for line in lines {
        match log_common::process_line_optimized(&line) {
            Some((Some(ufs), None, None)) => ufs_items.push(ufs),
            Some((None, Some(block), None)) => block_items.push(block),
            Some((None, None, Some(ufscustom))) => ufscustom_items.push(ufscustom),
            _ => continue,
        }
    }

    (ufs_items, block_items, ufscustom_items)
}

// Main log file parsing function
pub fn parse_log_file(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let parse_start_time = Instant::now();
    println!("Starting log file parsing: {filepath}");

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

    // Try memory mapping
    let mut file = File::open(filepath)?;
    let file_size = file.metadata()?.len();

    println!("File size: {}MB", file_size / 1_048_576);

    // Configure thread pool for parallel processing
    let num_threads = num_cpus::get();
    println!("Processing with {num_threads} threads");

    // Try memory mapping using common function
    if let Ok(mmap) = log_common::try_memory_map(&file) {
        println!("Using memory mapping for file processing");

        // Process memory mapped file using common function
        const CHUNK_SIZE: usize = 100_000;
        match log_common::process_memory_mapped_file(&mmap, process_lines, CHUNK_SIZE) {
            Ok((mut mapped_ufs, mut mapped_blocks, mut mapped_ufscustom)) => {
                // Collect results
                ufs_traces.append(&mut mapped_ufs);
                block_traces.append(&mut mapped_blocks);
                ufscustom_traces.append(&mut mapped_ufscustom);

                println!(
                    "Memory mapped processing complete: {} UFS, {} Block, {} UFSCUSTOM",
                    ufs_traces.len(),
                    block_traces.len(),
                    ufscustom_traces.len()
                );
            }
            Err(e) => {
                println!("Error processing memory mapped file: {e}");
                println!("Falling back to standard file reading");
                // Fall back to standard reading mode
                // Reset the file position to the beginning
                file.seek(std::io::SeekFrom::Start(0))?;
            }
        }
    } else {
        // Fall back to traditional method if memory mapping fails
        println!("Memory mapping failed, processing with standard file reading");

        let mut reader = open_encoded_reader(filepath, 16 * 1024 * 1024)?;

        // Set chunk size
        const CHUNK_SIZE: usize = 100_000;
        let mut lines_chunk = Vec::with_capacity(CHUNK_SIZE);
        let mut total_lines = 0;
        let mut last_report_time = Instant::now();
        let mut buf = Vec::new();

        while let Some(line) = read_line_lossy(&mut reader, &mut buf)? {
            lines_chunk.push(line);

            if lines_chunk.len() >= CHUNK_SIZE {
                let chunks_to_process = vec![std::mem::replace(
                    &mut lines_chunk,
                    Vec::with_capacity(CHUNK_SIZE),
                )];
                total_lines += chunks_to_process[0].len();

                // Process chunks in parallel
                let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) =
                    process_chunk_parallel(chunks_to_process);
                ufs_traces.append(&mut chunk_ufs);
                block_traces.append(&mut chunk_blocks);
                ufscustom_traces.append(&mut chunk_ufscustom);

                // Report progress every 5 seconds
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "Processed: {} lines, elapsed time: {:.2} seconds",
                        total_lines,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report_time = now;
                }
            }
        }

        // Process remaining chunk
        if !lines_chunk.is_empty() {
            // Removed unused variable comment (translated from Korean)
            let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) =
                process_chunk_parallel(vec![lines_chunk]);
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);
            ufscustom_traces.append(&mut chunk_ufscustom);
        }
    }

    println!(
        "Total processed events: UFS={}, Block={}, UFSCUSTOM={}, Time taken: {:.2} seconds",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    // Calculate block latency (Q->C mapping to dtoc) for block events
    if !block_traces.is_empty() {
        println!(
            "Calculating Dispatch-to-Complete (dtoc) latency for {} block events...",
            block_traces.len()
        );
        let dtoc_start = Instant::now();
        crate::parsers::log_common::calculate_block_latency_advanced(&mut block_traces);
        println!(
            "dtoc calculation completed in {:.2}s",
            dtoc_start.elapsed().as_secs_f64()
        );
    }

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

    // Set thread count for parallel processing
    let num_threads = num_cpus::get();
    println!("Processing with {num_threads} threads");

    // Rayon uses a global thread pool, no need to create a separate one

    // Try using memory-mapped file
    let file = File::open(filepath)?;
    let metadata = file.metadata()?;
    let file_size = metadata.len();

    // Set larger buffer size (32MB)
    const BUFFER_SIZE: usize = 32 * 1024 * 1024;

    // Configure chunks for multithreading
    const CHUNK_SIZE: usize = 4 * 1024 * 1024; // 4MB chunks
    let total_chunks = (file_size as usize).div_ceil(CHUNK_SIZE);

    println!(
        "File size: {}MB, processing with {} chunks",
        file_size / 1_048_576,
        total_chunks
    );

    let (sender, receiver) = mpsc::channel();

    // Try memory mapping
    if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
        println!("Memory mapping successful, starting parallel processing");

        let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufs_temp_file);
        let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, &block_temp_file);
        let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufscustom_temp_file);

        // Process chunks in parallel
        (0..total_chunks)
            .into_par_iter()
            .for_each_with(sender.clone(), |s, chunk_idx| {
                let start = chunk_idx * CHUNK_SIZE;
                let end = std::cmp::min((chunk_idx + 1) * CHUNK_SIZE, mmap.len());

                // Extract chunk data
                let chunk_bytes = &mmap[start..end];

                // Split chunk into lines (processing to find line endings)
                let mut lines = Vec::new();
                let mut line_start = 0;

                for i in 0..chunk_bytes.len() {
                    if chunk_bytes[i] == b'\n' {
                        // Use from_utf8_lossy to handle non-UTF8 characters
                        let line = decode_bytes_auto(&chunk_bytes[line_start..i]);
                        lines.push(line);
                        line_start = i + 1;
                    }
                }

                // Process last line
                if line_start < chunk_bytes.len() {
                    // Use from_utf8_lossy to handle non-UTF8 characters
                    let line = decode_bytes_auto(&chunk_bytes[line_start..]);
                    if !line.is_empty() {
                        lines.push(line);
                    }
                }

                // Send results
                s.send((chunk_idx, lines)).expect("Failed to send results");
            });

        drop(sender); // Indicate that all threads are done

        // 각 청크별 순서대로 처리
        let mut processed_lines = 0;
        let mut last_report_time = Instant::now();

        for (_, lines) in receiver.iter().collect::<Vec<_>>().into_iter() {
            processed_lines += lines.len();

            // Process lines
            let (ufs_count, block_count, ufscustom_count) = process_chunk_streaming(
                &lines,
                &mut ufs_writer,
                &mut block_writer,
                &mut ufscustom_writer,
            );

            total_ufs += ufs_count;
            total_blocks += block_count;
            total_ufscustom += ufscustom_count;

            // Report progress
            let now = Instant::now();
            if now.duration_since(last_report_time).as_secs() >= 5 {
                println!(
                    "Processing... {} million lines (UFS: {}, Block: {}, UFSCUSTOM: {}), elapsed time: {:.2} seconds",
                    processed_lines / 1_000_000,
                    total_ufs,
                    total_blocks,
                    total_ufscustom,
                    start_time.elapsed().as_secs_f64()
                );
                last_report_time = now;
            }
        }

        // Flush buffers
        ufs_writer.flush()?;
        block_writer.flush()?;
        ufscustom_writer.flush()?;
    } else {
        // Use standard streaming with larger buffers if memory mapping fails
        println!("Memory mapping failed, processing with standard streaming");

        let mut reader = open_encoded_reader(filepath, BUFFER_SIZE)?;

        let mut ufs_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufs_temp_file);
        let mut block_writer = BufWriter::with_capacity(BUFFER_SIZE, &block_temp_file);
        let mut ufscustom_writer = BufWriter::with_capacity(BUFFER_SIZE, &ufscustom_temp_file);

        // Set chunk size
        const LINES_PER_CHUNK: usize = 500_000;
        let mut lines_chunk = Vec::with_capacity(LINES_PER_CHUNK);
        let mut processed_lines = 0;
        let mut last_report_time = Instant::now();

        let mut buf = Vec::new();
        while let Some(line) = read_line_lossy(&mut reader, &mut buf)? {
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
                        "Processing... {} million lines (UFS: {}, Block: {}, UFSCUSTOM: {}), elapsed time: {:.2} seconds", 
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
                "Processing complete: Total {} million lines (UFS: {}, Block: {}, UFSCUSTOM: {})",
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
        "First pass complete: UFS={}, Block={}, UFSCUSTOM={}, elapsed time: {:.2} seconds",
        total_ufs,
        total_blocks,
        total_ufscustom,
        start_time.elapsed().as_secs_f64()
    );

    // Load data from temporary files
    let ufs_traces;
    let mut block_traces;
    let ufscustom_traces;

    // Configure parallel processing
    let loading_start_time = Instant::now();

    println!("Starting data deserialization from bincode...");

    // Load UFS data using common deserializer
    {
        println!("Loading UFS data from {ufs_temp_path}");
        let file = File::open(&ufs_temp_path)?;
        let file_size = file.metadata()?.len();
        println!("UFS data file size: {} MB", file_size / 1_048_576);

        let mut reader =
            BufReader::with_capacity(log_common::get_optimal_buffer_size(file_size), file);

        match log_common::deserialize_ufs_items(&mut reader) {
            Ok(items) => ufs_traces = items,
            Err(e) => {
                eprintln!("Error deserializing UFS data: {e}");
                // Continue with empty vec instead of failing completely
                ufs_traces = Vec::new();
            }
        }
    }

    // Load Block data using common deserializer
    {
        println!("Loading Block data from {block_temp_path}");
        let file = File::open(&block_temp_path)?;
        let file_size = file.metadata()?.len();
        println!("Block data file size: {} MB", file_size / 1_048_576);

        let mut reader =
            BufReader::with_capacity(log_common::get_optimal_buffer_size(file_size), file);

        match log_common::deserialize_block_items(&mut reader) {
            Ok(items) => block_traces = items,
            Err(e) => {
                eprintln!("Error deserializing Block data: {e}");
                // Continue with empty vec instead of failing completely
                block_traces = Vec::new();
            }
        }
    }

    // Load UFSCUSTOM data using common deserializer
    {
        println!("Loading UFSCUSTOM data from {ufscustom_temp_path}");
        let file = File::open(&ufscustom_temp_path)?;
        let file_size = file.metadata()?.len();
        println!("UFSCUSTOM data file size: {} MB", file_size / 1_048_576);

        let mut reader =
            BufReader::with_capacity(log_common::get_optimal_buffer_size(file_size), file);

        match log_common::deserialize_ufscustom_items(&mut reader) {
            Ok(items) => ufscustom_traces = items,
            Err(e) => {
                eprintln!("Error deserializing UFSCUSTOM data: {e}");
                // Continue with empty vec instead of failing completely
                ufscustom_traces = Vec::new();
            }
        }
    }

    println!(
        "Data loading complete (time taken: {:.2} seconds): UFS={}, Block={}, UFSCUSTOM={}",
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

    // Calculate block latency (Q->C mapping to dtoc) for block events
    if !block_traces.is_empty() {
        println!(
            "Calculating Dispatch-to-Complete (dtoc) latency for {} block events...",
            block_traces.len()
        );
        let dtoc_start = Instant::now();
        crate::parsers::log_common::calculate_block_latency_advanced(&mut block_traces);
        println!(
            "dtoc calculation completed in {:.2}s",
            dtoc_start.elapsed().as_secs_f64()
        );
    }

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Parse UFSCustom log file
pub fn parse_ufscustom_log(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let _parse_start_time = Instant::now();
    println!("Starting: UFSCustom log parsing - {filepath}");

    // Check file size
    let file = File::open(filepath)?;
    let file_size = file.metadata()?.len();
    println!("File size: {}MB", file_size / 1_048_576);

    // Configure thread pool for parallel processing
    let num_threads = num_cpus::get();
    println!("Processing with {num_threads} threads");

    // Try memory mapping
    if let Ok(mmap) = unsafe { MmapOptions::new().map(&file) } {
        println!("Memory mapping successful, starting parallel processing");

        // Split file content into lines using lossy conversion
        let content = String::from_utf8_lossy(&mmap[..]).into_owned();

        // Process content at once without parallel processing
        process_ufscustom_content(&content)
    } else {
        println!("Memory mapping failed, processing with standard file reading");

        // Use larger buffer (16MB)
        let mut reader = BufReader::with_capacity(16 * 1024 * 1024, file);

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        let content = String::from_utf8_lossy(&bytes).into_owned();

        process_ufscustom_content(&content)
    }
}

// Helper function for processing UFSCustom content
fn process_ufscustom_content(content: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();

    // Split into lines and process in parallel
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();
    println!("Starting to parse {total_lines} lines");

    let mut ufscustom_traces: Vec<UFSCUSTOM> = Vec::new();
    let mut parsed_lines = 0;
    let mut skipped_lines = 0;
    let mut _header_found = false;

    // Split lines into chunks for parallel processing
    const CHUNK_SIZE: usize = 100_000;

    // Check first line (if it's a header)
    if !lines.is_empty() && lines[0].starts_with("opcode,lba,size,start_time,end_time") {
        _header_found = true;
        println!("Header found: {}", lines[0]);
        skipped_lines += 1;
    }

    // Parallel processing
    let results: Vec<_> = lines
        .par_chunks(CHUNK_SIZE)
        .map(|chunk| {
            let mut chunk_results = Vec::with_capacity(chunk.len());
            let mut chunk_parsed = 0;
            let mut chunk_skipped = 0;

            for &line in chunk {
                // Skip header line (already checked first line, no need for additional verification)
                if line.starts_with("opcode,lba,size,start_time,end_time") {
                    chunk_skipped += 1;
                    continue;
                }

                // Skip comments or empty lines
                if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
                    chunk_skipped += 1;
                    continue;
                }

                // Parse line using regex
                if let Some(caps) = UFSCUSTOM_RE.captures(line) {
                    let opcode = caps["opcode"].to_string();
                    let lba: u64 = caps["lba"].parse().unwrap_or(0);
                    let size: u32 = caps["size"].parse().unwrap_or(0);
                    let start_time: f64 = caps["start_time"].parse().unwrap_or(0.0);
                    let end_time: f64 = caps["end_time"].parse().unwrap_or(0.0);

                    // Calculate dtoc (in milliseconds)
                    let dtoc = (end_time - start_time) * 1000.0;

                    let ufscustom = UFSCUSTOM {
                        opcode,
                        lba,
                        size,
                        start_time,
                        end_time,
                        dtoc,
                        // 새 필드들 초기값으로 설정 (후처리에서 계산됨)
                        start_qd: 0,
                        end_qd: 0,
                        ctoc: 0.0,
                        ctod: 0.0,
                        continuous: false,
                        aligned: crate::utils::is_block_aligned(lba),
                    };

                    chunk_results.push(ufscustom);
                    chunk_parsed += 1;
                } else {
                    // Skip lines that failed to parse
                    chunk_skipped += 1;
                }
            }

            (chunk_results, chunk_parsed, chunk_skipped)
        })
        .collect();

    // Merge results
    for (mut chunk_results, chunk_parsed, chunk_skipped) in results {
        ufscustom_traces.append(&mut chunk_results);
        parsed_lines += chunk_parsed;
        skipped_lines += chunk_skipped;
    }

    // Print statistics
    println!(
        "UFSCustom log parsing completed: Total {total_lines} lines (Parsed: {parsed_lines}, Skipped: {skipped_lines})"
    );
    println!(
        "Parsing time: {:.2} seconds",
        parse_start_time.elapsed().as_secs_f64()
    );

    // Sort by dtoc
    println!("Sorting results by dtoc...");
    let sort_start = Instant::now();
    ufscustom_traces.par_sort_by(|a, b| {
        a.dtoc
            .partial_cmp(&b.dtoc)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    println!(
        "Sorting completed: {:.2} seconds",
        sort_start.elapsed().as_secs_f64()
    );

    // Print basic statistics
    if !ufscustom_traces.is_empty() {
        let min_dtoc = ufscustom_traces.first().unwrap().dtoc;
        let max_dtoc = ufscustom_traces.last().unwrap().dtoc;

        // Calculate average with parallel processing
        let sum_dtoc = ufscustom_traces.par_iter().map(|u| u.dtoc).sum::<f64>();
        let avg_dtoc = sum_dtoc / ufscustom_traces.len() as f64;

        println!("UFSCustom dtoc statistics (milliseconds):");
        println!("  Min: {min_dtoc:.3} ms");
        println!("  Max: {max_dtoc:.3} ms");
        println!("  Avg: {avg_dtoc:.3} ms");
    }

    Ok(ufscustom_traces)
}

// Parse UFSCustom CSV file for dtoc calculation
pub fn parse_ufscustom_file(filepath: &str) -> io::Result<Vec<UFSCUSTOM>> {
    let parse_start_time = Instant::now();
    println!("Starting UFSCustom file parsing: {filepath}");

    // Check if file exists
    if !std::path::Path::new(filepath).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("File not found: {filepath}"),
        ));
    }

    // Use existing parse_ufscustom_log function for parsing
    let ufscustom_traces = parse_ufscustom_log(filepath)?;

    println!(
        "UFSCustom file parsing completed: {} items, time taken: {:.2} seconds",
        ufscustom_traces.len(),
        parse_start_time.elapsed().as_secs_f64()
    );

    Ok(ufscustom_traces)
}
