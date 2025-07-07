// Common code for log parsing

use crate::models::{Block, UFS, UFSCUSTOM};
use lazy_static::lazy_static;
use regex::Regex;
use std::io;
use rand::random;
use std::fs::{self, OpenOptions, File};
use std::io::{Read, BufWriter, BufRead, Write};
use std::time::Instant;
use memmap2::{Mmap, MmapOptions};
use rayon::prelude::*;
use crate::utils::encoding::decode_bytes_auto;
use crate::utils::constants::{MAX_VALID_UFS_LBA, UFS_DEBUG_LBA};

// Common regex patterns for all three log types
lazy_static! {
    pub static ref UFS_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)").unwrap();    
    pub static ref BLOCK_RE: Regex = Regex::new(r"^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$").unwrap();
    pub static ref UFSCUSTOM_RE: Regex = Regex::new(r"^(?P<opcode>0x[0-9a-f]+),(?P<lba>\d+),(?P<size>\d+),(?P<start_time>\d+(?:\.\d+)?),(?P<end_time>\d+(?:\.\d+)?)$").unwrap();
    
    // Pre-compiled regex for performance optimizations
    static ref UFS_QUICK_CHECK: Regex = Regex::new(r"ufshcd_command:").unwrap();
    static ref BLOCK_QUICK_CHECK: Regex = Regex::new(r"(blk_|block_)").unwrap();
    static ref UFSCUSTOM_QUICK_CHECK: Regex = Regex::new(r"^0x[0-9a-f]+,\d+,\d+,").unwrap();
}

// Create temporary file with given prefix
pub fn create_temp_file(prefix: &str) -> io::Result<(File, String)> {
    let start_time = Instant::now();
    let temp_path = format!("/tmp/{}_{}.tmp", prefix, random::<u64>());
    
    match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path) 
    {
        Ok(file) => {
            println!("Created temporary file: {}, time: {:.3} ms", 
                temp_path, start_time.elapsed().as_millis());
            Ok((file, temp_path))
        },
        Err(e) => {
            println!("Failed to create temporary file: {}, error: {}", temp_path, e);
            Err(e)
        }
    }
}

// Parse UFS event from a line
pub fn parse_ufs_event(line: &str) -> Result<UFS, &'static str> {
    if let Some(caps) = UFS_RE.captures(line) {
        let raw_lba: u64 = caps["lba"].parse().unwrap_or(0);
        let raw_size: i64 = caps["size"].parse::<i64>().unwrap_or(0).unsigned_abs() as i64;

        // Debug 또는 비정상적으로 큰 LBA 값은 0으로 처리
        let cleaned_lba = if raw_lba == UFS_DEBUG_LBA || raw_lba > MAX_VALID_UFS_LBA {
            0
        } else {
            raw_lba
        };

        // Convert bytes to 4KB units
        let lba_in_4kb = cleaned_lba / 4096;
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
        Err("Line does not match UFS pattern")
    }
}

// Parse Block IO event from a line
pub fn parse_block_io_event(line: &str) -> Result<Block, &'static str> {
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
        Err("Line does not match Block IO pattern")
    }
}

// Parse UFSCUSTOM event from a line
pub fn parse_ufscustom_event(line: &str) -> Result<UFSCUSTOM, &'static str> {
    // Skip header line
    if line.starts_with("opcode,lba,size,start_time,end_time") {
        return Err("Header line");
    }

    // Skip comments or empty lines
    if line.trim().is_empty() || line.starts_with("//") || line.starts_with('#') {
        return Err("Comment or empty line");
    }

    if let Some(caps) = UFSCUSTOM_RE.captures(line) {
        // Use string references to reduce copying
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
        };

        Ok(ufscustom)
    } else {
        Err("Line does not match UFSCUSTOM pattern")
    }
}

// Process a line and return parsed data structures
pub fn process_line(line: &str) -> Option<(Option<UFS>, Option<Block>, Option<UFSCUSTOM>)> {
    // Try to match UFS pattern first
    if let Ok(ufs) = parse_ufs_event(line) {
        return Some((Some(ufs), None, None));
    }
    // Then try Block IO pattern
    else if let Ok(block) = parse_block_io_event(line) {
        return Some((None, Some(block), None));
    }
    // Finally try UFSCUSTOM pattern
    else if let Ok(ufscustom) = parse_ufscustom_event(line) {
        return Some((None, None, Some(ufscustom)));
    }

    // Return None if no pattern matches
    None
}

