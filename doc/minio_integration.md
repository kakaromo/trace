# MinIO 통합 기능 가이드

## 개요

이 trace 분석 도구는 MinIO 스토리지와 통합되어 다음 기능을 제공합니다:

1. **MinIO에서 로그 읽기 → Parquet 생성 → MinIO에 저장**: 로그 파일을 MinIO에서 읽어 Parquet로 변환하고 다시 MinIO에 저장 (통계/차트 생성 없음)
2. **MinIO에서 Parquet 읽기 → 분석 + 차트 생성**: MinIO에 저장된 Parquet 파일을 다운로드하여 기존과 동일하게 분석 및 차트 생성
3. **MinIO에서 Parquet 읽기 → CSV 변환 → MinIO에 저장**: Parquet 파일을 CSV로 변환하여 MinIO에 저장 (Excel 호환성)

---

## 사전 준비

### 1. MinIO 설치 및 실행

#### Docker를 사용한 MinIO 실행
```bash
docker run -d \
  -p 9000:9000 \
  -p 9001:9001 \
  --name minio \
  -e "MINIO_ROOT_USER=minioadmin" \
  -e "MINIO_ROOT_PASSWORD=minioadmin" \
  -v /tmp/minio-data:/data \
  quay.io/minio/minio server /data --console-address ":9001"
```

#### MinIO 웹 콘솔 접속
- URL: http://localhost:9001
- Username: `minioadmin`
- Password: `minioadmin`

### 2. 버킷 생성

MinIO 콘솔에서 `trace` 버킷을 생성하거나, CLI를 사용:

```bash
# MinIO 클라이언트 설치
brew install minio/stable/mc

# MinIO 서버 설정
mc alias set local http://localhost:9000 minioadmin minioadmin

# 버킷 생성
mc mb local/trace
```

### 3. 환경 변수 설정

```bash
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY="minioadmin"
export MINIO_SECRET_KEY="minioadmin"
export MINIO_BUCKET="trace"
export MINIO_REGION="us-east-1"
```

또는 `.env` 파일 생성:
```bash
cat > .env << 'EOF'
MINIO_ENDPOINT=http://localhost:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin
MINIO_BUCKET=trace
MINIO_REGION=us-east-1
EOF

# 환경 변수 로드
export $(cat .env | xargs)
```

---

## 사용 방법

### 기능 1: MinIO 로그 → Parquet → MinIO 업로드

로그 파일을 MinIO에서 읽어서 Parquet로 변환하고, 다시 MinIO에 저장합니다.  
**통계 계산이나 차트 생성은 하지 않습니다** (bottomhalf 처리만 수행).

#### 사용법
```bash
./trace --minio-log <remote_log_path> <remote_output_path>
```

#### 예제

1. **먼저 로그 파일을 MinIO에 업로드:**
```bash
# CLI를 사용한 업로드
mc cp test/input/blktrace_test.csv local/trace/logs/blktrace_test.csv

# 또는 웹 콘솔에서 업로드
```

2. **MinIO 로그를 Parquet로 변환하고 업로드:**
```bash
./trace --minio-log logs/blktrace_test.csv output/parquet
```

3. **결과 확인:**
```bash
# MinIO에 저장된 Parquet 파일 확인
mc ls local/trace/output/parquet/

# 출력 예시:
# [2026-01-27 10:30:15 KST]  1.2MiB output_ufs.parquet
# [2026-01-27 10:30:16 KST]  800KiB output_block.parquet
```

#### 동작 과정
1. MinIO에서 로그 파일 다운로드 → `/tmp/trace_temp_log.txt`
2. 로그 파일 파싱 (UFS, Block, UFSCustom 감지)
3. Bottom-half 레이턴시 처리
4. Parquet 파일 생성 → `/tmp/trace_temp_output_*.parquet`
5. Parquet 파일을 MinIO에 업로드
6. 로컬 임시 파일 삭제

---

### 기능 2: MinIO Parquet → 분석 + 차트 생성

MinIO에 저장된 Parquet 파일을 다운로드하여 기존과 동일하게 통계 분석 및 차트를 생성합니다.

#### 사용법
```bash
./trace --minio-analyze <remote_parquet_path> <local_output_prefix>
```

#### 예제

1. **UFS Parquet 분석:**
```bash
./trace --minio-analyze output/parquet/output_ufs.parquet test/output/ufs_analysis
```

