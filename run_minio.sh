#!/bin/bash

export MINIO_ENDPOINT="http://localhost:9000"
export MINIO_ACCESS_KEY="admin"
export MINIO_SECRET_KEY="tka123tjd!"
export MINIO_BUCKET="trace"

./target/release/trace --minio-log standby/standy.log standby/trace
