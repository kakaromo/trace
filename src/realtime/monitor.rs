use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimeStats {
    pub total_entries: u64,
    pub entries_per_second: f64,
    pub error_count: u64,
    pub warning_count: u64,
    pub info_count: u64,
    pub debug_count: u64,
    pub trace_count: u64,
    pub unique_processes: u64,
    pub unique_threads: u64,
    pub average_latency: f64,
    pub max_latency: f64,
    pub min_latency: f64,
    pub last_updated_timestamp: u64,
}

impl RealtimeStats {
    pub fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            total_entries: 0,
            entries_per_second: 0.0,
            error_count: 0,
            warning_count: 0,
            info_count: 0,
            debug_count: 0,
            trace_count: 0,
            unique_processes: 0,
            unique_threads: 0,
            average_latency: 0.0,
            max_latency: 0.0,
            min_latency: 0.0,
            last_updated_timestamp: now,
        }
    }

    pub fn add_entry(&mut self, entry: &ParsedLogEntry) {
        // 임시 구현 - 실제로는 엔트리를 처리해야 함
        self.total_entries += 1;
        
        // 레벨별 카운트 업데이트
        match entry.level.to_lowercase().as_str() {
            "error" => self.error_count += 1,
            "warning" | "warn" => self.warning_count += 1,
            "info" => self.info_count += 1,
            "debug" => self.debug_count += 1,
            "trace" => self.trace_count += 1,
            _ => {}
        }
    }
    
    pub fn reset(&mut self) {
        self.total_entries = 0;
        self.entries_per_second = 0.0;
        self.error_count = 0;
        self.warning_count = 0;
        self.info_count = 0;
        self.debug_count = 0;
        self.trace_count = 0;
        self.unique_processes = 0;
        self.unique_threads = 0;
        self.average_latency = 0.0;
        self.max_latency = 0.0;
        self.min_latency = 0.0;
        self.last_updated_timestamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
    }
}

