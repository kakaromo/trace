use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::realtime::monitor::{ParsedLogEntry, RealtimeStats};

/// 실시간 로그 분석기
pub struct RealtimeAnalyzer {
    stats: Arc<Mutex<RealtimeStats>>,
    alert_rules: Vec<AlertRule>,
    metrics_history: Arc<Mutex<MetricsHistory>>,
    recent_entries: Arc<Mutex<VecDeque<ParsedLogEntry>>>,
    analysis_window: Duration,
    last_analysis: Instant,
}

/// 알림 규칙
#[derive(Debug, Clone)]
pub struct AlertRule {
    pub name: String,
    pub metric: MetricType,
    pub threshold: f64,
    pub comparison: ComparisonType,
    pub window_size: Duration,
    pub enabled: bool,
}

/// 메트릭 타입
#[derive(Debug, Clone, PartialEq)]
pub enum MetricType {
    EntriesPerSecond,
    AverageLatency,
    MaxLatency,
    BlockRatio,
    UFSRatio,
    UFSCustomRatio,
    TotalEntries,
}

/// 비교 타입
#[derive(Debug, Clone, PartialEq)]
pub enum ComparisonType {
    GreaterThan,
    LessThan,
    Equal,
}

/// 메트릭 히스토리
#[derive(Debug)]
pub struct MetricsHistory {
    pub timestamps: VecDeque<Instant>,
    pub entries_per_second: VecDeque<f64>,
    pub avg_latency: VecDeque<f64>,
    pub max_latency: VecDeque<f64>,
    pub block_count: VecDeque<usize>,
    pub ufs_count: VecDeque<usize>,
    pub ufscustom_count: VecDeque<usize>,
    pub total_entries: VecDeque<usize>,
    pub max_history_size: usize,
}

/// 분석 결과
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    pub timestamp: Instant,
    pub current_stats: RealtimeStats,
    pub alerts: Vec<Alert>,
    pub trends: Vec<Trend>,
    pub anomalies: Vec<Anomaly>,
}

/// 알림
#[derive(Debug, Clone)]
pub struct Alert {
    pub rule_name: String,
    pub message: String,
    pub severity: AlertSeverity,
    pub timestamp: Instant,
    pub value: f64,
    pub threshold: f64,
}

/// 알림 심각도
#[derive(Debug, Clone, PartialEq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// 트렌드
#[derive(Debug, Clone)]
pub struct Trend {
    pub metric: MetricType,
    pub direction: TrendDirection,
    pub rate_of_change: f64,
    pub confidence: f64,
}

/// 트렌드 방향
#[derive(Debug, Clone, PartialEq)]
pub enum TrendDirection {
    Increasing,
    Decreasing,
    Stable,
}

/// 이상 징후
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub metric: MetricType,
    pub description: String,
    pub severity: AlertSeverity,
    pub timestamp: Instant,
    pub value: f64,
    pub expected_range: (f64, f64),
}

impl RealtimeAnalyzer {
    /// 새로운 실시간 분석기 생성
    pub fn new(analysis_window: Duration) -> Self {
        RealtimeAnalyzer {
            stats: Arc::new(Mutex::new(RealtimeStats::new())),
            alert_rules: Vec::new(),
            metrics_history: Arc::new(Mutex::new(MetricsHistory::new())),
            recent_entries: Arc::new(Mutex::new(VecDeque::new())),
            analysis_window,
            last_analysis: Instant::now(),
        }
    }

    /// 기본 알림 규칙들과 함께 분석기 생성
    pub fn with_default_rules(analysis_window: Duration) -> Self {
        let mut analyzer = Self::new(analysis_window);
        analyzer.add_default_alert_rules();
        analyzer
    }

