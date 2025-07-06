use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::models::{Block, TraceType, UFS, UFSCUSTOM};
use crate::parsers::log_common::parse_log_line;

/// 실시간 로그 모니터링을 위한 구조체
pub struct LogMonitor {
    file_path: String,
    last_position: u64,
    last_modified: std::time::SystemTime,
    sender: Sender<LogEvent>,
    receiver: Receiver<LogEvent>,
    is_running: bool,
    poll_interval: Duration,
}

/// 로그 이벤트 타입
#[derive(Debug, Clone)]
pub enum LogEvent {
    NewLine(String),
    FileRotated,
    Error(String),
    Shutdown,
}

/// 파싱된 로그 엔트리
#[derive(Debug, Clone)]
pub struct ParsedLogEntry {
    pub timestamp: f64,
    pub trace_type: TraceType,
    pub entry: LogEntryData,
}

/// 로그 엔트리 데이터
#[derive(Debug, Clone)]
pub enum LogEntryData {
    Block(Block),
    UFS(UFS),
    UFSCustom(UFSCUSTOM),
}

impl LogEntryData {
    /// 타임스탬프 추출
    pub fn get_timestamp(&self) -> f64 {
        match self {
            LogEntryData::Block(block) => block.time,
            LogEntryData::UFS(ufs) => ufs.time,
            LogEntryData::UFSCustom(ufscustom) => ufscustom.start_time,
        }
    }
}

/// 실시간 통계 정보
#[derive(Debug, Clone)]
pub struct RealtimeStats {
    pub total_entries: usize,
    pub entries_per_second: f64,
    pub last_update: Instant,
    pub window_size: Duration,
    pub entry_timestamps: VecDeque<Instant>,
    pub block_count: usize,
    pub ufs_count: usize,
    pub ufscustom_count: usize,
    pub avg_latency: f64,
    pub max_latency: f64,
    pub min_latency: f64,
}

impl LogMonitor {
    /// 새로운 로그 모니터 생성
    pub fn new(file_path: String) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let file_metadata = std::fs::metadata(&file_path)?;
        let last_modified = file_metadata.modified()?;
        
        Ok(LogMonitor {
            file_path,
            last_position: 0,
            last_modified,
            sender,
            receiver,
            is_running: false,
            poll_interval: Duration::from_millis(100), // 100ms 간격으로 폴링
        })
    }

    /// 폴링 간격 설정
    pub fn set_poll_interval(&mut self, interval: Duration) {
        self.poll_interval = interval;
    }

    /// 모니터링 시작
    pub fn start(&mut self) -> std::io::Result<()> {
        if self.is_running {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Monitor is already running",
            ));
        }

        self.is_running = true;
        let file_path = self.file_path.clone();
        let sender = self.sender.clone();
        let poll_interval = self.poll_interval;

        // 파일 모니터링 스레드 시작
        thread::spawn(move || {
            let mut monitor = FileMonitor::new(file_path, sender, poll_interval);
            monitor.run();
        });

        println!("실시간 로그 모니터링 시작: {}", self.file_path);
        Ok(())
    }

    /// 모니터링 중지
    pub fn stop(&mut self) {
        if self.is_running {
            let _ = self.sender.send(LogEvent::Shutdown);
            self.is_running = false;
            println!("실시간 로그 모니터링 중지");
        }
    }

    /// 새로운 로그 이벤트 수신
    pub fn receive_events(&self) -> Vec<LogEvent> {
        let mut events = Vec::new();
        
        // 논블로킹 방식으로 모든 이벤트 수집
        while let Ok(event) = self.receiver.try_recv() {
            events.push(event);
        }
        
        events
    }

    /// 블로킹 방식으로 다음 이벤트 대기
    pub fn wait_for_event(&self, timeout: Duration) -> Option<LogEvent> {
        match self.receiver.recv_timeout(timeout) {
            Ok(event) => Some(event),
            Err(_) => None,
        }
    }

    /// 실행 중인지 확인
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

/// 파일 모니터링 내부 구조체
struct FileMonitor {
    file_path: String,
    sender: Sender<LogEvent>,
    poll_interval: Duration,
    last_position: u64,
    last_modified: std::time::SystemTime,
}