impl Default for RealtimeStats {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedLogEntry {
    pub timestamp: String,
    pub level: String,
    pub process_id: u32,
    pub thread_id: u32,
    pub message: String,
    pub file_name: Option<String>,
    pub line_number: Option<u32>,
    pub latency: Option<f64>,
    pub trace_type: String, // 임시로 String 타입 사용
}

impl ParsedLogEntry {
    pub fn new(
        timestamp: String,
        level: String,
        process_id: u32,
        thread_id: u32,
        message: String,
    ) -> Self {
        Self {
            timestamp,
            level,
            process_id,
            thread_id,
            message,
            file_name: None,
            line_number: None,
            latency: None,
            trace_type: "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogEvent {
    NewLine(String),
    FileRotated,
    Error(String),
    Shutdown,
}

pub struct LogMonitor {
    pub stats: Arc<Mutex<RealtimeStats>>,
    pub recent_entries: Arc<Mutex<Vec<ParsedLogEntry>>>,
    pub process_threads: Arc<Mutex<HashMap<u32, Vec<u32>>>>,
    start_time: Instant,
}

impl LogMonitor {
    pub fn new() -> Self {
        Self {
            stats: Arc::new(Mutex::new(RealtimeStats::new())),
            recent_entries: Arc::new(Mutex::new(Vec::new())),
            process_threads: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    pub fn process_entry(&mut self, entry: &ParsedLogEntry) {
        let mut stats = self.stats.lock().unwrap();
        let mut recent = self.recent_entries.lock().unwrap();
        let mut pt = self.process_threads.lock().unwrap();

        // Update basic stats
        stats.total_entries += 1;
        match entry.level.to_lowercase().as_str() {
            "error" => stats.error_count += 1,
            "warning" | "warn" => stats.warning_count += 1,
            "info" => stats.info_count += 1,
            "debug" => stats.debug_count += 1,
            "trace" => stats.trace_count += 1,
            _ => {}
        }

        // Update process/thread tracking
        pt.entry(entry.process_id)
            .or_default()
            .push(entry.thread_id);
        
        stats.unique_processes = pt.len() as u64;
        stats.unique_threads = pt.values().flatten().collect::<std::collections::HashSet<_>>().len() as u64;

        // Update latency if available
        if let Some(latency) = entry.latency {
            if stats.max_latency < latency {
                stats.max_latency = latency;
            }
            if stats.min_latency == 0.0 || stats.min_latency > latency {
                stats.min_latency = latency;
            }
            stats.average_latency = (stats.average_latency + latency) / 2.0;
        }

        // Calculate entries per second
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            stats.entries_per_second = stats.total_entries as f64 / elapsed;
        }

        stats.last_updated_timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Keep only recent entries (last 100)
        recent.push(entry.clone());
        if recent.len() > 100 {
            recent.remove(0);
        }
    }

    pub fn get_stats(&self) -> RealtimeStats {
        self.stats.lock().unwrap().clone()
    }

    pub fn get_recent_entries(&self) -> Vec<ParsedLogEntry> {
        self.recent_entries.lock().unwrap().clone()
    }

    pub fn start(&mut self) -> Result<(), std::io::Error> {
        // 로그 모니터링 시작 - 임시 구현
        Ok(())
    }

    pub fn stop(&mut self) {
        // 로그 모니터링 중지 - 임시 구현
    }

    pub fn receive_events(&mut self) -> Vec<LogEvent> {
        // 이벤트 수신 - 임시 구현
        Vec::new()
    }
}

impl Default for LogMonitor {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RealtimeMonitor {
    pub monitor: LogMonitor,
}

impl RealtimeMonitor {
    pub fn new() -> Self {
        Self {
            monitor: LogMonitor::new(),
        }
    }

    pub fn process_entry(&mut self, entry: &ParsedLogEntry) {
        self.monitor.process_entry(entry);
    }

    pub fn get_stats(&self) -> RealtimeStats {
        self.monitor.get_stats()
    }

    pub fn get_recent_entries(&self) -> Vec<ParsedLogEntry> {
        self.monitor.get_recent_entries()
    }

    pub fn check_file_changes(&mut self, log_file: &str) -> Result<Vec<ParsedLogEntry>, std::io::Error> {
        use std::fs::File;
        use std::io::{BufRead, BufReader, Seek, SeekFrom};
        
        println!("📂 로그 파일 확인: {}", log_file);
        
        // 파일 존재 확인
        if !std::path::Path::new(log_file).exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("로그 파일을 찾을 수 없습니다: {}", log_file)
            ));
        }
        
        // 파일 열기
        let mut file = File::open(log_file)?;
        
        // 파일 끝으로 이동해서 새로운 내용이 있는지 확인
        let _file_size = file.seek(SeekFrom::End(0))?;
        
        // 간단한 구현: 파일의 처음 10줄을 읽어서 파싱
        file.seek(SeekFrom::Start(0))?;
        let reader = BufReader::new(&file);
        
        let mut entries = Vec::new();
        for (i, line) in reader.lines().enumerate() {
            if i >= 10 { break; } // 처음 10줄만 처리 (성능상 이유)
            
            let line = line?;
            if let Some(entry) = parse_log_entry(&line) {
                entries.push(entry);
            }
        }
        
        println!("📝 로그 파일에서 {} 개의 엔트리를 읽었습니다", entries.len());
        Ok(entries)
    }
}

pub fn parse_log_entry(line: &str) -> Option<ParsedLogEntry> {
    // 실제 로그 형식에 맞게 파싱
    // 예: <idle>-0 [003] d.h2. 141036.006962: ufshcd_command: complete_rsp: ...
    
    if line.trim().is_empty() {
        return None;
    }
    
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 4 {
        return None;
    }
    
    // 프로세스 이름과 PID 추출 (예: <idle>-0 또는 f2fs_discard-25-1461)
    let process_part = parts[0];
    let process_id = if let Some(dash_pos) = process_part.rfind('-') {
        process_part[dash_pos + 1..]
            .parse::<u32>()
            .unwrap_or(0)
    } else {
        0
    };
    
    // CPU 번호 추출 (예: [003])
    let cpu_part = parts.get(1).unwrap_or(&"[0]");
    let thread_id = if cpu_part.starts_with('[') && cpu_part.ends_with(']') {
        cpu_part[1..cpu_part.len()-1]
            .parse::<u32>()
            .unwrap_or(0)
    } else {
        0
    };
    
    // 타임스탬프 추출
    let timestamp = parts.get(3).unwrap_or(&"0.0").to_string();
    
    // 로그 레벨/이벤트 추출
    let level = if line.contains("ufshcd_command") {
        "UFS"
    } else if line.contains("block_rq") {
        "BLOCK"
    } else {
        "INFO"
    }.to_string();
    
    // 메시지는 나머지 전체
    let message = parts[4..].join(" ");
    
    // trace_type 설정
    let trace_type = if line.contains("ufshcd_command") {
        "UFS"
    } else if line.contains("block_rq") {
        "Block"
    } else {
        "Other"
    }.to_string();
    
    let mut entry = ParsedLogEntry::new(
        timestamp,
        level,
        process_id,
        thread_id,
        message,
    );
    entry.trace_type = trace_type;
    
    Some(entry)
}

impl Default for RealtimeMonitor {
    fn default() -> Self {
        Self::new()
    }
}