2. **Block Parquet 분석:**
```bash
./trace --minio-analyze output/parquet/output_block.parquet test/output/block_analysis
```

3. **생성된 파일 확인:**
```bash
ls -lh test/output/

# 출력 예시:
# ufs_analysis_result.log          - 통계 결과
# ufs_analysis_ufs_dtoc.html       - DTOC 차트
# ufs_analysis_ufs_dtoc.png        - DTOC 차트 (PNG)
# ufs_analysis_ufs_ctoc.html       - CTOC 차트
# ...
```

#### 동작 과정
1. MinIO에서 Parquet 파일 다운로드 → `/tmp/trace_temp_*.parquet`
2. Parquet 파일 로드 및 파싱
3. 통계 계산 (레이턴시, 분포, 큐 깊이 등)
4. Plotters 차트 생성 (HTML + PNG)
5. 로컬 임시 파일 삭제

---

### 기능 3: MinIO Parquet → CSV 변환 → MinIO 업로드

Parquet 파일을 MinIO에서 다운로드하여 CSV로 변환하고, 다시 MinIO에 저장합니다.  
**Excel에서 열기 쉽도록 CSV 형식으로 변환** (1,048,575 행 단위로 자동 분할).

#### 사용법
```bash
./trace --minio-csv <remote_parquet_path> <remote_csv_path>
```

- `<remote_parquet_path>`: MinIO에 저장된 Parquet 파일 경로 (파일명에 ufs.parquet, block.parquet, 또는 ufscustom.parquet 포함 필요)
- `<remote_csv_path>`: CSV 파일을 저장할 MinIO 경로
- 트레이스 타입은 파일명에서 자동으로 감지됩니다

#### 예제

1. **UFS Parquet를 CSV로 변환:**
```bash
./trace --minio-csv output/parquet/ufs.parquet output/csv
```

2. **Block Parquet를 CSV로 변환:**
```bash
./trace --minio-csv output/parquet/block.parquet output/csv
```

3. **스크립트 사용:**
```bash
# 환경 변수 설정
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY="minioadmin"
export MINIO_SECRET_KEY="minioadmin"
export MINIO_BUCKET="trace"

# CSV 변환 실행
./run_minio_csv.sh output/parquet/ufs.parquet output/csv
```

4. **결과 확인:**
```bash
# MinIO에 저장된 CSV 파일 확인
mc ls local/trace/output/csv/

# 출력 예시:
# [2026-01-29 10:30:15 KST]  2.5MiB ufs_0.0_1000.5.csv
# [2026-01-29 10:30:16 KST]  2.3MiB ufs_1000.5_2000.8.csv

# CSV 파일 다운로드
mc cp local/trace/output/csv/ufs_0.0_1000.5.csv ./
```

#### 동작 과정
1. MinIO에서 Parquet 파일 다운로드 → `$HOME/trace_temp.parquet`
2. Parquet 데이터 로드 (UFS/Block/UFSCUSTOM)
3. CSV 파일 생성 (시간 범위별로 자동 분할, Excel 행 제한 준수)
4. CSV 파일들을 MinIO에 업로드
5. 로컬 임시 파일 삭제

#### 특징
- **Excel 호환성**: 최대 1,048,575 행으로 자동 분할
- **시간 범위 파일명**: `ufs_<start_time>_<end_time>.csv`, `block_<start_time>_<end_time>.csv`
- **대용량 데이터 처리**: 자동으로 여러 파일로 분할하여 저장
- **효율적인 저장**: MinIO에 직접 업로드하여 로컬 디스크 사용 최소화

---

## 고급 사용법

### 옵션과 함께 사용

#### 청크 크기 설정
```bash
# Parquet 쓰기 시 청크 크기 지정 (기본: 50000)
./trace -c 100000 --minio-log logs/large_trace.csv output/parquet
```

#### Y축 범위 설정
```bash
# 차트 분석 시 Y축 범위 지정
./trace -y ufs_dtoc:0:10,ufs_qd:0:100 \
  --minio-analyze output/parquet/output_ufs.parquet test/output/custom
```

---

## 통합 워크플로우 예제

### 시나리오: 로그 수집 → 저장 → 분석

