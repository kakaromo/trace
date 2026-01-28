# gRPC Server for Log Processing

MinIO에서 로그 파일을 읽어 파싱하고 Parquet으로 변환하여 업로드하는 gRPC 서버입니다.

## 기능

- ✅ MinIO에서 로그 파일 다운로드
- ✅ 로그 파싱 (UFS, Block, UFSCustom 지원)
- ✅ Parquet 포맷으로 변환
- ✅ MinIO에 Parquet 파일 업로드
- ✅ 실시간 진행 상황 스트리밍
- ✅ 작업 상태 조회
- ✅ 파일 목록 조회

## 빌드

```bash
cargo build --release
```

## 환경 변수 설정

gRPC 서버 실행 전에 다음 환경 변수를 설정해야 합니다:

```bash
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=trace-logs
```

## 서버 실행

```bash
# 기본 포트(50051)로 실행
./target/release/trace --grpc-server

# 커스텀 포트로 실행
./target/release/trace --grpc-server --port 9090
```

## gRPC API

### 1. ProcessLogs (스트리밍 응답)

로그 파일을 처리하고 실시간으로 진행 상황을 전송합니다.

**요청:**
```protobuf
message ProcessLogsRequest {
  string source_bucket = 1;      // 소스 버킷
  string source_path = 2;        // 로그 파일 경로
  string target_bucket = 3;      // 타겟 버킷
  string target_path = 4;        // Parquet 저장 경로
  string log_type = 5;           // 로그 타입 (ufs/block/ufscustom/auto)
  optional int32 chunk_size = 6; // 청크 크기 (기본: 100000)
}
```

**스트리밍 응답:**
```protobuf
message ProcessLogsProgress {
  string job_id = 1;                  // 작업 ID
  ProgressStage stage = 2;            // 진행 단계
  string message = 3;                 // 진행 메시지
  int32 progress_percent = 4;         // 진행률 (0-100)
  int64 records_processed = 5;        // 처리된 레코드 수
  optional bool success = 6;          // 성공 여부 (완료시)
  optional string error = 7;          // 에러 메시지 (실패시)
  repeated string output_files = 8;   // 생성된 파일 목록
}
```

**진행 단계:**
- `STAGE_DOWNLOADING` (1): MinIO에서 다운로드 중
- `STAGE_PARSING` (2): 로그 파싱 중
- `STAGE_CONVERTING` (3): Parquet 변환 중
- `STAGE_UPLOADING` (4): MinIO에 업로드 중
- `STAGE_COMPLETED` (5): 완료
- `STAGE_FAILED` (6): 실패

### 2. GetJobStatus

작업 상태를 조회합니다.

**요청:**
```protobuf
message JobStatusRequest {
  string job_id = 1;
}
```

**응답:**
```protobuf
message JobStatusResponse {
  string job_id = 1;
  ProgressStage stage = 2;
  string message = 3;
  int32 progress_percent = 4;
  int64 records_processed = 5;
  bool is_completed = 6;
  optional bool success = 7;
  optional string error = 8;
}
```

### 3. ListFiles

버킷의 파일 목록을 조회합니다.

**요청:**
```protobuf
message ListFilesRequest {
  string bucket = 1;
  string prefix = 2;
}
```

**응답:**
```protobuf
message ListFilesResponse {
  repeated string files = 1;
}
```

## 클라이언트 예제

Python 클라이언트 예제는 `examples/grpc_client.py`를 참조하세요.

자세한 사용법은 [GRPC_CLIENT.md](examples/GRPC_CLIENT.md)를 확인하세요.

### 빠른 시작

```bash
# 1. Python 패키지 설치
pip install grpcio grpcio-tools

# 2. Proto 파일에서 Python 코드 생성
cd examples
python -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/log_processor.proto

# 3. 로그 처리 요청
python grpc_client.py process \
  trace-logs \
  logs/trace.csv \
  trace-parquet \
  output/data \
  ufs \
  100000
```

## 아키텍처

```
┌──────────────┐
│   클라이언트   │
│  (gRPC)      │
└──────┬───────┘
       │ ProcessLogs (스트리밍)
       │ GetJobStatus
       │ ListFiles
       ▼
┌──────────────────────────────────────┐
│         gRPC Server                  │
│  ┌────────────────────────────────┐  │
│  │  LogProcessorService           │  │
│  │  - process_logs_internal       │  │
│  │  - update_job_status           │  │
│  └────────────────────────────────┘  │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│           MinIO Client               │
│  - download_file()                   │
│  - upload_file()                     │
│  - list_files()                      │
└──────┬───────────────────────────────┘
       │
       ▼
┌──────────────────────────────────────┐
│  MinIO Server                        │
│  - Source Bucket (로그 파일)         │
│  - Target Bucket (Parquet 파일)      │
└──────────────────────────────────────┘
```

## 처리 흐름

```
1. DOWNLOADING (10-20%)
   ├─ MinIO에서 로그 파일 다운로드
   └─ 임시 디렉토리에 저장

2. PARSING (30-50%)
   ├─ 로그 파일 타입 자동 감지
   ├─ 고성능 파서로 파싱
   └─ 메모리에 데이터 로드

3. CONVERTING (60-70%)
   ├─ Parquet 포맷으로 변환
   ├─ 압축 적용 (SNAPPY/ZSTD)
   └─ 임시 파일에 저장

4. UPLOADING (75-95%)
   ├─ MinIO에 Parquet 파일 업로드
   │  ├─ ufs.parquet
   │  ├─ block.parquet
   │  └─ ufscustom.parquet
   └─ 임시 파일 정리

5. COMPLETED (100%)
   └─ 완료 메시지 전송
```

## 테스트

### MinIO 서버 실행 (Docker)

```bash
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --name minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  quay.io/minio/minio server /data --console-address ":9001"
```

### 버킷 생성

```bash
# MinIO 클라이언트 설치
brew install minio/stable/mc

# MinIO 서버 추가
mc alias set local http://localhost:9000 minioadmin minioadmin

# 버킷 생성
mc mb local/trace-logs
mc mb local/trace-parquet
```

### 테스트 로그 파일 업로드

```bash
# 테스트 로그 파일을 MinIO에 업로드
mc cp test/input/blktrace_test.csv local/trace-logs/logs/test.csv
```

### gRPC 서버 실행

```bash
export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=minioadmin
export MINIO_SECRET_KEY=minioadmin
export MINIO_BUCKET=trace-logs

./target/release/trace --grpc-server
```

### 클라이언트로 테스트

```bash
cd examples
python grpc_client.py process trace-logs logs/test.csv trace-parquet output/data ufs
```

## 문제 해결

### "Failed to load MinIO configuration" 오류

환경 변수가 제대로 설정되었는지 확인하세요:

```bash
echo $MINIO_ENDPOINT
echo $MINIO_ACCESS_KEY
echo $MINIO_SECRET_KEY
```

### gRPC 연결 실패

1. 서버가 실행 중인지 확인
2. 방화벽에서 포트가 열려있는지 확인
3. 클라이언트의 서버 주소가 올바른지 확인

### 파일을 찾을 수 없음

1. MinIO 버킷이 존재하는지 확인
2. 파일 경로가 올바른지 확인
3. MinIO 권한 확인

## 성능

- **처리 속도**: 약 50,000-100,000 레코드/초
- **메모리 사용량**: 파일 크기의 2-3배
- **압축률**: 원본의 10-30% (ZSTD 압축 사용)

## 라이선스

이 프로젝트는 프로젝트 루트의 라이선스를 따릅니다.
