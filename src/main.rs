use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::time::Instant;
use trace::commands::minio::{handle_minio_log_to_parquet, handle_minio_parquet_analysis};
use trace::grpc::run_grpc_server;
use trace::output::save_to_csv;
use trace::parsers::parse_log_file_high_perf;
use trace::processors;
use trace::storage::minio_client::MinioConfig;
use trace::utils::{
    parse_latency_ranges, read_filter_options, set_alignment_config, set_user_latency_ranges,
    AlignmentConfig, FilterOptions, Logger,
};
use trace::TraceType;
use trace::*;

/// gRPC 서버 실행 함수
#[tokio::main]
async fn run_grpc_server_mode(args: &[String]) -> io::Result<()> {
    // 기본값
    let mut port = "50051";

    // 옵션 파싱
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--port" | "-p" => {
                if i + 1 < args.len() {
                    port = &args[i + 1];
                    i += 2;
                } else {
                    eprintln!("Error: --port requires a value");
                    print_grpc_usage();
                    return Ok(());
                }
            }
            "--help" | "-h" => {
                print_grpc_usage();
                return Ok(());
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_grpc_usage();
                return Ok(());
            }
        }
    }

    // MinIO 설정 로드
    let minio_config = match MinioConfig::from_env() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load MinIO configuration: {}", e);
            eprintln!("\nPlease set the following environment variables:");
            eprintln!("  MINIO_ENDPOINT     - MinIO server endpoint (e.g., http://localhost:9000)");
            eprintln!("  MINIO_ACCESS_KEY   - MinIO access key");
            eprintln!("  MINIO_SECRET_KEY   - MinIO secret key");
            eprintln!("  MINIO_BUCKET       - Default MinIO bucket name (optional)");
            return Ok(());
        }
    };

    let addr = format!("0.0.0.0:{}", port);

    println!("Starting gRPC server...");
    println!("  Address: {}", addr);
    println!("  MinIO Endpoint: {}", minio_config.endpoint);
    println!("  Default Bucket: {}", minio_config.bucket);
    println!();

    if let Err(e) = run_grpc_server(addr, minio_config).await {
        eprintln!("gRPC server error: {}", e);
    }

    Ok(())
}

/// gRPC 서버 사용법 출력
fn print_grpc_usage() {
    println!("gRPC Server Usage:");
    println!("  trace --grpc-server [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --port, -p <PORT>    gRPC server port (default: 50051)");
    println!("  --help, -h           Show this help message");
    println!();
    println!("Environment Variables (required):");
    println!("  MINIO_ENDPOINT       MinIO server endpoint (e.g., http://localhost:9000)");
    println!("  MINIO_ACCESS_KEY     MinIO access key");
    println!("  MINIO_SECRET_KEY     MinIO secret key");
    println!("  MINIO_BUCKET         Default MinIO bucket name (optional)");
    println!();
    println!("Example:");
    println!("  export MINIO_ENDPOINT=http://localhost:9000");
    println!("  export MINIO_ACCESS_KEY=minioadmin");
    println!("  export MINIO_SECRET_KEY=minioadmin");
    println!("  export MINIO_BUCKET=trace-logs");
    println!("  trace --grpc-server --port 50051");
}

/// 파일 크기를 확인하여 처리 방식을 결정하는 함수
fn get_file_size(file_path: &str) -> io::Result<u64> {
    let metadata = fs::metadata(file_path)?;
    Ok(metadata.len())
}

/// Parse y-axis ranges from command line argument
/// Format: "metric:min:max,metric:min:max"
/// Example: "ufs_dtoc:0:100,block_dtoc:0:50"
fn parse_y_axis_ranges(input: &str) -> Result<HashMap<String, (f64, f64)>, String> {
    let mut ranges = HashMap::new();

    for part in input.split(',') {
        let components: Vec<&str> = part.split(':').collect();
        if components.len() != 3 {
            return Err(format!(
                "Invalid format for y-axis range: '{part}'. Expected format: metric:min:max"
            ));
        }

        let metric = components[0].to_string();
        let min = components[1].parse::<f64>().map_err(|_| {
            format!(
                "Invalid minimum value '{}' for metric '{}'",
                components[1], metric
            )
        })?;
        let max = components[2].parse::<f64>().map_err(|_| {
            format!(
                "Invalid maximum value '{}' for metric '{}'",
                components[2], metric
            )
        })?;

        if min >= max {
            return Err(format!(
                "Minimum value ({min}) must be less than maximum value ({max}) for metric '{metric}'"
            ));
        }

        ranges.insert(metric, (min, max));
    }

    Ok(ranges)
}

/// Parse alignment size from command line argument
/// Format: "64KB", "128KB", "4KB", "1MB" etc.
fn parse_alignment_size(input: &str) -> Result<u64, String> {
    let input = input.to_uppercase();

    if let Some(size_str) = input.strip_suffix("KB") {
        let size = size_str
            .parse::<u64>()
            .map_err(|_| format!("Invalid alignment size: '{input}'"))?;
        Ok(size * 1024)
    } else if let Some(size_str) = input.strip_suffix("MB") {
        let size = size_str
            .parse::<u64>()
            .map_err(|_| format!("Invalid alignment size: '{input}'"))?;
        Ok(size * 1024 * 1024)
    } else if let Some(size_str) = input.strip_suffix("GB") {
        let size = size_str
            .parse::<u64>()
            .map_err(|_| format!("Invalid alignment size: '{input}'"))?;
        Ok(size * 1024 * 1024 * 1024)
    } else {
        // 단위가 없으면 bytes로 처리
        input.parse::<u64>().map_err(|_| {
            format!("Invalid alignment size: '{input}'. Use format like '64KB', '1MB'")
        })
    }
}

