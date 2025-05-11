use crate::models::{Block, UFS, UFSCUSTOM};
use lazy_static::lazy_static;
use rand::random;
use regex::Regex;
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
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
        } 
        // Block IO 패턴 매칭
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
        // UFSCUSTOM 패턴 매칭
        else if let Some(caps) = UFSCUSTOM_RE.captures(line) {
            let opcode: String = caps["opcode"].to_string();
            let lba: u64 = caps["lba"].parse().unwrap();
            let size: u32 = caps["size"].parse().unwrap();
            let start_time: f64 = caps["start_time"].parse().unwrap();
            let end_time: f64 = caps["end_time"].parse().unwrap();
            
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
            
            // Serialize to binary format and store
            serde_json::to_writer(&mut *ufscustom_writer, &ufscustom).unwrap();
            // Add record separator
            ufscustom_writer.write_all(b"\n").unwrap();
            ufscustom_count += 1;
        }
    }

    (ufs_count, block_count, ufscustom_count)
}

// Process chunks in parallel and return UFS and Block I/O and UFSCUSTOM items
fn process_chunk_parallel(chunks: Vec<Vec<String>>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    let num_cores = num_cpus::get();
    // ThreadPool 생성 방식 변경
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_cores)
        .build()
        .expect("스레드 풀 생성 실패");
    let (sender, receiver) = mpsc::channel();

    // Process each chunk in a separate thread
    for chunk in chunks {
        let sender = sender.clone();
        pool.spawn(move || {
            let result = process_lines(chunk);
            sender.send(result).unwrap();
        });
    }
    drop(sender);

    // Collect results from all threads
    let mut ufs_items = Vec::new();
    let mut block_items = Vec::new();
    let mut ufscustom_items = Vec::new();

    for (ufs, block, ufscustom) in receiver.iter() {
        ufs_items.extend(ufs);
        block_items.extend(block);
        ufscustom_items.extend(ufscustom);
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
        let opcode: String = caps["opcode"].to_string();
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
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();

    // Chunk size for parallel processing
    const CHUNK_SIZE: usize = 100_000;
    let mut lines_chunk = Vec::with_capacity(CHUNK_SIZE);
    let mut total_lines = 0;

    for line_result in reader.lines() {
        let line = line_result?;
        lines_chunk.push(line);

        if lines_chunk.len() >= CHUNK_SIZE {
            let chunks_to_process = vec![
                std::mem::replace(&mut lines_chunk, Vec::with_capacity(CHUNK_SIZE))
            ];
            total_lines += chunks_to_process[0].len();

            // Parallel chunk processing
            let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) = process_chunk_parallel(chunks_to_process);
            ufs_traces.append(&mut chunk_ufs);
            block_traces.append(&mut chunk_blocks);
            ufscustom_traces.append(&mut chunk_ufscustom);

            println!("Processed lines: {}", total_lines);
        }
    }

    // Process remaining chunk
    if !lines_chunk.is_empty() {
        total_lines += lines_chunk.len();
        let (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) = process_chunk_parallel(vec![lines_chunk]);
        ufs_traces.append(&mut chunk_ufs);
        block_traces.append(&mut chunk_blocks);
        ufscustom_traces.append(&mut chunk_ufscustom);
    }

    println!("Total processed lines: {}", total_lines);
    println!(
        "Processed UFS events: {}, Block events: {}, UFSCUSTOM events: {}",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len()
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

    // Create thread pool for parallel processing
    let num_threads = num_cpus::get();
    println!("Processing with {} threads", num_threads);

    // File line streaming processing
    {
        let file = File::open(filepath)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file); // 8MB buffer

        let mut ufs_writer = BufWriter::new(&ufs_temp_file);
        let mut block_writer = BufWriter::new(&block_temp_file);
        let mut ufscustom_writer = BufWriter::new(&ufscustom_temp_file);

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
                let (ufs_count, block_count, ufscustom_count) =
                    process_chunk_streaming(&chunks_to_process, &mut ufs_writer, &mut block_writer, &mut ufscustom_writer);
                total_ufs += ufs_count;
                total_blocks += block_count;
                total_ufscustom += ufscustom_count;

                // Report progress every 5 seconds
                let now = Instant::now();
                if now.duration_since(last_report_time).as_secs() >= 5 {
                    println!(
                        "Processing... {} million lines (UFS: {}, Block: {}, UFSCUSTOM: {}), Elapsed time: {:.2} seconds", 
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
            let (ufs_count, block_count, ufscustom_count) =
                process_chunk_streaming(&lines_chunk, &mut ufs_writer, &mut block_writer, &mut ufscustom_writer);
            total_ufs += ufs_count;
            total_blocks += block_count;
            total_ufscustom += ufscustom_count;
            processed_lines += remaining_lines;

            // Final progress report
            println!(
                "Processing completed: Total {} million lines (UFS: {}, Block: {}, UFSCUSTOM: {})",
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
        "First pass completed: UFS={}, Block={}, UFSCUSTOM={}, Elapsed time: {:.2} seconds",
        total_ufs,
        total_blocks,
        total_ufscustom,
        start_time.elapsed().as_secs_f64()
    );

    // Load data from temporary files
    let mut ufs_traces = Vec::with_capacity(total_ufs);
    let mut block_traces = Vec::with_capacity(total_blocks);
    let mut ufscustom_traces = Vec::with_capacity(total_ufscustom);

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

    // Load UFSCUSTOM data
    {
        let file = File::open(&ufscustom_temp_path)?;
        let reader = BufReader::with_capacity(8 * 1024 * 1024, file);

        for line_str in reader.lines().map_while(Result::ok) {
            if let Ok(ufscustom) = serde_json::from_str::<UFSCUSTOM>(&line_str) {
                ufscustom_traces.push(ufscustom);
            } else {
                println!("UFSCUSTOM deserialization error: {}", line_str);
            }
        }
    }

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
    let parse_start_time = Instant::now();
    println!("시작: UFSCustom 로그 파싱 - {}", filepath);

    // 파일 열기
    let file = File::open(filepath)?;
    let reader = BufReader::new(file);
    let mut ufscustom_traces = Vec::new();
    
    // 처리 통계
    let mut total_lines = 0;
    let mut parsed_lines = 0;
    let mut skipped_lines = 0;
    let mut header_found = false;

    for line_result in reader.lines() {
        let line = line_result?;
        total_lines += 1;

        // 헤더 라인은 건너뜁니다
        if !header_found && line.starts_with("opcode,lba,size,start_time,end_time") {
            header_found = true;
            println!("헤더 발견: {}", line);
            continue;
        }

        // 주석이나 빈 라인은 건너뜁니다
        if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
            skipped_lines += 1;
            continue;
        }

        // 정규식으로 라인 파싱
        if let Some(caps) = UFSCUSTOM_RE.captures(&line) {
            let opcode: String = caps["opcode"].to_string();
            let lba: u64 = caps["lba"].parse().unwrap();
            let size: u32 = caps["size"].parse().unwrap();
            let start_time: f64 = caps["start_time"].parse().unwrap();
            let end_time: f64 = caps["end_time"].parse().unwrap();
            
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
            
            ufscustom_traces.push(ufscustom);
            parsed_lines += 1;

            // 처리 진행 상황 출력
            if parsed_lines % 100_000 == 0 {
                println!(
                    "처리 중: {} 라인 (성공: {}, 건너뜀: {})",
                    total_lines, parsed_lines, skipped_lines
                );
            }
        } else {
            // 파싱 실패한 라인은 스킵
            skipped_lines += 1;
            if skipped_lines <= 5 {
                println!("파싱 실패한 라인: {}", line);
            }
        }
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
    ufscustom_traces.sort_by(|a, b| a.dtoc.partial_cmp(&b.dtoc).unwrap_or(std::cmp::Ordering::Equal));
    
    // 기본 통계 출력
    if !ufscustom_traces.is_empty() {
        let min_dtoc = ufscustom_traces.first().unwrap().dtoc;
        let max_dtoc = ufscustom_traces.last().unwrap().dtoc;
        let avg_dtoc = ufscustom_traces.iter().map(|u| u.dtoc).sum::<f64>() / ufscustom_traces.len() as f64;
        
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
