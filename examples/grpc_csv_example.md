# gRPC CSV 변환 예제

MinIO에 저장된 Parquet 파일을 CSV로 변환하여 다시 MinIO에 업로드하는 예제입니다.

## 사전 준비

### 1. gRPC 서버 실행
```bash
# 환경 변수 설정
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=trace

# gRPC 서버 실행
./target/release/trace --grpc-server --port 50051
```

### 2. Python 클라이언트 proto 파일 생성
```bash
cd examples
python3 -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/log_processor.proto
```

## CSV 변환 실행

### UFS Parquet → CSV
```bash
python3 grpc_client.py csv trace output/parquet/ufs.parquet trace output/csv
```

### Block Parquet → CSV
```bash
python3 grpc_client.py csv trace output/parquet/block.parquet trace output/csv
```

### UFSCUSTOM Parquet → CSV
```bash
python3 grpc_client.py csv trace output/parquet/ufscustom.parquet trace output/csv
```

## 진행 상황

실시간으로 다음 단계의 진행 상황을 확인할 수 있습니다:

1. **DOWNLOADING (10-30%)**: MinIO에서 Parquet 다운로드
2. **CONVERTING (40-60%)**: Parquet를 CSV로 변환
3. **UPLOADING (70-90%)**: CSV 파일을 MinIO에 업로드
4. **COMPLETED (100%)**: 변환 완료

## 출력 예시

```
Converting Parquet to CSV:
  Source: trace/output/parquet/ufs.parquet
  Target: trace/output/csv

Job ID: 12345678-1234-5678-9abc-def012345678
------------------------------------------------------------
[DOWNLOADING ] 10% | Downloading Parquet from trace/output/parquet/ufs.parquet
[DOWNLOADING ] 30% | Download completed
[CONVERTING  ] 40% | Converting ufs Parquet to CSV
[CONVERTING  ] 60% | CSV conversion completed
                    Records: 985,566
[UPLOADING   ] 70% | Uploading CSV files to MinIO
[COMPLETED   ] 100% | CSV conversion completed successfully

Generated CSV files:
  - output/csv/ufs.csv

✅ CSV conversion completed successfully!
```

## 생성되는 CSV 파일

변환 후 다음 CSV 파일이 생성됩니다:
- `ufs.parquet` → `ufs.csv`
- `block.parquet` → `block.csv`
- `ufscustom.parquet` → `ufscustom.csv`

## 오류 처리

변환 실패 시 상세한 오류 메시지가 표시됩니다:

```
[FAILED      ] 0% | CSV conversion failed

❌ CSV conversion failed: Failed to read UFS parquet: Invalid file format
```

## 타입 자동 감지

Parquet 파일명에서 자동으로 타입을 감지합니다:
- `ufs.parquet` → UFS 타입
- `block.parquet` → Block 타입
- `ufscustom.parquet` → UFSCUSTOM 타입

파일명에 타입이 포함되어 있어야 합니다.