impl FileMonitor {
    fn new(file_path: String, sender: Sender<LogEvent>, poll_interval: Duration) -> Self {
        FileMonitor {
            file_path,
            sender,
            poll_interval,
            last_position: 0,
            last_modified: std::time::SystemTime::UNIX_EPOCH,
        }
    }

    fn run(&mut self) {
        // 파일 초기 위치 설정 (파일 끝부터 시작)
        if let Ok(metadata) = std::fs::metadata(&self.file_path) {
            self.last_position = metadata.len();
            self.last_modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        }

        loop {
            match self.check_file_changes() {
                Ok(has_changes) => {
                    if has_changes {
                        if let Err(e) = self.read_new_lines() {
                            let _ = self.sender.send(LogEvent::Error(format!("파일 읽기 오류: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    let _ = self.sender.send(LogEvent::Error(format!("파일 상태 확인 오류: {}", e)));
                }
            }

            // 더 짧은 간격으로 종료 신호 확인
            for _ in 0..10 {
                thread::sleep(Duration::from_millis(self.poll_interval.as_millis() as u64 / 10));
                // 채널이 닫혔는지 확인 (receiver가 드롭되었는지)
                if self.sender.send(LogEvent::NewLine(String::new())).is_err() {
                    return; // 채널이 닫혔으므로 종료
                }
            }
        }
    }

    fn check_file_changes(&mut self) -> std::io::Result<bool> {
        let metadata = std::fs::metadata(&self.file_path)?;
        let current_modified = metadata.modified()?;
        let current_size = metadata.len();

        // 파일이 로테이션되었는지 확인 (크기가 줄어들었을 때)
        if current_size < self.last_position {
            let _ = self.sender.send(LogEvent::FileRotated);
            self.last_position = 0;
            self.last_modified = current_modified;
            return Ok(true);
        }

        // 파일이 수정되었고 크기가 증가했는지 확인
        if current_modified > self.last_modified && current_size > self.last_position {
            self.last_modified = current_modified;
            return Ok(true);
        }

        Ok(false)
    }

    fn read_new_lines(&mut self) -> std::io::Result<()> {
        let mut file = File::open(&self.file_path)?;
        file.seek(SeekFrom::Start(self.last_position))?;
        
        let reader = BufReader::new(file);
        let mut new_position = self.last_position;
        
        for line in reader.lines() {
            let line = line?;
            new_position += line.len() as u64 + 1; // +1 for newline character
            
            if let Err(_) = self.sender.send(LogEvent::NewLine(line)) {
                // 수신자가 없으면 루프 종료
                break;
            }
        }
        
        self.last_position = new_position;
        Ok(())
    }
}

impl RealtimeStats {
    /// 새로운 실시간 통계 생성
    pub fn new() -> Self {
        RealtimeStats {
            total_entries: 0,
            entries_per_second: 0.0,
            last_update: Instant::now(),
            window_size: Duration::from_secs(10), // 10초 윈도우
            entry_timestamps: VecDeque::new(),
            block_count: 0,
            ufs_count: 0,
            ufscustom_count: 0,
            avg_latency: 0.0,
            max_latency: 0.0,
            min_latency: f64::INFINITY,
        }
    }

    /// 새로운 로그 엔트리 추가
    pub fn add_entry(&mut self, entry: &ParsedLogEntry) {
        let now = Instant::now();
        
        // 총 엔트리 수 증가
        self.total_entries += 1;
        
        // 타입별 카운트 증가
        match entry.entry {
            LogEntryData::Block(_) => self.block_count += 1,
            LogEntryData::UFS(_) => self.ufs_count += 1,
            LogEntryData::UFSCustom(_) => self.ufscustom_count += 1,
        }
        
        // 레이턴시 통계 업데이트
        let latency = self.extract_latency(&entry.entry);
        if latency > 0.0 {
            self.max_latency = self.max_latency.max(latency);
            self.min_latency = self.min_latency.min(latency);
            
            // 평균 레이턴시 계산 (간단한 이동 평균)
            let alpha = 0.1; // 평활화 계수
            if self.avg_latency == 0.0 {
                self.avg_latency = latency;
            } else {
                self.avg_latency = alpha * latency + (1.0 - alpha) * self.avg_latency;
            }
        }
        
        // 윈도우 내 엔트리 타임스탬프 추가
        self.entry_timestamps.push_back(now);
        
        // 윈도우 크기 초과 엔트리 제거
        while let Some(&front_time) = self.entry_timestamps.front() {
            if now.duration_since(front_time) > self.window_size {
                self.entry_timestamps.pop_front();
            } else {
                break;
            }
        }
        
        // 초당 엔트리 수 계산
        let entries_in_window = self.entry_timestamps.len();
        let window_duration = self.window_size.as_secs_f64();
        self.entries_per_second = entries_in_window as f64 / window_duration;
        
        self.last_update = now;
    }

    /// 로그 엔트리에서 레이턴시 추출
    fn extract_latency(&self, entry: &LogEntryData) -> f64 {
        match entry {
            LogEntryData::Block(block) => block.dtoc,
            LogEntryData::UFS(ufs) => ufs.dtoc,
            LogEntryData::UFSCustom(ufscustom) => ufscustom.dtoc,
        }
    }

    /// 통계 리셋
    pub fn reset(&mut self) {
        self.total_entries = 0;
        self.entries_per_second = 0.0;
        self.last_update = Instant::now();
        self.entry_timestamps.clear();
        self.block_count = 0;
        self.ufs_count = 0;
        self.ufscustom_count = 0;
        self.avg_latency = 0.0;
        self.max_latency = 0.0;
        self.min_latency = f64::INFINITY;
    }

    /// 통계 정보를 문자열로 포맷
    pub fn format_stats(&self) -> String {
        format!(
            "Total: {} | Rate: {:.1}/s | Block: {} | UFS: {} | Custom: {} | Avg Latency: {:.2}ms | Max: {:.2}ms | Min: {:.2}ms",
            self.total_entries,
            self.entries_per_second,
            self.block_count,
            self.ufs_count,
            self.ufscustom_count,
            self.avg_latency,
            if self.max_latency == 0.0 { 0.0 } else { self.max_latency },
            if self.min_latency == f64::INFINITY { 0.0 } else { self.min_latency }
        )
    }
}

impl Default for RealtimeStats {
    fn default() -> Self {
        Self::new()
    }
}

/// 로그 라인을 파싱하여 ParsedLogEntry로 변환
pub fn parse_log_entry(line: &str) -> Option<ParsedLogEntry> {
    // 로그 라인 파싱 시도
    if let Some((trace_type, _)) = parse_log_line(line) {
        match trace_type {
            TraceType::Block => {
                if let Ok(block) = crate::parsers::log_common::parse_block_io_event(line) {
                    let entry_data = LogEntryData::Block(block);
                    let timestamp = entry_data.get_timestamp();
                    Some(ParsedLogEntry {
                        timestamp,
                        trace_type,
                        entry: entry_data,
                    })
                } else {
                    None
                }
            }
            TraceType::UFS => {
                if let Ok(ufs) = crate::parsers::log_common::parse_ufs_event(line) {
                    let entry_data = LogEntryData::UFS(ufs);
                    let timestamp = entry_data.get_timestamp();
                    Some(ParsedLogEntry {
                        timestamp,
                        trace_type,
                        entry: entry_data,
                    })
                } else {
                    None
                }
            }
            TraceType::UFSCUSTOM => {
                if let Ok(ufscustom) = crate::parsers::log_common::parse_ufscustom_event(line) {
                    let entry_data = LogEntryData::UFSCustom(ufscustom);
                    let timestamp = entry_data.get_timestamp();
                    Some(ParsedLogEntry {
                        timestamp,
                        trace_type,
                        entry: entry_data,
                    })
                } else {
                    None
                }
            }
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realtime_stats_creation() {
        let stats = RealtimeStats::new();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.entries_per_second, 0.0);
        assert_eq!(stats.block_count, 0);
        assert_eq!(stats.ufs_count, 0);
        assert_eq!(stats.ufscustom_count, 0);
    }

    #[test]
    fn test_log_monitor_creation() {
        let monitor = LogMonitor::new("test.log".to_string());
        assert!(monitor.is_ok());
        
        let monitor = monitor.unwrap();
        assert_eq!(monitor.file_path, "test.log");
        assert!(!monitor.is_running());
    }
}
