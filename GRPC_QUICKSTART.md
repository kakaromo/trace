# gRPC 서버 시작 가이드

## 1. 프로젝트 빌드

```bash
cargo build --release
```

## 2. 환경 변수 설정

```bash
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=trace-logs
```

## 3. gRPC 서버 실행

```bash
# 기본 포트(50051)로 실행
./target/release/trace --grpc-server

# 또는 커스텀 포트로 실행
./target/release/trace --grpc-server --port 9090
```

## 4. Python 클라이언트 사용

### 패키지 설치
```bash
pip install grpcio grpcio-tools
```

### Proto 파일에서 Python 코드 생성
```bash
cd examples
python -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/log_processor.proto
```

### 로그 처리 실행
```bash
python grpc_client.py process \
  trace-logs \
  logs/trace.csv \
  trace-parquet \
  output/data \
  ufs \
  100000
```

## 출력 예시

```
Processing logs:
  Source: trace-logs/logs/trace.csv
  Target: trace-parquet/output/data
  Type: ufs
  Chunk Size: 100000

Job ID: 12345678-1234-1234-1234-123456789abc
------------------------------------------------------------
[DOWNLOADING  ]  10% | Downloading log file from trace-logs/logs/trace.csv
[DOWNLOADING  ]  20% | Download completed
[PARSING      ]  30% | Parsing log file
[PARSING      ]  50% | Parsing completed: 123456 records
                    Records: 123,456
[CONVERTING   ]  60% | Converting to Parquet format
[CONVERTING   ]  70% | Conversion completed
[UPLOADING    ]  75% | Uploading Parquet files to MinIO
[COMPLETED    ] 100% | Processing completed successfully. Uploaded 3 files
                    Records: 123,456

Generated files:
  - output/data/ufs.parquet
  - output/data/block.parquet
  - output/data/ufscustom.parquet

✅ Processing completed successfully!
```

## 자세한 문서

- [gRPC 서버 문서](doc/grpc_server.md)
- [Python 클라이언트 가이드](examples/GRPC_CLIENT.md)
