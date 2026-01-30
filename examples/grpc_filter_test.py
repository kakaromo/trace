#!/usr/bin/env python3
"""
gRPC 필터 기능 테스트 예제

시간 필터를 적용하여 로그를 처리하는 예제입니다.
"""

import grpc
import sys
import log_processor_pb2
import log_processor_pb2_grpc
from grpc_client import create_filter_options, process_logs, convert_to_csv


def example_time_filter():
    """시간 범위 필터 예제 - 100ms ~ 500ms"""
    print("=" * 60)
    print("Example 1: Time Range Filter (100ms - 500ms)")
    print("=" * 60)
    
    server_address = "localhost:50051"
    
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
        filter_opts = create_filter_options(
            start_time=100.0,
            end_time=500.0
        )
        
        process_logs(
            stub,
            source_bucket="trace",
            source_path="logs/trace.log",
            target_bucket="trace",
            target_path="filtered/time_100_500",
            log_type="ufs",
            filter_options=filter_opts
        )


def example_sector_filter():
    """섹터 범위 필터 예제 - 섹터 0 ~ 1,000,000"""
    print("\n" + "=" * 60)
    print("Example 2: Sector Range Filter (0 - 1,000,000)")
    print("=" * 60)
    
    server_address = "localhost:50051"
    
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
        filter_opts = create_filter_options(
            start_sector=0,
            end_sector=1000000
        )
        
        process_logs(
            stub,
            source_bucket="trace",
            source_path="logs/trace.log",
            target_bucket="trace",
            target_path="filtered/sector_0_1m",
            log_type="block",
            filter_options=filter_opts
        )


def example_latency_filter():
    """레이턴시 필터 예제 - DTOC 1ms ~ 10ms"""
    print("\n" + "=" * 60)
    print("Example 3: Latency Filter (DTOC 1ms - 10ms)")
    print("=" * 60)
    
    server_address = "localhost:50051"
    
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
        filter_opts = create_filter_options(
            min_dtoc=1.0,
            max_dtoc=10.0
        )
        
        process_logs(
            stub,
            source_bucket="trace",
            source_path="logs/trace.log",
            target_bucket="trace",
            target_path="filtered/latency_1_10",
            log_type="ufs",
            filter_options=filter_opts
        )


def example_complex_filter():
    """복합 필터 예제 - 시간 + 섹터 + 레이턴시"""
    print("\n" + "=" * 60)
    print("Example 4: Complex Filter (Time + Sector + Latency)")
    print("=" * 60)
    
    server_address = "localhost:50051"
    
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
        filter_opts = create_filter_options(
            start_time=100.0,
            end_time=500.0,
            start_sector=0,
            end_sector=1000000,
            min_dtoc=1.0,
            max_dtoc=10.0
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


def example_csv_with_filter():
    """CSV 변환 시 필터 적용 예제"""
    print("\n" + "=" * 60)
    print("Example 5: CSV Conversion with Time Filter")
    print("=" * 60)
    
    server_address = "localhost:50051"
    
    with grpc.insecure_channel(server_address) as channel:
        stub = log_processor_pb2_grpc.LogProcessorStub(channel)
        
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


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 grpc_filter_test.py <example_number>")
        print()
        print("Available examples:")
        print("  1 - Time range filter (100ms - 500ms)")
        print("  2 - Sector range filter (0 - 1,000,000)")
        print("  3 - Latency filter (DTOC 1ms - 10ms)")
        print("  4 - Complex filter (time + sector + latency)")
        print("  5 - CSV conversion with filter")
        print()
        print("Example:")
        print("  python3 grpc_filter_test.py 1")
        sys.exit(1)
    
    example = sys.argv[1]
    
    if example == "1":
        example_time_filter()
    elif example == "2":
        example_sector_filter()
    elif example == "3":
        example_latency_filter()
    elif example == "4":
        example_complex_filter()
    elif example == "5":
        example_csv_with_filter()
    else:
        print(f"Unknown example: {example}")
        print("Available examples: 1, 2, 3, 4, 5")
        sys.exit(1)


if __name__ == "__main__":
    main()
