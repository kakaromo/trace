# Trace 프로젝트: UFS 및 Block I/O 로그 분석 도구

이 프로젝트는 UFS(Universal Flash Storage)와 Block I/O 관련 로그 파일을 분석하여 성능 지표를 추출하고 시각화하는 도구입니다.

## 주요 기능

1. 로그 파일 파싱: UFS와 Block I/O 이벤트 추출
   - 동기 및 비동기 파싱 지원
   - 커스텀 UFS 형식 지원
2. 데이터 처리: latency 계산 및 continuity 분석
3. 결과 시각화: 
   - Plotly를 사용한 인터랙티브 차트 생성
   - Plotters를 사용한 네이티브 차트 생성
   - 지연 시간 분포 히스토그램
   - 시간별 지연 시간 추이
   - Queue Depth 변화 차트
   - 지연 시간 범위별 분포 차트
   - LBA/섹터와 지연 시간 관계 산점도
4. 데이터 저장 및 읽기: 
   - Parquet 형식으로 분석 데이터 저장
   - 저장된 Parquet 데이터 읽기 및 재분석

## 코드 구조 및 설명

### 데이터 모델

#### UFS 구조체
```rust
// src/models/ufs.rs
pub struct UFS {
    pub time: f64,         // 이벤트 발생 시간
    pub process: String,   // 프로세스 식별자
    pub cpu: u32,          // CPU 번호
    pub action: String,    // "send_req" 또는 "complete_rsp" 등의 액션
    pub tag: u32,          // 요청 태그
    pub opcode: String,    // 작업 유형(READ, WRITE 등)
    pub lba: u64,          // Logical Block Address
    pub size: u32,         // 요청 크기(섹터 수)
    pub groupid: u32,      // 그룹 ID
    pub hwqid: u32,        // 하드웨어 큐 ID
    pub qd: u32,           // Queue Depth
    pub dtoc: f64,         // Dispatch to Complete latency
    pub ctoc: f64,         // Complete to Complete latency
    pub ctod: f64,         // Complete to Dispatch latency
    pub continuous: bool,  // 연속적인 요청 여부
}
```

#### UFSCUSTOM 구조체
```rust
// src/models/ufscustom.rs
pub struct UFSCUSTOM {
    pub opcode: String,     // 작업 유형(READ, WRITE 등)
    pub lba: u64,           // Logical Block Address
    pub size: u32,          // 요청 크기(섹터 수)
    pub start_time: f64,    // 요청 시작 시간
    pub end_time: f64,      // 요청 완료 시간
    pub dtoc: f64,          // Dispatch to Complete latency
}
```

#### Block 구조체
```rust
// src/models/block.rs
pub struct Block {
    pub time: f64,         // 이벤트 발생 시간
    pub process: String,   // 프로세스 식별자
    pub cpu: u32,          // CPU 번호
    pub flags: String,     // 플래그
    pub action: String,    // "D"(Dispatch), "C"(Complete) 등의 액션
    pub devmajor: u32,     // 디바이스 주 번호
    pub devminor: u32,     // 디바이스 부 번호
    pub io_type: String,   // I/O 유형(READ, WRITE 등)
    pub extra: u32,        // 추가 정보
    pub sector: u64,       // 시작 섹터
    pub size: u32,         // 요청 크기(섹터 수)
    pub comm: String,      // 명령어 이름
    pub qd: u32,           // Queue Depth
    pub dtoc: f64,         // Dispatch to Complete latency
    pub ctoc: f64,         // Complete to Complete latency
    pub ctod: f64,         // Complete to Dispatch latency
    pub continuous: bool,  // 연속적인 요청 여부
}
```

### 프로그램 흐름

#### 메인 프로세스 (main.rs)
1. 로그 파일 경로와 출력 파일 접두사를 명령줄 인자로 받거나 사용자에게 입력 요청
2. 로그 파일 파싱: `parse_log_file()` 함수를 통해 UFS 및 Block I/O 이벤트 추출
3. 데이터 후처리: 추출된 이벤트를 병렬 처리하여 latency 계산 및 continuity 분석
   - UFS 데이터: `ufs_bottom_half_latency_process()`
   - Block I/O 데이터: `block_bottom_half_latency_process()`
4. 분석 결과 계산 및 출력: 통계 정보 계산 및 콘솔 출력
5. Parquet 파일 저장: `save_to_parquet()` 함수로 분석 데이터를 Parquet 형식으로 저장
6. 시각화: `generate_charts()` 함수로 Plotly 및 Matplotlib 차트 생성

### 주요 모듈 설명

#### 1. 파서 모듈 (src/parsers/log.rs)
- `parse_log_file()`: 로그 파일을 읽고 파싱하여 UFS 및 Block 이벤트로 변환
- 정규 표현식을 사용하여 로그 라인에서 필요한 정보 추출
- 파싱된 이벤트는 각각 `UFS` 및 `Block` 구조체 인스턴스로 변환

#### 2. 프로세서 모듈 (src/processors/)
- `ufs_bottom_half_latency_process()`: UFS 이벤트를 처리하여 latency 및 continuity 계산
  - Queue Depth(QD) 추적
  - 요청-완료 쌍 매칭을 통한 latency 계산
  - continuous 요청 식별
- `block_bottom_half_latency_process()`: Block I/O 이벤트에 대해 유사한 처리 수행

#### 3. 출력 모듈 (src/output/)
- `save_to_parquet()`: 처리된 UFS 및 Block 데이터를 Parquet 형식으로 저장
  - Arrow 및 Parquet 라이브러리 활용
  - `save_ufs_to_parquet()`: UFS 데이터를 "_ufs.parquet" 파일로 저장
  - `save_block_to_parquet()`: Block 데이터를 "_block.parquet" 파일로 저장
