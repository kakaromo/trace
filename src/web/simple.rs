use crate::realtime::analyzer::RealtimeAnalyzer;
use crate::realtime::monitor::RealtimeMonitor;
use crate::realtime::streaming::StreamingProcessor;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub total_entries: u64,
    pub processing_rate: f64,
    pub avg_latency: f64,
    pub max_latency: f64,
    pub min_latency: f64,
    pub block_count: u64,
    pub ufs_count: u64,
    pub custom_count: u64,
    pub alerts: Vec<Alert>,
    pub trends: Vec<Trend>,
    pub anomalies: Vec<Anomaly>,
    pub recent_entries: Vec<RecentEntry>,
    pub block_traces: Vec<BlockTrace>,
    pub ufs_traces: Vec<UfsTrace>,
    pub ufscustom_traces: Vec<UfscustomTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub timestamp: u64,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub metric: String,
    pub direction: String,
    pub change_rate: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub z_score: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentEntry {
    pub timestamp: u64,
    pub trace_type: String,
    pub latency: f64,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub io_type: String,
    pub latency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfsTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub opcode: String,
    pub latency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfscustomTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub opcode: String,
    pub latency: f64,
}

pub struct WebDashboard {
    analyzer: Arc<RwLock<RealtimeAnalyzer>>,
    #[allow(dead_code)]
    monitor: Arc<RwLock<RealtimeMonitor>>,
    streaming_processor: Arc<StreamingProcessor>,
    port: u16,
}

impl WebDashboard {
    pub fn new(port: u16) -> Self {
        Self {
            analyzer: Arc::new(RwLock::new(RealtimeAnalyzer::new(std::time::Duration::from_secs(60)))),
            monitor: Arc::new(RwLock::new(RealtimeMonitor::new())),
            streaming_processor: Arc::new(StreamingProcessor::new(None)),
            port,
        }
    }

    pub fn new_with_output(port: u16, output_prefix: Option<&str>) -> Self {
        Self {
            analyzer: Arc::new(RwLock::new(RealtimeAnalyzer::new(std::time::Duration::from_secs(60)))),
            monitor: Arc::new(RwLock::new(RealtimeMonitor::new())),
            streaming_processor: Arc::new(StreamingProcessor::new(output_prefix)),
            port,
        }
    }

    pub async fn start(&self, log_file: &str, _output_prefix: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
        let analyzer = self.analyzer.clone();

        // 정적 파일 서빙 (HTML, CSS, JS)
        let static_files = warp::path("static")
            .and(warp::fs::dir("src/web/static"));

        // 메인 HTML 페이지
        let index = warp::path::end()
            .map(|| warp::reply::html(include_str!("static/index.html")));

        // API 엔드포인트
        let streaming_processor = self.streaming_processor.clone();
        let api = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::get())
            .and(warp::any().map(move || analyzer.clone()))
            .and(warp::any().map(move || streaming_processor.clone()))
            .and_then(get_dashboard_data);

        let routes = static_files
            .or(index)
            .or(api)
            .with(warp::cors().allow_any_origin());

        // 스트리밍 프로세서 시작
        let log_file_path = log_file.to_string();
        let streaming_processor_clone = self.streaming_processor.clone();
        tokio::spawn(async move {
            if let Err(e) = streaming_processor_clone.start_streaming(&log_file_path).await {
                eprintln!("스트리밍 프로세서 오류: {}", e);
            }
        });

        // 로그 모니터링 시작 (기존 코드는 유지하지만 비활성화)
        // let log_file_path = log_file.to_string();
        // let monitor_clone = self.monitor.clone();
        // let analyzer_clone = self.analyzer.clone();

        // tokio::spawn(async move {
        //     if let Err(e) = start_log_monitoring(
        //         log_file_path,
        //         monitor_clone,
        //         analyzer_clone,
        //     ).await {
        //         eprintln!("Log monitoring error: {}", e);
        //     }
        // });

        println!("🚀 웹 대시보드가 시작되었습니다!");
        println!("📊 브라우저에서 http://localhost:{}를 열어보세요", self.port);
        println!("💡 실시간 로그 분석이 진행 중입니다...");
        println!("🔧 웹 서버를 127.0.0.1:{} 포트에서 시작합니다...", self.port);

        let server = warp::serve(routes)
            .run(([127, 0, 0, 1], self.port));
        
        println!("✅ 웹 서버가 성공적으로 시작되었습니다!");
        
