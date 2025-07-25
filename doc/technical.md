# UFS 및 Block I/O 로그 분석 도구 기술 문서

## 목차

1. [개요](#개요)
2. [시스템 아키텍처](#시스템-아키텍처)
3. [주요 알고리즘 및 데이터 구조](#주요-알고리즘-및-데이터-구조)
4. [성능 최적화 전략](#성능-최적화-전략)
5. [데이터 처리 파이프라인](#데이터-처리-파이프라인)
6. [메모리 관리](#메모리-관리)
7. [오류 처리 전략](#오류-처리-전략)
8. [확장성 및 유지보수](#확장성-및-유지보수)
9. [사용 사례 및 예시](#사용-사례-및-예시)
10. [성능 벤치마크](#성능-벤치마크)
11. [부록: 개발 가이드](#부록-개발-가이드)

## 개요

이 문서는 UFS(Universal Flash Storage) 및 Block I/O 로그 분석 도구의 상세한 기술적 설명을 제공합니다. 이 도구는 스토리지 시스템의 성능을 분석하고 병목 현상을 식별하기 위해 개발되었으며, 대용량 로그 파일에서 효율적으로 데이터를 추출, 처리, 시각화하는 기능을 제공합니다.

### 주요 기능 요약

- UFS 및 Block I/O 이벤트 로그 파싱
- Latency 계산 및 분석
- Queue Depth 추적
- Continuity 분석
- 통계 계산 및 보고
- Parquet 형식 데이터 저장
- Plotly 및 Matplotlib 기반 차트 생성

## 시스템 아키텍처

이 시스템은 모듈식 설계를 채택하여 유지보수성과 확장성을 높였습니다. 핵심 컴포넌트는 다음과 같습니다:

### 1. 핵심 모듈 구조

```
src/
├── lib.rs           # 핵심 기능 내보내기
├── main.rs          # 명령줄 인터페이스
├── models/          # 데이터 모델 정의
├── parsers/         # 로그 파일 파싱 기능
├── processors/      # 데이터 처리 및 분석 로직
├── output/          # 결과 출력 및 시각화
└── utils/           # 유틸리티 및 공통 기능
```

### 2. 컴포넌트 상호작용

시스템 컴포넌트는 다음과 같은 순서로 상호작용합니다:

1. **입력 처리**: 명령줄 인자를 통해 로그 파일 경로와 출력 접두사를 받습니다.
2. **로그 파싱**: `parsers` 모듈에서 로그 파일을 파싱하여 구조화된 이벤트 데이터로 변환합니다.
3. **데이터 처리**: `processors` 모듈에서 파싱된 이벤트를 처리하여 latency, queue depth 등을 계산합니다.
4. **통계 생성**: 처리된 데이터에서 의미 있는 통계를 추출합니다.
5. **결과 저장**: 분석 결과를 Parquet 파일로 저장합니다.
6. **시각화**: 분석 결과를 차트로 시각화합니다.

### 3. 데이터 흐름도

```
로그 파일 → 파싱 엔진(동기 또는 비동기) → 이벤트 객체 → 프로세서 → 분석 결과 → 출력(통계/Parquet/차트)
```

## 주요 알고리즘 및 데이터 구조

### 1. 데이터 모델

시스템은 세 가지 주요 데이터 구조를 사용합니다:

#### UFS 구조체 (models/ufs.rs)

UFS 이벤트를 표현하는 구조체입니다. 각 필드의 의미는 다음과 같습니다:

```rust
pub struct UFS {
    pub time: f64,         // 이벤트 발생 시간 (초 단위)
    pub process: String,   // 프로세스 이름
    pub cpu: u32,          // CPU 코어 번호
    pub action: String,    // 액션 타입 (send_req, complete_rsp)
    pub tag: u32,          // 요청 식별 태그
    pub opcode: String,    // 작업 코드 (READ, WRITE 등)
    pub lba: u64,          // 논리 블록 주소
    pub size: u32,         // 요청 크기 (바이트)
    pub groupid: u32,      // 그룹 ID
    pub hwqid: u32,        // 하드웨어 큐 ID
    pub qd: u32,           // 계산된 Queue Depth
    pub dtoc: f64,         // Dispatch to Complete latency (ms)
    pub ctoc: f64,         // Complete to Complete latency (ms)
    pub ctod: f64,         // Complete to Dispatch latency (ms)
    pub continuous: bool,  // 연속적인 요청 여부
}
```

#### UFSCUSTOM 구조체 (models/ufscustom.rs)

단순화된 UFS 이벤트를 표현하는 커스텀 구조체입니다:

```rust
pub struct UFSCUSTOM {
    pub opcode: String,     // 작업 코드 (READ, WRITE 등)
    pub lba: u64,           // 논리 블록 주소
    pub size: u32,          // 요청 크기 (바이트)
    pub start_time: f64,    // 요청 시작 시간 (초 단위)
    pub end_time: f64,      // 요청 완료 시간 (초 단위)
    pub dtoc: f64,          // Dispatch to Complete latency (ms)
}
```

#### Block 구조체 (models/block.rs)

Block I/O 이벤트를 표현하는 구조체입니다:

```rust
pub struct Block {
    pub time: f64,         // 이벤트 발생 시간 (초 단위)
    pub process: String,   // 프로세스 이름
    pub cpu: u32,          // CPU 코어 번호
    pub flags: String,     // 플래그 정보
    pub action: String,    // 액션 타입 (D, C 등)
    pub devmajor: u32,     // 디바이스 주 번호
    pub devminor: u32,     // 디바이스 부 번호
    pub io_type: String,   // I/O 유형 (READ, WRITE 등)
    pub extra: u32,        // 추가 정보
    pub sector: u64,       // 시작 섹터 번호
    pub size: u32,         // 요청 크기 (섹터 수)
    pub comm: String,      // 명령어 이름
    pub qd: u32,           // 계산된 Queue Depth
    pub dtoc: f64,         // Dispatch to Complete latency (ms)
    pub ctoc: f64,         // Complete to Complete latency (ms)
    pub ctod: f64,         // Complete to Dispatch latency (ms)
    pub continuous: bool,  // 연속적인 요청 여부
}
```

### 2. 로그 파서 알고리즘

로그 파일 파싱은 크기에 따라 두 가지 다른 알고리즘을 사용합니다:

#### 인메모리 파싱 (작은 파일용)

메모리에 전체 파일을 로드하여 병렬 처리합니다:

1. 파일을 한 번에 로드
2. 청크 단위로 분할(100,000줄)
3. 병렬 처리하여 이벤트 추출
4. 결과를 벡터에 수집

#### 스트리밍 파싱 (대용량 파일용)

메모리 사용을 최소화하며 파일을 스트리밍 방식으로 처리합니다:

1. 임시 파일 생성
2. 한 줄씩 읽어 청크 단위로 처리(500,000줄)
3. 파싱된 이벤트를 임시 파일에 저장
4. 첫 번째 패스 완료 후 임시 파일에서 데이터 로드
5. 메모리 효율적으로 대용량 파일 처리(수십 GB 파일 처리 가능)

### 3. 정규 표현식 패턴

로그 라인 파싱을 위해 최적화된 정규 표현식을 사용합니다:

#### UFS 로그 패턴

```regex
^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)
```

#### Block I/O 로그 패턴

```regex
^\s*(?P<process>.*?)\s+\[(?P<cpu>\d+)\]\s+(?P<flags>.+?)\s+(?P<time>[\d\.]+):\s+(?P<action>\S+):\s+(?P<devmajor>\d+),(?P<devminor>\d+)\s+(?P<io_type>[A-Z]+)(?:\s+(?P<extra>\d+))?\s+\(\)\s+(?P<sector>\d+)\s+\+\s+(?P<size>\d+)(?:\s+\S+)?\s+\[(?P<comm>.*?)\]$
```

### 4. Queue Depth 추적 알고리즘

Queue Depth 추적은 다음과 같은 알고리즘으로 수행됩니다:

1. 이벤트를 시간순으로 정렬
2. 액션 타입에 따라 카운터 조정:
   - UFS: 'send_req'일 때 +1, 'complete_rsp'일 때 -1
   - Block: 'D'(Dispatch)일 때 +1, 'C'(Complete)일 때 -1
3. 각 이벤트 시점의 카운터 값을 queue depth로 기록

### 5. Latency 계산 알고리즘

Latency 계산은 다음과 같은 방식으로 수행됩니다:

#### Dispatch to Complete (dtoc) latency

1. 태그나 섹터 번호로 요청-완료 쌍 매칭
2. 완료 시간에서 요청 시간 차이 계산
3. 밀리초 단위로 변환 (시간 차이 × 1000)

#### Complete to Complete (ctoc) latency

1. 연속된 완료 이벤트 간의 시간 차이 계산
2. 밀리초 단위로 변환 (시간 차이 × 1000)

#### Complete to Dispatch (ctod) latency

1. 완료 이벤트 이후 다음 요청 이벤트까지의 시간 차이 계산
2. 밀리초 단위로 변환 (시간 차이 × 1000)

**시간 단위 변환**:
모든 지연 시간 계산에서는 `MILLISECONDS` 상수(값: 1000)를 사용하여 초 단위의 시간 차이를 밀리초 단위로 변환합니다. 이 상수는 `src/utils/constants.rs`에 정의되어 있습니다.

```rust
// 초 단위 시간 차이를 밀리초로 변환하는 예
let dtoc_ms = (complete_time - dispatch_time) * MILLISECONDS as f64;
```

### 6. Continuity 분석 알고리즘

요청의 continuity 분석은 다음 기준으로 수행됩니다:

1. UFS: 현재 LBA가 이전 요청의 (LBA + 크기)와 같거나 인접한 경우 continuous로 판단
2. Block: 현재 섹터가 이전 요청의 (섹터 + 크기)와 같거나 인접한 경우 continuous로 판단
3. 시간 간격이 임계값(기본값: 1ms) 이내인 경우에만 continuous로 인정

### 7. 지연 시간 범위별 분포 분석 알고리즘

지연 시간 범위별 분포 분석은 성능 패턴을 더 세밀하게 파악하기 위한 기능입니다:

1. **사용자 정의 또는 기본 범위 설정**:
   - 사용자 정의: 명령줄 `-l` 옵션을 통해 제공된 값 (예: `-l 0.1,1,10,100`)
   - 기본 범위: 0.1ms, 0.5ms, 1ms, 5ms, 10ms, 50ms, 100ms, 500ms, 1s, 5s, 10s, 50s, 100s, 500s, 1000s

2. **분포 계산 방식**:
   - 각 지연 시간 값을 해당 범위에 매핑 (예: 3ms는 "1ms < v ≤ 5ms" 범위에 속함)
   - 각 범위별 이벤트 수 집계
   - 비율로 환산하여 분포 계산

3. **범위 표현 형식**:
   - 첫 번째 범위: "≤ X ms" (X 이하)
   - 중간 범위: "X ms < v ≤ Y ms" (X 초과 Y 이하)
   - 마지막 범위: "> Z ms" (Z 초과)

4. **성능 최적화**:
   - 정렬된 데이터 사용으로 효율적인 매핑
   - 해시맵을 활용한 빠른 계산 및 조회

이 알고리즘은 `LatencyStats` 구조체의 `latency_ranges` 메서드에서 구현되며, `print_generic_latency_ranges_by_type` 함수를 통해 결과가 표시됩니다.

## 성능 최적화 전략

### 1. 메모리 관리

로그 파일 크기에 따라 다음과 같은 최적화 전략을 사용합니다:

#### 소형 파일 (< 1GB)

전체 파일을 메모리에 로드하고 병렬 처리:

1. 파일 전체를 메모리에 로드
2. 청크로 분할(100,000줄)
3. 병렬 스레드에서 각 청크 처리
4. 결과 병합

#### 대용량 파일 (> 1GB)

메모리 사용을 최소화하며 파일을 스트리밍 방식으로 처리합니다:

1. 임시 파일 생성
2. 한 줄씩 읽어 청크 단위로 처리(500,000줄)
3. 파싱된 이벤트를 임시 파일에 저장
4. 첫 번째 패스 완료 후 임시 파일에서 데이터 로드
5. 메모리 효율적으로 대용량 파일 처리(수십 GB 파일 처리 가능)

### 2. 비동기 처리

고성능 처리를 위한 토카이오(Tokio) 기반의 비동기 파싱:

1. `log_async.rs` 모듈을 통한 비동기 파일 처리
2. 비동기 I/O 작업으로 I/O 대기 시간 최소화
3. 동시 처리를 통한 CPU 사용률 최적화
4. 대용량 파일 처리 시 스트리밍 방식과 결합하여 성능 향상

### 3. 정규 표현식 패턴

로그 라인 파싱을 위해 최적화된 정규 표현식을 사용합니다:

#### UFS 로그 패턴

```regex
^\s*(?P<process>.*?)\s+\[(?P<cpu>[0-9]+)\].*?(?P<time>[0-9]+\.[0-9]+):\s+ufshcd_command:\s+(?P<command>send_req|complete_rsp):.*?tag:\s*(?P<tag>\d+).*?size:\s*(?P<size>[-]?\d+).*?LBA:\s*(?P<lba>\d+).*?opcode:\s*(?P<opcode>0x[0-9a-f]+).*?group_id:\s*0x(?P<group_id>[0-9a-f]+).*?hwq_id:\s*(?P<hwq_id>[-]?\d+)
```

## 데이터 처리 파이프라인

### 1. 파이프라인 개요

데이터 처리는 다음과 같은 파이프라인을 따릅니다:

```
로그 파일 → 파싱 → 후처리 → 통계 계산 → 저장 및 시각화
```

### 2. 단계별 처리 과정

#### 1) 로그 파일 파싱 단계

1. 파일 크기 확인
2. 파싱 전략 선택 (인메모리 vs 스트리밍)
3. 정규 표현식으로 로그 줄 파싱
4. 구조화된 이벤트 생성
5. 처리 결과 반환 또는 임시 저장

#### 2) 데이터 후처리 단계

1. 이벤트 시간순 정렬
2. Queue Depth 계산
3. Latency 분석
   - Dispatch to Complete (dtoc)
   - Complete to Complete (ctoc)
   - Complete to Dispatch (ctod)
4. Continuity 분석
5. 처리된 이벤트 업데이트

#### 3) 통계 계산 단계

1. 기본 통계 계산
   - 총 요청 수
   - 평균/최소/최대 latency
   - 표준 편차
   - Percentile (99%, 99.9%)
2. 오퍼레이션 코드/I/O 타입별 그룹화
3. Latency 범위별 분포 계산
   - 사용자 정의 범위 또는 기본 범위 사용
   - 각 범위별 요청 수 및 비율 계산
   - 요청 타입별로 분포 비교
4. 요청 크기 분포 계산

#### 4) 저장 및 시각화 단계

1. Parquet 파일 생성
   - Arrow 배열로 변환
   - Parquet schema 정의
   - 압축 및 인코딩 설정
2. 차트 생성
   - Latency histogram
   - Timeline chart
   - Queue Depth chart
   - Operation/I/O type distribution
   - Latency 범위 분포 차트
   - LBA vs Latency 산점도
   - 성능 비교 차트

## 메모리 관리

### 1. 메모리 사용 패턴

이 도구의 메모리 사용 패턴:

1. **파싱 단계**: 
   - 소형 파일: 전체 파일을 메모리에 로드 (제한된 크기)
   - 대형 파일: 한 번에 500,000줄만 메모리에 로드

2. **후처리 단계**:
   - 모든 이벤트 객체가 메모리에 로드
   - 고속 접근을 위한 해시맵 사용 (요청-완료 매칭용)

3. **시각화 단계**:
   - 차트 데이터 구조를 위한 임시 메모리 할당
   - 대용량 데이터의 경우 downsampling 수행

### 2. 메모리 사용량 최적화

메모리 사용량 최적화 전략:

1. **벡터 용량 사전 할당**:
   - 예상 크기로 벡터 용량 미리 할당하여 재할당 방지
   - 예: `Vec::with_capacity(estimated_size)`

2. **임시 파일 활용**:
   - 대용량 중간 결과를 임시 파일에 저장
   - 필요시에만 메모리에 로드

3. **청크 처리**:
   - 데이터를 청크 단위로 처리하여 최대 메모리 사용량 제한
   - 청크 크기는 시스템 메모리 특성에 맞게 조정 가능

4. **메모리 효율적인 데이터 구조**:
   - 문자열 중복 방지 (opcode, io_type 등)
   - 불필요한 필드 제거 및 최적 타입 사용

## 오류 처리 전략

### 1. 오류 유형 및 처리 방식

시스템에서 발생 가능한 오류와 처리 방식:

1. **파일 I/O 오류**:
   - 파일 없음, 권한 부족, 디스크 공간 부족 등
   - Result 타입으로 전파 후 사용자에게 의미 있는 메시지 표시

2. **파싱 오류**:
   - 잘못된 형식의 로그 줄
   - 해당 줄 건너뛰기 및 경고 로그 출력
   - 총 오류 수 카운트 및 보고

3. **데이터 처리 오류**:
   - 잘못된 계산 또는 예상치 못한 값
   - 안전한 기본값 제공 및 경고 로그 출력

4. **메모리 부족**:
   - 대용량 파일 처리 중 메모리 고갈
   - 스트리밍 모드로 전환 또는 청크 크기 감소

### 2. Sanitization 전략

입력 데이터의 sanitization 전략:

1. **범위 검사**:
   - 시간 값이 음수인 경우 0으로 정상화
   - 크기가 비정상적으로 큰 경우 상한값으로 제한

2. **유효성 검사**:
   - 섹터 값이 최대 u64 값인 경우 0으로 정상화
   - 문자열 필드가 지나치게 긴 경우 잘라내기

3. **중복 제거**:
   - 중복 이벤트 필터링
   - 동일 시간에 중복 이벤트가 있는 경우 처리 전략 적용

## 확장성 및 유지보수

### 1. 모듈 구조 설계

확장성을 고려한 모듈 구조:

1. **기능별 모듈 분리**:
   - 모델, 파서, 프로세서, 출력 기능 분리
   - 각 모듈은 명확한 책임 영역 보유

2. **인터페이스 중심 설계**:
   - TraceItem 트레이트를 통한 다형성 지원
   - 모듈 간 상호작용은 명확한 인터페이스를 통해 수행
   - 내부 구현 변경 시 외부 영향 최소화

3. **유틸리티 모듈**:
   - 공통 기능 및 상수를 별도 모듈로 분리
   - 코드 중복 방지 및 일관성 유지

4. **비동기 지원**:
   - 동기 및 비동기 처리 방식 병행 지원
   - 사용 사례에 맞게 선택 가능

### 2. 확장 지점

새로운 기능 추가를 위한 확장 지점:

1. **새로운 이벤트 타입 지원**:
   - `models` 모듈에 새 구조체 추가
   - TraceType 열거형에 새 타입 추가
   - TraceItem 트레이트 구현
   - `parsers` 모듈에 해당 파서 구현
   - `processors` 모듈에 처리 로직 추가

2. **분석 기능 확장**:
   - `processors` 모듈에 새 분석 함수 추가
   - 기존 데이터 구조를 활용한 새로운 지표 계산

3. **출력 형식 확장**:
   - `output` 모듈에 새 출력 형식 지원 추가
   - 예: CSV, JSON, XML 등의 형식 지원

4. **시각화 확장**:
   - `output/charts.rs`에 새로운 Plotly 기반 차트 유형 추가
   - `output/plotters_charts.rs`에 새로운 네이티브 차트 유형 추가
   - 기존 데이터를 활용한 새로운 시각화 제공
   - 사용자 환경에 맞는 시각화 방법 선택 (웹 기반 vs 네이티브)

## 사용 사례 및 예시

### 1. 스토리지 성능 분석

UFS 저장 장치의 성능 특성 분석:

1. 읽기/쓰기 latency 분포 확인
2. I/O 크기별 성능 패턴 식별
3. Queue Depth와 성능 간의 상관관계 분석
4. Sequential vs Random I/O 성능 비교

```bash
# 대용량 UFS 로그 파일 분석
$ ./trace /logs/ufs_stress_test.log ufs_analysis
```

생성되는 차트:
- `ufs_analysis_ufs_latency_histogram.html`: Latency 분포
- `ufs_analysis_ufs_latency_timeline.html`: 시간별 latency 추이
- `ufs_analysis_ufs_qd.html`: Queue Depth 변화
- `ufs_analysis_ufs_opcode_distribution.html`: 작업 유형 분포

### 2. 병목 현상 진단

시스템에서 I/O 병목 현상 진단:

1. 높은 Queue Depth 시점 식별
2. 비정상적으로 긴 latency 패턴 탐지
3. 특정 프로세스의 I/O 패턴 분석
4. 디바이스별 성능 비교

```bash
# Block I/O 로그 분석
$ ./trace /logs/block_io_slow.log block_analysis
```

분석 결과:
- 평균 latency 통계
- 최대 Queue Depth 이벤트
- Latency outlier
- 디바이스별 성능 요약

### 3. 커스텀 UFS 로그 분석

커스텀 형식의 UFS 로그 분석:

1. 간소화된 형식의 UFS 로그 데이터 처리
2. 시작/종료 시간 및 latency 분석
3. I/O 크기와 성능 간의 상관관계 분석

```bash
# 커스텀 UFS 로그 파일 분석
$ ./trace /logs/ufs_custom_data.log ufscustom_analysis
```

생성되는 차트:
- `ufscustom_analysis_latency_histogram.html`: Latency 분포
- `ufscustom_analysis_opcode_distribution.html`: 작업 유형 분포
- `ufscustom_analysis_lba_vs_latency.html`: LBA와 지연 시간 관계

### 4. 워크로드 프로파일링

애플리케이션 워크로드 특성 분석:

1. I/O 패턴 분석 (sequential vs random)
2. 읽기/쓰기 비율 계산
3. 요청 크기 분포 검사
4. 시간별 I/O 활동 패턴 식별

## 성능 벤치마크

## 성능 벤치마크 및 테스트 결과

### 대용량 파일 처리 성능 (2024년 12월 테스트)

#### 테스트 환경
- **시스템**: macOS, 16코어 CPU, 64GB RAM, SSD 저장소
- **테스트 파일**: 5GB (3천7백만 이벤트), 20GB (1억4천8백만 이벤트)
- **파서 모드**: Sync, High-Performance, Streaming

#### 성능 측정 결과

##### 20GB 파일 처리 성능
| 모드 | 총 시간 | 파싱 시간 | 성능 향상 | 추천 용도 |
|------|----------|----------|----------|----------|
| 동기 모드 | 26분 48초 | 13분 7초 | 기준 | 소규모 파일 |
| 고성능 모드 | 14분 21초 | 4분 38초 | **46% 향상** | **대용량 파일** |
| 스트리밍 모드 | 15분 33초 | 2분 50초 | **42% 향상** | 메모리 효율성 |

##### 5GB 파일 처리 성능
| 모드 | 총 시간 | 파싱 시간 | 성능 향상 | 메모리 효율성 |
|------|----------|----------|----------|---------------|
| 동기 모드 | 6분 2초 | 2분 44초 | 기준 | 보통 |
| 고성능 모드 | 3분 21초 | 1분 1초 | **45% 향상** | 우수 |
| 스트리밍 모드 | 3분 17초 | 32초 | **46% 향상** | **최우수** |

#### 최적화 성과

##### 후처리 병렬화 효과 (5GB 파일 기준)
- **UFS 처리**: 27.24초 → 20.76초 (**24% 향상**)
- **Block 처리**: 41.16초 → 37.07초 (**10% 향상**)
- **Rayon 병렬화**: 멀티코어 시스템 효과적 활용

##### 확장성 분석
- **파일 크기**: 5GB → 20GB (4배 증가)
- **처리 시간**: 거의 선형 스케일링 (4.29-4.44배)
- **메모리 효율성**: 스트리밍 모드에서 안정적인 메모리 관리

#### 핵심 성과 지표
- **파싱 속도**: 최대 78% 개선 (Streaming 모드)
- **전체 처리**: 최대 48% 개선 (5GB 파일)
- **메모리 효율성**: 안정적인 20GB 파일 처리
- **출력 품질**: 모든 모드에서 완전한 Parquet/차트 생성

#### 사용 권장사항

##### 대용량 파일 (10GB 이상)
- **고성능 모드 권장**: 최적의 전체 성능
- **메모리 요구사항**: 파일 크기의 4배 이상
- **SSD 저장소**: I/O 성능 최적화 필수

##### 중간 크기 파일 (1-10GB)
- **고성능 모드**: 가장 균형잡힌 성능
- **스트리밍 모드**: 메모리 효율성 중요시

##### 소규모 파일 (1GB 미만)
- **모든 모드 사용 가능**: 동기 모드도 충분히 빠름

## 부록: 개발 가이드

### 1. 개발 환경 설정

1. Rust 설치 (1.56.0 이상 필요)
2. 필수 종속성 설치:
   - Arrow 및 Parquet 라이브러리
   - Python 및 관련 패키지 (시각화용)

### 2. 프로젝트 빌드

```bash
# 디버그 빌드
$ cargo build

# 릴리스 빌드 (최적화)
$ cargo build --release
```

### 3. 테스트 실행

```bash
# 단위 테스트 실행
$ cargo test

# 특정 모듈 테스트
$ cargo test --package trace --lib -- parsers::log::tests
```

### 4. 성능 프로파일링

```bash
# Cargo flamegraph를 사용한 프로파일링
$ cargo flamegraph -- /logs/sample.log output_prefix

# perf를 사용한 상세 프로파일링
$ perf record -g ./target/release/trace /logs/sample.log output_prefix
$ perf report
```

### 5. 코드 기여 가이드라인

1. **코딩 스타일**:
   - Rust 표준 포맷팅 준수 (`cargo fmt`)
   - Clippy 검사 통과 (`cargo clippy`)

2. **테스트 요구사항**:
   - 모든 새 기능에 단위 테스트 필요
   - 코드 변경 시 기존 테스트 통과 확인

3. **성능 요구사항**:
   - 대용량 파일 처리 시 메모리 사용 제한
   - 처리 속도 기존 대비 성능 저하 없음

4. **문서화**:
   - 공개 API에 문서 주석 필수
   - 복잡한 알고리즘에 주석 및 설명 추가

## 최근 변경사항 (2025년 7월 6일)

### 1. 자동 처리 모드 선택
- **변경내용**: `--highperf` 옵션 제거 및 파일 크기 기반 자동 모드 선택 구현
- **기능**: 1GB 이상 파일은 자동으로 고성능 모드, 1GB 미만은 스트리밍 모드 사용
- **수동 오버라이드**: `--streaming` 옵션으로 수동 스트리밍 모드 강제 가능
- **영향**: 사용자가 파일 크기를 고려하여 모드를 선택할 필요 없음

### 2. Block Parquet 스키마 일관성 개선
- **변경내용**: `models/block.rs` 정의와 Parquet 저장 스키마 완전 일치
- **추가 필드**: `comm`, `qd` 필드 Parquet 저장 시 포함
- **필드 매핑**: `sector` 필드 정확한 매핑 (기존 `lba` 필드 혼용 문제 해결)
- **영향**: Block 데이터 Parquet 파일의 스키마가 모델 정의와 완전히 일치

### 3. 성능 최적화
- **병렬 처리**: Parquet 저장 시 chunk 단위 병렬 처리
- **메모리 최적화**: 사전 할당된 벡터 사용으로 메모리 효율성 향상
- **I/O 최적화**: 순차적 파일 쓰기로 I/O 성능 향상