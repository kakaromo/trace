#!/bin/bash

# MinIO 환경 변수 설정 (실제 사용 시 환경에 맞게 수정하세요)
# export MINIO_ENDPOINT="${MINIO_ENDPOINT:-http://localhost:9000}"
# export MINIO_ACCESS_KEY="${MINIO_ACCESS_KEY:?MINIO_ACCESS_KEY must be set}"
# export MINIO_SECRET_KEY="${MINIO_SECRET_KEY:?MINIO_SECRET_KEY must be set}"
# export MINIO_BUCKET="${MINIO_BUCKET:-trace}"

# ./target/release/trace --minio-log standby/standby.log standby/trace

export MINIO_ENDPOINT=http://localhost:9000
export MINIO_ACCESS_KEY=admin
export MINIO_SECRET_KEY=tka123tjd!
export MINIO_BUCKET=trace

RUST_BACKTRACE=1 ./target/release/trace --grpc-server
