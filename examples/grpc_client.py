#!/usr/bin/env python3
"""
gRPC 클라이언트 예제
로그 파일을 MinIO에서 읽어 파싱하고 Parquet으로 변환하여 업로드합니다.
"""

import grpc
import sys
from typing import Iterator

# proto 파일에서 생성된 모듈 임포트
# 생성 방법: python3 -m grpc_tools.protoc -I../proto --python_out=. --grpc_python_out=. ../proto/log_processor.proto
import log_processor_pb2
import log_processor_pb2_grpc


def process_logs(
    stub: log_processor_pb2_grpc.LogProcessorStub,
    source_bucket: str,
    source_path: str,
    target_bucket: str,
    target_path: str,
    log_type: str = "auto",
    chunk_size: int = 100000,
) -> None:
    """로그 파일 처리 요청 및 진행 상황 모니터링"""
    
    request = log_processor_pb2.ProcessLogsRequest(
        source_bucket=source_bucket,
        source_path=source_path,
        target_bucket=target_bucket,
        target_path=target_path,
        log_type=log_type,
        chunk_size=chunk_size,
    )

    print(f"Processing logs:")
    print(f"  Source: {source_bucket}/{source_path}")
    print(f"  Target: {target_bucket}/{target_path}")
    print(f"  Type: {log_type}")
    print(f"  Chunk Size: {chunk_size}")
    print()

    try:
        # 스트리밍 응답 수신
        responses: Iterator[log_processor_pb2.ProcessLogsProgress] = stub.ProcessLogs(request)
        
        job_id = None
        for response in responses:
            if job_id is None:
                job_id = response.job_id
                print(f"Job ID: {job_id}")
                print("-" * 60)

            # 진행 단계 이름
            stage_names = {
                0: "UNKNOWN",
                1: "DOWNLOADING",
                2: "PARSING",
                3: "CONVERTING",
                4: "UPLOADING",
                5: "COMPLETED",
                6: "FAILED",
            }
            
            stage_name = stage_names.get(response.stage, "UNKNOWN")
            
            print(f"[{stage_name:12}] {response.progress_percent:3}% | {response.message}")
            
            if response.records_processed > 0:
                print(f"                    Records: {response.records_processed:,}")
            
            # 완료 시 출력 파일 표시
            if response.output_files:
                print("\nGenerated files:")
                for file in response.output_files:
                    print(f"  - {file}")
            
            # 성공/실패 처리 - 완료 또는 실패 단계에서만 종료
            if response.stage == 5:  # COMPLETED
                if response.HasField('success') and response.success:
                    print("\n✅ Processing completed successfully!")
                else:
                    print("\n✅ Processing completed!")
                break
            elif response.stage == 6:  # FAILED
                error_msg = response.error if response.HasField('error') else "Unknown error"
                print(f"\n❌ Processing failed: {error_msg}")
                break
        
    except grpc.RpcError as e:
        print(f"\n❌ RPC Error: {e.code()}: {e.details()}")


def get_job_status(stub: log_processor_pb2_grpc.LogProcessorStub, job_id: str) -> None:
    """작업 상태 조회"""
    
    request = log_processor_pb2.JobStatusRequest(job_id=job_id)
    
    try:
        response = stub.GetJobStatus(request)
        
        stage_names = {
            0: "UNKNOWN",
            1: "DOWNLOADING",
            2: "PARSING",
            3: "CONVERTING",
            4: "UPLOADING",
            5: "COMPLETED",
            6: "FAILED",
        }
        
        stage_name = stage_names.get(response.stage, "UNKNOWN")
        
        print(f"Job Status: {job_id}")
        print(f"  Stage: {stage_name}")
        print(f"  Progress: {response.progress_percent}%")
        print(f"  Message: {response.message}")
        print(f"  Records: {response.records_processed:,}")
        print(f"  Completed: {response.is_completed}")
        
        if response.success is not None:
            if response.success:
                print("  Result: ✅ Success")
            else:
                print(f"  Result: ❌ Failed - {response.error}")
                
    except grpc.RpcError as e:
        print(f"❌ RPC Error: {e.code()}: {e.details()}")


def list_files(stub: log_processor_pb2_grpc.LogProcessorStub, bucket: str, prefix: str = "") -> None:
    """버킷의 파일 목록 조회"""
    
    request = log_processor_pb2.ListFilesRequest(bucket=bucket, prefix=prefix)
    
    try:
        response = stub.ListFiles(request)
        
        print(f"Files in bucket '{bucket}' with prefix '{prefix}':")
        print("-" * 60)
        
        if not response.files:
            print("  (No files found)")
        else:
            for file in response.files:
                print(f"  {file}")
        
        print(f"\nTotal: {len(response.files)} files")
        
    except grpc.RpcError as e:
        print(f"❌ RPC Error: {e.code()}: {e.details()}")