// Fast pattern matching for line categorization
pub fn categorize_line_fast(line: &str) -> LineCategory {
    if line.is_empty() {
        return LineCategory::Empty;
    }
    
    // Quick checks before expensive regex matching
    if UFS_QUICK_CHECK.is_match(line) {
        return LineCategory::UFS;
    }
    
    if BLOCK_QUICK_CHECK.is_match(line) {
        return LineCategory::Block;
    }
    
    if UFSCUSTOM_QUICK_CHECK.is_match(line) {
        return LineCategory::UFSCustom;
    }
    
    LineCategory::Unknown
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineCategory {
    UFS,
    Block,
    UFSCustom,
    Empty,
    Unknown,
}

// Optimized line processing that returns all possible matches
pub fn process_line_optimized(line: &str) -> Option<(Option<UFS>, Option<Block>, Option<UFSCUSTOM>)> {
    let category = categorize_line_fast(line);
    
    match category {
        LineCategory::UFS => {
            if let Ok(ufs) = parse_ufs_event(line) {
                Some((Some(ufs), None, None))
            } else {
                None
            }
        }
        LineCategory::Block => {
            if let Ok(block) = parse_block_io_event(line) {
                Some((None, Some(block), None))
            } else {
                None
            }
        }
        LineCategory::UFSCustom => {
            if let Ok(ufscustom) = parse_ufscustom_event(line) {
                Some((None, None, Some(ufscustom)))
            } else {
                None
            }
        }
        _ => None,
    }
}

// Common deserialization functions

/// Deserialize UFS items from a binary reader
pub fn deserialize_ufs_items<R: Read>(reader: &mut R) -> io::Result<Vec<UFS>> {
    let mut ufs_items = Vec::new();
    let start_time = Instant::now();
    let config = bincode::config::standard();
    
    // Read all bytes into buffer
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    
    let mut reader_slice = &buffer[..];
    let mut count = 0;
    
    // Deserialize while we have data remaining
    while !reader_slice.is_empty() {
        match bincode::decode_from_slice::<UFS, _>(reader_slice, config) {
            Ok((ufs, size)) => {
                ufs_items.push(ufs);
                reader_slice = &reader_slice[size..];
                count += 1;
                
                // Progress reporting
                if count % 1_000_000 == 0 {
                    println!("Loaded {} million UFS items", count / 1_000_000);
                }
            }
            Err(e) => {
                eprintln!("UFS deserialization error at item {}: {:?}", count, e);
                // Try to skip this corrupted item and continue with the next one
                if reader_slice.len() > 8 {  // Skip at least 8 bytes to try to find next valid item
                    reader_slice = &reader_slice[8..];
                } else {
                    break; // Not enough data remaining, exit
                }
            }
        }
    }
    
    println!("Deserialized {} UFS items in {:.2} seconds", 
             count, start_time.elapsed().as_secs_f64());
    Ok(ufs_items)
}

/// Deserialize Block items from a binary reader
pub fn deserialize_block_items<R: Read>(reader: &mut R) -> io::Result<Vec<Block>> {
    let mut block_items = Vec::new();
    let start_time = Instant::now();
    let config = bincode::config::standard();
    
    // Read all bytes into buffer
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    
    let mut reader_slice = &buffer[..];
    let mut count = 0;
    
    // Deserialize while we have data remaining
    while !reader_slice.is_empty() {
        match bincode::decode_from_slice::<Block, _>(reader_slice, config) {
            Ok((block, size)) => {
                block_items.push(block);
                reader_slice = &reader_slice[size..];
                count += 1;
                
                // Progress reporting
                if count % 1_000_000 == 0 {
                    println!("Loaded {} million Block items", count / 1_000_000);
                }
            }
            Err(e) => {
                eprintln!("Block deserialization error at item {}: {:?}", count, e);
                // Try to skip this corrupted item and continue with the next one
                if reader_slice.len() > 8 {
                    reader_slice = &reader_slice[8..];
                } else {
                    break; // Not enough data remaining, exit
                }
            }
        }
    }
    
    println!("Deserialized {} Block items in {:.2} seconds", 
             count, start_time.elapsed().as_secs_f64());
    Ok(block_items)
}

/// Deserialize UFSCUSTOM items from a binary reader
pub fn deserialize_ufscustom_items<R: Read>(reader: &mut R) -> io::Result<Vec<UFSCUSTOM>> {
    let mut ufscustom_items = Vec::new();
    let start_time = Instant::now();
    let config = bincode::config::standard();
    
    // Read all bytes into buffer
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    
    let mut reader_slice = &buffer[..];
    let mut count = 0;
    
    // Deserialize while we have data remaining
    while !reader_slice.is_empty() {
        match bincode::decode_from_slice::<UFSCUSTOM, _>(reader_slice, config) {
            Ok((ufscustom, size)) => {
                ufscustom_items.push(ufscustom);
                reader_slice = &reader_slice[size..];
                count += 1;
                
                // Progress reporting
                if count % 1_000_000 == 0 {
                    println!("Loaded {} million UFSCUSTOM items", count / 1_000_000);
                }
            }
            Err(e) => {
                eprintln!("UFSCUSTOM deserialization error at item {}: {:?}", count, e);
                // Try to skip this corrupted item and continue with the next one
                if reader_slice.len() > 8 {
                    reader_slice = &reader_slice[8..];
                } else {
                    break; // Not enough data remaining, exit
                }
            }
        }
    }
    
    println!("Deserialized {} UFSCUSTOM items in {:.2} seconds", 
             count, start_time.elapsed().as_secs_f64());
    Ok(ufscustom_items)
}

/// Serialize items to a binary file using bincode
pub fn serialize_to_bincode<T: bincode::Encode>(
    items: &[T], 
    file_path: &str
) -> io::Result<()> {
    let start_time = Instant::now();
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;
        
    let mut writer = BufWriter::new(file);
    let config = bincode::config::standard();
    
    println!("Serializing {} items to {}", items.len(), file_path);
    
    for item in items {
        bincode::encode_into_std_write(item, &mut writer, config)
            .map_err(|e| io::Error::new(
                io::ErrorKind::Other, 
                format!("Bincode serialization error: {}", e)
            ))?;
    }
    
    writer.flush()?;
    
    println!("Serialized {} items in {:.2} seconds", 
        items.len(), start_time.elapsed().as_secs_f64());
    
    Ok(())
}

/// Check if a file exists and is readable
pub fn check_file_readable(file_path: &str) -> io::Result<u64> {
    let metadata = fs::metadata(file_path)?;
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Path is not a file: {}", file_path),
        ));
    }
    
    // Try opening the file to verify read access
    let file = File::open(file_path)?;
    let file_size = file.metadata()?.len();
    
    Ok(file_size)
}

