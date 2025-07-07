# 실시간 처리 및 웹 대시보드 설계 문서

**작성일: 2025년 7월 8일**

## 목차

1. [개요](#1-개요)
2. [아키텍처](#2-아키텍처)
3. [실시간 모듈 설계](#3-실시간-모듈-설계)
   - [3.1 모니터링 시스템](#31-모니터링-시스템)
   - [3.2 분석 엔진](#32-분석-엔진)
   - [3.3 대시보드](#33-대시보드)
   - [3.4 스트리밍 처리기](#34-스트리밍-처리기)
4. [웹 모듈 설계](#4-웹-모듈-설계)
   - [4.1 웹 서버 구조](#41-웹-서버-구조)
   - [4.2 웹소켓 통신](#42-웹소켓-통신)
   - [4.3 데이터 모델](#43-데이터-모델)
5. [시스템 통합](#5-시스템-통합)
6. [확장성 및 성능 최적화](#6-확장성-및-성능-최적화)
7. [기술 스택](#7-기술-스택)
8. [향후 개선점](#8-향후-개선점)

## 1. 개요

이 문서는 트레이스 프로젝트의 실시간 처리 모듈(`realtime`)과 웹 인터페이스 모듈(`web`)의 설계 및 구현에 대해 설명합니다. 이 시스템은 로그 데이터를 실시간으로 수집, 분석하고, 웹 기반 대시보드를 통해 시각화하는 기능을 제공합니다.

### 1.1 목적

- 로그 데이터를 실시간으로 처리하고 모니터링하는 기능 제공
- 사용자에게 직관적인 웹 인터페이스를 통한 데이터 시각화 제공
- 이상 징후 및 트렌드 실시간 탐지 및 알림 제공
- 데이터 스트리밍을 통한 지속적인 분석 지원

### 1.2 주요 기능

- 실시간 로그 모니터링 및 분석
- 웹 기반 대시보드
- 실시간 알림 및 이벤트 트래킹
- 데이터 스트리밍 및 처리
- 트렌드 분석 및 이상 탐지

## 2. 아키텍처

![아키텍처 다이어그램](../doc/images/realtime_web_architecture.png)

*참고: 위 이미지 경로는 생성해야 할 다이어그램 예시입니다*

### 2.1 전체 시스템 구조

트레이스 프로젝트의 실시간/웹 아키텍처는 다음과 같은 주요 구성 요소로 이루어져 있습니다:

1. **실시간 처리 모듈 (`realtime`)**
   - 모니터 (Monitor): 로그 데이터 수집 및 기본 통계 생성
   - 분석기 (Analyzer): 패턴 탐지, 이상 징후 감지, 트렌드 분석
   - 대시보드 (Dashboard): 텍스트 기반 터미널 UI 제공
   - 스트리밍 (Streaming): 지속적인 데이터 처리 파이프라인

2. **웹 모듈 (`web`)**
   - 간단한 웹 서버 (Simple): HTTP/웹소켓 엔드포인트 제공
   - 정적 자원 (Static): HTML, CSS, JavaScript 파일 제공

### 2.2 데이터 흐름

```
로그 파일 -> 로그 모니터 -> 스트리밍 처리기 -> 분석기 -> [웹 서버] -> 클라이언트 브라우저
                                        |
                                        v
                                     대시보드
```

## 3. 실시간 모듈 설계

### 3.1 모니터링 시스템

**주요 구성 요소:**

- `RealtimeStats`: 실시간 통계 정보를 저장하는 구조체
- `ParsedLogEntry`: 파싱된 로그 항목을 나타내는 구조체

```rust
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
```

**기능:**
- 로그 파일 실시간 모니터링
- 로그 항목 파싱 및 이벤트 생성
- 기본 통계 계산 (로그 레벨, 엔트리 수, 지연 시간 등)

### 3.2 분석 엔진

**주요 구성 요소:**

- `RealtimeAnalyzer`: 실시간 로그 분석 기능 제공
- `AlertRule`: 알림 규칙 정의
- `MetricsHistory`: 메트릭 기록 관리
- `AnalysisResult`: 분석 결과 저장

```rust
pub struct RealtimeAnalyzer {
    stats: Arc<Mutex<RealtimeStats>>,
    alert_rules: Vec<AlertRule>,
    metrics_history: Arc<Mutex<MetricsHistory>>,
    recent_entries: Arc<Mutex<VecDeque<ParsedLogEntry>>>,
    analysis_window: Duration,
    last_analysis: Instant,
}
```

**기능:**
- 알림 규칙 기반 모니터링
- 트렌드 분석 및 패턴 탐지
- 이상 징후 감지
- 통계적 분석 및 예측

### 3.3 대시보드

**주요 구성 요소:**

- `RealtimeDashboard`: 실시간 대시보드 관리
- `DisplayConfig`: 대시보드 표시 설정
- `DashboardData`: 대시보드에 표시할 데이터

```rust
pub struct RealtimeDashboard {
    analyzer: Arc<Mutex<RealtimeAnalyzer>>,
    monitor: Arc<Mutex<LogMonitor>>,
    display_config: DisplayConfig,
    is_running: bool,
    update_interval: Duration,
    last_update: Instant,
    shutdown_flag: Option<Arc<AtomicBool>>,
}
```

**기능:**
- 텍스트 기반 터미널 UI
- 실시간 통계 표시
- 알림 및 이벤트 표시
- 구성 가능한 디스플레이 옵션

### 3.4 스트리밍 처리기

**주요 구성 요소:**

- `StreamingProcessor`: 지속적인 데이터 스트림 처리

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

**기능:**
- 비동기 로그 파일 모니터링
- 배치 처리 및 성능 최적화
- 타입별 트레이스 데이터 분류 (Block, UFS, UFSCUSTOM)
- 주기적인 통계 생성

## 4. 웹 모듈 설계

### 4.1 웹 서버 구조

**주요 구성 요소:**

- `WebDashboard`: 웹 대시보드 관리
- `warp`: 경량 웹 서버 프레임워크

**엔드포인트:**
- `/`: 메인 대시보드 페이지
- `/api/stats`: 실시간 통계 API
- `/ws`: 웹소켓 엔드포인트
- `/static/*`: 정적 파일 서빙

### 4.2 웹소켓 통신

**주요 구성 요소:**

- `WebSocketMessage`: 클라이언트-서버 간 메시지 구조

```rust
pub struct WebSocketMessage {
    pub message_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
}
```

**메시지 타입:**
- `stats_update`: 통계 업데이트
- `alert`: 새 알림
- `trend`: 트렌드 정보
- `anomaly`: 이상 징후 감지
- `trace`: 개별 트레이스 정보

### 4.3 데이터 모델

**주요 데이터 구조체:**

- `DashboardData`: 대시보드에 표시될 전체 데이터
- `Alert`: 알림 정보
- `Trend`: 트렌드 정보
- `Anomaly`: 이상 징후 정보
- `RecentEntry`: 최근 로그 항목
- `BlockTrace`, `UfsTrace`, `UfscustomTrace`: 타입별 트레이스 정보

```rust
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
```

## 5. 시스템 통합

### 5.1 실시간 모듈과 웹 모듈 통합

두 모듈은 다음과 같이 통합됩니다:

1. `WebDashboard`는 `RealtimeAnalyzer`와 `RealtimeMonitor`의 인스턴스를 소유
2. 웹 서버는 실시간 분석 엔진으로부터 정기적으로 데이터를 폴링
3. 분석 결과는 웹소켓을 통해 클라이언트에게 실시간으로 전송
4. 클라이언트 요청은 웹 서버를 통해 실시간 분석 엔진으로 전달

### 5.2 동시성 및 스레드 안전성

- `Arc<Mutex<T>>` 및 `Arc<RwLock<T>>`를 사용하여 스레드 안전성 보장
- Tokio를 활용한 비동기 처리
- 스레드 간 통신에 채널 패턴 활용

## 6. 확장성 및 성능 최적화

### 6.1 확장성

- 모듈화된 구조로 개별 구성 요소 교체 용이
- 설정 가능한 매개변수를 통한 유연한 시스템 구성
- 다양한 로그 형식 지원 확장성

### 6.2 성능 최적화

- 배치 처리를 통한 I/O 최적화
- 메모리 효율성을 위한 순환 버퍼 사용
- 비동기 프로그래밍 모델 활용
- 효율적인 직렬화/역직렬화

## 7. 기술 스택

- **언어**: Rust
- **비동기 런타임**: Tokio
- **웹 프레임워크**: Warp
- **직렬화/역직렬화**: Serde
- **프론트엔드**: HTML, CSS, JavaScript, ECharts
- **동시성 관리**: Arc, Mutex, RwLock, AtomicBool
- **채널**: mpsc, oneshot

## 8. 향후 개선점

### 8.1 기능 개선

- 사용자 인증 및 권한 관리
- 커스텀 대시보드 및 위젯
- 알림 구독 및 전달 메커니즘
- 과거 데이터 쿼리 및 분석

### 8.2 기술 개선

- GraphQL API 도입 고려
- 분산 로그 수집 지원
- WebAssembly를 통한 클라이언트 측 계산
- 데이터베이스 통합 및 영구 저장소
