// High-performance memory-mapped log parser implementation

use crate::models::{Block, UFS, UFSCUSTOM};
use crate::parsers::log_common::process_line_optimized;
use crate::utils::{PerformanceMetrics, PerformanceProfiler, MemoryMonitor, calculate_optimal_chunk_size};
use memmap2::MmapOptions;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::sync::Arc;
use std::time::Instant;

// SIMD-optimized line splitting function
fn find_line_boundaries(data: &[u8]) -> Vec<usize> {
    let mut boundaries = Vec::new();
    boundaries.push(0);
    
    // Use SIMD-like approach for finding newlines
    let mut i = 0;
    while i < data.len() {
        // Process 64 bytes at a time for better cache performance
        let end = std::cmp::min(i + 64, data.len());
        let chunk = &data[i..end];
        
        for (offset, &byte) in chunk.iter().enumerate() {
            if byte == b'\n' {
                boundaries.push(i + offset + 1);
            }
        }
        i = end;
    }
    
    boundaries
}

// Zero-copy line processing
fn process_line_zero_copy(line: &[u8]) -> Option<(Option<UFS>, Option<Block>, Option<UFSCUSTOM>)> {
    // Convert to string only if necessary
    let line_str = match std::str::from_utf8(line) {
        Ok(s) => s.trim(),
        Err(_) => return None,
    };
    
    if line_str.is_empty() {
        return None;
    }
    
    // Use optimized processing from log_common
    process_line_optimized(line_str)
}