/// Get optimal buffer size based on file size
pub fn get_optimal_buffer_size(file_size: u64) -> usize {
    // For very large files (>1GB), use a larger buffer
    if file_size > 1_073_741_824 {
        32 * 1024 * 1024  // 32MB
    } else if file_size > 100_000_000 {
        8 * 1024 * 1024   // 8MB
    } else {
        1024 * 1024   // 1MB
    }
}

/// Try to memory map a file and return the mapping if successful
pub fn try_memory_map(file: &File) -> io::Result<Mmap> {
    let start_time = Instant::now();
    let result = unsafe { MmapOptions::new().map(file) };
    
    match &result {
        Ok(mmap) => {
            println!("Memory mapping successful: {:.2} MB mapped in {:.2} seconds", 
                mmap.len() as f64 / 1_048_576.0, // Convert bytes to MB
                start_time.elapsed().as_secs_f64());
        },
        Err(e) => {
            println!("Memory mapping failed: {}", e);
        }
    }
    
    result
}

/// Split memory mapped file into lines and process them
pub fn process_memory_mapped_file<F>(
    mmap: &Mmap,
    processor: F,
    chunk_size: usize
) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)>
where
    F: Fn(Vec<String>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) + Send + Sync,
{
    let start_time = Instant::now();
    
    // Convert memory mapped file to string using lossy conversion
    // Convert memory mapped file to string using automatic encoding detection
    let content = decode_bytes_auto(&mmap[..]);

    // Split into lines
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    println!("File loaded: {} lines, preparing for parallel processing", total_lines);

    // Divide into chunks for parallel processing
    let chunks: Vec<_> = lines
        .par_chunks(chunk_size)
        .map(|chunk| {
            chunk
                .iter()
                .map(|&s| s.to_string())
                .collect::<Vec<String>>()
        })
        .collect();

    let chunk_count = chunks.len();
    println!("Split into {} chunks of approximately {} lines each", chunk_count, chunk_size);

    // Process chunks in parallel
    let results: Vec<_> = chunks.into_par_iter().map(&processor).collect();
    println!("Parallel processing completed in {:.2} seconds", start_time.elapsed().as_secs_f64());

    // Collect results
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();

    let result_start = Instant::now();
    for (mut chunk_ufs, mut chunk_blocks, mut chunk_ufscustom) in results {
        ufs_traces.append(&mut chunk_ufs);
        block_traces.append(&mut chunk_blocks);
        ufscustom_traces.append(&mut chunk_ufscustom);
    }
    println!("Results collected: {} UFS items, {} Block items, {} UFSCUSTOM items in {:.2} seconds",
        ufs_traces.len(), block_traces.len(), ufscustom_traces.len(),
        result_start.elapsed().as_secs_f64());

    Ok((ufs_traces, block_traces, ufscustom_traces))
}

