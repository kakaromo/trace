# 실시간 웹 모듈 구현 가이드

**작성일: 2025년 7월 8일**

## 목차

1. [소개](#1-소개)
2. [실시간 모듈 구현](#2-실시간-모듈-구현)
   - [2.1 모니터 구현](#21-모니터-구현)
   - [2.2 분석기 구현](#22-분석기-구현)
   - [2.3 대시보드 구현](#23-대시보드-구현)
   - [2.4 스트리밍 처리기 구현](#24-스트리밍-처리기-구현)
3. [웹 모듈 구현](#3-웹-모듈-구현)
   - [3.1 웹 서버 구현](#31-웹-서버-구현)
   - [3.2 웹소켓 구현](#32-웹소켓-구현)
   - [3.3 프론트엔드 구현](#33-프론트엔드-구현)
4. [구현 상세](#4-구현-상세)
5. [테스트 및 디버깅 가이드](#5-테스트-및-디버깅-가이드)

## 1. 소개

이 문서는 트레이스 프로젝트의 실시간 처리 및 웹 인터페이스 모듈의 구현 가이드입니다. 개발자가 코드를 이해하고 확장하는 데 도움을 주기 위한 상세 정보를 제공합니다.

## 2. 실시간 모듈 구현

### 2.1 모니터 구현

`monitor.rs` 모듈은 로그 데이터를 실시간으로 수집하고 기본 통계를 생성하는 역할을 담당합니다.

#### 주요 클래스 및 구조체

```rust
// 실시간 통계 구조체
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

// 파싱된 로그 항목 구조체
pub struct ParsedLogEntry {
    pub timestamp: String,
    pub level: String,
    pub process_id: u32,
    pub thread_id: u32,
    pub message: String,
    pub file_name: Option<String>,
    pub line_number: Option<u32>,
    pub latency: Option<f64>,
    pub trace_type: String,
}
```

#### 구현 고려사항

- `RealtimeStats`는 로그 처리 통계를 추적하고 업데이트함
- `add_entry()` 메서드를 통해 새 로그 항목이 처리될 때마다 통계가 업데이트됨
- 레벨별 카운트(error, warning, info, debug, trace)를 추적하여 로그 분포를 모니터링
- 성능 통계 수집은 오버헤드가 적도록 최적화되어야 함

### 2.2 분석기 구현

`analyzer.rs` 모듈은 수집된 로그 데이터를 분석하여 트렌드, 이상 징후 및 알림을 생성합니다.

#### 주요 클래스 및 구조체

```rust
// 실시간 로그 분석기
pub struct RealtimeAnalyzer {
    stats: Arc<Mutex<RealtimeStats>>,
    alert_rules: Vec<AlertRule>,
    metrics_history: Arc<Mutex<MetricsHistory>>,
    recent_entries: Arc<Mutex<VecDeque<ParsedLogEntry>>>,
    analysis_window: Duration,
    last_analysis: Instant,
}

// 알림 규칙
pub struct AlertRule {
    pub name: String,
    pub metric: MetricType,
    pub threshold: f64,
    pub comparison: ComparisonType,
    pub window_size: Duration,
    pub enabled: bool,
}
```

#### 알림 규칙 시스템

분석기는 다음과 같은 메트릭 유형에 대한 알림 규칙을 지원합니다:

1. `EntriesPerSecond`: 초당 로그 항목 수
2. `AverageLatency`: 평균 지연 시간
3. `MaxLatency`: 최대 지연 시간
4. `BlockRatio`: Block 트레이스 비율
5. `UFSRatio`: UFS 트레이스 비율
6. `UFSCustomRatio`: UFS Custom 트레이스 비율
7. `TotalEntries`: 총 항목 수

비교 유형은 다음과 같습니다:
- `GreaterThan`: 임계값보다 큼
- `LessThan`: 임계값보다 작음
- `Equal`: 임계값과 같음

#### 트렌드 분석 및 이상 탐지

분석기는 시계열 데이터를 분석하여 트렌드와 이상 징후를 탐지합니다:

1. **트렌드 분석**: 지난 시간 동안의 메트릭 변화를 추적하고 방향과 비율 계산
2. **이상 탐지**: Z-점수 및 다른 통계적 방법을 사용하여 비정상적인 값 식별

### 2.3 대시보드 구현

`dashboard.rs` 모듈은 터미널 기반 사용자 인터페이스를 제공합니다.

#### 주요 클래스 및 구조체

```rust
// 실시간 대시보드
pub struct RealtimeDashboard {
    analyzer: Arc<Mutex<RealtimeAnalyzer>>,
    monitor: Arc<Mutex<LogMonitor>>,
    display_config: DisplayConfig,
    is_running: bool,
    update_interval: Duration,
    last_update: Instant,
    shutdown_flag: Option<Arc<AtomicBool>>,
}

// 디스플레이 설정
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
```

#### 디스플레이 로직

대시보드는 다음 섹션을 표시할 수 있습니다:

1. 요약 통계 (총 항목, 초당 항목 수 등)
2. 알림 목록 (중요도별로 정렬)
3. 최근 탐지된 트렌드
4. 탐지된 이상 징후
5. 최근 로그 항목 샘플

### 2.4 스트리밍 처리기 구현

`streaming.rs` 모듈은 로그 파일을 비동기적으로 처리하는 스트리밍 파이프라인을 제공합니다.

#### 주요 클래스 및 구조체

```rust
pub struct StreamingProcessor {
    pub block_traces: Arc<Mutex<Vec<Block>>>,
    pub ufs_traces: Arc<Mutex<Vec<UFS>>>,
    pub ufscustom_traces: Arc<Mutex<Vec<UFSCUSTOM>>>,
    pub parsed_lines: Arc<Mutex<u64>>,
    pub last_processed_time: Arc<Mutex<Instant>>,
    pub output_prefix: Option<String>,
    pub initial_load_completed: Arc<Mutex<bool>>,
}
```

#### 스트리밍 처리 로직

1. **파일 모니터링**: 로그 파일을 지속적으로 모니터링하고 새 라인 감지
2. **배치 처리**: 최적화를 위해 라인을 배치로 처리
3. **병렬 처리**: 여러 작업을 비동기적으로 병렬 실행
4. **주기적 통계**: 10초마다 통계 업데이트

```rust
async fn start_streaming(&self, log_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel::<String>(10000);
    
    // 로그 파일 모니터링 태스크
    tokio::spawn(async move { /* ... */ });

    // 로그 파싱 태스크
    tokio::spawn(async move { /* ... */ });

    // 주기적 후처리 태스크 (1초마다)
    tokio::spawn(async move { /* ... */ });

    // 통계 태스크 (10초마다)
    tokio::spawn(async move { /* ... */ });

    // 메인 루프
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

## 3. 웹 모듈 구현

### 3.1 웹 서버 구현

`web/simple.rs` 모듈은 웹 서버와 API 엔드포인트를 제공합니다.

#### 주요 클래스 및 구조체

```rust
pub struct WebDashboard {
    analyzer: Arc<RwLock<RealtimeAnalyzer>>,
    monitor: Arc<RwLock<RealtimeMonitor>>,
    // 기타 필드
}
```

#### 엔드포인트 구현

웹 서버는 다음과 같은 주요 엔드포인트를 제공합니다:

1. **GET /**: 메인 대시보드 HTML 페이지
2. **GET /api/stats**: 현재 통계 정보를 JSON 형식으로 반환
3. **GET /ws**: 웹소켓 연결 엔드포인트
4. **GET /static/***:` 정적 자원 서빙 (CSS, JS 등)

### 3.2 웹소켓 구현

웹소켓은 클라이언트 브라우저와 서버 간의 양방향 통신을 제공합니다.

#### 메시지 구조

```rust
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}
```

#### 메시지 흐름

1. **연결 수립**: 클라이언트가 `/ws` 엔드포인트에 연결
2. **초기 데이터**: 서버가 현재 상태를 포함한 초기 데이터 전송
3. **지속적 업데이트**: 서버가 새 통계, 알림, 트렌드 등을 전송
4. **클라이언트 요청**: 클라이언트가 필터 변경, 설정 업데이트 등의 요청 전송

### 3.3 프론트엔드 구현

프론트엔드는 `/web/static/` 디렉토리에 있는 HTML, CSS, JavaScript 파일로 구성됩니다.

#### 주요 파일

1. **index.html**: 메인 대시보드 페이지
2. **dashboard.js**: 클라이언트 측 로직 및 웹소켓 처리
3. **echarts.min.js**: 데이터 시각화 라이브러리
4. **styles.css**: 스타일시트

#### 주요 기능

1. **실시간 차트**: ECharts를 사용한 시계열 데이터 시각화
2. **알림 패널**: 최근 알림 표시 및 필터링
3. **트렌드 뷰**: 탐지된 트렌드 표시
4. **로그 뷰어**: 최근 로그 항목 표시 및 필터링

## 4. 구현 상세

### 4.1 동시성 모델

실시간 및 웹 모듈은 다음과 같은 동시성 모델을 사용합니다:

1. **Tokio 비동기 런타임**: 효율적인 I/O 멀티플렉싱
2. **공유 상태**: `Arc<Mutex<T>>` 및 `Arc<RwLock<T>>`를 통한 스레드 안전 접근
3. **채널**: `mpsc` 및 기타 채널 타입을 통한 메시지 전달
4. **비동기 태스크**: `tokio::spawn`을 사용한 비동기 태스크 생성

### 4.2 성능 최적화

시스템은 다음과 같은 성능 최적화를 사용합니다:

1. **배치 처리**: 개별 로그 항목 대신 배치 처리
2. **효율적인 직렬화**: Serde를 사용한 효율적인 JSON 처리
3. **메모리 효율성**: 버퍼 크기 제한 및 재사용
4. **병렬 처리**: 다중 스레드를 활용한 처리 속도 향상

### 4.3 오류 처리

오류 처리는 다음 원칙을 따릅니다:

1. **Result 타입**: 대부분의 함수는 `Result<T, E>` 타입을 반환
2. **로깅**: 중요한 오류는 로그에 기록
3. **그레이스풀 디그레이드**: 오류 발생 시 가능한 부분 기능 유지
4. **재시도 메커니즘**: 일시적인 오류에 대한 자동 재시도

## 5. 테스트 및 디버깅 가이드

### 5.1 단위 테스트

주요 모듈은 단위 테스트를 포함해야 합니다:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realtime_stats() {
        let mut stats = RealtimeStats::new();
        // 테스트 로직
    }
    
    // 기타 테스트 함수
}
```

### 5.2 통합 테스트

웹 서버와 실시간 처리 모듈의 통합은 다음과 같이 테스트할 수 있습니다:

1. 테스트 로그 파일 준비
2. 웹 서버 및 실시간 처리 모듈 시작
3. API 엔드포인트 호출 및 응답 검증
4. 웹소켓 연결 및 메시지 검증

### 5.3 디버깅 기법

1. **로그 활성화**: `RUST_LOG=trace cargo run`
2. **메모리 프로파일링**: 메모리 사용량 모니터링
3. **웹소켓 디버깅**: 브라우저 개발자 도구 사용
4. **상태 덤프**: 현재 상태를 파일에 덤프하여 분석