        server.await;

        Ok(())
    }
}

async fn get_dashboard_data(
    analyzer: Arc<RwLock<RealtimeAnalyzer>>,
    streaming_processor: Arc<StreamingProcessor>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let analyzer_guard = analyzer.read().await;
    let stats = analyzer_guard.get_current_stats();
    let alerts = analyzer_guard.get_active_alerts();
    let trends = analyzer_guard.get_trends();
    let anomalies = analyzer_guard.get_anomalies();

    // 스트리밍 프로세서에서 실제 트레이스 데이터 가져오기
    let (block_traces, ufs_traces, ufscustom_traces) = streaming_processor.get_current_data();

    let dashboard_data = DashboardData {
        total_entries: stats.total_entries,
        processing_rate: stats.entries_per_second,
        avg_latency: stats.average_latency,
        max_latency: stats.max_latency,
        min_latency: if stats.min_latency == f64::INFINITY { 0.0 } else { stats.min_latency },
        block_count: block_traces.len() as u64,
        ufs_count: ufs_traces.len() as u64,
        custom_count: ufscustom_traces.len() as u64,
        alerts: alerts.into_iter().map(|alert| Alert {
            id: uuid::Uuid::new_v4().to_string(),
            severity: match alert.severity {
                crate::realtime::analyzer::AlertSeverity::Info => "info".to_string(),
                crate::realtime::analyzer::AlertSeverity::Warning => "warning".to_string(),
                crate::realtime::analyzer::AlertSeverity::Critical => "critical".to_string(),
            },
            message: alert.message,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            metric: alert.rule_name,
            value: alert.value,
            threshold: alert.threshold,
        }).collect(),
        trends: trends.into_iter().map(|trend| Trend {
            metric: format!("{:?}", trend.metric),
            direction: match trend.direction {
                crate::realtime::analyzer::TrendDirection::Increasing => "increasing".to_string(),
                crate::realtime::analyzer::TrendDirection::Decreasing => "decreasing".to_string(),
                crate::realtime::analyzer::TrendDirection::Stable => "stable".to_string(),
            },
            change_rate: trend.rate_of_change,
            confidence: trend.confidence,
        }).collect(),
        anomalies: anomalies.into_iter().map(|anomaly| Anomaly {
            metric: format!("{:?}", anomaly.metric),
            value: anomaly.value,
            z_score: 0.0, // 임시값
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }).collect(),
        recent_entries: analyzer_guard.get_recent_entries().into_iter().take(10).map(|entry| RecentEntry {
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
            trace_type: entry.trace_type,
            latency: entry.latency.unwrap_or(0.0),
            operation: if entry.message.len() > 100 { 
                format!("{}...", &entry.message[..97]) 
            } else { 
                entry.message 
            },
        }).collect(),
        block_traces: block_traces.into_iter().take(1000).map(|trace| BlockTrace {
            timestamp: trace.time,
            lba: trace.sector,
            size: trace.size,
            io_type: trace.io_type,
            latency: trace.dtoc,
        }).collect(),
        ufs_traces: ufs_traces.into_iter().take(1000).map(|trace| UfsTrace {
            timestamp: trace.time,
            lba: trace.lba,
            size: trace.size,
            opcode: trace.opcode,
            latency: trace.dtoc,
        }).collect(),
        ufscustom_traces: ufscustom_traces.into_iter().take(1000).map(|trace| UfscustomTrace {
            timestamp: trace.start_time,
            lba: trace.lba,
            size: trace.size,
            opcode: trace.opcode,
            latency: trace.dtoc,
        }).collect(),
    };

    Ok(warp::reply::json(&dashboard_data))
}

#[allow(dead_code)]
async fn start_log_monitoring(
    log_file: String,
    monitor: Arc<RwLock<RealtimeMonitor>>,
    analyzer: Arc<RwLock<RealtimeAnalyzer>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(1000));

    loop {
        interval.tick().await;

        // 로그 파일 모니터링
        if let Ok(mut monitor_guard) = monitor.try_write() {
            if let Ok(new_entries) = monitor_guard.check_file_changes(&log_file) {
                if !new_entries.is_empty() {
                    drop(monitor_guard);
                    
                    // 분석기에 새 엔트리 추가
                    if let Ok(mut analyzer_guard) = analyzer.try_write() {
                        for entry in new_entries {
                            analyzer_guard.add_entry(entry);
                        }
                    }
                }
            }
        }
    }
}
