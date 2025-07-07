use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicBool, Ordering};

use crate::realtime::analyzer::{
    Alert, AlertSeverity, AnalysisResult, Anomaly, MetricType, RealtimeAnalyzer, Trend,
    TrendDirection,
};
use crate::realtime::monitor::{LogEvent, LogMonitor, ParsedLogEntry, RealtimeStats};

/// 실시간 대시보드
pub struct RealtimeDashboard {
    analyzer: Arc<Mutex<RealtimeAnalyzer>>,
    monitor: Arc<Mutex<LogMonitor>>,
    display_config: DisplayConfig,
    is_running: bool,
    #[allow(dead_code)]
    update_interval: Duration,
    last_update: Instant,
    shutdown_flag: Option<Arc<AtomicBool>>,  // 종료 플래그 추가
}

/// 디스플레이 설정
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub show_stats: bool,
    pub show_alerts: bool,
    pub show_trends: bool,
    pub show_anomalies: bool,
    pub show_recent_entries: bool,
    pub max_recent_entries: usize,
    pub refresh_rate: Duration,
    pub compact_mode: bool,
}

/// 대시보드 디스플레이 데이터
#[derive(Debug, Clone)]
pub struct DashboardData {
    pub current_time: Instant,
    pub stats: RealtimeStats,
    pub alerts: Vec<Alert>,
    pub trends: Vec<Trend>,
    pub anomalies: Vec<Anomaly>,
    pub recent_entries: VecDeque<ParsedLogEntry>,
    pub uptime: Duration,
}

impl RealtimeDashboard {
    /// 새로운 실시간 대시보드 생성
    pub fn new(
        _file_path: String,
        update_interval: Duration,
        display_config: DisplayConfig,
    ) -> io::Result<Self> {
        let monitor = LogMonitor::new();
        let analyzer = RealtimeAnalyzer::with_default_rules(update_interval);

        Ok(RealtimeDashboard {
            analyzer: Arc::new(Mutex::new(analyzer)),
            monitor: Arc::new(Mutex::new(monitor)),
            display_config,
            is_running: false,
            update_interval,
            last_update: Instant::now(),
            shutdown_flag: None,  // 초기값으로 None 설정
        })
    }

    /// 종료 플래그 설정
    pub fn set_shutdown_flag(&mut self, flag: Arc<AtomicBool>) {
        self.shutdown_flag = Some(flag);
    }