/// Parse filter option from command line
/// Format: "type:value1:value2" or "type:value1,value2,value3"
/// Examples:
///   time:0.0:1000.0       - time range
///   sector:1000:2000      - sector/LBA range
///   dtoc:0.0:10.0         - DTOC latency range
///   ctoc:0.0:10.0         - CTOC latency range
///   ctod:0.0:10.0         - CTOD latency range
///   cpu:0-3               - CPU range
///   cpu:0,4,7             - CPU list
fn parse_filter_option(
    filter_str: &str,
    filter: &mut FilterOptions,
) -> Result<(), String> {
    let parts: Vec<&str> = filter_str.split(':').collect();
    if parts.len() < 2 {
        return Err(format!(
            "Invalid filter format: '{}'. Expected format: type:value1:value2 or type:values",
            filter_str
        ));
    }

    let filter_type = parts[0];
    
    match filter_type {
        "time" => {
            if parts.len() != 3 {
                return Err(format!("Invalid time filter: '{}'. Expected: time:min:max", filter_str));
            }
            let start = parts[1].parse::<f64>()
                .map_err(|_| format!("Invalid time min value: '{}'", parts[1]))?;
            let end = parts[2].parse::<f64>()
                .map_err(|_| format!("Invalid time max value: '{}'", parts[2]))?;
            
            if start < 0.0 {
                return Err(format!("Time start value must be >= 0, got: {}", start));
            }
            if end <= start {
                return Err(format!("Time end value ({}) must be greater than start value ({})", end, start));
            }
            
            filter.start_time = start;
            filter.end_time = end;
        }
        "sector" | "lba" => {
            if parts.len() != 3 {
                return Err(format!("Invalid sector filter: '{}'. Expected: sector:min:max", filter_str));
            }
            filter.start_sector = parts[1].parse::<u64>()
                .map_err(|_| format!("Invalid sector min value: '{}'", parts[1]))?;
            filter.end_sector = parts[2].parse::<u64>()
                .map_err(|_| format!("Invalid sector max value: '{}'", parts[2]))?;
        }
        "dtoc" => {
            if parts.len() != 3 {
                return Err(format!("Invalid dtoc filter: '{}'. Expected: dtoc:min:max", filter_str));
            }
            filter.min_dtoc = parts[1].parse::<f64>()
                .map_err(|_| format!("Invalid dtoc min value: '{}'", parts[1]))?;
            filter.max_dtoc = parts[2].parse::<f64>()
                .map_err(|_| format!("Invalid dtoc max value: '{}'", parts[2]))?;
        }
        "ctoc" => {
            if parts.len() != 3 {
                return Err(format!("Invalid ctoc filter: '{}'. Expected: ctoc:min:max", filter_str));
            }
            filter.min_ctoc = parts[1].parse::<f64>()
                .map_err(|_| format!("Invalid ctoc min value: '{}'", parts[1]))?;
            filter.max_ctoc = parts[2].parse::<f64>()
                .map_err(|_| format!("Invalid ctoc max value: '{}'", parts[2]))?;
        }
        "ctod" => {
            if parts.len() != 3 {
                return Err(format!("Invalid ctod filter: '{}'. Expected: ctod:min:max", filter_str));
            }
            filter.min_ctod = parts[1].parse::<f64>()
                .map_err(|_| format!("Invalid ctod min value: '{}'", parts[1]))?;
            filter.max_ctod = parts[2].parse::<f64>()
                .map_err(|_| format!("Invalid ctod max value: '{}'", parts[2]))?;
        }
        "cpu" => {
            if parts.len() != 2 {
                return Err(format!("Invalid cpu filter: '{}'. Expected: cpu:0-3 or cpu:0,4,7", filter_str));
            }
            let cpu_spec = parts[1];
            
            // Check if it's a range (e.g., "0-3")
            if cpu_spec.contains('-') {
                let range_parts: Vec<&str> = cpu_spec.split('-').collect();
                if range_parts.len() != 2 {
                    return Err(format!("Invalid CPU range: '{}'. Expected: start-end", cpu_spec));
                }
                let start = range_parts[0].parse::<u32>()
                    .map_err(|_| format!("Invalid CPU range start: '{}'", range_parts[0]))?;
                let end = range_parts[1].parse::<u32>()
                    .map_err(|_| format!("Invalid CPU range end: '{}'", range_parts[1]))?;
                
                if start > end {
                    return Err(format!("Invalid CPU range: start ({}) must be <= end ({})", start, end));
                }
                
                filter.cpu_list.extend(start..=end);
            } else if cpu_spec.contains(',') {
                // List of CPUs (e.g., "0,4,7")
                for cpu_str in cpu_spec.split(',') {
                    let cpu = cpu_str.trim().parse::<u32>()
                        .map_err(|_| format!("Invalid CPU value: '{}'", cpu_str))?;
                    filter.cpu_list.push(cpu);
                }
            } else {
                // Single CPU
                let cpu = cpu_spec.parse::<u32>()
                    .map_err(|_| format!("Invalid CPU value: '{}'", cpu_spec))?;
                filter.cpu_list.push(cpu);
            }
        }
        _ => {
            return Err(format!(
                "Unknown filter type: '{}'. Supported types: time, sector, lba, dtoc, ctoc, ctod, cpu",
                filter_type
            ));
        }
    }

    Ok(())
}

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {program} [options] <log_file> <output_prefix>                      - Parse log file and generate statistics");
    eprintln!("  {program} [options] --parquet <type> <parquet_file> <output_prefix> - Read Parquet file and generate statistics");
    eprintln!("    where <type> is one of: 'ufs', 'block'");

    eprintln!("  {program} --migrate <path> [migration_options]                      - Migrate existing Parquet files to new schema");
    eprintln!("\nMinIO Integration (supports --filter options):");
    eprintln!("  {program} --minio-log <remote_log_path> <remote_output_path>        - Read log from MinIO, generate Parquet, upload to MinIO (no stats/charts)");
    eprintln!("  {program} --minio-analyze <remote_parquet_path> <local_output_prefix> - Download Parquet from MinIO, analyze and generate charts");
    eprintln!("  {program} --minio-csv <remote_parquet_path> <remote_csv_path>       - Download Parquet from MinIO, convert to CSV, upload to MinIO");
    eprintln!("                 (Type auto-detected from filename: ufs.parquet, block.parquet, ufscustom.parquet)");
    eprintln!("\nOptions:");
    eprintln!("  -p           - Performance benchmark mode: Auto-detects FIO, TIOtest, IOzone results and trace types");
    eprintln!("                 Creates iteration-based folders: <output_prefix>/1/, <output_prefix>/2/, ...");
    eprintln!("                 Example: {program} -p benchmark.log fio_result");
    eprintln!("  -l <values>  - Custom latency ranges in ms (comma-separated). Example: -l 0.1,0.5,1,5,10,50,100");
    eprintln!("  -f           - Apply filters (time, sector/lba, latency, queue depth) with interactive input");
    eprintln!("  --filter <spec> - Apply specific filter (can be used multiple times)");
    eprintln!("                 Format: type:value1:value2 or type:values");
    eprintln!("                 Examples:");
    eprintln!("                   --filter time:0.0:1000.0     (time range in ms)");
    eprintln!("                   --filter sector:1000:2000    (sector/LBA range)");
    eprintln!("                   --filter dtoc:0.0:10.0       (DTOC latency range in ms)");
    eprintln!("                   --filter ctoc:0.0:10.0       (CTOC latency range in ms)");
    eprintln!("                   --filter ctod:0.0:10.0       (CTOD latency range in ms)");
    eprintln!("                   --filter cpu:0-3             (CPU range)");
    eprintln!("                   --filter cpu:0,4,7           (specific CPUs)");
    eprintln!(
        "  -y <ranges>  - Set y-axis ranges for charts. Format: metric:min:max,metric:min:max"
    );
    eprintln!("                 Metrics: ufs_dtoc, ufs_ctoc, ufs_ctod, ufs_qd, ufs_lba, block_dtoc, block_ctoc, block_ctod, block_qd, block_lba");
    eprintln!("                 Example: -y ufs_dtoc:0:100,block_dtoc:0:50");
    eprintln!("  -c <size>    - Set chunk size for Parquet file writing (default: 50000). Example: -c 100000");
    eprintln!("  --csv        - Export filtered data to CSV files (works with all modes including --parquet)");
    eprintln!("  --align <size> - Set alignment size for sector/LBA alignment check (default: 64KB). Example: --align 128KB, --align 4KB");
    eprintln!("\nMigration Options:");
    eprintln!("  --chunk-size <size> - Set chunk size for migration (default: 10000)");
    eprintln!("  --no-backup        - Don't create backup files before migration");
    eprintln!("  --recursive        - Recursively migrate all Parquet files in subdirectories");
    eprintln!("\nMinIO Environment Variables:");
    eprintln!("  MINIO_ENDPOINT    - MinIO server endpoint (default: http://localhost:9000)");
    eprintln!("  MINIO_ACCESS_KEY  - MinIO access key (required)");
    eprintln!("  MINIO_SECRET_KEY  - MinIO secret key (required)");
    eprintln!("  MINIO_BUCKET      - MinIO bucket name (default: trace)");
    eprintln!("  MINIO_REGION      - MinIO region (default: us-east-1)");
}

