# Trace 프로젝트: UFS 및 Block I/O 로그 분석 도구

이 프로젝트는 UFS(Universal Flash Storage)와 Block I/O 관련 로그 파일을 분석하여 성능 지표를 추출하고 시각화하는 도구입니다.

## 주요 기능

1. 로그 파일 파싱: UFS와 Block I/O 이벤트 추출
2. 데이터 처리: latency 계산 및 continuity 분석
3. 결과 시각화: Plotly를 사용한 차트 생성
4. 데이터 저장: Parquet 형식으로 분석 데이터 저장

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

## 확장 및 개선 방향

1. 추가 이벤트 유형 지원
2. 더 많은 통계 지표 및 시각화 옵션
3. 실시간 로그 분석 기능
4. 분산 처리 지원