    /// 기본 알림 규칙 추가
    pub fn add_default_alert_rules(&mut self) {
        // 높은 레이턴시 알림
        self.add_alert_rule(AlertRule {
            name: "High Average Latency".to_string(),
            metric: MetricType::AverageLatency,
            threshold: 100.0, // 100ms
            comparison: ComparisonType::GreaterThan,
            window_size: Duration::from_secs(30),
            enabled: true,
        });

        // 매우 높은 레이턴시 알림
        self.add_alert_rule(AlertRule {
            name: "Very High Max Latency".to_string(),
            metric: MetricType::MaxLatency,
            threshold: 1000.0, // 1초
            comparison: ComparisonType::GreaterThan,
            window_size: Duration::from_secs(10),
            enabled: true,
        });

        // 낮은 처리량 알림
        self.add_alert_rule(AlertRule {
            name: "Low Throughput".to_string(),
            metric: MetricType::EntriesPerSecond,
            threshold: 10.0, // 10 entries/sec
            comparison: ComparisonType::LessThan,
            window_size: Duration::from_secs(60),
            enabled: true,
        });

        // 높은 처리량 알림 (잠재적 문제)
        self.add_alert_rule(AlertRule {
            name: "Very High Throughput".to_string(),
            metric: MetricType::EntriesPerSecond,
            threshold: 10000.0, // 10,000 entries/sec
            comparison: ComparisonType::GreaterThan,
            window_size: Duration::from_secs(30),
            enabled: true,
        });
    }

    /// 알림 규칙 추가
    pub fn add_alert_rule(&mut self, rule: AlertRule) {
        self.alert_rules.push(rule);
    }

    /// 알림 규칙 제거
    pub fn remove_alert_rule(&mut self, rule_name: &str) {
        self.alert_rules.retain(|rule| rule.name != rule_name);
    }

    /// 로그 엔트리 처리
    pub fn process_entry(&mut self, entry: &ParsedLogEntry) {
        // 통계 업데이트
        if let Ok(mut stats) = self.stats.lock() {
            stats.add_entry(entry);
        }

        // 분석 윈도우가 지났으면 분석 수행
        if self.last_analysis.elapsed() >= self.analysis_window {
            self.update_metrics_history();
            self.last_analysis = Instant::now();
        }
    }

    /// 분석 수행
    pub fn analyze(&self) -> AnalysisResult {
        let current_stats = if let Ok(stats) = self.stats.lock() {
            stats.clone()
        } else {
            RealtimeStats::new()
        };

        let alerts = self.check_alerts(&current_stats);
        let trends = self.analyze_trends();
        let anomalies = self.detect_anomalies(&current_stats);

        AnalysisResult {
            timestamp: Instant::now(),
            current_stats,
            alerts,
            trends,
            anomalies,
        }
    }

    /// 현재 통계 가져오기
    pub fn get_current_stats(&self) -> crate::realtime::monitor::RealtimeStats {
        if let Ok(stats) = self.stats.lock() {
            stats.clone()
        } else {
            crate::realtime::monitor::RealtimeStats::new()
        }
    }

    /// 활성 알림 가져오기
    pub fn get_active_alerts(&self) -> Vec<Alert> {
        // 임시로 빈 벡터 반환 (실제 구현에서는 알림 저장소에서 가져옴)
        Vec::new()
    }

    /// 트렌드 가져오기
    pub fn get_trends(&self) -> Vec<Trend> {
        // 임시로 빈 벡터 반환 (실제 구현에서는 트렌드 계산)
        Vec::new()
    }

    /// 이상 징후 가져오기
    pub fn get_anomalies(&self) -> Vec<Anomaly> {
        // 임시로 빈 벡터 반환 (실제 구현에서는 이상 징후 탐지)
        Vec::new()
    }

    /// 로그 엔트리 추가
    pub fn add_entry(&mut self, entry: crate::realtime::monitor::ParsedLogEntry) {
        // 통계 업데이트
        if let Ok(mut stats) = self.stats.lock() {
            stats.add_entry(&entry);
        }
        
        // 최근 엔트리에 추가
        if let Ok(mut recent) = self.recent_entries.lock() {
            recent.push_back(entry.clone());
            // 최대 100개 엔트리만 유지
            if recent.len() > 100 {
                recent.pop_front();
            }
        }
        
        // 메트릭 히스토리에 추가
        if let Ok(mut history) = self.metrics_history.lock() {
            let current_stats = if let Ok(stats) = self.stats.lock() {
                stats.clone()
            } else {
                return;
            };
            history.add_snapshot(&current_stats);
        }
        
        println!("📊 엔트리 추가됨: {} - {}", entry.level, entry.trace_type);
    }