fn main() -> io::Result<()> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    // 인자가 없으면 사용법 출력
    if args.len() <= 1 {
        eprintln!("Error: No arguments provided");
        print_usage(&args[0]);
        return Ok(());
    }

    // gRPC 서버 모드 체크 (가장 먼저 확인)
    if args.len() > 1 && args[1] == "--grpc-server" {
        return run_grpc_server_mode(&args);
    }

    // 옵션 파싱
    let mut i = 1;
    let mut log_file_index = 0;
    let mut output_prefix_index = 0;
    let mut is_parquet_mode = false;
    let mut parquet_type_index = 0;
    let mut parquet_path_index = 0;
    let mut use_filter = false;
    let mut filter_options = FilterOptions::default(); // CLI filter options
    let mut y_axis_ranges: Option<HashMap<String, (f64, f64)>> = None;
    let mut chunk_size: usize = 50_000; // 기본 청크 크기
    let mut export_csv = false; // CSV export 옵션
    let mut alignment_size: Option<u64> = None; // Alignment size 옵션 (None이면 기본값 64KB 사용)
    let mut benchmark_mode = false; // 벤치마크 모드

    while i < args.len() {
        match args[i].as_str() {
            "-p" => {
                benchmark_mode = true;
                i += 1;
            }
            "-l" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: -l option requires values");
                    print_usage(&args[0]);
                    return Ok(());
                }

                match parse_latency_ranges(&args[i + 1]) {
                    Ok(ranges) => {
                        set_user_latency_ranges(ranges);
                        log!("Using custom latency ranges: {:?}", args[i + 1]);
                    }
                    Err(e) => {
                        eprintln!("Error in latency ranges: {e}");
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }

                i += 2; // 옵션과 값을 건너뜀
            }
            "-f" => {
                use_filter = true;
                i += 1;
            }
            "--filter" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --filter option requires filter specification");
                    print_usage(&args[0]);
                    return Ok(());
                }

                match parse_filter_option(&args[i + 1], &mut filter_options) {
                    Ok(_) => {
                        use_filter = true; // Enable filter mode
                        log!("Applied filter: {}", &args[i + 1]);
                    }
                    Err(e) => {
                        eprintln!("Error in filter specification: {e}");
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }

                i += 2; // 옵션과 값을 건너뜀
            }
            "-y" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: -y option requires y-axis range values");
                    print_usage(&args[0]);
                    return Ok(());
                }

                match parse_y_axis_ranges(&args[i + 1]) {
                    Ok(ranges) => {
                        y_axis_ranges = Some(ranges);
                        log!("Using custom y-axis ranges: {:?}", args[i + 1]);
                    }
                    Err(e) => {
                        eprintln!("Error in y-axis ranges: {e}");
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }

                i += 2; // 옵션과 값을 건너뜀
            }
            "-c" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: -c option requires chunk size value");
                    print_usage(&args[0]);
                    return Ok(());
                }

                match args[i + 1].parse::<usize>() {
                    Ok(size) => {
                        if size < 1000 {
                            eprintln!("Error: Chunk size must be at least 1000");
                            print_usage(&args[0]);
                            return Ok(());
                        }
                        chunk_size = size;
                        log!("Using custom chunk size: {}", chunk_size);
                    }
                    Err(_) => {
                        eprintln!("Error: Invalid chunk size value '{}'", args[i + 1]);
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }

                i += 2; // 옵션과 값을 건너뜀
            }
            "--csv" => {
                export_csv = true;
                i += 1;
            }
            "--align" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --align option requires alignment size value");
                    print_usage(&args[0]);
                    return Ok(());
                }

                match parse_alignment_size(&args[i + 1]) {
                    Ok(size) => {
                        alignment_size = Some(size);
                        log!(
                            "Using custom alignment size: {} bytes ({})",
                            size,
                            &args[i + 1]
                        );
                    }
                    Err(e) => {
                        eprintln!("Error in alignment size: {e}");
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }

                i += 2; // 옵션과 값을 건너뜀
            }
            "--migrate" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --migrate option requires input path");
                    print_usage(&args[0]);
                    return Ok(());
                }

                let input_path = &args[i + 1];
                let mut migrate_chunk_size = None;
                let mut backup_enabled = true;
                let mut recursive = false;
                let mut j = i + 2;

                // 마이그레이션 옵션 파싱
                while j < args.len() {
                    match args[j].as_str() {
                        "--chunk-size" => {
                            if j + 1 < args.len() {
                                if let Ok(size) = args[j + 1].parse::<usize>() {
                                    migrate_chunk_size = Some(size);
                                    j += 2;
                                } else {
                                    eprintln!("Error: Invalid chunk size value '{}'", args[j + 1]);
                                    return Ok(());
                                }
                            } else {
                                eprintln!("Error: --chunk-size requires a value");
                                return Ok(());
                            }
                        }
                        "--no-backup" => {
                            backup_enabled = false;
                            j += 1;
                        }
                        "--recursive" => {
                            recursive = true;
                            j += 1;
                        }
                        _ => break,
                    }
                }

                // 마이그레이션 실행
                match trace::migration::run_migration(
                    input_path,
                    migrate_chunk_size,
                    backup_enabled,
                    recursive,
                ) {
                    Ok(_) => println!("Migration completed successfully"),
                    Err(e) => eprintln!("Migration failed: {e}"),
                }

                return Ok(());
            }
            "--parquet" => {
                is_parquet_mode = true;
                parquet_type_index = i + 1;
                parquet_path_index = i + 2;
                output_prefix_index = i + 3;
                i += 1;
            }
            "--minio-log" => {
                // MinIO에서 로그 읽기 -> Parquet 생성 -> MinIO에 저장
                if i + 2 >= args.len() {
                    eprintln!("Error: --minio-log requires <remote_log_path> <remote_output_path>");
                    print_usage(&args[0]);
                    return Ok(());
                }

                let remote_log_path = &args[i + 1];
                let remote_output_path = &args[i + 2];

                // 필터 옵션이 있으면 출력
                if use_filter {
                    println!("\nApplying filters to MinIO log processing...");
                }

                match handle_minio_log_to_parquet(
                    remote_log_path,
                    remote_output_path,
                    chunk_size,
                    if use_filter { Some(filter_options.clone()) } else { None },
                ) {
                    Ok(_) => println!("MinIO log to Parquet completed successfully"),
                    Err(e) => eprintln!("MinIO log to Parquet failed: {e}"),
                }

                return Ok(());
            }
            "--minio-analyze" => {
                // MinIO에서 Parquet 읽기 -> 분석 + 차트 생성
                if i + 2 >= args.len() {
                    eprintln!("Error: --minio-analyze requires <remote_parquet_path> <local_output_prefix>");
                    print_usage(&args[0]);
                    return Ok(());
                }

                let remote_parquet_path = &args[i + 1];
                let local_output_prefix = &args[i + 2];

                // 필터 옵션이 있으면 출력
                if use_filter {
                    println!("\nApplying filters to MinIO analysis...");
                }

                match handle_minio_parquet_analysis(
                    remote_parquet_path,
                    local_output_prefix,
                    y_axis_ranges,
                    if use_filter { Some(filter_options.clone()) } else { None },
                ) {
                    Ok(_) => println!("MinIO Parquet analysis completed successfully"),
                    Err(e) => eprintln!("MinIO Parquet analysis failed: {e}"),
                }

                return Ok(());
            }
            "--minio-csv" => {
                // MinIO에서 Parquet 읽기 -> CSV 변환 -> MinIO에 저장
                if i + 2 >= args.len() {
                    eprintln!(
                        "Error: --minio-csv requires <remote_parquet_path> <remote_csv_path>"
                    );
                    eprintln!(
                        "  Type is auto-detected from filename (ufs.parquet, block.parquet, or ufscustom.parquet)"
                    );
                    print_usage(&args[0]);
                    return Ok(());
                }

                let remote_parquet_path = &args[i + 1];
                let remote_csv_path = &args[i + 2];

                // 필터 옵션이 있으면 출력
                if use_filter {
                    println!("\nApplying filters to MinIO CSV export...");
                }

                match trace::commands::minio::handle_minio_parquet_to_csv(
                    remote_parquet_path,
                    remote_csv_path,
                    if use_filter { Some(filter_options.clone()) } else { None },
                ) {
                    Ok(_) => println!("MinIO Parquet to CSV completed successfully"),
                    Err(e) => eprintln!("MinIO Parquet to CSV failed: {e}"),
                }

                return Ok(());
            }

            _ => {
                // 일반 위치 인수 처리
                if !is_parquet_mode {
                    if log_file_index == 0 {
                        log_file_index = i;
                    } else if output_prefix_index == 0 {
                        output_prefix_index = i;
                    }
                }
                i += 1;
            }
        }
    }

    // Alignment configuration 설정
    if let Some(size_bytes) = alignment_size {
        let size_kb = size_bytes / 1024;
        let config = AlignmentConfig {
            alignment_size_kb: size_kb,
        };
        set_alignment_config(config);
        log!(
            "Alignment configuration set to {} KB ({} bytes)",
            size_kb,
            size_bytes
        );
    }

    // 필터 옵션 처리
    let filter_options = if use_filter {
        // CLI에서 --filter 옵션이 사용된 경우
        let filter = if filter_options.is_time_filter_active() 
            || filter_options.is_sector_filter_active()
            || filter_options.is_dtoc_filter_active()
            || filter_options.is_ctoc_filter_active()
            || filter_options.is_ctod_filter_active()
            || filter_options.is_cpu_filter_active() {
            // CLI 필터 사용
            filter_options.clone()
        } else {
            // 대화형 필터 입력
            println!("필터 옵션을 입력하세요 (입력하지 않거나 0으로 입력 시 필터링하지 않습니다):");
            match read_filter_options() {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("필터 옵션 입력 오류: {e}");
                    return Ok(());
                }
            }
        };

        // 필터 정보 출력
        println!("적용된 필터 옵션:");
        if filter.is_time_filter_active() {
            println!(
                "  시간 필터: {:.3} - {:.3} ms",
                filter.start_time, filter.end_time
            );
        } else {
            println!("  시간 필터: 사용하지 않음");
        }

        if filter.is_sector_filter_active() {
            println!(
                "  섹터/LBA 필터: {} - {}",
                filter.start_sector, filter.end_sector
            );
        } else {
            println!("  섹터/LBA 필터: 사용하지 않음");
        }

        if filter.is_dtoc_filter_active() {
            println!(
                "  DTOC 레이턴시 필터: {:.3} - {:.3} ms",
                if filter.min_dtoc > 0.0 {
                    filter.min_dtoc
                } else {
                    0.0
                },
                if filter.max_dtoc > 0.0 {
                    filter.max_dtoc
                } else {
                    f64::INFINITY
                }
            );
        } else {
            println!("  DTOC 레이턴시 필터: 사용하지 않음");
        }

        if filter.is_ctoc_filter_active() {
            println!(
                "  CTOC 레이턴시 필터: {:.3} - {:.3} ms",
                if filter.min_ctoc > 0.0 {
                    filter.min_ctoc
                } else {
                    0.0
                },
                if filter.max_ctoc > 0.0 {
                    filter.max_ctoc
                } else {
                    f64::INFINITY
                }
            );
        } else {
            println!("  CTOC 레이턴시 필터: 사용하지 않음");
        }

        if filter.is_ctod_filter_active() {
            println!(
                "  CTOD 레이턴시 필터: {:.3} - {:.3} ms",
                if filter.min_ctod > 0.0 {
                    filter.min_ctod
                } else {
                    0.0
                },
                if filter.max_ctod > 0.0 {
                    filter.max_ctod
                } else {
                    f64::INFINITY
                }
            );
        } else {
            println!("  CTOD 레이턴시 필터: 사용하지 않음");
        }

        if filter.is_qd_filter_active() {
            println!(
                "  QD 필터: {} - {}",
                if filter.min_qd > 0 { filter.min_qd } else { 0 },
                if filter.max_qd > 0 {
                    filter.max_qd
                } else {
                    u32::MAX
                }
            );
        } else {
            println!("  QD 필터: 사용하지 않음");
        }

        if filter.is_cpu_filter_active() {
            print!("  CPU 필터: ");
            let cpus: Vec<String> = filter.cpu_list.iter().map(|c| c.to_string()).collect();
            println!("{}", cpus.join(", "));
        } else {
            println!("  CPU 필터: 사용하지 않음");
        }

        // 전역 필터 옵션 설정
        set_filter_options(filter.clone());
        Some(filter)
    } else {
        None
    };

    // 명령줄 인수 처리
    let result: io::Result<()> = if benchmark_mode && log_file_index > 0 && output_prefix_index > 0
    {
        // 벤치마크 모드: iteration 자동 감지 및 trace 타입 자동 분류
        println!("Performance benchmark mode: Auto-detecting iterations and trace types...");
        trace::processors::parse_benchmark_log(&args[log_file_index], &args[output_prefix_index])
    } else if !is_parquet_mode && log_file_index > 0 && output_prefix_index > 0 {
        // 일반 trace 로그 파일 처리
        match get_file_size(&args[log_file_index]) {
            Ok(file_size) => {
                let file_size_mb = file_size as f64 / (1024.0 * 1024.0);

                // 항상 highperf 모드 사용
                println!("File size: {file_size_mb:.2} MB - Using high-performance mode");
                process_highperf_log_file(
                    &args[log_file_index],
                    &args[output_prefix_index],
                    filter_options.as_ref(),
                    y_axis_ranges.as_ref(),
                    chunk_size,
                    export_csv,
                )
            }
            Err(e) => {
                eprintln!("Error reading file size: {e}");
                return Ok(());
            }
        }
    } else if is_parquet_mode
        && parquet_type_index > 0
        && parquet_type_index < args.len()
        && parquet_path_index > 0
        && parquet_path_index < args.len()
        && output_prefix_index > 0
        && output_prefix_index < args.len()
    {
        // Parquet 분석 모드
        let trace_type = match args[parquet_type_index].parse::<TraceType>() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error: {e}");
                eprintln!("Supported types: 'ufs', 'block'"); // 새 타입 추가 시 업데이트
                print_usage(&args[0]);
                return Ok(());
            }
        };
        process_single_parquet_file(
            trace_type,
            &args[parquet_path_index],
            &args[output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
            export_csv,
        )
    } else {
        // 인자 설정이 잘못된 경우
        eprintln!("Error: Invalid arguments");
        print_usage(&args[0]);
        return Ok(());
    };

    // 에러 처리: 프로세싱 함수에서 에러가 발생한 경우 메시지 출력
    if let Err(e) = result {
        eprintln!("Error: {e}");
        print_usage(&args[0]);
    }

    Ok(())
}

