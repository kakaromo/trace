# gRPC Python 클라이언트 설정

## 1. Python 패키지 설치

```bash
pip install grpcio grpcio-tools
```

## 2. Proto 파일에서 Python 코드 생성

```bash
cd examples
python3 -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/log_processor.proto
```

이 명령어는 다음 파일들을 생성합니다:
- `log_processor_pb2.py` - 메시지 정의
- `log_processor_pb2_grpc.py` - gRPC 서비스 정의

## 3. 클라이언트 사용 예제

### 로그 파일 처리 (파싱 + Parquet 변환 + 업로드)

```bash
python3 grpc_client.py process \
  trace \
  standby/standby/standby.log \
  trace \
  standby/standby/trace \
  ufs \
  100000
```

파라미터:
- `trace-logs`: 소스 버킷 이름
- `logs/trace.csv`: 소스 로그 파일 경로
- `trace-parquet`: 타겟 버킷 이름
- `output/data`: Parquet 파일이 저장될 경로
- `ufs`: 로그 타입 (ufs, block, ufscustom, auto)
- `100000`: 청크 크기 (옵션)

### 작업 상태 조회

```bash
python3 grpc_client.py status 12345678-1234-1234-1234-123456789abc
```

### 버킷의 파일 목록 조회

```bash
# 전체 파일 목록
python3 grpc_client.py list trace-logs

# 특정 prefix의 파일만 조회
python3 grpc_client.py list trace-logs logs/
```

## 4. 서버 실행

먼저 Rust gRPC 서버를 실행해야 합니다:

```bash
# 환경 변수 설정
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=trace-logs

# gRPC 서버 실행
./target/release/trace --grpc-server --port 50051
```

## 5. 진행 상황 모니터링

클라이언트는 스트리밍 응답을 통해 실시간으로 진행 상황을 출력합니다:

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

## 6. 에러 처리

클라이언트는 다음과 같은 에러를 처리합니다:
- gRPC 연결 실패
- 파일을 찾을 수 없음
- 파싱 에러
- MinIO 연결 실패
- 업로드 실패

에러 발생 시 상세한 에러 메시지가 표시됩니다.
