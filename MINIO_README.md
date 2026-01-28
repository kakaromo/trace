# MinIO Integration - Quick Start

## 빠른 시작

### 1. MinIO 서버 시작

```bash
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --name minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  quay.io/minio/minio server /data --console-address ":9001"
```

### 2. 환경 변수 설정

```bash
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY="minioadmin"
export MINIO_SECRET_KEY="minioadmin"
export MINIO_BUCKET="trace"
```

### 3. 프로젝트 빌드

```bash
cargo build --release
```

## 사용법

### 기능 1: MinIO 로그 → Parquet → MinIO 업로드

```bash
# 1. 로그 파일을 MinIO에 업로드
mc cp test/input/blktrace_test.csv local/trace/logs/test.csv

# 2. 로그를 Parquet로 변환하고 MinIO에 저장 (통계/차트 X)
./target/release/trace --minio-log logs/test.csv output/parquet
```

### 기능 2: MinIO Parquet → 분석 + 차트 생성

```bash
# MinIO에서 Parquet 다운로드 후 분석 및 차트 생성
./target/release/trace --minio-analyze output/parquet/ufs.parquet test/output/analysis
```

### 기능 3: MinIO Parquet → CSV 변환 → MinIO 업로드

```bash
# Parquet를 CSV로 변환하여 MinIO에 저장 (Excel 호환)
# Type은 파일명에서 자동 감지 (ufs.parquet, block.parquet, ufscustom.parquet)
./target/release/trace --minio-csv output/parquet/ufs.parquet output/csv

# 또는 스크립트 사용
./run_minio_csv.sh output/parquet/ufs.parquet output/csv
```

## 테스트

상세 테스트는 스크립트를 확인하세요:
- run_minio.sh: 로그 → Parquet 변환
- run_minio_analysis.sh: Parquet 분석
- run_minio_csv.sh: Parquet → CSV 변환

## 상세 문서

전체 문서는 [doc/minio_integration.md](doc/minio_integration.md)를 참조하세요.

## 주요 변경사항

- ✅ MinIO S3 호환 스토리지 통합
- ✅ 로그 파일을 MinIO에서 읽고 Parquet로 변환 후 업로드
- ✅ MinIO의 Parquet 파일을 다운로드하여 분석 및 차트 생성
- ✅ Parquet 파일을 CSV로 변환하여 MinIO에 업로드 (Excel 호환)
- ✅ 환경 변수를 통한 MinIO 설정 관리
- ✅ 동기 방식의 간단한 API

## 새로운 파일

- `src/storage/mod.rs` - Storage 모듈
- `src/storage/minio_client.rs` - MinIO 클라이언트
- `doc/minio_integration.md` - 상세 가이드
- `run_minio.sh` - 로그 → Parquet 변환 스크립트
- `run_minio_analysis.sh` - Parquet 분석 스크립트
