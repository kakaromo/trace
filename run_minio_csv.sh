#!/bin/bash

# MinIO Parquet to CSV 변환 스크립트
# Parquet 파일을 MinIO에서 다운로드하여 CSV로 변환하고 다시 MinIO에 업로드합니다.
# Type은 파일명에서 자동으로 감지됩니다 (ufs.parquet, block.parquet, ufscustom.parquet)

set -e

# MinIO 설정 (환경 변수 또는 기본값)
export MINIO_ENDPOINT="${MINIO_ENDPOINT:-http://localhost:9000}"
export MINIO_ACCESS_KEY="${MINIO_ACCESS_KEY:?환경 변수 MINIO_ACCESS_KEY를 설정하세요}"
export MINIO_SECRET_KEY="${MINIO_SECRET_KEY:?환경 변수 MINIO_SECRET_KEY를 설정하세요}"
export MINIO_BUCKET="${MINIO_BUCKET:-trace}"

# 기본 경로 설정
REMOTE_PARQUET_PATH="${1:-output/parquet/ufs.parquet}"
REMOTE_CSV_PATH="${2:-output/csv}"

# 파일명에서 타입 추출 (검증용)
if [[ "$REMOTE_PARQUET_PATH" == *"ufs.parquet"* ]]; then
    DETECTED_TYPE="ufs"
elif [[ "$REMOTE_PARQUET_PATH" == *"block.parquet"* ]]; then
    DETECTED_TYPE="block"
elif [[ "$REMOTE_PARQUET_PATH" == *"ufscustom.parquet"* ]]; then
    DETECTED_TYPE="ufscustom"
else
    echo "Error: Cannot detect trace type from filename."
    echo "Please use 'ufs.parquet', 'block.parquet', or 'ufscustom.parquet' in the filename."
    exit 1
fi

echo "======================================"
echo "MinIO Parquet to CSV Conversion"
echo "======================================"
echo "Endpoint: $MINIO_ENDPOINT"
echo "Bucket: $MINIO_BUCKET"
echo "Source Parquet: $REMOTE_PARQUET_PATH"
echo "Target CSV Path: $REMOTE_CSV_PATH"
echo "Detected Type: $DETECTED_TYPE"
echo "======================================"
echo ""

# Parquet를 CSV로 변환하고 MinIO에 업로드
./target/release/trace --minio-csv "$REMOTE_PARQUET_PATH" "$REMOTE_CSV_PATH"

echo ""
echo "======================================"
echo "CSV Conversion Complete!"
echo "======================================"
echo ""
echo "CSV 파일 확인:"
echo "  mc ls local/$MINIO_BUCKET/$REMOTE_CSV_PATH/"
echo ""
echo "CSV 파일 다운로드 예제:"
echo "  mc cp local/$MINIO_BUCKET/$REMOTE_CSV_PATH/${DETECTED_TYPE}_*.csv ./"
