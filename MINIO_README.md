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
./target/release/trace --minio-analyze output/parquet/parquet_ufs.parquet test/output/analysis
```

## 테스트 스크립트

```bash
./test_minio.sh
```

## 상세 문서

전체 문서는 [doc/minio_integration.md](doc/minio_integration.md)를 참조하세요.

## 주요 변경사항

- ✅ MinIO S3 호환 스토리지 통합
- ✅ 로그 파일을 MinIO에서 읽고 Parquet로 변환 후 업로드
- ✅ MinIO의 Parquet 파일을 다운로드하여 분석 및 차트 생성
- ✅ 환경 변수를 통한 MinIO 설정 관리
- ✅ 동기 방식의 간단한 API

## 새로운 파일

- `src/storage/mod.rs` - Storage 모듈
- `src/storage/minio_client.rs` - MinIO 클라이언트
- `doc/minio_integration.md` - 상세 가이드
- `test_minio.sh` - 테스트 스크립트
