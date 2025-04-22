# Trace 프로젝트 클래스 다이어그램

이 문서는 Trace 프로젝트의 주요 구조체와 모듈 간의 관계를 설명합니다.

## 1. 프로젝트 구조 개요

Trace 프로젝트는 다음과 같은 주요 모듈로 구성되어 있습니다:

- **Models**: 데이터 구조 정의
- **Parsers**: 로그 데이터 파싱
- **Processors**: 데이터 처리 및 분석
- **Output**: 결과 출력 및 시각화
- **Utils**: 유틸리티 함수 및 상수

## 2. 클래스 다이어그램

```
+-------------------------+     +--------------------------+
|          Models         |     |        Processors        |
+-------------------------+     +--------------------------+
| - UFS                   |<--->| - ufs_processor          |
| - Block                 |<--->| - block_processor        |
+-------------------------+     +--------------------------+
           ^                              ^
           |                              |
           v                              v
+-------------------------+     +--------------------------+
|         Parsers         |     |          Output          |
+-------------------------+     +--------------------------+
| - log_parser            |---->| - charts                 |
+-------------------------+     | - statistics             |
                                | - parquet                |
                                +--------------------------+
```

## 3. 주요 구조체 상세 설명

### 3.1 Models

#### UFS 구조체
```rust
pub struct UFS {
    pub time: f64,
    pub process: String,
    pub cpu: u32,
    pub action: String,
    pub tag: u32,
    pub opcode: String,
    pub lba: u64,
    pub size: u32,
    pub groupid: u32,
    pub hwqid: u32,
    pub qd: u32,   // Queue Depth
    pub dtoc: f64, // Dispatch to Complete latency
    pub ctoc: f64, // Complete to Complete latency
    pub ctod: f64, // Complete to Dispatch latency
    pub continuous: bool,
}
```

#### Block 구조체
```rust
pub struct Block {
    pub time: f64,
    pub process: String,
    pub cpu: u32,
    pub flags: String,
    pub action: String,
    pub devmajor: u32,
    pub devminor: u32,
    pub io_type: String,
    pub extra: u32,
    pub sector: u64,
    pub size: u32,
    pub comm: String,
    pub qd: u32,   // Queue Depth
    pub dtoc: f64, // Dispatch to Complete latency
    pub ctoc: f64, // Complete to Complete latency
    pub ctod: f64, // Complete to Dispatch latency
    pub continuous: bool,
}
```

### 3.2 Processors

UFS와 Block 데이터를 처리하는 모듈입니다. 주요 기능:
- 이벤트 처리
- 지연 시간(latency) 계산
- 큐 깊이(queue depth) 추적
- 연속성(continuity) 확인

### 3.3 Parsers

로그 파일을 파싱하여 UFS 및 Block 구조체로 변환하는 모듈입니다.
- 정규식을 사용하여 로그 라인 파싱
- 데이터 유효성 검사
- 구조체 인스턴스 생성

### 3.4 Output

처리된 데이터의 출력을 담당하는 모듈입니다:
- **charts**: Plotly를 사용한 차트 생성
- **statistics**: 데이터 통계 계산 및 출력
- **parquet**: Arrow/Parquet 형식으로 데이터 저장

## 4. 데이터 흐름

1. **로그 파싱**: Parsers 모듈이 로그 파일을 읽어 UFS/Block 구조체 생성
2. **데이터 처리**: Processors 모듈이 데이터를 처리하고 지연 시간, 큐 깊이 등 계산
3. **결과 출력**: Output 모듈이 처리된 데이터를 시각화하거나 저장

## 5. 확장 가능성

새로운 로그 타입이나 분석 방법을 추가하려면:
1. Models에 새 구조체 추가
2. Parsers에 새 파서 구현
3. Processors에 데이터 처리 로직 추가
4. Output에 시각화/출력 방법 구현