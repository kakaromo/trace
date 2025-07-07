# 실시간 웹 API 레퍼런스

**작성일: 2025년 7월 8일**

## 목차

1. [개요](#1-개요)
2. [REST API](#2-rest-api)
   - [2.1 통계 API](#21-통계-api)
   - [2.2 알림 API](#22-알림-api)
   - [2.3 트렌드 API](#23-트렌드-api)
   - [2.4 트레이스 API](#24-트레이스-api)
   - [2.5 설정 API](#25-설정-api)
3. [웹소켓 API](#3-웹소켓-api)
   - [3.1 연결 설정](#31-연결-설정)
   - [3.2 서버에서 클라이언트로의 메시지](#32-서버에서-클라이언트로의-메시지)
   - [3.3 클라이언트에서 서버로의 메시지](#33-클라이언트에서-서버로의-메시지)
4. [데이터 모델](#4-데이터-모델)
   - [4.1 통계 모델](#41-통계-모델)
   - [4.2 알림 모델](#42-알림-모델)
   - [4.3 트렌드 모델](#43-트렌드-모델)
   - [4.4 이상 탐지 모델](#44-이상-탐지-모델)
   - [4.5 트레이스 모델](#45-트레이스-모델)
5. [오류 처리](#5-오류-처리)
6. [인증 및 권한](#6-인증-및-권한)
7. [제한 사항](#7-제한-사항)

## 1. 개요

트레이스 프로젝트의 실시간 웹 API는 로그 데이터에 대한 실시간 분석 결과와 통계 정보에 액세스할 수 있는 기능을 제공합니다. API는 REST 엔드포인트와 웹소켓 인터페이스로 구성됩니다.

### 1.1 기본 URL

```
http://localhost:3000/api
```

### 1.2 인증

현재 버전에서는 별도의 인증 메커니즘이 구현되어 있지 않습니다. 향후 버전에서 인증 기능을 추가할 계획입니다.

## 2. REST API

### 2.1 통계 API

#### 현재 통계 가져오기

**요청:**

```
GET /api/stats
```

**응답:**

```json
{
  "total_entries": 123456,
  "processing_rate": 1234.5,
  "avg_latency": 45.67,
  "max_latency": 120.5,
  "min_latency": 0.8,
  "block_count": 8976,
  "ufs_count": 5432,
  "custom_count": 1234,
  "timestamp": 1657324800
}
```

#### 통계 이력 가져오기

**요청:**

```
GET /api/stats/history?duration=1h&resolution=5m
```

**매개변수:**

- `duration`: 이력 데이터 기간 (예: 1h, 24h, 7d)
- `resolution`: 데이터 포인트 해상도 (예: 1s, 1m, 5m)

**응답:**

```json
{
  "timestamps": [1657324500, 1657324800, 1657325100, ...],
  "total_entries": [123000, 123456, 123789, ...],
  "processing_rate": [1200, 1234.5, 1250, ...],
  "avg_latency": [44.3, 45.67, 46.1, ...],
  "max_latency": [110.2, 120.5, 115.7, ...],
  "min_latency": [0.7, 0.8, 0.9, ...],
  "block_count": [8900, 8976, 9012, ...],
  "ufs_count": [5400, 5432, 5465, ...],
  "custom_count": [1200, 1234, 1256, ...]
}
```

### 2.2 알림 API

#### 최근 알림 가져오기

**요청:**

```
GET /api/alerts?limit=10&severity=warning,critical
```

**매개변수:**

- `limit`: 반환할 최대 알림 수 (기본값: 10)
- `severity`: 필터링할 알림 심각도 (쉼표로 구분, 기본값: 모든 심각도)

**응답:**

```json
{
  "alerts": [
    {
      "id": "a1b2c3d4",
      "severity": "critical",
      "message": "최대 지연 시간이 임계값을 초과했습니다",
      "timestamp": 1657324800,
      "metric": "max_latency",
      "value": 250.5,
      "threshold": 200.0
    },
    {
      "id": "e5f6g7h8",
      "severity": "warning",
      "message": "처리 속도가 감소 추세입니다",
      "timestamp": 1657324500,
      "metric": "processing_rate",
      "value": 800.3,
      "threshold": 1000.0
    }
  ],
  "total": 25
}
```

### 2.3 트렌드 API

#### 현재 트렌드 가져오기

**요청:**

```
GET /api/trends?metrics=processing_rate,avg_latency
```

**매개변수:**

- `metrics`: 필터링할 메트릭 이름 (쉼표로 구분, 기본값: 모든 메트릭)

**응답:**

```json
{
  "trends": [
    {
      "metric": "processing_rate",
      "direction": "decreasing",
      "change_rate": -5.2,
      "confidence": 0.87
    },
    {
      "metric": "avg_latency",
      "direction": "increasing",
      "change_rate": 2.1,
      "confidence": 0.76
    }
  ]
}
```

### 2.4 트레이스 API

#### 최근 트레이스 가져오기

**요청:**

```
GET /api/traces?type=block&limit=5
```

**매개변수:**

- `type`: 트레이스 타입 (`block`, `ufs`, `ufscustom`, 기본값: 모든 타입)
- `limit`: 반환할 최대 트레이스 수 (기본값: 10)

**응답:**

```json
{
  "traces": [
    {
      "timestamp": 1657324800.123,
      "trace_type": "block",
      "lba": 123456789,
      "size": 4096,
      "io_type": "read",
      "latency": 12.5
    },
    {
      "timestamp": 1657324799.987,
      "trace_type": "block",
      "lba": 123456790,
      "size": 8192,
      "io_type": "write",
      "latency": 18.2
    }
  ],
  "total": 1000
}
```

### 2.5 설정 API

#### 현재 설정 가져오기

**요청:**

```
GET /api/settings
```

**응답:**

```json
{
  "alert_rules": [
    {
      "id": "rule1",
      "name": "High Latency Alert",
      "metric": "max_latency",
      "threshold": 200.0,
      "comparison": "greater_than",
      "window_size": 60,
      "enabled": true
    }
  ],
  "display_options": {
    "show_alerts": true,
    "show_trends": true,
    "show_anomalies": true,
    "refresh_rate": 5
  }
}
```

#### 설정 업데이트하기

**요청:**

```
PUT /api/settings
Content-Type: application/json

{
  "display_options": {
    "show_alerts": false,
    "refresh_rate": 10
  }
}
```

**응답:**

```json
{
  "success": true,
  "message": "설정이 업데이트되었습니다"
}
```

## 3. 웹소켓 API

### 3.1 연결 설정

웹소켓 연결 URL:

```
ws://localhost:3000/ws
```

연결 후 서버는 초기 상태를 전송합니다.

### 3.2 서버에서 클라이언트로의 메시지

서버는 다음과 같은 유형의 메시지를 클라이언트에게 전송합니다:

#### 통계 업데이트

```json
{
  "message_type": "stats_update",
  "data": {
    "total_entries": 123456,
    "processing_rate": 1234.5,
    "avg_latency": 45.67,
    "max_latency": 120.5,
    "min_latency": 0.8,
    "block_count": 8976,
    "ufs_count": 5432,
    "custom_count": 1234
  },
  "timestamp": 1657324800
}
```

#### 알림

```json
{
  "message_type": "alert",
  "data": {
    "id": "a1b2c3d4",
    "severity": "critical",
    "message": "최대 지연 시간이 임계값을 초과했습니다",
    "metric": "max_latency",
    "value": 250.5,
    "threshold": 200.0
  },
  "timestamp": 1657324800
}
```

#### 트렌드

```json
{
  "message_type": "trend",
  "data": {
    "metric": "processing_rate",
    "direction": "decreasing",
    "change_rate": -5.2,
    "confidence": 0.87
  },
  "timestamp": 1657324800
}
```

#### 이상 탐지

```json
{
  "message_type": "anomaly",
  "data": {
    "metric": "avg_latency",
    "value": 85.2,
    "z_score": 3.6
  },
  "timestamp": 1657324800
}
```

#### 트레이스

```json
{
  "message_type": "trace",
  "data": {
    "trace_type": "block",
    "lba": 123456789,
    "size": 4096,
    "io_type": "read",
    "latency": 12.5
  },
  "timestamp": 1657324800.123
}
```

### 3.3 클라이언트에서 서버로의 메시지

클라이언트는 다음과 같은 유형의 메시지를 서버에 전송할 수 있습니다:

#### 구독 관리

```json
{
  "message_type": "subscribe",
  "data": {
    "topics": ["stats", "alerts", "traces"],
    "filters": {
      "traces": {
        "types": ["block"],
        "min_latency": 10.0
      },
      "alerts": {
        "severities": ["warning", "critical"]
      }
    }
  }
}
```

#### 필터 업데이트

```json
{
  "message_type": "update_filters",
  "data": {
    "traces": {
      "types": ["block", "ufs"],
      "min_latency": 5.0,
      "max_latency": 50.0
    }
  }
}
```

#### 명령 실행

```json
{
  "message_type": "command",
  "data": {
    "command": "reset_stats"
  }
}
```

## 4. 데이터 모델

### 4.1 통계 모델

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

### 4.2 알림 모델

```rust
pub struct Alert {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub timestamp: u64,
    pub metric: String,
    pub value: f64,
    pub threshold: f64,
}
```

### 4.3 트렌드 모델

```rust
pub struct Trend {
    pub metric: String,
    pub direction: String,
    pub change_rate: f64,
    pub confidence: f64,
}
```

### 4.4 이상 탐지 모델

```rust
pub struct Anomaly {
    pub metric: String,
    pub value: f64,
    pub z_score: f64,
    pub timestamp: u64,
}
```

### 4.5 트레이스 모델

```rust
pub struct BlockTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub io_type: String,
    pub latency: f64,
}

pub struct UfsTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub opcode: String,
    pub latency: f64,
}

pub struct UfscustomTrace {
    pub timestamp: f64,
    pub lba: u64,
    pub size: u32,
    pub opcode: String,
    pub latency: f64,
}
```

## 5. 오류 처리

API 응답은 오류가 발생할 경우 다음 형식을 따릅니다:

```json
{
  "error": true,
  "code": "rate_limit_exceeded",
  "message": "API 요청 한도를 초과했습니다.",
  "details": {
    "limit": 100,
    "reset_time": 1657324900
  }
}
```

### 5.1 오류 코드

- `invalid_request`: 요청 형식이 잘못됨
- `not_found`: 요청한 리소스를 찾을 수 없음
- `internal_error`: 서버 내부 오류
- `rate_limit_exceeded`: API 요청 한도 초과
- `validation_error`: 입력 데이터가 유효하지 않음

## 6. 인증 및 권한

현재 버전에서는 간단한 액세스를 위한 기본 인증만 지원합니다. 향후 버전에서는 OAuth 또는 JWT 기반 인증이 추가될 예정입니다.

## 7. 제한 사항

현재 API 구현에는 다음과 같은 제한 사항이 있습니다:

- 동시 웹소켓 연결 수: 최대 100개
- REST API 요청 빈도: 분당 300개 요청
- 웹소켓 메시지 크기: 최대 1MB
- 히스토리 데이터 요청: 최대 7일