// 단일 Parquet 파일 처리 로직
fn process_single_parquet_file(
    trace_type: TraceType,
    parquet_path: &str,
    output_prefix: &str,
    filter: Option<&FilterOptions>,
    y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
    _chunk_size: usize, // Parquet 읽기에서는 사용하지 않지만 일관성을 위해 유지
    export_csv: bool,
) -> io::Result<()> {
    // Logger 초기화
    Logger::init(output_prefix);

    // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
    if let Some(ranges) = trace::utils::get_user_latency_ranges() {
        log!("Using custom latency ranges: {:?} ms", ranges);
    }

    let total_start_time = Instant::now();
    let data_label = trace_type.display_name();

    log!(
        "===== Starting Parquet File Analysis (Type: {}) =====",
        data_label
    );

    // 1. 파일 로딩
    log!("\n[1/3] Loading {} Parquet file...", data_label);
    let load_start = Instant::now();

    // TraceData를 사용하여 데이터 로드 및 처리를 추상화
    let mut trace_data = match load_trace_data(&trace_type, parquet_path) {
        Ok(data) => {
            log!(
                "{} Parquet loaded successfully: {} events (Time taken: {:.2}s)",
                data_label,
                data.count(),
                load_start.elapsed().as_secs_f64()
            );
            data
        }
        Err(e) => {
            log_error!("Error loading {} Parquet file: {}", data_label, e);
            return Ok(());
        }
    };

    // 필터 옵션이 있다면 로그에 기록
    if let Some(f) = filter {
        if f.start_time > 0.0 && f.end_time > 0.0 {
            log!(
                "Using time filter: {:.3} - {:.3} ms",
                f.start_time,
                f.end_time
            );
        }

        if f.start_sector > 0 && f.end_sector > 0 {
            log!(
                "Using sector/LBA filter: {} - {}",
                f.start_sector,
                f.end_sector
            );
        }

        if f.is_dtoc_filter_active() {
            log!(
                "Using DTOC latency filter: {:.3} - {:.3} ms",
                if f.min_dtoc > 0.0 { f.min_dtoc } else { 0.0 },
                if f.max_dtoc > 0.0 {
                    f.max_dtoc
                } else {
                    f64::INFINITY
                }
            );
        }

        if f.is_ctoc_filter_active() {
            log!(
                "Using CTOC latency filter: {:.3} - {:.3} ms",
                if f.min_ctoc > 0.0 { f.min_ctoc } else { 0.0 },
                if f.max_ctoc > 0.0 {
                    f.max_ctoc
                } else {
                    f64::INFINITY
                }
            );
        }

        if f.is_ctod_filter_active() {
            log!(
                "Using CTOD latency filter: {:.3} - {:.3} ms",
                if f.min_ctod > 0.0 { f.min_ctod } else { 0.0 },
                if f.max_ctod > 0.0 {
                    f.max_ctod
                } else {
                    f64::INFINITY
                }
            );
        }

        if f.is_qd_filter_active() {
            log!(
                "Using QD filter: {} - {}",
                if f.min_qd > 0 { f.min_qd } else { 0 },
                if f.max_qd > 0 { f.max_qd } else { u32::MAX }
            );
        }

        // 필터링 적용
        if f.is_time_filter_active()
            || f.is_sector_filter_active()
            || f.is_dtoc_filter_active()
            || f.is_ctoc_filter_active()
            || f.is_ctod_filter_active()
            || f.is_qd_filter_active()
        {
            log!("\n[1.5/3] Applying filters...");
            let filter_start = Instant::now();

            let original_count = trace_data.count();
            trace_data = trace_data.apply_filter(f);

            log!(
                "{} data filtered: {} -> {} events (Time taken: {:.2}s)",
                data_label,
                original_count,
                trace_data.count(),
                filter_start.elapsed().as_secs_f64()
            );
        }
    };

    // 2. 통계 계산 및 출력
    log!("\n[2/3] Calculating {} statistics...", data_label);
    let stats_start = Instant::now();
    log!("\n=== {} Analysis Results ===", data_label);

    trace_data.print_statistics();

    log!(
        "\nStatistics calculation complete (Time taken: {:.2}s)",
        stats_start.elapsed().as_secs_f64()
    );

    // 3. CSV 내보내기 (요청된 경우)
    if export_csv {
        log!("\n[2.5/4] Saving filtered data to CSV files...");
        let csv_save_start = Instant::now();

        match trace_data.save_to_csv(output_prefix) {
            Ok(()) => {
                log!(
                    "Filtered CSV file saved successfully (Time taken: {:.2}s): {}_{}.csv",
                    csv_save_start.elapsed().as_secs_f64(),
                    output_prefix,
                    data_label.to_lowercase()
                );
            }
            Err(e) => log_error!("Error while saving CSV file: {}", e),
        }
    }

    // 4. 차트 생성
    log!(
        "\n[{}/{}] Generating {} Plotters charts...",
        if export_csv { 3 } else { 2 },
        if export_csv { 4 } else { 3 },
        data_label
    );
    let charts_start = Instant::now();

    match trace_data.generate_charts(output_prefix, y_axis_ranges) {
        Ok(()) => log!(
            "{} Plotters charts generated successfully (Time taken: {:.2}s)",
            data_label,
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!(
            "Error while generating {} Plotters charts: {}",
            data_label,
            e
        ),
    }

    // 5. 요약 정보 출력
    log!("\n===== {} Parquet Analysis Complete! =====", data_label);
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    trace_data.print_summary(output_prefix);

    if export_csv {
        log!(
            "- {} CSV file: {}_{}.csv",
            data_label,
            output_prefix,
            data_label.to_lowercase()
        );
    }
    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

// 고성능 메모리 매핑 로그 파일 처리 로직
fn process_highperf_log_file(
    log_file_path: &str,
    output_prefix: &str,
    filter: Option<&FilterOptions>,
    y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
    chunk_size: usize,
    export_csv: bool,
) -> io::Result<()> {
    // Logger 초기화
    Logger::init(output_prefix);

    // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
    if let Some(ranges) = trace::utils::get_user_latency_ranges() {
        log!("Using custom latency ranges: {:?} ms", ranges);
    }

    let total_start_time = Instant::now();
    log!("===== Starting High-Performance Log File Processing =====");

    // 고성능 로그 파일 파싱
    log!("\n[1/3] Parsing log file with high-performance memory-mapped I/O...");
    let parse_start = Instant::now();

    let mut traces = match parse_log_file_high_perf(log_file_path) {
        Ok(data) => {
            let (ref ufs_data, ref block_data, ref ufscustom_data) = data;
            let total_events = ufs_data.len() + block_data.len() + ufscustom_data.len();
            log!(
                "High-performance log file parsing complete: {} events (Time taken: {:.2}s)",
                total_events,
                parse_start.elapsed().as_secs_f64()
            );
            data
        }
        Err(e) => {
            log_error!("Error parsing high-performance log file: {}", e);
            return Ok(());
        }
    };

    // UFS 데이터 처리 (Latency 계산 등)
    let (ufs_data, block_data, ufscustom_data) = traces;

    log!("\n[1.2/3] Processing UFS data for latency calculations...");
    let ufs_process_start = Instant::now();

    let processed_ufs = if !ufs_data.is_empty() {
        log!("Applying latency analysis to UFS data...");
        ufs_bottom_half_latency_process(ufs_data)
    } else {
        ufs_data
    };

    log!(
        "UFS data processing complete: {} events (Time taken: {:.2}s)",
        processed_ufs.len(),
        ufs_process_start.elapsed().as_secs_f64()
    );

    // Block I/O 데이터 처리 (Latency 계산 등)
    log!("\n[1.3/4] Processing Block I/O data for latency calculations...");
    let block_process_start = Instant::now();

    let processed_block = if !block_data.is_empty() {
        log!("Applying latency analysis to Block I/O data...");
        block_bottom_half_latency_process(block_data)
    } else {
        block_data
    };

    log!(
        "Block I/O data processing complete: {} events (Time taken: {:.2}s)",
        processed_block.len(),
        block_process_start.elapsed().as_secs_f64()
    );

    // UFSCUSTOM 데이터 처리 (Latency 계산 등)
    log!("\n[1.4/4] Processing UFSCUSTOM data for latency calculations...");
    let ufscustom_process_start = Instant::now();

    let processed_ufscustom = if !ufscustom_data.is_empty() {
        log!("Applying latency analysis to UFSCUSTOM data...");
        crate::processors::ufscustom_bottom_half_latency_process(ufscustom_data)
    } else {
        ufscustom_data
    };

    log!(
        "UFSCUSTOM data processing complete: {} events (Time taken: {:.2}s)",
        processed_ufscustom.len(),
        ufscustom_process_start.elapsed().as_secs_f64()
    );

    // 처리된 데이터로 업데이트
    traces = (processed_ufs, processed_block, processed_ufscustom);

    // 필터 옵션이 있다면 로그에 기록
    if let Some(f) = filter {
        if f.start_time > 0.0 && f.end_time > 0.0 {
            log!(
                "Using time filter: {:.3} - {:.3} ms",
                f.start_time,
                f.end_time
            );
        }

        if f.start_sector > 0 && f.end_sector > 0 {
            log!(
                "Using sector/LBA filter: {} - {}",
                f.start_sector,
                f.end_sector
            );
        }

        if f.is_dtoc_filter_active() {
            log!(
                "Using DTOC latency filter: {:.3} - {:.3} ms",
                if f.min_dtoc > 0.0 { f.min_dtoc } else { 0.0 },
                if f.max_dtoc > 0.0 {
                    f.max_dtoc
                } else {
                    f64::MAX
                }
            );
        }

        if f.is_qd_filter_active() {
            log!(
                "Using queue depth filter: {} - {}",
                f.min_qd,
                if f.max_qd > 0 { f.max_qd } else { u32::MAX }
            );
        }
    }

    // 필터링 적용
    if let Some(f) = filter {
        if f.is_time_filter_active()
            || f.is_sector_filter_active()
            || f.is_dtoc_filter_active()
            || f.is_ctoc_filter_active()
            || f.is_ctod_filter_active()
            || f.is_qd_filter_active()
        {
            log!("\n[1.5/6] Applying filters...");
            let filter_start = Instant::now();

            // 튜플에서 개별 요소 추출
            let (mut ufs_data, mut block_data, mut ufscustom_data) = traces;

            let original_counts = (ufs_data.len(), block_data.len(), ufscustom_data.len());

            // 필터 적용
            if !ufs_data.is_empty() {
                ufs_data = filter_ufs_data(ufs_data, f);
            }

            if !block_data.is_empty() {
                block_data = filter_block_data(block_data, f);
            }

            if !ufscustom_data.is_empty() {
                ufscustom_data = filter_ufscustom_data(ufscustom_data, f);
            }

            // 필터링된 결과로 traces 업데이트
            traces = (ufs_data, block_data, ufscustom_data);

            log!(
                "High-performance data filtered: ({} -> {}, {} -> {}, {} -> {}) events (Time taken: {:.2}s)",
                original_counts.0,
                traces.0.len(),
                original_counts.1,
                traces.1.len(),
                original_counts.2,
                traces.2.len(),
                filter_start.elapsed().as_secs_f64()
            );
        }
    }

    // 통계 계산 및 출력
    log!("\n[2/6] Calculating statistics...");
    let stats_start = Instant::now();
    log!("\n=== High-Performance Log File Analysis Results ===");

    // 튜플에서 개별 요소 추출
    let (ufs_traces, block_traces, ufscustom_traces) = &traces;

    // 기존 통계 함수 사용
    trace::output::print_ufs_statistics(ufs_traces);
    trace::output::print_block_statistics(block_traces);
    trace::output::print_ufscustom_statistics(ufscustom_traces);

    log!(
        "Statistics calculation complete (Time taken: {:.2}s)",
        stats_start.elapsed().as_secs_f64()
    );

    // Parquet 파일 저장
    log!("\n[3/6] Saving to Parquet files...");
    let save_start = Instant::now();

    let (ufs_data, block_data, ufscustom_data) = &traces;
    let has_ufs = !ufs_data.is_empty();
    let has_block = !block_data.is_empty();
    let has_ufscustom = !ufscustom_data.is_empty();

    match save_to_parquet(
        ufs_data,
        block_data,
        ufscustom_data,
        output_prefix,
        chunk_size,
    ) {
        Ok(()) => {
            let mut saved_files = Vec::new();
            if has_ufs {
                saved_files.push(format!("{output_prefix}_ufs.parquet"));
            }
            if has_block {
                saved_files.push(format!("{output_prefix}_block.parquet"));
            }
            if has_ufscustom {
                saved_files.push(format!("{output_prefix}_ufscustom.parquet"));
            }
            log!(
                "Parquet files saved successfully (Time taken: {:.2}s):\n{}",
                save_start.elapsed().as_secs_f64(),
                saved_files.join("\n")
            );
        }
        Err(e) => log_error!("Error while saving Parquet files: {}", e),
    }

    // CSV 내보내기 (요청된 경우)
    if export_csv {
        log!("\n[4/6] Saving filtered data to CSV files...");
        let csv_save_start = Instant::now();

        match save_to_csv(ufs_data, block_data, ufscustom_data, output_prefix) {
            Ok(()) => {
                let mut saved_csv_files = Vec::new();
                if has_ufs && !ufs_data.is_empty() {
                    saved_csv_files.push(format!("{output_prefix}_ufs.csv"));
                }
                if has_block && !block_data.is_empty() {
                    saved_csv_files.push(format!("{output_prefix}_block.csv"));
                }
                if has_ufscustom && !ufscustom_data.is_empty() {
                    saved_csv_files.push(format!("{output_prefix}_ufscustom.csv"));
                }
                if !saved_csv_files.is_empty() {
                    log!(
                        "Filtered CSV files saved successfully (Time taken: {:.2}s):\n{}",
                        csv_save_start.elapsed().as_secs_f64(),
                        saved_csv_files.join("\n")
                    );
                } else {
                    log!("No CSV files saved (all filtered data is empty)");
                }
            }
            Err(e) => log_error!("Error while saving CSV files: {}", e),
        }
    }

    // 차트 생성
    log!(
        "\n[{}/6] Generating charts...",
        if export_csv { 5 } else { 4 }
    );
    let charts_start = Instant::now();

    match trace::output::charts::generate_charts_with_config(
        ufs_traces,
        block_traces,
        ufscustom_traces,
        output_prefix,
        y_axis_ranges,
    ) {
        Ok(()) => log!(
            "High-performance charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating high-performance charts: {}", e),
    }

    // 요약 정보 출력
    log!("\n===== High-Performance Log File Processing Complete =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    // 결과 요약
    log!("Processed events:");
    if has_ufs {
        log!("- UFS events: {}", ufs_data.len());
    }
    if has_block {
        log!("- Block I/O events: {}", block_data.len());
    }
    if has_ufscustom {
        log!("- UFSCUSTOM events: {}", ufscustom_data.len());
    }

    log!("Generated files:");

    // 생성된 파일 목록
    if has_ufs {
        log!("- UFS Parquet file: {}_ufs.parquet", output_prefix);
        if export_csv {
            log!("- UFS CSV file: {}_ufs.csv", output_prefix);
        }
        log!("- UFS Plotters charts: {}_ufs_*.png", output_prefix);
    }

    if has_block {
        log!("- Block I/O Parquet file: {}_block.parquet", output_prefix);
        if export_csv {
            log!("- Block I/O CSV file: {}_block.csv", output_prefix);
        }
        log!("- Block I/O Plotters charts: {}_block_*.png", output_prefix);
    }

    if has_ufscustom {
        log!(
            "- UFSCUSTOM Parquet file: {}_ufscustom.parquet",
            output_prefix
        );
        if export_csv {
            log!("- UFSCUSTOM CSV file: {}_ufscustom.csv", output_prefix);
        }
        log!(
            "- UFSCUSTOM Plotters charts: {}_ufscustom_*.png",
            output_prefix
        );
    }

    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
} /*
  // 스트리밍 로그 파일 처리 로직 (더 이상 사용되지 않음)
  fn process_streaming_log_file(
      log_file_path: &str,
      output_prefix: &str,
      filter: Option<&FilterOptions>,
      y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
      chunk_size: usize,
      export_csv: bool,
  ) -> io::Result<()> {
      // Logger 초기화
      Logger::init(output_prefix);

      // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
      if let Some(ranges) = trace::utils::get_user_latency_ranges() {
          log!("Using custom latency ranges: {:?} ms", ranges);
      }

      // 청크 크기 로그에 기록
      log!("Using Parquet chunk size: {}", chunk_size);

      let total_start_time = Instant::now();
      log!("===== Starting Streaming Log File Processing =====");

      // 스트리밍 로그 파일 파싱
      log!("\n[1/6] Parsing log file with streaming memory management...");
      let parse_start = Instant::now();

      let mut traces = match parse_log_file_streaming(log_file_path) {
          Ok(data) => {
              let (ref ufs_data, ref block_data, ref ufscustom_data) = data;
              let total_events = ufs_data.len() + block_data.len() + ufscustom_data.len();
              log!(
                  "Streaming log file parsing complete: {} events (Time taken: {:.2}s)",
                  total_events,
                  parse_start.elapsed().as_secs_f64()
              );
              data
          }
          Err(e) => {
              log_error!("Error parsing streaming log file: {}", e);
              return Ok(());
          }
      };

      // UFS 데이터 처리 (Latency 계산 등)
      let (ufs_data, block_data, ufscustom_data) = traces;

      log!("\n[1.2/6] Processing UFS data for latency calculations...");
      let ufs_process_start = Instant::now();

      let processed_ufs = if !ufs_data.is_empty() {
          log!("Applying latency analysis to UFS data...");
          ufs_bottom_half_latency_process(ufs_data)
      } else {
          ufs_data
      };

      log!(
          "UFS data processing complete: {} events (Time taken: {:.2}s)",
          processed_ufs.len(),
          ufs_process_start.elapsed().as_secs_f64()
      );

      // Block I/O 데이터 처리 (Latency 계산 등)
      log!("\n[1.3/6] Processing Block I/O data for latency calculations...");
      let block_process_start = Instant::now();

      let processed_block = if !block_data.is_empty() {
          log!("Applying latency analysis to Block I/O data...");
          block_bottom_half_latency_process(block_data)
      } else {
          block_data
      };

      log!(
          "Block I/O data processing complete: {} events (Time taken: {:.2}s)",
          processed_block.len(),
          block_process_start.elapsed().as_secs_f64()
      );

      // UFSCUSTOM 데이터 처리 (Latency 계산 등)
      log!("\n[1.4/6] Processing UFSCUSTOM data for latency calculations...");
      let ufscustom_process_start = Instant::now();

      let processed_ufscustom = if !ufscustom_data.is_empty() {
          log!("Applying latency analysis to UFSCUSTOM data...");
          crate::processors::ufscustom_bottom_half_latency_process(ufscustom_data)
      } else {
          ufscustom_data
      };

      log!(
          "UFSCUSTOM data processing complete: {} events (Time taken: {:.2}s)",
          processed_ufscustom.len(),
          ufscustom_process_start.elapsed().as_secs_f64()
      );

      // 처리된 데이터로 업데이트
      traces = (processed_ufs, processed_block, processed_ufscustom);

      // 필터 옵션이 있다면 로그에 기록
      if let Some(f) = filter {
          if f.start_time > 0.0 && f.end_time > 0.0 {
              log!(
                  "Using time filter: {:.3} - {:.3} ms",
                  f.start_time,
                  f.end_time
              );
          }

          if f.start_sector > 0 && f.end_sector > 0 {
              log!(
                  "Using sector/LBA filter: {} - {}",
                  f.start_sector,
                  f.end_sector
              );
          }

          if f.is_dtoc_filter_active() {
              log!(
                  "Using DTOC latency filter: {:.3} - {:.3} ms",
                  if f.min_dtoc > 0.0 { f.min_dtoc } else { 0.0 },
                  if f.max_dtoc > 0.0 { f.max_dtoc } else { f64::INFINITY }
              );
          }

          if f.is_ctoc_filter_active() {
              log!(
                  "Using CTOC latency filter: {:.3} - {:.3} ms",
                  if f.min_ctoc > 0.0 { f.min_ctoc } else { 0.0 },
                  if f.max_ctoc > 0.0 { f.max_ctoc } else { f64::INFINITY }
              );
          }

          if f.is_ctod_filter_active() {
              log!(
                  "Using CTOD latency filter: {:.3} - {:.3} ms",
                  if f.min_ctod > 0.0 { f.min_ctod } else { 0.0 },
                  if f.max_ctod > 0.0 { f.max_ctod } else { f64::INFINITY }
              );
          }

          if f.is_qd_filter_active() {
              log!(
                  "Using QD filter: {} - {}",
                  if f.min_qd > 0 { f.min_qd } else { 0 },
                  if f.max_qd > 0 { f.max_qd } else { u32::MAX }
              );
          }

          // 필터링 적용
          if f.is_time_filter_active() || f.is_sector_filter_active()
              || f.is_dtoc_filter_active() || f.is_ctoc_filter_active()
              || f.is_ctod_filter_active() || f.is_qd_filter_active() {
              log!("\n[1.5/6] Applying filters...");
              let filter_start = Instant::now();

              // 튜플에서 개별 요소 추출
              let (mut ufs_data, mut block_data, mut ufscustom_data) = traces;

              let original_counts = (ufs_data.len(), block_data.len(), ufscustom_data.len());

              // 필터 적용
              if !ufs_data.is_empty() {
                  ufs_data = filter_ufs_data(ufs_data, f);
              }

              if !block_data.is_empty() {
                  block_data = filter_block_data(block_data, f);
              }

              if !ufscustom_data.is_empty() {
                  ufscustom_data = filter_ufscustom_data(ufscustom_data, f);
              }

              // 필터링된 결과로 traces 업데이트
              traces = (ufs_data, block_data, ufscustom_data);

              log!(
                  "Streaming data filtered: ({} -> {}, {} -> {}, {} -> {}) events (Time taken: {:.2}s)",
                  original_counts.0,
                  traces.0.len(),
                  original_counts.1,
                  traces.1.len(),
                  original_counts.2,
                  traces.2.len(),
                  filter_start.elapsed().as_secs_f64()
              );
          }
      }

      // 통계 계산 및 출력
      log!("\n[2/6] Calculating statistics...");
      let stats_start = Instant::now();
      log!("\n=== Streaming Log File Analysis Results ===");

      // 튜플에서 개별 요소 추출
      let (ufs_traces, block_traces, ufscustom_traces) = &traces;

      // 기존 통계 함수 사용
      trace::output::print_ufs_statistics(ufs_traces);
      trace::output::print_block_statistics(block_traces);
      trace::output::print_ufscustom_statistics(ufscustom_traces);

      log!(
          "Statistics calculation complete (Time taken: {:.2}s)",
          stats_start.elapsed().as_secs_f64()
      );

      // Parquet 파일 저장
      log!("\n[3/6] Saving to Parquet files...");
      let save_start = Instant::now();

      let (ufs_data, block_data, ufscustom_data) = &traces;
      let has_ufs = !ufs_data.is_empty();
      let has_block = !block_data.is_empty();
      let has_ufscustom = !ufscustom_data.is_empty();

      match save_to_parquet(
          ufs_data,
          block_data,
          ufscustom_data,
          output_prefix,
          chunk_size,
      ) {
          Ok(()) => {
              let mut saved_files = Vec::new();
              if has_ufs {
                  saved_files.push(format!("{}_ufs.parquet", output_prefix));
              }
              if has_block {
                  saved_files.push(format!("{}_block.parquet", output_prefix));
              }
              if has_ufscustom {
                  saved_files.push(format!("{}_ufscustom.parquet", output_prefix));
              }
              log!(
                  "Parquet files saved successfully (Time taken: {:.2}s):\n{}",
                  save_start.elapsed().as_secs_f64(),
                  saved_files.join("\n")
              );
          }
          Err(e) => log_error!("Error while saving Parquet files: {}", e),
      }

      // CSV 내보내기 (요청된 경우)
      if export_csv {
          log!("\n[4/6] Saving filtered data to CSV files...");
          let csv_save_start = Instant::now();

          match save_to_csv(
              ufs_data,
              block_data,
              ufscustom_data,
              output_prefix,
          ) {
              Ok(()) => {
                  let mut saved_csv_files = Vec::new();
                  if has_ufs && !ufs_data.is_empty() {
                      saved_csv_files.push(format!("{}_ufs.csv", output_prefix));
                  }
                  if has_block && !block_data.is_empty() {
                      saved_csv_files.push(format!("{}_block.csv", output_prefix));
                  }
                  if has_ufscustom && !ufscustom_data.is_empty() {
                      saved_csv_files.push(format!("{}_ufscustom.csv", output_prefix));
                  }
                  if !saved_csv_files.is_empty() {
                      log!(
                          "Filtered CSV files saved successfully (Time taken: {:.2}s):\n{}",
                          csv_save_start.elapsed().as_secs_f64(),
                          saved_csv_files.join("\n")
                      );
                  } else {
                      log!("No CSV files saved (all filtered data is empty)");
                  }
              }
              Err(e) => log_error!("Error while saving CSV files: {}", e),
          }
      }

      // 차트 생성
      log!("\n[{}/6] Generating charts...", if export_csv { 5 } else { 4 });
      let charts_start = Instant::now();

      match trace::output::charts::generate_charts_with_config(ufs_traces, block_traces, ufscustom_traces, output_prefix, y_axis_ranges)
      {
          Ok(()) => log!(
              "Streaming charts generated successfully (Time taken: {:.2}s)",
              charts_start.elapsed().as_secs_f64()
          ),
          Err(e) => log_error!("Error while generating streaming charts: {}", e),
      }

      // 요약 정보 출력
      log!("\n===== Streaming Log File Processing Complete =====");
      log!(
          "Total time taken: {:.2}s",
          total_start_time.elapsed().as_secs_f64()
      );

      // 결과 요약
      log!("Processed events:");
      if has_ufs {
          log!("- UFS events: {}", ufs_data.len());
      }
      if has_block {
          log!("- Block I/O events: {}", block_data.len());
      }
      if has_ufscustom {
          log!("- UFSCUSTOM events: {}", ufscustom_data.len());
      }

      log!("Generated files:");

      // 생성된 파일 목록
      if has_ufs {
          log!("- UFS Parquet file: {}_ufs.parquet", output_prefix);
          if export_csv {
              log!("- UFS CSV file: {}_ufs.csv", output_prefix);
          }
          log!("- UFS Plotters charts: {}_ufs_*.png", output_prefix);
      }

      if has_block {
          log!("- Block I/O Parquet file: {}_block.parquet", output_prefix);
          if export_csv {
              log!("- Block I/O CSV file: {}_block.csv", output_prefix);
          }
          log!("- Block I/O Plotters charts: {}_block_*.png", output_prefix);
      }

      if has_ufscustom {
          log!("- UFSCUSTOM Parquet file: {}_ufscustom.parquet", output_prefix);
          if export_csv {
              log!("- UFSCUSTOM CSV file: {}_ufscustom.csv", output_prefix);
          }
          log!("- UFSCUSTOM Plotters charts: {}_ufscustom_*.png", output_prefix);
      }

      log!("- Log file: {}_result.log", output_prefix);

      // 로그 파일 버퍼 비우기
      let _ = Logger::flush();

      Ok(())
  }
  */

// TraceData 열거형 정의 - 각 트레이스 타입에 대한 데이터를 담습니다
#[allow(clippy::upper_case_acronyms)]
enum TraceData {
    UFS(Vec<UFS>),
    Block(Vec<Block>),
    UFSCUSTOM(Vec<UFSCUSTOM>),
    // 새 트레이스 타입 추가 시 여기에 추가
}

impl TraceData {
    // 데이터 개수 반환
    fn count(&self) -> usize {
        match self {
            TraceData::UFS(traces) => traces.len(),
            TraceData::Block(traces) => traces.len(),
            TraceData::UFSCUSTOM(traces) => traces.len(),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // 필터 적용
    fn apply_filter(&self, filter: &FilterOptions) -> Self {
        match self {
            TraceData::UFS(traces) => {
                let filtered = filter_ufs_data(traces.clone(), filter);
                TraceData::UFS(filtered)
            }
            TraceData::Block(traces) => {
                let filtered = filter_block_data(traces.clone(), filter);
                TraceData::Block(filtered)
            }
            TraceData::UFSCUSTOM(traces) => {
                let filtered = filter_ufscustom_data(traces.clone(), filter);
                TraceData::UFSCUSTOM(filtered)
            } // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // 통계 출력
    fn print_statistics(&self) {
        match self {
            TraceData::UFS(traces) => print_ufs_statistics(traces),
            TraceData::Block(traces) => print_block_statistics(traces),
            TraceData::UFSCUSTOM(traces) => print_ufscustom_statistics(traces),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // 차트 생성
    fn generate_charts(
        &self,
        output_prefix: &str,
        y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
    ) -> Result<(), String> {
        match self {
            TraceData::UFS(traces) => output::charts::generate_charts_with_config(
                traces,
                &[],
                &[],
                output_prefix,
                y_axis_ranges,
            ),
            TraceData::Block(traces) => output::charts::generate_charts_with_config(
                &[],
                traces,
                &[],
                output_prefix,
                y_axis_ranges,
            ),
            TraceData::UFSCUSTOM(traces) => output::charts::generate_charts_with_config(
                &[],
                &[],
                traces,
                output_prefix,
                y_axis_ranges,
            ),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // CSV 저장
    fn save_to_csv(&self, output_prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
        match self {
            TraceData::UFS(traces) => {
                save_to_csv(traces, &[], &[], output_prefix)?;
            }
            TraceData::Block(traces) => {
                save_to_csv(&[], traces, &[], output_prefix)?;
            }
            TraceData::UFSCUSTOM(traces) => {
                save_to_csv(&[], &[], traces, output_prefix)?;
            } // 새 트레이스 타입 추가 시 여기에 추가
        }
        Ok(())
    }

    // 요약 정보 출력
    fn print_summary(&self, output_prefix: &str) {
        match self {
            TraceData::UFS(traces) => {
                log!("Total UFS events analyzed: {}", traces.len());
                log!("Generated files:");
                log!("- UFS Plotters charts: {}_ufs_*.html", output_prefix);
                log!("- UFS Plotters charts: {}_ufs_*.png", output_prefix);
            }
            TraceData::Block(traces) => {
                log!("Total Block I/O events analyzed: {}", traces.len());
                log!("Generated files:");
                log!(
                    "- Block I/O Plotters charts: {}_block_*.html",
                    output_prefix
                );
                log!("- Block I/O Plotters charts: {}_block_*.png", output_prefix);
            }
            TraceData::UFSCUSTOM(traces) => {
                log!("Total UFSCustom events analyzed: {}", traces.len());
                log!("Generated files:");
                log!(
                    "- UFSCustom Plotters charts: {}_ufscustom_*.html",
                    output_prefix
                );
                log!(
                    "- UFSCustom Plotters charts: {}_ufscustom_*.png",
                    output_prefix
                );
            } // 새 트레이스 타입 추가 시 여기에 추가
        }
    }
}

// Parquet 파일에서 TraceData 로드
fn load_trace_data(
    trace_type: &TraceType,
    parquet_path: &str,
) -> Result<TraceData, Box<dyn std::error::Error + Send + Sync>> {
    match trace_type {
        TraceType::UFS => {
            let traces = read_ufs_from_parquet(parquet_path)?;
            Ok(TraceData::UFS(traces))
        }
        TraceType::Block => {
            let traces = read_block_from_parquet(parquet_path)?;
            Ok(TraceData::Block(traces))
        }
        TraceType::UFSCUSTOM => {
            let traces = read_ufscustom_from_parquet(parquet_path)?;
            Ok(TraceData::UFSCUSTOM(traces))
        } // 새 트레이스 타입 추가 시 여기에 추가
    }
}
