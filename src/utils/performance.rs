use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// 성능 모니터링을 위한 구조체
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_time: Duration,
    pub parse_time: Duration,
    pub process_time: Duration,
    pub total_lines: usize,
    pub processed_lines: usize,
    pub peak_memory_mb: usize,
    pub throughput_mb_per_sec: f64,
    pub lines_per_sec: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_time: Duration::new(0, 0),
            parse_time: Duration::new(0, 0),
            process_time: Duration::new(0, 0),
            total_lines: 0,
            processed_lines: 0,
            peak_memory_mb: 0,
            throughput_mb_per_sec: 0.0,
            lines_per_sec: 0.0,
        }
    }

    pub fn calculate_derived_metrics(&mut self, file_size_mb: f64) {
        let total_seconds = self.total_time.as_secs_f64();
        if total_seconds > 0.0 {
            self.throughput_mb_per_sec = file_size_mb / total_seconds;
            self.lines_per_sec = self.total_lines as f64 / total_seconds;
        }
    }

    pub fn print_summary(&self) {
        println!("\n=== Performance Summary ===");
        println!("Total Time: {:.2}s", self.total_time.as_secs_f64());
        println!("Parse Time: {:.2}s", self.parse_time.as_secs_f64());
        println!("Process Time: {:.2}s", self.process_time.as_secs_f64());
        println!("Total Lines: {}", self.total_lines);
        println!("Processed Lines: {}", self.processed_lines);
        println!("Peak Memory: {}MB", self.peak_memory_mb);
        println!("Throughput: {:.2} MB/s", self.throughput_mb_per_sec);
        println!("Lines/sec: {:.0}", self.lines_per_sec);
        
        if self.total_lines > 0 {
            let process_rate = (self.processed_lines as f64 / self.total_lines as f64) * 100.0;
            println!("Processing Rate: {:.1}%", process_rate);
        }
    }
}

/// 메모리 사용량 모니터링
pub struct MemoryMonitor {
    peak_usage: AtomicUsize,
    current_usage: AtomicUsize,
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            peak_usage: AtomicUsize::new(0),
            current_usage: AtomicUsize::new(0),
        }
    }

    pub fn record_allocation(&self, size: usize) {
        let current = self.current_usage.fetch_add(size, Ordering::Relaxed) + size;
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        
        while current > peak {
            match self.peak_usage.compare_exchange_weak(peak, current, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(x) => peak = x,
            }
        }
    }

    pub fn record_deallocation(&self, size: usize) {
        self.current_usage.fetch_sub(size, Ordering::Relaxed);
    }

    pub fn get_peak_mb(&self) -> usize {
        self.peak_usage.load(Ordering::Relaxed) / (1024 * 1024)
    }

    pub fn get_current_mb(&self) -> usize {
        self.current_usage.load(Ordering::Relaxed) / (1024 * 1024)
    }

    pub fn get_system_memory_info() -> Option<SystemMemoryInfo> {
        // macOS에서 메모리 정보를 가져오는 함수
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            let output = Command::new("vm_stat")
                .output()
                .ok()?;
            
            let output_str = String::from_utf8(output.stdout).ok()?;
            let mut total_mb = 0;
            let mut free_mb = 0;
            
            for line in output_str.lines() {
                if line.contains("Pages free:") {
                    if let Some(pages_str) = line.split_whitespace().nth(2) {
                        if let Ok(pages) = pages_str.trim_end_matches('.').parse::<u64>() {
                            free_mb = (pages * 4096) / (1024 * 1024); // 4KB per page
                        }
                    }
                }
            }
            
            // 총 메모리는 sysctl로 가져오기
            let output = Command::new("sysctl")
                .arg("-n")
                .arg("hw.memsize")
                .output()
                .ok()?;
            
            let output_str = String::from_utf8(output.stdout).ok()?;
            if let Ok(total_bytes) = output_str.trim().parse::<u64>() {
                total_mb = total_bytes / (1024 * 1024);
            }
            
            Some(SystemMemoryInfo {
                total_mb,
                free_mb,
                used_mb: total_mb - free_mb,
            })
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub struct SystemMemoryInfo {
    pub total_mb: u64,
    pub free_mb: u64,
    pub used_mb: u64,
}

impl SystemMemoryInfo {
    pub fn print_info(&self) {
        println!("System Memory - Total: {}MB, Used: {}MB, Free: {}MB", 
                 self.total_mb, self.used_mb, self.free_mb);
    }
}

/// 성능 프로파일러
pub struct PerformanceProfiler {
    start_time: Instant,
    checkpoints: Vec<(String, Instant)>,
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            checkpoints: Vec::new(),
        }
    }

    pub fn checkpoint(&mut self, name: &str) {
        self.checkpoints.push((name.to_string(), Instant::now()));
    }

    pub fn print_profile(&self) {
        println!("\n=== Performance Profile ===");
        
        let mut last_time = self.start_time;
        
        for (name, time) in &self.checkpoints {
            let duration = time.duration_since(last_time);
            let total_duration = time.duration_since(self.start_time);
            
            println!("{}: {:.3}s (total: {:.3}s)", 
                     name, 
                     duration.as_secs_f64(),
                     total_duration.as_secs_f64());
            
            last_time = *time;
        }
    }

    pub fn get_total_time(&self) -> Duration {
        if let Some((_, last_time)) = self.checkpoints.last() {
            last_time.duration_since(self.start_time)
        } else {
            Instant::now().duration_since(self.start_time)
        }
    }
}

/// 적응형 청크 크기 계산
pub fn calculate_optimal_chunk_size(file_size: usize, available_memory_mb: usize) -> usize {
    // 메모리의 10% 정도를 청크로 사용
    let max_chunk_size = (available_memory_mb * 1024 * 1024) / 10;
    
    // 최소 1MB, 최대 100MB
    let min_chunk_size = 1024 * 1024;
    let max_chunk_size = std::cmp::min(max_chunk_size, 100 * 1024 * 1024);
    
    // 파일 크기에 따라 적응적으로 조정
    let suggested_chunk_size = if file_size < 100 * 1024 * 1024 {
        // 100MB 미만: 전체 파일의 1/4
        file_size / 4
    } else if file_size < 1024 * 1024 * 1024 {
        // 1GB 미만: 전체 파일의 1/8
        file_size / 8
    } else {
        // 1GB 이상: 전체 파일의 1/16
        file_size / 16
    };
    
    std::cmp::max(min_chunk_size, std::cmp::min(max_chunk_size, suggested_chunk_size))
}