    /// 대시보드 시작
    pub fn start(&mut self) -> io::Result<()> {
        if self.is_running {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Dashboard is already running",
            ));
        }

        // 모니터 시작
        if let Ok(mut monitor) = self.monitor.lock() {
            monitor.start()?;
        }

        self.is_running = true;
        self.last_update = Instant::now();

        println!("실시간 대시보드 시작");
        Ok(())
    }

    /// 대시보드 중지
    pub fn stop(&mut self) {
        if !self.is_running {
            return;
        }

        // 모니터 중지
        if let Ok(mut monitor) = self.monitor.lock() {
            monitor.stop();
        }

        self.is_running = false;
        println!("실시간 대시보드 중지");
    }

    /// 대시보드 실행 (메인 루프)
    pub fn run(&mut self) -> io::Result<()> {
        self.start()?;

        let mut recent_entries = VecDeque::new();
        let start_time = Instant::now();

        println!("실시간 로그 분석 대시보드 시작");
        println!("종료하려면 Ctrl+C를 누르세요\n");

        // 메인 루프
        loop {
            // 먼저 종료 플래그 체크
            if let Some(ref flag) = self.shutdown_flag {
                if !flag.load(Ordering::SeqCst) {
                    eprintln!("종료 신호 감지됨, 대시보드 종료 중...");
                    break;
                }
            }

            // 새로운 로그 이벤트 처리
            let mut should_exit = false;
            if let Ok(mut monitor) = self.monitor.lock() {
                let events = monitor.receive_events();
                for event in events {
                    match event {
                        LogEvent::NewLine(line) => {
                            // 빈 라인은 무시 (채널 연결 확인용)
                            if line.is_empty() {
                                continue;
                            }
                            
                            if let Some(entry) = crate::realtime::monitor::parse_log_entry(&line) {
                                // 분석기에 엔트리 추가
                                if let Ok(mut analyzer) = self.analyzer.lock() {
                                    analyzer.process_entry(&entry);
                                }

                                // 최근 엔트리 목록 업데이트
                                recent_entries.push_back(entry);
                                if recent_entries.len() > self.display_config.max_recent_entries {
                                    recent_entries.pop_front();
                                }
                            }
                        }
                        LogEvent::FileRotated => {
                            println!("파일 로테이션 감지됨");
                        }
                        LogEvent::Error(error) => {
                            eprintln!("오류: {}", error);
                        }
                        LogEvent::Shutdown => {
                            should_exit = true;
                            break;
                        }
                    }
                }
            }

            if should_exit {
                break;
            }

            // 화면 업데이트
            if self.last_update.elapsed() >= self.display_config.refresh_rate {
                self.update_display(start_time, &recent_entries)?;
                self.last_update = Instant::now();
            }

            // CPU 사용량 줄이기 위해 짧은 대기
            thread::sleep(Duration::from_millis(50));  // 더 빠른 응답을 위해 50ms로 단축
        }

        println!("\n정리 중...");
        self.stop();
        println!("실시간 로그 분석이 종료되었습니다.");
        Ok(())
    }

    /// 현재 대시보드 데이터 가져오기
    pub fn get_dashboard_data(&self, start_time: Instant) -> DashboardData {
        let current_stats = if let Ok(analyzer) = self.analyzer.lock() {
            analyzer.get_current_stats()
        } else {
            RealtimeStats::new()
        };

        let analysis_result = if let Ok(analyzer) = self.analyzer.lock() {
            analyzer.analyze()
        } else {
            AnalysisResult {
                timestamp: Instant::now(),
                current_stats: RealtimeStats::new(),
                alerts: Vec::new(),
                trends: Vec::new(),
                anomalies: Vec::new(),
            }
        };

        DashboardData {
            current_time: Instant::now(),
            stats: current_stats,
            alerts: analysis_result.alerts,
            trends: analysis_result.trends,
            anomalies: analysis_result.anomalies,
            recent_entries: VecDeque::new(), // 필요시 구현
            uptime: start_time.elapsed(),
        }
    }

    /// 화면 업데이트
    fn update_display(&self, start_time: Instant, recent_entries: &VecDeque<ParsedLogEntry>) -> io::Result<()> {
        // 화면 지우기 (ANSI 이스케이프 코드)
        print!("\x1B[2J\x1B[H");

        let dashboard_data = self.get_dashboard_data(start_time);
        
        // 헤더 출력
        self.print_header(&dashboard_data)?;

        // 통계 정보 출력
        if self.display_config.show_stats {
            self.print_stats(&dashboard_data)?;
        }

        // 알림 출력
        if self.display_config.show_alerts && !dashboard_data.alerts.is_empty() {
            self.print_alerts(&dashboard_data)?;
        }

        // 트렌드 출력
        if self.display_config.show_trends && !dashboard_data.trends.is_empty() {
            self.print_trends(&dashboard_data)?;
        }

        // 이상 징후 출력
        if self.display_config.show_anomalies && !dashboard_data.anomalies.is_empty() {
            self.print_anomalies(&dashboard_data)?;
        }

        // 최근 엔트리 출력
        if self.display_config.show_recent_entries && !recent_entries.is_empty() {
            self.print_recent_entries(recent_entries)?;
        }

        // 하단 정보 출력
        self.print_footer()?;

        io::stdout().flush()?;
        Ok(())
    }

    /// 헤더 출력
    fn print_header(&self, data: &DashboardData) -> io::Result<()> {
        println!("╔══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╗");
        println!("║                                                                실시간 로그 분석 대시보드                                                                 ║");
        println!("║                                                    업타임: {:>8} | 업데이트: {:>8}                                                    ║", 
                 format_duration(data.uptime), 
                 format_duration(data.current_time.elapsed()));
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 통계 정보 출력
    fn print_stats(&self, data: &DashboardData) -> io::Result<()> {
        println!("║ 📊 실시간 통계                                                                                                                                   ║");
        println!("║                                                                                                                                               ║");
        println!("║   총 엔트리: {:>10}    처리율: {:>8.1}/초    Block: {:>8}    UFS: {:>8}    Custom: {:>8}              ║", 
                 data.stats.total_entries,
                 data.stats.entries_per_second,
                 data.stats.total_entries, // 임시로 total_entries 사용
                 data.stats.info_count,
                 data.stats.debug_count); // 임시로 debug_count 사용
        println!("║                                                                                                                                               ║");
        println!("║   평균 레이턴시: {:>8.2}ms    최대 레이턴시: {:>8.2}ms    최소 레이턴시: {:>8.2}ms                               ║", 
                 data.stats.average_latency,
                 if data.stats.max_latency == 0.0 { 0.0 } else { data.stats.max_latency },
                 if data.stats.min_latency == f64::INFINITY { 0.0 } else { data.stats.min_latency });
        println!("║                                                                                                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 알림 출력
    fn print_alerts(&self, data: &DashboardData) -> io::Result<()> {
        println!("║ 🚨 알림 ({})                                                                                                                                 ║", data.alerts.len());
        println!("║                                                                                                                                               ║");
        
        for alert in &data.alerts {
            let severity_icon = match alert.severity {
                AlertSeverity::Critical => "🔴",
                AlertSeverity::Warning => "🟡",
                AlertSeverity::Info => "🔵",
            };
            
            println!("║   {} {:<120} ║", severity_icon, truncate_string(&alert.message, 120));
        }
        
        println!("║                                                                                                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 트렌드 출력
    fn print_trends(&self, data: &DashboardData) -> io::Result<()> {
        println!("║ 📈 트렌드 ({})                                                                                                                               ║", data.trends.len());
        println!("║                                                                                                                                               ║");
        
        for trend in &data.trends {
            let direction_icon = match trend.direction {
                TrendDirection::Increasing => "⬆️",
                TrendDirection::Decreasing => "⬇️",
                TrendDirection::Stable => "➡️",
            };
            
            let metric_name = format_metric_name(&trend.metric);
            println!("║   {} {:<20} | 변화율: {:>8.2} | 신뢰도: {:>6.1}%                                                                   ║", 
                     direction_icon, 
                     metric_name,
                     trend.rate_of_change,
                     trend.confidence * 100.0);
        }
        
        println!("║                                                                                                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 이상 징후 출력
    fn print_anomalies(&self, data: &DashboardData) -> io::Result<()> {
        println!("║ ⚠️  이상 징후 ({})                                                                                                                            ║", data.anomalies.len());
        println!("║                                                                                                                                               ║");
        
        for anomaly in &data.anomalies {
            let severity_icon = match anomaly.severity {
                AlertSeverity::Critical => "🔴",
                AlertSeverity::Warning => "🟡",
                AlertSeverity::Info => "🔵",
            };
            
            println!("║   {} {:<120} ║", severity_icon, truncate_string(&anomaly.description, 120));
        }
        
        println!("║                                                                                                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 최근 엔트리 출력
    fn print_recent_entries(&self, entries: &VecDeque<ParsedLogEntry>) -> io::Result<()> {
        println!("║ 📋 최근 로그 엔트리 ({})                                                                                                                     ║", entries.len());
        println!("║                                                                                                                                               ║");
        
        for entry in entries.iter().rev().take(5) {
            let type_icon = match entry.trace_type.as_str() {
                "Block" => "🔷",
                "UFS" => "🔶",
                "UFSCUSTOM" => "🔸",
                _ => "📝",
            };
            
            let timestamp = entry.timestamp.clone();
            let trace_type = entry.trace_type.clone();
            println!("║   {} {:<8} | {:<10} | 타임스탬프: {:<15}                                                                           ║", 
                     type_icon, 
                     trace_type,
                     "",
                     timestamp);
        }
        
        println!("║                                                                                                                                               ║");
        println!("╠══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╣");
        Ok(())
    }

    /// 하단 정보 출력
    fn print_footer(&self) -> io::Result<()> {
        println!("║ 💡 팁: 실시간 분석 중... 종료하려면 Ctrl+C를 누르세요                                                                                        ║");
        println!("╚══════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════════╝");
        Ok(())
    }

    /// 대시보드 실행 중인지 확인
    pub fn is_running(&self) -> bool {
        self.is_running
    }

    /// 디스플레이 설정 업데이트
    pub fn update_display_config(&mut self, config: DisplayConfig) {
        self.display_config = config;
    }
}

impl Default for DisplayConfig {
    /// 기본 디스플레이 설정
    fn default() -> Self {
        DisplayConfig {
            show_stats: true,
            show_alerts: true,
            show_trends: true,
            show_anomalies: true,
            show_recent_entries: true,
            max_recent_entries: 10,
            refresh_rate: Duration::from_secs(1),
            compact_mode: false,
        }
    }
}

impl DisplayConfig {

    /// 컴팩트 모드 설정
    pub fn compact() -> Self {
        DisplayConfig {
            show_stats: true,
            show_alerts: true,
            show_trends: false,
            show_anomalies: false,
            show_recent_entries: false,
            max_recent_entries: 5,
            refresh_rate: Duration::from_millis(500),
            compact_mode: true,
        }
    }

    /// 상세 모드 설정
    pub fn detailed() -> Self {
        DisplayConfig {
            show_stats: true,
            show_alerts: true,
            show_trends: true,
            show_anomalies: true,
            show_recent_entries: true,
            max_recent_entries: 20,
            refresh_rate: Duration::from_millis(500),
            compact_mode: false,
        }
    }
}

/// 기간을 사람이 읽기 쉬운 형태로 포맷
fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    
    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

/// 문자열을 지정된 길이로 자르기
fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        format!("{:<width$}", s, width = max_len)
    } else {
        format!("{:<width$}", s[..max_len.saturating_sub(3)].to_string() + "...", width = max_len)
    }
}

/// 메트릭 이름 포맷
fn format_metric_name(metric: &MetricType) -> &'static str {
    match metric {
        MetricType::EntriesPerSecond => "처리율",
        MetricType::AverageLatency => "평균 레이턴시",
        MetricType::MaxLatency => "최대 레이턴시",
        MetricType::BlockRatio => "Block 비율",
        MetricType::UFSRatio => "UFS 비율",
        MetricType::UFSCustomRatio => "UFS Custom 비율",
        MetricType::TotalEntries => "총 엔트리",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_config_default() {
        let config = DisplayConfig::default();
        assert!(config.show_stats);
        assert!(config.show_alerts);
        assert!(config.show_trends);
        assert!(!config.compact_mode);
    }

    #[test]
    fn test_display_config_compact() {
        let config = DisplayConfig::compact();
        assert!(config.show_stats);
        assert!(config.show_alerts);
        assert!(!config.show_trends);
        assert!(config.compact_mode);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::from_secs(0)), "00:00");
        assert_eq!(format_duration(Duration::from_secs(90)), "01:30");
        assert_eq!(format_duration(Duration::from_secs(3661)), "01:01:01");
    }

    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello     ");
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }
}