def convert_to_csv(
    stub: log_processor_pb2_grpc.LogProcessorStub,
    source_bucket: str,
    source_parquet_path: str,
    target_bucket: str,
    target_csv_path: str,
    csv_prefix: str = None,
) -> None:
    """Parquet를 CSV로 변환 요청 및 진행 상황 모니터링"""
    
    request = log_processor_pb2.ConvertToCsvRequest(
        source_bucket=source_bucket,
        source_parquet_path=source_parquet_path,
        target_bucket=target_bucket,
        target_csv_path=target_csv_path,
    )
    
    # csv_prefix가 지정된 경우에만 설정
    if csv_prefix:
        request.csv_prefix = csv_prefix

    print(f"Converting Parquet to CSV:")
    print(f"  Source: {source_bucket}/{source_parquet_path}")
    print(f"  Target: {target_bucket}/{target_csv_path}")
    if csv_prefix:
        print(f"  CSV Prefix: {csv_prefix}")
    print()

    try:
        # 스트리밍 응답 수신
        responses: Iterator[log_processor_pb2.ConvertToCsvProgress] = stub.ConvertToCsv(request)
        
        job_id = None
        for response in responses:
            if job_id is None:
                job_id = response.job_id
                print(f"Job ID: {job_id}")
                print("-" * 60)

            # 진행 단계 이름
            stage_names = {
                0: "UNKNOWN",
                1: "DOWNLOADING",
                2: "CONVERTING",
                3: "UPLOADING",
                4: "COMPLETED",
                5: "FAILED",
            }
            
            stage_name = stage_names.get(response.stage, "UNKNOWN")
            
            print(f"[{stage_name:12}] {response.progress_percent:3}% | {response.message}")
            
            if response.records_processed > 0:
                print(f"                    Records: {response.records_processed:,}")
            
            # 완료 시 CSV 파일 표시
            if response.csv_files:
                print("\nGenerated CSV files:")
                for file in response.csv_files:
                    print(f"  - {file}")
            
            # 성공/실패 처리
            if response.stage == 4:  # COMPLETED
                if response.HasField('success') and response.success:
                    print("\n✅ CSV conversion completed successfully!")
                else:
                    print("\n✅ CSV conversion completed!")
                break
            elif response.stage == 5:  # FAILED
                error_msg = response.error if response.HasField('error') else "Unknown error"
                print(f"\n❌ CSV conversion failed: {error_msg}")
                break
        
    except grpc.RpcError as e:
        print(f"\n❌ RPC Error: {e.code()}: {e.details()}")


def main():
    # gRPC 서버 주소
    server_address = "localhost:50051"
    
    if len(sys.argv) < 2:
        print("Usage:")
        print("  Process logs:")
        print("    python client.py process <source_bucket> <source_path> <target_bucket> <target_path> [log_type] [chunk_size]")
        print("  ")
        print("  Convert to CSV:")
        print("    python client.py csv <source_bucket> <source_parquet_path> <target_bucket> <target_csv_path> [csv_prefix]")
        print("  ")
        print("  Get job status:")
        print("    python client.py status <job_id>")
        print("  ")
        print("  List files:")
        print("    python client.py list <bucket> [prefix]")
        print()
        print("Examples:")
        print("  python client.py process trace-logs logs/trace.csv trace-parquet output/data ufs 100000")
        print("  python client.py csv trace output/parquet/ufs.parquet trace output/csv")
        print("  python client.py csv trace output/parquet/ufs.parquet trace output/csv myprefix")
        print("  python client.py status 12345678-1234-1234-1234-123456789abc")
        print("  python client.py list trace-logs logs/")
        sys.exit(1)
    
    command = sys.argv[1]
    
    # gRPC 채널 생성
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
        if command == "process":
            if len(sys.argv) < 6:
                print("Error: 'process' command requires at least 4 arguments")
                print("Usage: python client.py process <source_bucket> <source_path> <target_bucket> <target_path> [log_type] [chunk_size]")
                sys.exit(1)
            
            source_bucket = sys.argv[2]
            source_path = sys.argv[3]
            target_bucket = sys.argv[4]
            target_path = sys.argv[5]
            log_type = sys.argv[6] if len(sys.argv) > 6 else "auto"
            chunk_size = int(sys.argv[7]) if len(sys.argv) > 7 else 100000
            
            process_logs(stub, source_bucket, source_path, target_bucket, target_path, log_type, chunk_size)
            
        elif command == "csv":
            if len(sys.argv) < 6:
                print("Error: 'csv' command requires 4 arguments")
                print("Usage: python client.py csv <source_bucket> <source_parquet_path> <target_bucket> <target_csv_path> [csv_prefix]")
                sys.exit(1)
            
            source_bucket = sys.argv[2]
            source_parquet_path = sys.argv[3]
            target_bucket = sys.argv[4]
            target_csv_path = sys.argv[5]
            csv_prefix = sys.argv[6] if len(sys.argv) > 6 else None
            
            convert_to_csv(stub, source_bucket, source_parquet_path, target_bucket, target_csv_path, csv_prefix)
            
        elif command == "status":
            if len(sys.argv) < 3:
                print("Error: 'status' command requires job_id")
                print("Usage: python client.py status <job_id>")
                sys.exit(1)
            
            job_id = sys.argv[2]
            get_job_status(stub, job_id)
            
        elif command == "list":
            if len(sys.argv) < 3:
                print("Error: 'list' command requires bucket name")
                print("Usage: python client.py list <bucket> [prefix]")
                sys.exit(1)
            
            bucket = sys.argv[2]
            prefix = sys.argv[3] if len(sys.argv) > 3 else ""
            list_files(stub, bucket, prefix)
            
        else:
            print(f"Unknown command: {command}")
            print("Available commands: process, csv, status, list")
            sys.exit(1)


if __name__ == "__main__":
    main()