    /// 메트릭 히스토리 가져오기
    pub fn get_metrics_history(&self) -> MetricsHistory {
        if let Ok(history) = self.metrics_history.lock() {
            history.clone()
        } else {
            MetricsHistory::new()
        }
    }

    /// 통계 리셋
    pub fn reset_stats(&mut self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.reset();
        }
        if let Ok(mut history) = self.metrics_history.lock() {
            history.clear();
        }
    }

    /// 메트릭 히스토리 업데이트
    fn update_metrics_history(&self) {
        if let (Ok(stats), Ok(mut history)) = (self.stats.lock(), self.metrics_history.lock()) {
            history.add_snapshot(&stats);
        }
    }

    /// 알림 체크
    fn check_alerts(&self, stats: &RealtimeStats) -> Vec<Alert> {
        let mut alerts = Vec::new();

        for rule in &self.alert_rules {
            if !rule.enabled {
                continue;
            }

            let current_value = match rule.metric {
                MetricType::EntriesPerSecond => stats.entries_per_second,
                MetricType::AverageLatency => stats.average_latency,
                MetricType::MaxLatency => stats.max_latency,
                MetricType::BlockRatio => {
                    // Since block_entries field doesn't exist, using a placeholder calculation
                    if stats.total_entries > 0 {
                        // Using error_count as a proxy for block entries temporarily
                        stats.error_count as f64 / stats.total_entries as f64
                    } else {
                        0.0
                    }
                }
                MetricType::UFSRatio => {
                    if stats.total_entries > 0 {
                        stats.info_count as f64 / stats.total_entries as f64
                    } else {
                        0.0
                    }
                }
                MetricType::UFSCustomRatio => {
                    if stats.total_entries > 0 {
                        stats.debug_count as f64 / stats.total_entries as f64
                    } else {
                        0.0
                    }
                }
                MetricType::TotalEntries => stats.total_entries as f64,
            };

            let triggered = match rule.comparison {
                ComparisonType::GreaterThan => current_value > rule.threshold,
                ComparisonType::LessThan => current_value < rule.threshold,
                ComparisonType::Equal => (current_value - rule.threshold).abs() < f64::EPSILON,
            };

            if triggered {
                let severity = if current_value > rule.threshold * 2.0 {
                    AlertSeverity::Critical
                } else if current_value > rule.threshold * 1.5 {
                    AlertSeverity::Warning
                } else {
                    AlertSeverity::Info
                };

                alerts.push(Alert {
                    rule_name: rule.name.clone(),
                    message: format!(
                        "{}: {} is {:.2} (threshold: {:.2})",
                        rule.name, 
                        format_metric_type(&rule.metric),
                        current_value,
                        rule.threshold
                    ),
                    severity,
                    timestamp: Instant::now(),
                    value: current_value,
                    threshold: rule.threshold,
                });
            }
        }

        alerts
    }

    /// 트렌드 분석
    fn analyze_trends(&self) -> Vec<Trend> {
        let mut trends = Vec::new();

        if let Ok(history) = self.metrics_history.lock() {
            // 최근 10개 데이터 포인트로 트렌드 분석
            let window_size = 10.min(history.timestamps.len());
            if window_size < 3 {
                return trends; // 충분한 데이터가 없음
            }

            // 엔트리 처리율 트렌드
            if let Some(trend) = self.calculate_trend(&history.entries_per_second, window_size) {
                trends.push(Trend {
                    metric: MetricType::EntriesPerSecond,
                    direction: trend.0,
                    rate_of_change: trend.1,
                    confidence: trend.2,
                });
            }

            // 평균 레이턴시 트렌드
            if let Some(trend) = self.calculate_trend(&history.avg_latency, window_size) {
                trends.push(Trend {
                    metric: MetricType::AverageLatency,
                    direction: trend.0,
                    rate_of_change: trend.1,
                    confidence: trend.2,
                });
            }

            // 최대 레이턴시 트렌드
            if let Some(trend) = self.calculate_trend(&history.max_latency, window_size) {
                trends.push(Trend {
                    metric: MetricType::MaxLatency,
                    direction: trend.0,
                    rate_of_change: trend.1,
                    confidence: trend.2,
                });
            }
        }

        trends
    }

    /// 트렌드 계산 (선형 회귀 사용)
    fn calculate_trend(&self, data: &VecDeque<f64>, window_size: usize) -> Option<(TrendDirection, f64, f64)> {
        if data.len() < window_size || window_size < 2 {
            return None;
        }

        let recent_data: Vec<f64> = data.iter().rev().take(window_size).cloned().collect();
        let n = recent_data.len() as f64;
        
        // 선형 회귀로 기울기 계산
        let x_sum: f64 = (0..recent_data.len()).map(|i| i as f64).sum();
        let y_sum: f64 = recent_data.iter().sum();
        let xy_sum: f64 = recent_data.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let x_squared_sum: f64 = (0..recent_data.len()).map(|i| (i as f64).powi(2)).sum();
        
        let slope = (n * xy_sum - x_sum * y_sum) / (n * x_squared_sum - x_sum.powi(2));
        
        // 결정계수 계산 (신뢰도)
        let y_mean = y_sum / n;
        let ss_tot: f64 = recent_data.iter().map(|&y| (y - y_mean).powi(2)).sum();
        let ss_res: f64 = recent_data.iter().enumerate()
            .map(|(i, &y)| {
                let predicted = slope * i as f64 + (y_sum - slope * x_sum) / n;
                (y - predicted).powi(2)
            })
            .sum();
        
        let r_squared = 1.0 - (ss_res / ss_tot);
        
        let direction = if slope > 0.01 {
            TrendDirection::Increasing
        } else if slope < -0.01 {
            TrendDirection::Decreasing
        } else {
            TrendDirection::Stable
        };

        Some((direction, slope, r_squared.clamp(0.0, 1.0)))
    }

    /// 이상 징후 감지
    fn detect_anomalies(&self, stats: &RealtimeStats) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        if let Ok(history) = self.metrics_history.lock() {
            // 평균 레이턴시 이상치 감지
            if let Some(anomaly) = self.detect_statistical_anomaly(
                &history.avg_latency,
                stats.average_latency,
                MetricType::AverageLatency,
            ) {
                anomalies.push(anomaly);
            }

            // 처리량 이상치 감지
            if let Some(anomaly) = self.detect_statistical_anomaly(
                &history.entries_per_second,
                stats.entries_per_second,
                MetricType::EntriesPerSecond,
            ) {
                anomalies.push(anomaly);
            }
        }

        anomalies
    }

    /// 통계적 이상치 감지 (Z-score 방법)
    fn detect_statistical_anomaly(
        &self,
        history: &VecDeque<f64>,
        current_value: f64,
        metric: MetricType,
    ) -> Option<Anomaly> {
        if history.len() < 10 {
            return None; // 충분한 히스토리가 없음
        }

        let values: Vec<f64> = history.iter().cloned().collect();
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev < f64::EPSILON {
            return None; // 표준편차가 0에 가까움
        }

        let z_score = (current_value - mean) / std_dev;
        let threshold = 2.0; // Z-score 임계값

        if z_score.abs() > threshold {
            let severity = if z_score.abs() > 3.0 {
                AlertSeverity::Critical
            } else {
                AlertSeverity::Warning
            };        Some(Anomaly {
            metric: metric.clone(),
            description: format!(
                "{} anomaly detected: current={:.2}, mean={:.2}, z-score={:.2}",
                format_metric_type(&metric),
                current_value,
                mean,
                z_score
            ),
            severity,
            timestamp: Instant::now(),
            value: current_value,
            expected_range: (mean - 2.0 * std_dev, mean + 2.0 * std_dev),
        })
        } else {
            None
        }
    }

    /// 최근 엔트리 가져오기
    pub fn get_recent_entries(&self) -> Vec<crate::realtime::monitor::ParsedLogEntry> {
        if let Ok(recent) = self.recent_entries.lock() {
            recent.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

impl MetricsHistory {
    /// 새로운 메트릭 히스토리 생성
    pub fn new() -> Self {
        MetricsHistory {
            timestamps: VecDeque::new(),
            entries_per_second: VecDeque::new(),
            avg_latency: VecDeque::new(),
            max_latency: VecDeque::new(),
            block_count: VecDeque::new(),
            ufs_count: VecDeque::new(),
            ufscustom_count: VecDeque::new(),
            total_entries: VecDeque::new(),
            max_history_size: 1000, // 최대 1000개 데이터 포인트
        }
    }

    /// 통계 스냅샷 추가
    pub fn add_snapshot(&mut self, stats: &RealtimeStats) {
        let now = Instant::now();
        
        self.timestamps.push_back(now);
        self.entries_per_second.push_back(stats.entries_per_second);
        self.avg_latency.push_back(stats.average_latency);
        self.max_latency.push_back(stats.max_latency);
        self.block_count.push_back(stats.total_entries as usize); // 임시로 total_entries 사용
        self.ufs_count.push_back(stats.info_count as usize);
        self.ufscustom_count.push_back(stats.debug_count as usize);
        self.total_entries.push_back(stats.total_entries.try_into().unwrap());

        // 최대 크기 초과 시 오래된 데이터 제거
        while self.timestamps.len() > self.max_history_size {
            self.timestamps.pop_front();
            self.entries_per_second.pop_front();
            self.avg_latency.pop_front();
            self.max_latency.pop_front();
            self.block_count.pop_front();
            self.ufs_count.pop_front();
            self.ufscustom_count.pop_front();
            self.total_entries.pop_front();
        }
    }

    /// 히스토리 클리어
    pub fn clear(&mut self) {
        self.timestamps.clear();
        self.entries_per_second.clear();
        self.avg_latency.clear();
        self.max_latency.clear();
        self.block_count.clear();
        self.ufs_count.clear();
        self.ufscustom_count.clear();
        self.total_entries.clear();
    }

    /// 히스토리 크기 가져오기
    pub fn len(&self) -> usize {
        self.timestamps.len()
    }

    /// 히스토리가 비어있는지 확인
    pub fn is_empty(&self) -> bool {
        self.timestamps.is_empty()
    }
}

impl Clone for MetricsHistory {
    fn clone(&self) -> Self {
        MetricsHistory {
            timestamps: self.timestamps.clone(),
            entries_per_second: self.entries_per_second.clone(),
            avg_latency: self.avg_latency.clone(),
            max_latency: self.max_latency.clone(),
            block_count: self.block_count.clone(),
            ufs_count: self.ufs_count.clone(),
            ufscustom_count: self.ufscustom_count.clone(),
            total_entries: self.total_entries.clone(),
            max_history_size: self.max_history_size,
        }
    }
}

impl Default for MetricsHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// 메트릭 타입을 문자열로 포맷
fn format_metric_type(metric: &MetricType) -> &'static str {
    match metric {
        MetricType::EntriesPerSecond => "Entries/Second",
        MetricType::AverageLatency => "Average Latency",
        MetricType::MaxLatency => "Max Latency",
        MetricType::BlockRatio => "Block Ratio",
        MetricType::UFSRatio => "UFS Ratio",
        MetricType::UFSCustomRatio => "UFS Custom Ratio",
        MetricType::TotalEntries => "Total Entries",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realtime_analyzer_creation() {
        let analyzer = RealtimeAnalyzer::new(Duration::from_secs(5));
        assert_eq!(analyzer.analysis_window, Duration::from_secs(5));
        assert_eq!(analyzer.alert_rules.len(), 0);
    }

    #[test]
    fn test_default_rules() {
        let analyzer = RealtimeAnalyzer::with_default_rules(Duration::from_secs(5));
        assert!(analyzer.alert_rules.len() > 0);
    }

    #[test]
    fn test_metrics_history() {
        let mut history = MetricsHistory::new();
        let stats = RealtimeStats::new();
        
        history.add_snapshot(&stats);
        assert_eq!(history.len(), 1);
        
        history.clear();
        assert_eq!(history.len(), 0);
        assert!(history.is_empty());
    }
}