/// Read a single line from the provided reader and return it as a UTF-8 `String`.
/// Any invalid UTF-8 sequences will be replaced with the Unicode replacement
/// character. Returns `Ok(None)` when EOF is reached.
pub fn read_line_lossy<R: BufRead>(reader: &mut R, buffer: &mut Vec<u8>) -> io::Result<Option<String>> {
    buffer.clear();
    let bytes_read = reader.read_until(b'\n', buffer)?;
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

/// Report processing progress
pub fn report_progress(total_items: usize, start_time: Instant, task_name: &str) {
    let elapsed = start_time.elapsed().as_secs_f64();
    let items_per_sec = if elapsed > 0.0 { total_items as f64 / elapsed } else { 0.0 };
    
    println!("{} progress: {} items processed in {:.2} seconds ({:.0} items/sec)",
        task_name, total_items, elapsed, items_per_sec);
}

/// Process chunks with progress reporting
pub fn process_chunks_with_progress<F>(
    chunks: Vec<Vec<String>>, 
    processor: F,
    task_name: &str
) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)
where
    F: Fn(Vec<String>) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) + Send + Sync,
{
    let start_time = Instant::now();
    let total_chunks = chunks.len();
    
    // Process in parallel
    let results: Vec<_> = chunks.into_par_iter().map(&processor).collect();
    
    // Collect results
    let mut ufs_items = Vec::new();
    let mut block_items = Vec::new();
    let mut ufscustom_items = Vec::new();

    for (mut ufs, mut block, mut ufscustom) in results {
        ufs_items.append(&mut ufs);
        block_items.append(&mut block);
        ufscustom_items.append(&mut ufscustom);
    }

    println!("{} completed: processed {} chunks in {:.2} seconds",
        task_name, total_chunks, start_time.elapsed().as_secs_f64());
    println!("Results: {} UFS items, {} Block items, {} UFSCUSTOM items",
        ufs_items.len(), block_items.len(), ufscustom_items.len());

    (ufs_items, block_items, ufscustom_items)
}

/// Parse a single log line and return trace type and parsed string
pub fn parse_log_line(line: &str) -> Option<(crate::TraceType, String)> {
    // UFSCustom format 체크 (가장 구체적인 형태부터)
    if UFSCUSTOM_QUICK_CHECK.is_match(line) {
        return Some((crate::TraceType::UFSCUSTOM, line.to_string()));
    }
    
    // UFS format 체크
    if UFS_QUICK_CHECK.is_match(line) {
        return Some((crate::TraceType::UFS, line.to_string()));
    }
    
    // Block format 체크
    if BLOCK_QUICK_CHECK.is_match(line) {
        return Some((crate::TraceType::Block, line.to_string()));
    }
    
    None
}
