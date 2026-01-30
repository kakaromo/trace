# gRPC Filter 기능 예제

gRPC 서버를 통해 로그 처리 및 CSV 변환 시 필터를 적용하는 방법을 설명합니다.

## 필터 옵션

FilterOptions는 다음과 같은 필터링 기능을 제공합니다:

### 1. 시간 필터
- `start_time`: 시작 시간 (ms)
- `end_time`: 종료 시간 (ms)

### 2. 섹터/LBA 필터
- `start_sector`: 시작 섹터/LBA
- `end_sector`: 종료 섹터/LBA

### 3. 레이턴시 필터
- `min_dtoc`, `max_dtoc`: Device to Complete 레이턴시 범위 (ms)
- `min_ctoc`, `max_ctoc`: Complete to Complete 레이턴시 범위 (ms)
- `min_ctod`, `max_ctod`: Complete to Device 레이턴시 범위 (ms)

### 4. Queue Depth 필터
- `min_qd`: 최소 Queue Depth
- `max_qd`: 최대 Queue Depth

### 5. CPU 필터
- `cpu_list`: 필터링할 CPU 번호 목록

## Python 클라이언트 사용 예제

### 1. 시간 범위 필터

100ms ~ 500ms 시간 범위의 데이터만 처리:

```python
#!/usr/bin/env python3
import grpc
import log_processor_pb2
import log_processor_pb2_grpc
from grpc_client import create_filter_options, process_logs

server_address = "localhost:50051"

with grpc.insecure_channel(server_address) as channel:
    stub = log_processor_pb2_grpc.LogProcessorStub(channel)
    
    # 시간 필터 생성
    filter_opts = create_filter_options(
        start_time=100.0,
        end_time=500.0
    )
    
    # 필터를 적용하여 로그 처리
    process_logs(
        stub,
        source_bucket="trace",
        source_path="logs/trace.log",
        target_bucket="trace",
        target_path="filtered/time",
        log_type="ufs",
        filter_options=filter_opts
    )
```

### 2. 섹터 범위 필터

특정 섹터 범위의 I/O만 필터링:

```python
# 섹터 0 ~ 1,000,000 범위만 처리
filter_opts = create_filter_options(
    start_sector=0,
    end_sector=1000000
)

process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="filtered/sector",
    log_type="block",
    filter_options=filter_opts
)
```

### 3. 레이턴시 필터

높은 레이턴시(1ms ~ 10ms) 이벤트만 추출:

```python
# Device to Complete 레이턴시가 1ms ~ 10ms인 이벤트만
filter_opts = create_filter_options(
    min_dtoc=1.0,
    max_dtoc=10.0
)

process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="filtered/latency",
    log_type="ufs",
    filter_options=filter_opts
)
```

### 4. Queue Depth 필터

높은 Queue Depth(16-32) 상황만 분석:

```python
# QD가 16 이상 32 이하인 경우만
filter_opts = create_filter_options(
    min_qd=16,
    max_qd=32
)

process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="filtered/qd",
    log_type="block",
    filter_options=filter_opts
)
```

### 5. CPU 필터

특정 CPU 코어(0, 1, 2, 3)에서 발생한 이벤트만:

```python
# CPU 0, 1, 2, 3에서 발생한 이벤트만
filter_opts = create_filter_options(
    cpu_list=[0, 1, 2, 3]
)

process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="filtered/cpu",
    log_type="ufs",
    filter_options=filter_opts
)
```

### 6. 복합 필터

여러 필터를 동시에 적용:

```python
# 시간 + 섹터 + 레이턴시 복합 필터
filter_opts = create_filter_options(
    start_time=100.0,
    end_time=500.0,
    start_sector=0,
    end_sector=1000000,
    min_dtoc=1.0,
    max_dtoc=10.0,
    cpu_list=[0, 1]
)

process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="filtered/complex",
    log_type="ufs",
    filter_options=filter_opts
)
```

## CSV 변환 시 필터 적용

Parquet → CSV 변환 시에도 필터를 적용할 수 있습니다:

```python
from grpc_client import convert_to_csv

# 시간 필터를 적용한 CSV 변환
filter_opts = create_filter_options(
    start_time=100.0,
    end_time=500.0
)

convert_to_csv(
    stub,
    source_bucket="trace",
    source_parquet_path="parquet/ufs.parquet",
    target_bucket="trace",
    target_csv_path="csv/filtered",
    csv_prefix="ufs_100_500",
    filter_options=filter_opts
)
```

## 필터 적용 결과

필터 적용 시 콘솔에 필터링 결과가 표시됩니다:

```
Processing logs:
  Source: trace/logs/trace.log
  Target: trace/filtered/time
  Type: ufs
  Chunk Size: 100000
  Filter: Applied

Job ID: 12345678-1234-5678-9abc-def012345678
------------------------------------------------------------
[DOWNLOADING ] 10% | Downloading log file from trace/logs/trace.log
[PARSING     ] 40% | Parsing log file
[FILTER] Applying filters...
[FILTER] After filtering - Total: 85000 (UFS: 85000, Block: 0, UFSCUSTOM: 0)
[PARSING     ] 50% | Parsing completed: 985566 records
[CONVERTING  ] 60% | Converting to Parquet format
[UPLOADING   ] 75% | Uploading Parquet files to MinIO

✅ Processing completed successfully!
```

## 필터 비활성화

필터를 지정하지 않으면 모든 데이터가 처리됩니다:

```python
# 필터 없이 처리
process_logs(
    stub,
    source_bucket="trace",
    source_path="logs/trace.log",
    target_bucket="trace",
    target_path="output/all",
    log_type="ufs"
)
```

## 필터 조합 가이드

### 성능 분석용
```python
# 높은 레이턴시 + 높은 QD 조합
filter_opts = create_filter_options(
    min_dtoc=5.0,  # 5ms 이상
    min_qd=20      # QD 20 이상
)
```

### 특정 시간대 분석용
```python
# 특정 시간 범위의 특정 섹터
filter_opts = create_filter_options(
    start_time=1000.0,
    end_time=2000.0,
    start_sector=100000,
    end_sector=200000
)
```

### CPU 병목 분석용
```python
# 특정 CPU 코어의 높은 레이턴시 이벤트
filter_opts = create_filter_options(
    min_dtoc=3.0,
    cpu_list=[4, 5, 6, 7]
)
```

## 주의사항

1. **필터 값이 0인 경우**: 해당 필터는 비활성화됩니다
   - `start_time=0, end_time=0` → 시간 필터 없음
   - `min_dtoc=0, max_dtoc=0` → DTOC 레이턴시 필터 없음

2. **CPU 리스트가 비어있는 경우**: CPU 필터가 비활성화됩니다
   - `cpu_list=[]` → 모든 CPU 포함

3. **필터 적용 순서**: 모든 필터 조건을 동시에 만족하는 데이터만 반환됩니다 (AND 조건)

4. **성능 고려사항**: 필터를 적용하면 데이터 크기가 줄어들어 처리 속도가 빨라지고 저장 공간이 절약됩니다
