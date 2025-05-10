# trace

로그 파일을 트레이스하고 분석하는 도구입니다.

## 빌드 방법

### 필수 요구 사항
- Rust 및 Cargo가 설치되어 있어야 합니다. [Rust 설치 방법](https://www.rust-lang.org/tools/install)

### 개발 빌드
```bash
git clone https://github.com/your-username/trace.git
cd trace

cargo build
```

### 릴리스 빌드
```bash
cargo build --release
```

## 실행 방법

### 기본 사용법
```bash
# 로그 파일 처리 모드
./target/debug/trace [옵션] <로그_파일_경로> <출력_파일_접두사>

# Parquet 파일 분석 모드
./target/debug/trace [옵션] --parquet <타입> <parquet_파일_경로> <출력_파일_접두사>
```

### 옵션
- **-l <값들>**: 사용자 정의 latency 범위를 밀리초(ms) 단위로 설정 (쉼표로 구분)
  - 예: `-l 0.1,0.5,1,5,10,30,100,200,500,1000,1100`
  - 설정된 값들은 범위 계산에 사용됩니다 (예: ≤0.1ms, 0.1ms< v ≤0.5ms, ..., >1100ms)
  - 이 옵션을 사용하지 않으면 기본 latency 범위가 사용됩니다

- **--parquet <타입>**: 이미 생성된 Parquet 파일을 분석하는 모드로 전환
  - `<타입>`: 분석할 데이터 타입 (현재 지원: 'ufs', 'block')
  - 이 모드에서는 로그 파일 대신 Parquet 파일을 입력으로 사용

### 명령줄 인자 설명
#### 로그 파일 처리 모드
- **로그_파일_경로**: 분석할 로그 파일의 경로 (첫 번째 인자)
- **출력_파일_접두사**: Parquet 파일이 저장될 경로 및 파일명 접두사 (두 번째 인자)
  - 프로그램은 이 접두사에 "_ufs.parquet"과 "_block.parquet"을 각각 추가하여 두 개의 파일을 생성합니다.

#### Parquet 파일 분석 모드
- **타입**: 분석할 데이터 타입 ('ufs' 또는 'block')
- **parquet_파일_경로**: 분석할 Parquet 파일의 경로
- **출력_파일_접두사**: 결과 파일이 저장될 경로 및 파일명 접두사

### 예제
```bash
# 로그 파일 분석 및 결과를 /output 경로에 저장
./target/release/trace /path/to/logfile.log /output/trace_result
# 결과: /output/trace_result_ufs.parquet 및 /output/trace_result_block.parquet 파일 생성

# 사용자 정의 latency 범위 사용
./target/release/trace -l 0.1,0.5,1,5,10,30,100,200,500,1000,1100 /path/to/logfile.log /output/trace_result

# 이미 생성된 UFS Parquet 파일 분석
./target/release/trace --parquet ufs /output/trace_result_ufs.parquet /output/new_result

# 이미 생성된 Block Parquet 파일 분석 (사용자 정의 latency 범위 적용)
./target/release/trace -l 0.1,1,10,100,1000 --parquet block /output/trace_result_block.parquet /output/new_result
```

## 설치 방법

### Cargo를 통한 설치
```bash
cargo install trace
```

### 바이너리 다운로드
GitHub [릴리스 페이지](https://github.com/kakaromo/trace/releases)에서 최신 바이너리를 다운로드할 수 있습니다.

