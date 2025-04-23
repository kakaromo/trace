use crate::models::{Block, UFS};
use lazy_static::lazy_static;
use rand::random;
use regex::Regex;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::time::Instant;

// Use static regex to avoid repeated compilation
lazy_static! {
    static ref UFS_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)").unwrap();    
    static ref BLOCK_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$").unwrap();
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

// Process log chunks and save to temporary files
fn process_chunk(
    chunk: &[String],
    ufs_writer: &mut BufWriter<&File>,
    block_writer: &mut BufWriter<&File>,
) -> (usize, usize) {
    let mut ufs_count = 0;
    let mut block_count = 0;

    for line in chunk {
        if let Some(caps) = UFS_RE.captures(line) {
            let raw_lba: u64 = caps["lba"].parse().unwrap();
            let raw_size: i64 = caps["size"].parse::<i64>().unwrap().unsigned_abs() as i64;

            // Convert bytes to 4KB units
            let lba_in_4kb = raw_lba / 4096;
            let size_in_4kb = (raw_size as f64 / 4096.0).ceil() as u32;

            let ufs = UFS {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                action: caps["command"].to_string(),
                tag: caps["tag"].parse().unwrap(),
                opcode: caps["opcode"].to_string(),
                lba: lba_in_4kb,
                size: size_in_4kb,
                groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap(),
                hwqid: caps["hwq_id"].parse().unwrap(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };

            // Serialize to binary format and store (serde format)
            serde_json::to_writer(&mut *ufs_writer, &ufs).unwrap();
            // Add record separator
            ufs_writer.write_all(b"\n").unwrap();
            ufs_count += 1;
        } else if let Some(caps) = BLOCK_RE.captures(line) {
            let block = Block {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                flags: caps["flags"].to_string(),
                action: caps["action"].to_string(),
                devmajor: caps["devmajor"].parse().unwrap(),
                devminor: caps["devminor"].parse().unwrap(),
                io_type: caps["io_type"].to_string(),
                extra: caps
                    .name("extra")
                    .map_or(0, |m| m.as_str().parse().unwrap()),
                // If sector is == 18446744073709551615 (u64 max value), set to 0
                sector: match caps["sector"].parse::<u64>() {
                    Ok(18446744073709551615) => 0,
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

            // Serialize to binary format and store
            serde_json::to_writer(&mut *block_writer, &block).unwrap();
            // Add record separator
            block_writer.write_all(b"\n").unwrap();
            block_count += 1;
        }
    }

    (ufs_count, block_count)
}

// Process chunks in parallel
fn process_chunk_parallel(chunk: Vec<String>) -> (Vec<UFS>, Vec<Block>) {
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();

    for line in &chunk {
        if let Some(caps) = UFS_RE.captures(line) {
            let raw_lba: u64 = caps["lba"].parse().unwrap();
            let raw_size: i64 = caps["size"].parse::<i64>().unwrap().unsigned_abs() as i64;

            // Convert bytes to 4KB units
            let lba_in_4kb = raw_lba / 4096;
            let size_in_4kb = (raw_size as f64 / 4096.0).ceil() as u32;

            let ufs = UFS {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                action: caps["command"].to_string(),
                tag: caps["tag"].parse().unwrap(),
                opcode: caps["opcode"].to_string(),
                lba: lba_in_4kb,
                size: size_in_4kb,
                groupid: u32::from_str_radix(&caps["group_id"], 16).unwrap(),
                hwqid: caps["hwq_id"].parse().unwrap(),
                qd: 0,
                dtoc: 0.0,
                ctoc: 0.0,
                ctod: 0.0,
                continuous: false,
            };
            ufs_traces.push(ufs);
        } else if let Some(caps) = BLOCK_RE.captures(line) {
            let block = Block {
                time: caps["time"].parse().unwrap(),
                process: caps["process"].to_string(),
                cpu: caps["cpu"].parse().unwrap(),
                flags: caps["flags"].to_string(),
                action: caps["action"].to_string(),
                devmajor: caps["devmajor"].parse().unwrap(),
                devminor: caps["devminor"].parse().unwrap(),
                io_type: caps["io_type"].to_string(),
                extra: caps
                    .name("extra")
                    .map_or(0, |m| m.as_str().parse().unwrap()),
                // If sector is 18446744073709551615 (u64 max), set to 0
                sector: match caps["sector"].parse::<u64>() {
                    Ok(18446744073709551615) => 0,
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

// Main log file parsing function
pub fn parse_log_file(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
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
fn parse_log_file_in_memory(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();

    // Chunk size for parallel processing
    const CHUNK_SIZE: usize = 100_000;
    let mut lines_chunk = Vec::with_capacity(CHUNK_SIZE);
    let mut total_lines = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        lines_chunk.push(line);

        if lines_chunk.len() >= CHUNK_SIZE {
            let chunks_to_process =
                std::mem::replace(&mut lines_chunk, Vec::with_capacity(CHUNK_SIZE));
            total_lines += chunks_to_process.len();

            // Parallel chunk processing
            let (mut chunk_ufs, mut chunk_blocks) = process_chunk_parallel(chunks_to_process);
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);

            println!("Processed lines: {}", total_lines);
        }
    }

    // Process remaining chunk
    if !lines_chunk.is_empty() {
        total_lines += lines_chunk.len();
        let (mut chunk_ufs, mut chunk_blocks) = process_chunk_parallel(lines_chunk);
        ufs_traces.append(&mut chunk_ufs);
        block_traces.append(&mut chunk_blocks);
    }

    println!("Total processed lines: {}", total_lines);
    println!(
        "Processed UFS events: {}, Block events: {}",
        ufs_traces.len(),
        block_traces.len()
    );

    Ok((ufs_traces, block_traces))
}

// Parse large log files with streaming (for large files)
fn parse_log_file_streaming(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>)> {
    // Create temporary files
    let (ufs_temp_file, ufs_temp_path) = create_temp_file("ufs")?;
    let (block_temp_file, block_temp_path) = create_temp_file("block")?;

    let start_time = Instant::now();
    let mut total_ufs = 0;
    let mut total_blocks = 0;

    // Create thread pool for parallel processing
    let num_threads = num_cpus::get();
    println!("Processing with {} threads", num_threads);

    // File line streaming processing
    {
        let file = File::open(filepath)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8MB buffer

        let mut ufs_writer = BufWriter::new(&ufs_temp_file);
        let mut block_writer = BufWriter::new(&block_temp_file);

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
                let (ufs_count, block_count) =
                    process_chunk(&chunks_to_process, &mut ufs_writer, &mut block_writer);
                total_ufs += ufs_count;
                total_blocks += block_count;

                // Report progress every 5 seconds
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "Processing... {} million lines (UFS: {}, Block: {}), Elapsed time: {:.2} seconds", 
                        processed_lines / 1_000_000,
                        total_ufs,
                        total_blocks,
                        start_time.elapsed().as_secs_f64()
                    );
                    last_report_time = now;
                }
            }
        }

        // Process remaining chunk
        if !lines_chunk.is_empty() {
            let remaining_lines = lines_chunk.len();
            let (ufs_count, block_count) =
                process_chunk(&lines_chunk, &mut ufs_writer, &mut block_writer);
            total_ufs += ufs_count;
            total_blocks += block_count;
            processed_lines += remaining_lines;

            // Final progress report
            println!(
                "Processing completed: Total {} million lines (UFS: {}, Block: {})",
                processed_lines / 1_000_000,
                total_ufs,
                total_blocks
            );
        }

        // Flush buffers
        ufs_writer.flush()?;
        block_writer.flush()?;
    }

    println!(
        "First pass completed: UFS={}, Block={}, Elapsed time: {:.2} seconds",
        total_ufs,
        total_blocks,
        start_time.elapsed().as_secs_f64()
    );

    // Load data from temporary files
    let mut ufs_traces = Vec::with_capacity(total_ufs);
    let mut block_traces = Vec::with_capacity(total_blocks);

    // Load UFS data
    {
        let file = File::open(&ufs_temp_path)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        for line_str in reader.lines().map_while(Result::ok) {
            if let Ok(ufs) = serde_json::from_str::<UFS>(&line_str) {
                ufs_traces.push(ufs);
            } else {
                println!("UFS deserialization error: {}", line_str);
            }
        }
    }

    // Load Block data
    {
        let file = File::open(&block_temp_path)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        for line_str in reader.lines().map_while(Result::ok) {
            if let Ok(block) = serde_json::from_str::<Block>(&line_str) {
                block_traces.push(block);
            } else {
                println!("Block deserialization error: {}", line_str);
            }
        }
    }

    // Remove temporary files
    let _ = fs::remove_file(ufs_temp_path);
    let _ = fs::remove_file(block_temp_path);

    println!(
        "Log file parsing completed: UFS={}, Block={}, Total time: {:.2} seconds",
        ufs_traces.len(),
        block_traces.len(),
        start_time.elapsed().as_secs_f64()
    );

    Ok((ufs_traces, block_traces))
}