// High-performance chunk processing
fn process_chunk(data: &[u8], start: usize, end: usize) -> (Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>) {
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();
    
    // Find line boundaries in this chunk
    let chunk_data = &data[start..end];
    let boundaries = find_line_boundaries(chunk_data);
    
    // Process each line
    for window in boundaries.windows(2) {
        let line_start = window[0];
        let line_end = window[1].saturating_sub(1); // Remove newline
        
        if line_start < line_end && line_end <= chunk_data.len() {
            let line = &chunk_data[line_start..line_end];
            
            if let Some((maybe_ufs, maybe_block, maybe_ufscustom)) = process_line_zero_copy(line) {
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
        }
    }
    
    (ufs_traces, block_traces, ufscustom_traces)
}

// Main high-performance parsing function
pub fn parse_log_file_high_perf(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();
    println!("Starting high-performance log parsing: {}", filepath);
    
    // Initialize performance monitoring
    let mut profiler = PerformanceProfiler::new();
    let memory_monitor = Arc::new(MemoryMonitor::new());
    let mut metrics = PerformanceMetrics::new();
    
    // Print system memory info
    if let Some(sys_info) = MemoryMonitor::get_system_memory_info() {
        sys_info.print_info();
    }
    
    // Open file and get metadata
    let file = File::open(filepath)?;
    let file_size = file.metadata()?.len();
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
    println!("File size: {:.2} MB ({:.2} GB)", file_size_mb, file_size_mb / 1024.0);
    
    profiler.checkpoint("File opened");
    
    // Calculate optimal chunk size based on available memory
    let chunk_size = if let Some(sys_info) = MemoryMonitor::get_system_memory_info() {
        let available_mb = sys_info.free_mb as usize;
        let optimal_size = calculate_optimal_chunk_size(file_size as usize, available_mb);
        println!("Using adaptive chunk size: {:.2} MB", optimal_size as f64 / (1024.0 * 1024.0));
        optimal_size
    } else {
        // Default fallback
        std::cmp::min(file_size as usize, 50 * 1024 * 1024) // 50MB default
    };
    
    // Memory-map the entire file
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    let data = Arc::new(mmap);
    
    // Record memory mapping
    memory_monitor.record_allocation(file_size as usize);
    profiler.checkpoint("File memory-mapped");
    
    println!("File memory-mapped successfully, starting parallel processing...");
    
    // Determine optimal chunk size based on file size and CPU count
    let cpu_count = num_cpus::get();
    let optimal_chunk_size = std::cmp::max(chunk_size as u64, file_size / (cpu_count as u64 * 4));
    let final_chunk_size = std::cmp::max(optimal_chunk_size, 64 * 1024 * 1024); // Min 64MB chunks
    
    println!("Using {} CPU cores with chunk size: {:.2} MB", cpu_count, final_chunk_size as f64 / 1_048_576.0);
    
    // Find chunk boundaries that don't break lines
    let mut chunk_boundaries = Vec::new();
    let mut pos = 0;
    
    while pos < file_size {
        let next_pos = std::cmp::min(pos + final_chunk_size, file_size);
        
        // Find the next line boundary to avoid breaking lines
        let mut boundary = next_pos;
        if boundary < file_size {
            while boundary < file_size && data[boundary as usize] != b'\n' {
                boundary += 1;
            }
            if boundary < file_size {
                boundary += 1; // Include the newline
            }
        }
        
        chunk_boundaries.push((pos, boundary));
        pos = boundary;
    }
    
    println!("Processing {} chunks in parallel...", chunk_boundaries.len());
    
    // Process chunks in parallel
    let results: Vec<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> = chunk_boundaries
        .par_iter()
        .enumerate()
        .map(|(i, &(start, end))| {
            let chunk_start_time = Instant::now();
            let result = process_chunk(&data, start as usize, end as usize);
            
            // Progress reporting
            if i % 10 == 0 {
                let progress = (end as f64 / file_size as f64) * 100.0;
                println!(
                    "Chunk {}: {:.1}% complete, {} UFS + {} Block + {} UFSCUSTOM items, time: {:.2}s",
                    i, progress, result.0.len(), result.1.len(), result.2.len(),
                    chunk_start_time.elapsed().as_secs_f64()
                );
            }
            
            result
        })
        .collect();
    
    // Merge results
    let merge_start_time = Instant::now();
    println!("Merging results from {} chunks...", results.len());
    
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();
    
    // Pre-allocate capacity based on estimated total
    let total_estimate = results.iter().map(|r| r.0.len() + r.1.len() + r.2.len()).sum::<usize>();
    ufs_traces.reserve(total_estimate / 3);
    block_traces.reserve(total_estimate / 3);
    ufscustom_traces.reserve(total_estimate / 3);
    
    for (ufs, block, ufscustom) in results {
        ufs_traces.extend(ufs);
        block_traces.extend(block);
        ufscustom_traces.extend(ufscustom);
    }
    
    println!(
        "Merge completed in {:.2}s",
        merge_start_time.elapsed().as_secs_f64()
    );
    
    profiler.checkpoint("Results merged");
    
    // Sort traces by time to ensure proper ordering for QD calculation
    println!("Sorting traces by time for accurate QD calculation...");
    let sort_start = Instant::now();
    
    // 하이퍼프 모드에서도 안정적 정렬 적용 (stable sort)
    ufs_traces.sort_by(|a, b| match a.time.partial_cmp(&b.time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일할 경우:
            // 1. complete_rsp가 send_req보다 우선 (QD를 먼저 낮춤)
            if a.action == "complete_rsp" && b.action == "send_req" {
                std::cmp::Ordering::Less
            } else if a.action == "send_req" && b.action == "complete_rsp" {
                std::cmp::Ordering::Greater
            } else {
                // 동일 액션인 경우 태그로 정렬
                a.tag.cmp(&b.tag)
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    block_traces.sort_by(|a, b| match a.time.partial_cmp(&b.time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일하면 섹터(sector)와 size로 정렬
            match a.sector.cmp(&b.sector) {
                std::cmp::Ordering::Equal => a.size.cmp(&b.size),
                ordering => ordering
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    ufscustom_traces.sort_by(|a, b| match a.start_time.partial_cmp(&b.start_time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일하면 LBA와 size로 정렬
            match a.lba.cmp(&b.lba) {
                std::cmp::Ordering::Equal => a.size.cmp(&b.size),
                ordering => ordering
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    println!("Sorting completed in {:.2}s", sort_start.elapsed().as_secs_f64());
    profiler.checkpoint("Traces sorted by time");
    
    // Update performance metrics
    let parse_time = profiler.get_total_time();
    let total_lines = chunk_boundaries.len() * 1000; // 추정값
    let processed_lines = ufs_traces.len() + block_traces.len() + ufscustom_traces.len();
    
    metrics.total_time = parse_time;
    metrics.parse_time = parse_time;
    metrics.total_lines = total_lines;
    metrics.processed_lines = processed_lines;
    metrics.peak_memory_mb = memory_monitor.get_peak_mb();
    metrics.calculate_derived_metrics(file_size_mb);
    
    // Print performance summary
    profiler.print_profile();
    metrics.print_summary();
    
    // Cleanup memory tracking
    memory_monitor.record_deallocation(file_size as usize);
    
    // Calculate block latency (Q->C mapping to dtoc) for block events
    if !block_traces.is_empty() {
        println!("Calculating Dispatch-to-Complete (dtoc) latency for {} block events...", block_traces.len());
        let dtoc_start = Instant::now();
        crate::parsers::log_common::calculate_block_latency_advanced(&mut block_traces);
        println!("dtoc calculation completed in {:.2}s", dtoc_start.elapsed().as_secs_f64());
        profiler.checkpoint("dtoc latency calculated");
    }
    
    println!(
        "High-performance parsing completed: {} UFS, {} Block, {} UFSCUSTOM items in {:.2}s",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );
    
    Ok((ufs_traces, block_traces, ufscustom_traces))
}

// Alternative implementation using streaming with better memory management
pub fn parse_log_file_streaming(filepath: &str) -> io::Result<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> {
    let start_time = Instant::now();
    println!("Starting streaming log parsing: {}", filepath);
    
    // Initialize performance monitoring
    let mut profiler = PerformanceProfiler::new();
    let memory_monitor = Arc::new(MemoryMonitor::new());
    let mut metrics = PerformanceMetrics::new();
    
    // Print system memory info
    if let Some(sys_info) = MemoryMonitor::get_system_memory_info() {
        sys_info.print_info();
    }
    
    let file = File::open(filepath)?;
    let file_size = file.metadata()?.len();
    let file_size_mb = file_size as f64 / (1024.0 * 1024.0);
    println!("File size: {:.2} MB ({:.2} GB)", file_size_mb, file_size_mb / 1024.0);
    
    profiler.checkpoint("File opened");
    
    // Memory-map the file
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    memory_monitor.record_allocation(file_size as usize);
    profiler.checkpoint("File memory-mapped");
    
    // Use adaptive buffer size based on available memory
    let buffer_size = if let Some(sys_info) = MemoryMonitor::get_system_memory_info() {
        let available_mb = sys_info.free_mb as usize;
        let suggested_buffer = std::cmp::min(available_mb / 4, 256) * 1024 * 1024; // 최대 256MB
        std::cmp::max(suggested_buffer, 64 * 1024 * 1024) // 최소 64MB
    } else {
        128 * 1024 * 1024 // Default 128MB
    };
    
    println!("Using streaming buffer size: {:.2} MB", buffer_size as f64 / (1024.0 * 1024.0));
    
    let cpu_count = num_cpus::get();
    println!("Using {} CPU cores for parallel processing", cpu_count);
    
    let mut ufs_traces = Vec::new();
    let mut block_traces = Vec::new();
    let mut ufscustom_traces = Vec::new();
    
    let mut processed_bytes = 0;
    let mut last_report_time = Instant::now();
    
    // Process file in streaming fashion
    while processed_bytes < file_size {
        let chunk_start = processed_bytes as usize;
        let chunk_end = std::cmp::min(processed_bytes + buffer_size as u64, file_size) as usize;
        
        // Extend to line boundary
        let mut actual_end = chunk_end;
        if actual_end < mmap.len() {
            while actual_end < mmap.len() && mmap[actual_end] != b'\n' {
                actual_end += 1;
            }
            if actual_end < mmap.len() {
                actual_end += 1; // Include newline
            }
        }
        
        let chunk_data = &mmap[chunk_start..actual_end];
        
        // Process this chunk in parallel sub-chunks
        let sub_chunk_size = chunk_data.len() / cpu_count;
        let sub_results: Vec<(Vec<UFS>, Vec<Block>, Vec<UFSCUSTOM>)> = (0..cpu_count)
            .into_par_iter()
            .map(|i| {
                let mut start = i * sub_chunk_size;
                
                // 첫 번째 청크가 아닌 경우, 시작 지점도 라인 경계에 맞춤
                if i > 0 && start < chunk_data.len() {
                    // 이전 라인의 끝을 찾아 그 다음부터 시작
                    let mut boundary = start;
                    while boundary > 0 && chunk_data[boundary - 1] != b'\n' {
                        boundary -= 1;
                    }
                    start = boundary;
                }
                
                let end = if i == cpu_count - 1 {
                    chunk_data.len()
                } else {
                    // Find next line boundary
                    let mut boundary = (i + 1) * sub_chunk_size;
                    while boundary < chunk_data.len() && chunk_data[boundary] != b'\n' {
                        boundary += 1;
                    }
                    if boundary < chunk_data.len() {
                        boundary += 1;
                    }
                    boundary
                };
                
                if start < end {
                    process_chunk(chunk_data, start, end)
                } else {
                    (Vec::new(), Vec::new(), Vec::new())
                }
            })
            .collect();
        
        // Merge sub-results
        for (ufs, block, ufscustom) in sub_results {
            ufs_traces.extend(ufs);
            block_traces.extend(block);
            ufscustom_traces.extend(ufscustom);
        }
        
        processed_bytes = actual_end as u64;
        
        // Progress reporting
        let now = Instant::now();
        if now.duration_since(last_report_time).as_secs() >= 5 {
            let progress = (processed_bytes as f64 / file_size as f64) * 100.0;
            println!(
                "Streaming progress: {:.1}% ({:.2} GB / {:.2} GB), items: {} UFS + {} Block + {} UFSCUSTOM",
                progress,
                processed_bytes as f64 / 1_073_741_824.0,
                file_size as f64 / 1_073_741_824.0,
                ufs_traces.len(),
                block_traces.len(),
                ufscustom_traces.len()
            );
            last_report_time = now;
        }
    }
    
    profiler.checkpoint("Streaming parsing completed");
    
    // 중복 이벤트 제거 (청크 간 경계에서 발생할 수 있는 중복)
    println!("Removing duplicate events...");
    
    // UFS 중복 제거 - f64를 문자열로 변환하여 hashable 하게 만듦
    let ufs_dedup_start = Instant::now();
    let mut ufs_unique = Vec::with_capacity(ufs_traces.len());
    let mut ufs_dedup_map: HashSet<(String, u32, String, String)> = HashSet::new();
    
    for ufs in ufs_traces.iter() {
        let time_str = format!("{:.6}", ufs.time); // 소수점 6자리까지 정밀도 유지
        let key = (time_str, ufs.tag, ufs.action.clone(), ufs.opcode.clone());
        if ufs_dedup_map.insert(key) {
            ufs_unique.push(ufs.clone());
        }
    }
    
    let ufs_duplicates = ufs_traces.len() - ufs_unique.len();
    ufs_traces = ufs_unique;
    
    println!(
        "UFS duplicate events removed: {} (in {:.2}s)",
        ufs_duplicates,
        ufs_dedup_start.elapsed().as_secs_f64()
    );
    
    // Block 중복 제거
    let block_dedup_start = Instant::now();
    let mut block_unique = Vec::with_capacity(block_traces.len());
    let mut block_dedup_map: HashSet<(String, u64, u32, String)> = HashSet::new();
    
    for block in block_traces.iter() {
        let time_str = format!("{:.6}", block.time);
        let key = (time_str, block.sector, block.size, block.io_type.clone());
        if block_dedup_map.insert(key) {
            block_unique.push(block.clone());
        }
    }
    
    let block_duplicates = block_traces.len() - block_unique.len();
    block_traces = block_unique;
    
    println!(
        "Block duplicate events removed: {} (in {:.2}s)",
        block_duplicates,
        block_dedup_start.elapsed().as_secs_f64()
    );
    
    // UFSCustom 중복 제거
    if !ufscustom_traces.is_empty() {
        let ufscustom_dedup_start = Instant::now();
        let mut ufscustom_unique = Vec::with_capacity(ufscustom_traces.len());
        let mut ufscustom_dedup_map: HashSet<(String, u64, u32)> = HashSet::new();
        
        for ufsc in ufscustom_traces.iter() {
            let time_str = format!("{:.6}", ufsc.start_time);
            let key = (time_str, ufsc.lba, ufsc.size);
            if ufscustom_dedup_map.insert(key) {
                ufscustom_unique.push(ufsc.clone());
            }
        }
        
        let ufscustom_duplicates = ufscustom_traces.len() - ufscustom_unique.len();
        ufscustom_traces = ufscustom_unique;
        
        println!(
            "UFSCustom duplicate events removed: {} (in {:.2}s)",
            ufscustom_duplicates,
            ufscustom_dedup_start.elapsed().as_secs_f64()
        );
    }
    
    profiler.checkpoint("Duplicate events removed");
    
    // Sort traces by time to ensure proper ordering for QD calculation
    println!("Sorting traces by time for accurate QD calculation...");
    let sort_start = Instant::now();
    
    // 스트리밍 모드에서 안정적 정렬 적용 (stable sort)
    ufs_traces.sort_by(|a, b| match a.time.partial_cmp(&b.time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일할 경우:
            // 1. complete_rsp가 send_req보다 우선 (QD를 먼저 낮춤)
            if a.action == "complete_rsp" && b.action == "send_req" {
                std::cmp::Ordering::Less
            } else if a.action == "send_req" && b.action == "complete_rsp" {
                std::cmp::Ordering::Greater
            } else {
                // 동일 액션인 경우 태그로 정렬
                a.tag.cmp(&b.tag)
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    block_traces.sort_by(|a, b| match a.time.partial_cmp(&b.time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일하면 섹터(sector)와 size로 정렬
            match a.sector.cmp(&b.sector) {
                std::cmp::Ordering::Equal => a.size.cmp(&b.size),
                ordering => ordering
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    ufscustom_traces.sort_by(|a, b| match a.start_time.partial_cmp(&b.start_time) {
        Some(std::cmp::Ordering::Equal) => {
            // 타임스탬프가 동일하면 LBA와 size로 정렬
            match a.lba.cmp(&b.lba) {
                std::cmp::Ordering::Equal => a.size.cmp(&b.size),
                ordering => ordering
            }
        },
        Some(ordering) => ordering,
        None => std::cmp::Ordering::Equal,
    });
    
    println!("Sorting completed in {:.2}s", sort_start.elapsed().as_secs_f64());
    profiler.checkpoint("Traces sorted by time");
    
    // Update performance metrics
    let parse_time = profiler.get_total_time();
    let total_lines = (file_size / 100) as usize; // 추정값 (평균 100바이트 per line)
    let processed_lines = ufs_traces.len() + block_traces.len() + ufscustom_traces.len();
    
    metrics.total_time = parse_time;
    metrics.parse_time = parse_time;
    metrics.total_lines = total_lines;
    metrics.processed_lines = processed_lines;
    metrics.peak_memory_mb = memory_monitor.get_peak_mb();
    metrics.calculate_derived_metrics(file_size_mb);
    
    // Print performance summary
    profiler.print_profile();
    metrics.print_summary();
    
    // Cleanup memory tracking
    memory_monitor.record_deallocation(file_size as usize);
    
    println!(
        "Streaming parsing completed: {} UFS, {} Block, {} UFSCUSTOM items in {:.2}s",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len(),
        start_time.elapsed().as_secs_f64()
    );
    
    // Calculate block latency (Q->C mapping to dtoc) for block events
    if !block_traces.is_empty() {
        println!("Calculating Dispatch-to-Complete (dtoc) latency for {} block events...", block_traces.len());
        let dtoc_start = Instant::now();
        crate::parsers::log_common::calculate_block_latency_advanced(&mut block_traces);
        println!("dtoc calculation completed in {:.2}s", dtoc_start.elapsed().as_secs_f64());
    }
    
    Ok((ufs_traces, block_traces, ufscustom_traces))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_find_line_boundaries() {
        let data = b"line1\nline2\nline3\n";
        let boundaries = find_line_boundaries(data);
        assert_eq!(boundaries, vec![0, 6, 12, 18]);
    }
    
    #[test]
    fn test_process_line_zero_copy() {
        let line = b"some test line";
        let result = process_line_zero_copy(line);
        assert!(result.is_none());
    }
}
