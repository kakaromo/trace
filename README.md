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
# 개발 모드로 빌드한 바이너리 실행
./target/debug/trace <로그_파일_경로> <출력_파일_접두사>

# 릴리스 모드로 빌드한 바이너리 실행
./target/release/trace <로그_파일_경로> <출력_파일_접두사>
```

### 명령줄 인자 설명
- **로그_파일_경로**: 분석할 로그 파일의 경로 (첫 번째 인자)
- **출력_파일_접두사**: Parquet 파일이 저장될 경로 및 파일명 접두사 (두 번째 인자)
  - 프로그램은 이 접두사에 "_ufs.parquet"과 "_block.parquet"을 각각 추가하여 두 개의 파일을 생성합니다.

### 예제
```bash
# 로그 파일 분석 및 결과를 /output 경로에 저장
./target/release/trace /path/to/logfile.log /output/trace_result
# 결과: /output/trace_result_ufs.parquet 및 /output/trace_result_block.parquet 파일 생성
```

## 설치 방법

### Cargo를 통한 설치
```bash
cargo install trace
```

### 바이너리 다운로드
GitHub [릴리스 페이지](https://github.com/kakaromo/trace/releases)에서 최신 바이너리를 다운로드할 수 있습니다.