- `generate_charts()`: 분석 데이터를 바탕으로 시각화 차트 생성
  - Plotly를 사용한 HTML 차트
  - Matplotlib을 사용한 PNG 이미지
- `print_ufs_statistics()`, `print_block_statistics()`: 분석 통계 콘솔 출력
- `statistics.rs`: 통계 계산 및 출력 기능
  - 기본 통계 (평균, 최소/최대, 표준편차)
  - Percentile 계산 (99%, 99.9%, 99.99%)
  - 지연 시간 범위별 분포 계산
  - 요청 타입별 분포 통계
- `charts.rs`: 차트 시각화 기능
  - Plotly 및 Charming 라이브러리 통합
  - 다양한 차트 유형 지원
  - 사용자 정의 디자인 및 레이아웃
- `parquet.rs`: Parquet 파일 저장 및 로드 기능
  - Arrow 포맷 변환
  - Parquet 스키마 정의 및 최적화
- `reader.rs`: Parquet 파일 읽기 기능

## 프로젝트 구조

- `src/models`: 데이터 모델 정의 (UFS, Block 구조체)
- `src/parsers`: 로그 파일 파싱 기능
- `src/processors`: UFS 및 Block I/O 데이터 처리 로직
- `src/output`: 차트 생성 및 Parquet 저장 기능
- `src/utils`: 유틸리티 및 상수 정의
- `src/python`: Python 스크립트 (Parquet 파일 시각화)

## 사용 방법

### 기본 실행
```bash
cargo run -- <로그_파일_경로> <출력_파일_접두사>
```

### 바이너리 실행
```bash
./target/release/trace <로그_파일_경로> <출력_파일_접두사>
```

### 기본 사용법

```bash
./trace <로그_파일_경로> <출력_파일_접두사>
```

### 옵션

- **-l, --latency-ranges <범위>**: 사용자 정의 latency 범위 설정 (쉼표로 구분된 밀리초 값)
  예: `-l 0.1,1,5,10,50,100,500,1000`
- **--parquet <타입>**: 기존 Parquet 파일 분석 모드
  <타입> 옵션: 'ufs', 'block'

### 예제

```bash
# 기본 로그 분석
./trace /logs/sample.log output_prefix

# 사용자 정의 latency 범위 사용
./trace -l 0.1,0.5,1,5,10,30,100 /logs/sample.log output_prefix

# Parquet 파일 분석
./trace --parquet ufs output_prefix_ufs.parquet new_output_prefix
```

로그 파일이 분석된 후 다음과 같은 결과물이 생성됩니다:
- `<출력_파일_접두사>_ufs.parquet`: UFS 분석 데이터
- `<출력_파일_접두사>_block.parquet`: Block I/O 분석 데이터
- `<출력_파일_접두사>_ufs_*.html`: UFS 데이터 Plotly 차트
- `<출력_파일_접두사>_block_*.html`: Block I/O 데이터 Plotly 차트
- `<출력_파일_접두사>_ufs_*.png`: UFS 데이터 Matplotlib 차트
- `<출력_파일_접두사>_block_*.png`: Block I/O 데이터 Matplotlib 차트

## 성능 최적화

- 병렬 처리: Rayon 라이브러리를 활용한 병렬 데이터 처리
- 메모리 효율성: 스트리밍 처리를 통한 대용량 로그 파일 처리
- 진행 상황 표시: 대용량 파일 처리 과정에서 진행률 표시
- 배치 처리: 메모리 효율적인 청크 단위 데이터 처리

## 성능 최적화 업데이트 (2024년 12월)

### 파서 모드별 성능 비교
이 프로젝트는 세 가지 파서 모드를 지원하며, 각각 다른 성능 특성을 가집니다:

#### 1. 동기 모드 (Sync)
- **특징**: 순차적 파일 읽기 및 처리
- **사용 사례**: 소규모 파일 (1GB 미만)
- **성능**: 기준 성능 (20GB 파일: 26분 48초)

#### 2. 고성능 모드 (High-Performance)
- **특징**: 메모리 매핑 I/O 및 멀티코어 활용
- **사용 사례**: 대용량 파일 (10GB 이상) 권장
- **성능**: 46% 향상 (20GB 파일: 14분 21초)

#### 3. 스트리밍 모드 (Streaming)
- **특징**: 메모리 효율적 처리 및 최고 파싱 속도
- **사용 사례**: 메모리 제약이 있는 환경
- **성능**: 42% 향상 (20GB 파일: 15분 33초)

### 후처리 최적화
Rayon 라이브러리를 활용한 병렬 처리로 후처리 성능을 크게 개선했습니다:

- **UFS 처리**: 24% 성능 향상 (병렬 정렬 및 필터링)
- **Block 처리**: 10% 성능 향상 (배치 처리 최적화)
- **멀티코어 활용**: 효율적인 CPU 자원 활용

### 확장성 및 메모리 관리
- **선형 스케일링**: 파일 크기 4배 증가 시 처리 시간 약 4-5배 증가
- **메모리 효율성**: 스트리밍 모드에서 안정적인 대용량 파일 처리
- **압축 효율성**: Parquet 형식으로 12-14:1 압축률 달성

## 확장 및 개선 방향

1. 추가 이벤트 유형 지원
2. 더 많은 통계 지표 및 시각화 옵션
3. 실시간 로그 분석 기능
4. 분산 처리 지원
5. 사용자 정의 필터링 및 분석 규칙