```bash
#!/bin/bash

# 1. 환경 변수 설정
export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY="minioadmin"
export MINIO_SECRET_KEY="minioadmin"
export MINIO_BUCKET="trace"

# 2. 로그 파일을 MinIO에 업로드
mc cp /var/log/trace/device.log local/trace/logs/2026-01-27/device.log

# 3. MinIO 로그를 Parquet로 변환하고 저장
./trace --minio-log logs/2026-01-27/device.log archive/2026-01-27/parquet

# 4. Parquet 파일을 다운로드하여 분석
./trace --minio-analyze archive/2026-01-27/parquet/output_ufs.parquet \
  reports/2026-01-27/analysis

# 5. 결과 확인
echo "Analysis complete!"
ls -lh reports/2026-01-27/analysis/
```

---

## 파일 구조

### MinIO 버킷 구조 예시
```
trace/                          # 버킷
├── logs/                       # 원본 로그 파일
│   ├── 2026-01-27/
│   │   └── device.log
│   └── 2026-01-28/
│       └── device.log
├── archive/                    # 처리된 Parquet 파일
│   ├── 2026-01-27/
│   │   └── parquet/
│   │       ├── output_ufs.parquet
│   │       └── output_block.parquet
│   └── 2026-01-28/
│       └── parquet/
│           └── output_ufs.parquet
├── output/                     # 변환된 CSV 파일
│   └── csv/
│       ├── ufs_0.0_1000.5.csv
│       ├── ufs_1000.5_2000.8.csv
│       └── block_0.0_1500.2.csv
└── reports/                    # 분석 보고서 (선택사항)
```

### 로컬 출력 구조
```
test/output/
├── analysis_result.log         # 통계 결과 로그
├── analysis_ufs_dtoc.html      # DTOC 차트 (HTML)
├── analysis_ufs_dtoc.png       # DTOC 차트 (PNG)
├── analysis_ufs_ctoc.html      # CTOC 차트
├── analysis_ufs_ctoc.png
├── analysis_ufs_ctod.html      # CTOD 차트
├── analysis_ufs_ctod.png
├── analysis_ufs_qd.html        # Queue Depth 차트
├── analysis_ufs_qd.png
└── ...
```

---

## 트러블슈팅

### 1. MinIO 연결 실패
```
Error: Failed to load MinIO configuration
```

**해결책:**
- 환경 변수가 올바르게 설정되었는지 확인
- MinIO 서버가 실행 중인지 확인: `docker ps | grep minio`

### 2. 버킷이 존재하지 않음
```
Error: The specified bucket does not exist
```

**해결책:**
```bash
# 버킷 생성
mc mb local/trace
```

### 3. 권한 오류
```
Error: Access Denied
```

**해결책:**
- Access Key와 Secret Key가 올바른지 확인
- MinIO 버킷 정책 확인

### 4. 파일 타입 감지 실패
```
Error: Cannot detect trace type from file name
```

**해결책:**
- Parquet 파일명에 `ufs.parquet`, `block.parquet` 또는 `ufscustom.parquet` 포함 필요
- 예: `output_ufs.parquet`, `trace_block.parquet`, `sample_ufscustom.parquet`

---

## 성능 최적화

### 대용량 파일 처리

#### 1. 청크 크기 조정
```bash
# 더 큰 청크로 메모리 효율성 증가
./trace -c 100000 --minio-log logs/large.log output/parquet
```

#### 2. 병렬 처리
```bash
# 여러 로그 파일을 병렬로 처리
parallel -j 4 ./trace --minio-log {} output/parquet/{/.} ::: logs/*.log
```

---

## 환경 변수 참조

| 변수 | 설명 | 기본값 | 필수 |
|------|------|--------|------|
| `MINIO_ENDPOINT` | MinIO 서버 주소 | `http://localhost:9000` | 아니오 |
| `MINIO_ACCESS_KEY` | Access Key | - | **예** |
| `MINIO_SECRET_KEY` | Secret Key | - | **예** |
| `MINIO_BUCKET` | 버킷 이름 | `trace` | 아니오 |
| `MINIO_REGION` | 리전 이름 | `us-east-1` | 아니오 |

---

## 관련 문서

- [MinIO 공식 문서](https://min.io/docs/minio/linux/index.html)
- [Parquet 포맷 가이드](../doc/parquet_migration_tech_summary.md)
- [성능 최적화 가이드](../doc/highperf_user_guide.md)

---

## 라이선스

이 기능은 메인 프로젝트와 동일한 라이선스를 따릅니다.
