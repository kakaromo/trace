use std::collections::HashMap;
use std::env;
use std::io;
use std::time::Instant;
use trace::parsers::{parse_log_file_async, parse_log_file_high_perf, parse_log_file_streaming};
use trace::utils::{
    parse_latency_ranges, read_filter_options, set_user_latency_ranges, FilterOptions, Logger,
};
use trace::TraceType;
use trace::*;
use trace::output::save_to_csv;

/// Parse y-axis ranges from command line argument
/// Format: "metric:min:max,metric:min:max"
/// Example: "ufs_dtoc:0:100,block_dtoc:0:50"
fn parse_y_axis_ranges(input: &str) -> Result<HashMap<String, (f64, f64)>, String> {
    let mut ranges = HashMap::new();
    
    for part in input.split(',') {
        let components: Vec<&str> = part.split(':').collect();
        if components.len() != 3 {
            return Err(format!("Invalid format for y-axis range: '{}'. Expected format: metric:min:max", part));
        }
        
        let metric = components[0].to_string();
        let min = components[1].parse::<f64>()
            .map_err(|_| format!("Invalid minimum value '{}' for metric '{}'", components[1], metric))?;
        let max = components[2].parse::<f64>()
            .map_err(|_| format!("Invalid maximum value '{}' for metric '{}'", components[2], metric))?;
        
        if min >= max {
            return Err(format!("Minimum value ({}) must be less than maximum value ({}) for metric '{}'", min, max, metric));
        }
        
        ranges.insert(metric, (min, max));
    }
    
    Ok(ranges)
}

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {} [options] <log_file> <output_prefix>                      - Parse log file and generate statistics", program);
    eprintln!("  {} [options] --parquet <type> <parquet_file> <output_prefix> - Read Parquet file and generate statistics", program);
    eprintln!("    where <type> is one of: 'ufs', 'block', 'ufscustom'");
    eprintln!("  {} [options] --ufscustom <custom_file> <output_prefix>       - Parse UFSCustom CSV file and generate statistics", program);
    eprintln!("  {} [options] --async <log_file> <output_prefix>              - Parse log file using async I/O (optimized)", program);
    eprintln!("  {} [options] --highperf <log_file> <output_prefix>           - Parse log file using high-performance memory-mapped I/O", program);
    eprintln!("  {} [options] --streaming <log_file> <output_prefix>          - Parse log file using streaming with memory management", program);
    eprintln!("\nOptions:");
    eprintln!("  -l <values>  - Custom latency ranges in ms (comma-separated). Example: -l 0.1,0.5,1,5,10,50,100");
    eprintln!("  -f           - Apply filters (time, sector/lba, latency, queue depth) with interactive input");
    eprintln!("  -y <ranges>  - Set y-axis ranges for charts. Format: metric:min:max,metric:min:max");
    eprintln!("                 Metrics: ufs_dtoc, ufs_ctoc, ufs_ctod, ufs_qd, ufs_lba, block_dtoc, block_ctoc, block_ctod, block_qd, block_lba");
    eprintln!("                 Example: -y ufs_dtoc:0:100,block_dtoc:0:50");
    eprintln!("  -c <size>    - Set chunk size for Parquet file writing (default: 50000). Example: -c 100000");
    eprintln!("  --csv        - Export filtered data to CSV files (works with all modes including --parquet)");
    // 새 트레이스 타입이나 옵션이 추가되면 여기에 업데이트
}

#[tokio::main]
async fn main() -> io::Result<()> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();

    // 인자가 없으면 사용법 출력
    if args.len() <= 1 {
        eprintln!("Error: No arguments provided");
        print_usage(&args[0]);
        return Ok(());
    }

    // 옵션 파싱
    let mut i = 1;
    let mut log_file_index = 0;
    let mut output_prefix_index = 0;
    let mut is_parquet_mode = false;
    let mut parquet_type_index = 0;
    let mut parquet_path_index = 0;
    let mut is_ufscustom_mode = false;
    let mut ufscustom_file_index = 0;
    let mut is_async_mode = false;
    let mut async_log_file_index = 0;
    let mut async_output_prefix_index = 0;
    let mut is_highperf_mode = false;
    let mut highperf_log_file_index = 0;
    let mut highperf_output_prefix_index = 0;
    let mut is_streaming_mode = false;
    let mut streaming_log_file_index = 0;
    let mut streaming_output_prefix_index = 0;
    let mut use_filter = false;
    let mut y_axis_ranges: Option<HashMap<String, (f64, f64)>> = None;
    let mut chunk_size: usize = 50_000; // 기본 청크 크기
    let mut export_csv = false; // CSV export 옵션

    while i < args.len() {
        match args[i].as_str() {
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
                        eprintln!("Error in latency ranges: {}", e);
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
                        eprintln!("Error in y-axis ranges: {}", e);
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
            "--parquet" => {
                is_parquet_mode = true;
                parquet_type_index = i + 1;
                parquet_path_index = i + 2;
                output_prefix_index = i + 3;
                i += 1;
            }
            "--ufscustom" => {
                is_ufscustom_mode = true;
                ufscustom_file_index = i + 1;
                output_prefix_index = i + 2;
                i += 1;
            }
            "--async" => {
                is_async_mode = true;
                async_log_file_index = i + 1;
                async_output_prefix_index = i + 2;
                i += 1;
            }
            "--highperf" => {
                is_highperf_mode = true;
                highperf_log_file_index = i + 1;
                highperf_output_prefix_index = i + 2;
                i += 1;
            }
            "--streaming" => {
                is_streaming_mode = true;
                streaming_log_file_index = i + 1;
                streaming_output_prefix_index = i + 2;
                i += 1;
            }
            _ => {
                // 일반 위치 인수 처리
                if !is_parquet_mode && !is_ufscustom_mode && !is_async_mode && !is_highperf_mode && !is_streaming_mode {
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

    // 필터 옵션 처리
    let filter_options = if use_filter {
        println!("필터 옵션을 입력하세요 (입력하지 않거나 0으로 입력 시 필터링하지 않습니다):");
        match read_filter_options() {
            Ok(filter) => {
                // 필터 정보 출력
                println!("적용된 필터 옵션:");
                if filter.start_time > 0.0 && filter.end_time > 0.0 {
                    println!(
                        "  시간 필터: {:.3} - {:.3} ms",
                        filter.start_time, filter.end_time
                    );
                } else {
                    println!("  시간 필터: 사용하지 않음");
                }

                if filter.start_sector > 0 && filter.end_sector > 0 {
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
                        if filter.min_dtoc > 0.0 { filter.min_dtoc } else { 0.0 },
                        if filter.max_dtoc > 0.0 { filter.max_dtoc } else { f64::INFINITY }
                    );
                } else {
                    println!("  DTOC 레이턴시 필터: 사용하지 않음");
                }

                if filter.is_ctoc_filter_active() {
                    println!(
                        "  CTOC 레이턴시 필터: {:.3} - {:.3} ms",
                        if filter.min_ctoc > 0.0 { filter.min_ctoc } else { 0.0 },
                        if filter.max_ctoc > 0.0 { filter.max_ctoc } else { f64::INFINITY }
                    );
                } else {
                    println!("  CTOC 레이턴시 필터: 사용하지 않음");
                }

                if filter.is_ctod_filter_active() {
                    println!(
                        "  CTOD 레이턴시 필터: {:.3} - {:.3} ms",
                        if filter.min_ctod > 0.0 { filter.min_ctod } else { 0.0 },
                        if filter.max_ctod > 0.0 { filter.max_ctod } else { f64::INFINITY }
                    );
                } else {
                    println!("  CTOD 레이턴시 필터: 사용하지 않음");
                }

                if filter.is_qd_filter_active() {
                    println!(
                        "  QD 필터: {} - {}",
                        if filter.min_qd > 0 { filter.min_qd } else { 0 },
                        if filter.max_qd > 0 { filter.max_qd } else { u32::MAX }
                    );
                } else {
                    println!("  QD 필터: 사용하지 않음");
                }

                // 전역 필터 옵션 설정
                set_filter_options(filter.clone());
                Some(filter)
            }
            Err(e) => {
                eprintln!("필터 옵션 읽기 오류: {}", e);
                None
            }
        }
    } else {
        None
    };

    // 명령줄 인수 처리
    let result = if !is_parquet_mode
        && !is_ufscustom_mode
        && !is_async_mode
        && log_file_index > 0
        && output_prefix_index > 0
    {
        // 일반 로그 파싱 모드
        process_log_file(
            &args[log_file_index],
            &args[output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
            export_csv,
        )
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
                eprintln!("Error: {}", e);
                eprintln!("Supported types: 'ufs', 'block', 'ufscustom'"); // 새 타입 추가 시 업데이트
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
    } else if is_ufscustom_mode
        && ufscustom_file_index > 0
        && ufscustom_file_index < args.len()
        && output_prefix_index > 0
        && output_prefix_index < args.len()
    {
        // UFSCustom 파일 처리 모드
        process_ufscustom_file(
            &args[ufscustom_file_index],
            &args[output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
        )
    } else if is_async_mode
        && async_log_file_index > 0
        && async_log_file_index < args.len()
        && async_output_prefix_index > 0
        && async_output_prefix_index < args.len()
    {
        // 비동기 로그 파일 처리 모드 - 이미 tokio 런타임 내에 있으므로 직접 호출
        process_async_log_file(
            &args[async_log_file_index],
            &args[async_output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
            export_csv,
        )
        .await
    } else if is_highperf_mode
        && highperf_log_file_index > 0
        && highperf_log_file_index < args.len()
        && highperf_output_prefix_index > 0
        && highperf_output_prefix_index < args.len()
    {
        // 고성능 메모리 매핑 로그 파일 처리 모드
        process_highperf_log_file(
            &args[highperf_log_file_index],
            &args[highperf_output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
            export_csv,
        )
        .await
    } else if is_streaming_mode
        && streaming_log_file_index > 0
        && streaming_log_file_index < args.len()
        && streaming_output_prefix_index > 0
        && streaming_output_prefix_index < args.len()
    {
        // 스트리밍 로그 파일 처리 모드
        process_streaming_log_file(
            &args[streaming_log_file_index],
            &args[streaming_output_prefix_index],
            filter_options.as_ref(),
            y_axis_ranges.as_ref(),
            chunk_size,
            export_csv,
        )
        .await
    } else {
        // 인자 설정이 잘못된 경우
        eprintln!("Error: Invalid arguments");
        print_usage(&args[0]);
        return Ok(());
    };

    // 에러 처리: 프로세싱 함수에서 에러가 발생한 경우 메시지 출력
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        print_usage(&args[0]);
    }

    Ok(())
}

// 기존 로그 파일 처리 로직을 별도 함수로 분리
fn process_log_file(
    log_file_path: &str,
    output_prefix: &str,
    filter: Option<&FilterOptions>,
    y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
    chunk_size: usize,
    export_csv: bool,
) -> io::Result<()> {
    // Logger 초기화 - 로그 파일은 trace가 저장되는 경로와 동일하게 설정
    Logger::init(output_prefix);

    // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
    if let Some(ranges) = trace::utils::get_user_latency_ranges() {
        log!("Using custom latency ranges: {:?}", ranges);
    }

    // 청크 크기 로그에 기록
    log!("Using Parquet chunk size: {}", chunk_size);

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
    }

    let total_start_time = Instant::now();
    log!("===== Starting Log File Processing =====");

    // Parse log file
    log!("\n[1/5] Parsing log file...");
    let parse_start = Instant::now();

    // 로그 파일 파싱 - UFS, Block IO, UFSCUSTOM 타입 지원
    let parse_result = match parse_log_file(log_file_path) {
        Ok(result) => result,
        Err(e) => {
            log_error!("File parsing error: {}", e);
            (Vec::new(), Vec::new(), Vec::new()) // Return empty vectors on error
        }
    };

    // 여기서 각 트레이스 타입의 수와 처리 시간 로깅
    log!(
        "Log parsing complete: Time taken: {:.2}s",
        parse_start.elapsed().as_secs_f64()
    );

    // 각 트레이스 타입 결과 출력
    let mut trace_counts = Vec::new();
    let has_ufs = !parse_result.0.is_empty();
    let has_block = !parse_result.1.is_empty();
    let has_ufscustom = !parse_result.2.is_empty();

    if has_ufs {
        trace_counts.push(format!("UFS={}", parse_result.0.len()));
    }
    if has_block {
        trace_counts.push(format!("Block={}", parse_result.1.len()));
    }
    if has_ufscustom {
        trace_counts.push(format!("UFSCUSTOM={}", parse_result.2.len()));
    }

    log!("Parsed traces: {}", trace_counts.join(", "));

    // Post-processing (parallel processing)
    log!("\n[2/5] Post-processing data...");
    let process_start = Instant::now();

    // 발견된 각 트레이스 타입 처리
    let mut processed_traces = Vec::new();

    // 처리된 결과를 담을 변수들
    let mut processed_ufs = if has_ufs {
        log!("Post-processing UFS data...");
        let processed = ufs_bottom_half_latency_process(parse_result.0);
        processed_traces.push(("UFS", processed.len()));
        processed
    } else {
        Vec::new()
    };

    let mut processed_blocks = if has_block {
        log!("Post-processing Block I/O data...");
        let processed = block_bottom_half_latency_process(parse_result.1);
        processed_traces.push(("Block", processed.len()));
        processed
    } else {
        Vec::new()
    };

    let mut processed_ufscustom = if has_ufscustom {
        log!("Post-processing UFSCUSTOM data...");
        processed_traces.push(("UFSCUSTOM", parse_result.2.len()));
        parse_result.2
    } else {
        Vec::new()
    };

    // 필터링 적용
    if let Some(filter) = filter {
        if filter.is_time_filter_active() || filter.is_sector_filter_active() 
            || filter.is_dtoc_filter_active() || filter.is_ctoc_filter_active() 
            || filter.is_ctod_filter_active() || filter.is_qd_filter_active() {
            log!("\n[2.5/6] Applying filters...");
            let filter_start = Instant::now();

            let original_counts = (
                processed_ufs.len(),
                processed_blocks.len(),
                processed_ufscustom.len(),
            );

            // 시간 및 섹터/LBA 필터 적용
            if !processed_ufs.is_empty() {
                processed_ufs = filter_ufs_data(processed_ufs, filter);
                log!(
                    "  UFS data filtered: {} -> {} events",
                    original_counts.0,
                    processed_ufs.len()
                );
            }

            if !processed_blocks.is_empty() {
                processed_blocks = filter_block_data(processed_blocks, filter);
                log!(
                    "  Block data filtered: {} -> {} events",
                    original_counts.1,
                    processed_blocks.len()
                );
            }

            if !processed_ufscustom.is_empty() {
                processed_ufscustom = filter_ufscustom_data(processed_ufscustom, filter);
                log!(
                    "  UFSCUSTOM data filtered: {} -> {} events",
                    original_counts.2,
                    processed_ufscustom.len()
                );
            }

            log!(
                "Filtering complete: Time taken: {:.2}s",
                filter_start.elapsed().as_secs_f64()
            );
        }
    }

    log!(
        "Post-processing complete: Time taken: {:.2}s",
        process_start.elapsed().as_secs_f64()
    );

    // Output analysis results
    log!("\n[3/5] Calculating analysis results...");
    let analysis_start = Instant::now();

    // 각 트레이스 타입에 대한 통계 계산
    if has_ufs {
        log!("\n=== UFS Analysis Results ===");
        print_ufs_statistics(&processed_ufs);
    }

    if has_block {
        log!("\n=== Block I/O Analysis Results ===");
        print_block_statistics(&processed_blocks);
    }

    if has_ufscustom {
        log!("\n=== UFSCUSTOM Analysis Results ===");
        print_ufscustom_statistics(&processed_ufscustom);
    }

    log!(
        "\nAnalysis complete: Time taken: {:.2}s",
        analysis_start.elapsed().as_secs_f64()
    );

    // Save to Parquet files
    log!("\n[4/5] Saving to Parquet files...");
    let save_start = Instant::now();

    match save_to_parquet(
        &processed_ufs,
        &processed_blocks,
        &processed_ufscustom,
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

    // Save to CSV files if requested (using filtered data)
    if export_csv {
        log!("\n[4.5/5] Saving filtered data to CSV files...");
        let csv_save_start = Instant::now();

        match save_to_csv(
            &processed_ufs,
            &processed_blocks,
            &processed_ufscustom,
            output_prefix,
        ) {
            Ok(()) => {
                let mut saved_csv_files = Vec::new();
                if has_ufs && !processed_ufs.is_empty() {
                    saved_csv_files.push(format!("{}_ufs.csv", output_prefix));
                }
                if has_block && !processed_blocks.is_empty() {
                    saved_csv_files.push(format!("{}_block.csv", output_prefix));
                }
                if has_ufscustom && !processed_ufscustom.is_empty() {
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

    // Generate Plotly charts
    log!("\n[5/5] Generating  charts...");
    let charts_start = Instant::now();

    match output::charts::generate_charts_with_config(
        &processed_ufs,
        &processed_blocks,
        &processed_ufscustom,
        output_prefix,
        y_axis_ranges,
    ) {
        Ok(()) => log!(
            "Plotly charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating Plotly charts: {}", e),
    }

    log!("\n===== All Processing Complete! =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    // 결과 요약
    log!("Processed events:");
    for (trace_type, count) in processed_traces {
        log!("- {} events: {}", trace_type, count);
    }

    log!("Generated files:");

    // 생성된 파일 목록
    if has_ufs {
        log!("- UFS Parquet file: {}_ufs.parquet", output_prefix);
        if export_csv {
            log!("- UFS CSV file: {}_ufs.csv", output_prefix);
        }
        log!("- UFS Plotly charts: {}_ufs_*.html", output_prefix);
        log!("- UFS Matplotlib charts: {}_ufs_*.png", output_prefix);
    }

    if has_block {
        log!("- Block I/O Parquet file: {}_block.parquet", output_prefix);
        if export_csv {
            log!("- Block I/O CSV file: {}_block.csv", output_prefix);
        }
        log!("- Block I/O Plotly charts: {}_block_*.html", output_prefix);
        log!(
            "- Block I/O Matplotlib charts: {}_block_*.png",
            output_prefix
        );
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
            "- UFSCUSTOM Plotly charts: {}_ufscustom_*.html",
            output_prefix
        );
        log!(
            "- UFSCUSTOM Matplotlib charts: {}_ufscustom_*.png",
            output_prefix
        );
    }

    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

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
    log!("\n[{}/{}] Generating {} Plotly charts...", if export_csv { 3 } else { 2 }, if export_csv { 4 } else { 3 }, data_label);
    let charts_start = Instant::now();

    match trace_data.generate_charts(output_prefix, y_axis_ranges) {
        Ok(()) => log!(
            "{} Plotly charts generated successfully (Time taken: {:.2}s)",
            data_label,
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating {} Plotly charts: {}", data_label, e),
    }

    // 5. 요약 정보 출력
    log!("\n===== {} Parquet Analysis Complete! =====", data_label);
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    trace_data.print_summary(output_prefix);

    if export_csv {
        log!("- {} CSV file: {}_{}.csv", data_label, output_prefix, data_label.to_lowercase());
    }
    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

// UFSCustom 파일 처리 로직
fn process_ufscustom_file(
    custom_file_path: &str,
    output_prefix: &str,
    filter: Option<&FilterOptions>,
    y_axis_ranges: Option<&HashMap<String, (f64, f64)>>,
    _chunk_size: usize, // UFSCustom은 Parquet 저장하지 않으므로 사용하지 않지만 일관성을 위해 유지
) -> io::Result<()> {
    // Logger 초기화
    Logger::init(output_prefix);

    // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
    if let Some(ranges) = trace::utils::get_user_latency_ranges() {
        log!("Using custom latency ranges: {:?} ms", ranges);
    }

    let total_start_time = Instant::now();
    log!("===== Starting UFSCustom File Processing =====");

    // 1. 파일 로딩
    log!("\n[1/3] Loading UFSCustom file...");
    let load_start = Instant::now();

    let mut traces = match parse_ufscustom_file(custom_file_path) {
        Ok(data) => {
            log!(
                "UFSCustom file loaded successfully: {} events (Time taken: {:.2}s)",
                data.len(),
                load_start.elapsed().as_secs_f64()
            );
            data
        }
        Err(e) => {
            log_error!("Error loading UFSCustom file: {}", e);
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

        // UFSCUSTOM은 dtoc만 지원
        if f.is_dtoc_filter_active() {
            log!(
                "Using DTOC latency filter: {:.3} - {:.3} ms",
                if f.min_dtoc > 0.0 { f.min_dtoc } else { 0.0 },
                if f.max_dtoc > 0.0 { f.max_dtoc } else { f64::INFINITY }
            );
        }

        // 필터링 적용 (UFSCUSTOM은 dtoc만 지원)
        if f.is_time_filter_active() || f.is_sector_filter_active() || f.is_dtoc_filter_active() {
            log!("\n[1.5/3] Applying filters...");
            let filter_start = Instant::now();

            let original_count = traces.len();
            traces = filter_ufscustom_data(traces, f);

            log!(
                "UFSCustom data filtered: {} -> {} events (Time taken: {:.2}s)",
                original_count,
                traces.len(),
                filter_start.elapsed().as_secs_f64()
            );
        }
    };

    // 2. 통계 계산 및 출력
    log!("\n[2/3] Calculating UFSCustom statistics...");
    let stats_start = Instant::now();
    log!("\n=== UFSCustom Analysis Results ===");

    print_ufscustom_statistics(&traces);

    log!(
        "\nStatistics calculation complete (Time taken: {:.2}s)",
        stats_start.elapsed().as_secs_f64()
    );

    // 3. 차트 생성
    log!("\n[3/3] Generating UFSCustom Plotly charts...");
    let charts_start = Instant::now();

    match output::charts::generate_charts_with_config(&[], &[], &traces, output_prefix, y_axis_ranges) {
        Ok(()) => log!(
            "UFSCustom Plotly charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating UFSCustom Plotly charts: {}", e),
    }


    // 4. 요약 정보 출력
    log!("\n===== UFSCustom File Processing Complete! =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    log!("Total UFSCustom events analyzed: {}", traces.len());
    log!("Generated files:");
    log!(
        "- UFSCUSTOM Plotly charts: {}_ufscustom_*.html",
        output_prefix
    );
    log!(
        "- UFSCUSTOM Matplotlib charts: {}_ufscustom_*.png",
        output_prefix
    );
    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

// 비동기 로그 파일 처리 로직
async fn process_async_log_file(
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
    log!("===== Starting Async Log File Processing =====");

    // 비동기 로그 파일 파싱
    log!("\n[1/3] Parsing log file asynchronously...");
    let parse_start = Instant::now();

    let mut traces = match parse_log_file_async(log_file_path).await {
        Ok(data) => {
            let (ref ufs_data, ref block_data, ref ufscustom_data) = data;
            let total_events = ufs_data.len() + block_data.len() + ufscustom_data.len();
            log!(
                "Async log file parsing complete: {} events (Time taken: {:.2}s)",
                total_events,
                parse_start.elapsed().as_secs_f64()
            );
            data
        }
        Err(e) => {
            log_error!("Error parsing async log file: {}", e);
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
    log!("\n[1.3/3] Processing Block I/O data for latency calculations...");
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
    
    // 처리된 데이터로 업데이트
    traces = (processed_ufs, processed_block, ufscustom_data);

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
            log!("\n[1.5/3] Applying filters...");
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
                "Async data filtered: ({} -> {}, {} -> {}, {} -> {}) events (Time taken: {:.2}s)",
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
    log!("\n[2/3] Calculating statistics...");
    let stats_start = Instant::now();
    log!("\n=== Async Log File Analysis Results ===");

    // 튜플에서 개별 요소 추출
    let (ufs_traces, block_traces, ufscustom_traces) = &traces;

    // 기존 통계 함수 사용
    trace::output::print_ufs_statistics(ufs_traces);
    trace::output::print_block_statistics(block_traces);
    trace::output::print_ufscustom_statistics(ufscustom_traces);

    log!(
        "\nStatistics calculation complete (Time taken: {:.2}s)",
        stats_start.elapsed().as_secs_f64()
    );

    // 차트 생성
    log!("\n[3/3] Generating charts...");
    let charts_start = Instant::now();

    match trace::output::charts::generate_charts_with_config(ufs_traces, block_traces, ufscustom_traces, output_prefix, y_axis_ranges)
    {
        Ok(()) => log!(
            "Async charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating async charts: {}", e),
    }

    // 요약 정보 출력
    log!("\n===== Async Log File Processing Complete! =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    let (ref ufs_data, ref block_data, ref ufscustom_data) = traces;
    let total_events = ufs_data.len() + block_data.len() + ufscustom_data.len();
    log!("Total async events analyzed: {}", total_events);
    log!("Generated files:");
    log!("- Async charts: {}_async_*.html", output_prefix);
    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

// 고성능 메모리 매핑 로그 파일 처리 로직
async fn process_highperf_log_file(
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
    log!("\n[1.3/3] Processing Block I/O data for latency calculations...");
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
    
    // 처리된 데이터로 업데이트
    traces = (processed_ufs, processed_block, ufscustom_data);

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
                if f.max_dtoc > 0.0 { f.max_dtoc } else { f64::MAX }
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

    // 통계 생성 및 차트 생성
    log!("\n[2/3] Generating statistics and charts...");
    let statistics_start = Instant::now();

    // 튜플에서 개별 요소 추출
    let (ufs_traces, block_traces, ufscustom_traces) = &traces;

    // 기존 통계 함수 사용
    trace::output::print_ufs_statistics(ufs_traces);
    trace::output::print_block_statistics(block_traces);
    trace::output::print_ufscustom_statistics(ufscustom_traces);

    log!(
        "Statistics and charts generation complete (Time taken: {:.2}s)",
        statistics_start.elapsed().as_secs_f64()
    );

    // 전체 처리 시간 로그
    log!(
        "\n===== High-Performance Log File Processing Complete =====\nTotal processing time: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    Ok(())
}    // 스트리밍 로그 파일 처리 로직
async fn process_streaming_log_file(
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
    
    // 처리된 데이터로 업데이트
    traces = (processed_ufs, processed_block, ufscustom_data);

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
        log!("- UFS Plotly charts: {}_ufs_*.html", output_prefix);
        log!("- UFS Matplotlib charts: {}_ufs_*.png", output_prefix);
    }

    if has_block {
        log!("- Block I/O Parquet file: {}_block.parquet", output_prefix);
        if export_csv {
            log!("- Block I/O CSV file: {}_block.csv", output_prefix);
        }
        log!("- Block I/O Plotly charts: {}_block_*.html", output_prefix);
        log!("- Block I/O Matplotlib charts: {}_block_*.png", output_prefix);
    }

    if has_ufscustom {
        log!("- UFSCUSTOM Parquet file: {}_ufscustom.parquet", output_prefix);
        if export_csv {
            log!("- UFSCUSTOM CSV file: {}_ufscustom.csv", output_prefix);
        }
        log!("- UFSCUSTOM Plotly charts: {}_ufscustom_*.html", output_prefix);
        log!("- UFSCUSTOM Matplotlib charts: {}_ufscustom_*.png", output_prefix);
    }

    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

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
            }
            // 새 트레이스 타입 추가 시 여기에 추가
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
    fn generate_charts(&self, output_prefix: &str, y_axis_ranges: Option<&HashMap<String, (f64, f64)>>) -> Result<(), String> {
        match self {
            TraceData::UFS(traces) => output::charts::generate_charts_with_config(traces, &[], &[], output_prefix, y_axis_ranges),
            TraceData::Block(traces) => output::charts::generate_charts_with_config(&[], traces, &[], output_prefix, y_axis_ranges),
            TraceData::UFSCUSTOM(traces) => output::charts::generate_charts_with_config(&[], &[], traces, output_prefix, y_axis_ranges),
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
            }
            // 새 트레이스 타입 추가 시 여기에 추가
        }
        Ok(())
    }

    // 요약 정보 출력
    fn print_summary(&self, output_prefix: &str) {
        match self {
            TraceData::UFS(traces) => {
                log!("Total UFS events analyzed: {}", traces.len());
                log!("Generated files:");
                log!("- UFS Plotly charts: {}_ufs_*.html", output_prefix);
                log!("- UFS Matplotlib charts: {}_ufs_*.png", output_prefix);
            }
            TraceData::Block(traces) => {
                log!("Total Block I/O events analyzed: {}", traces.len());
                log!("Generated files:");
                log!("- Block I/O Plotly charts: {}_block_*.html", output_prefix);
                log!(
                    "- Block I/O Matplotlib charts: {}_block_*.png",
                    output_prefix
                );
            }
            TraceData::UFSCUSTOM(traces) => {
                log!("Total UFSCustom events analyzed: {}", traces.len());
                log!("Generated files:");
                log!(
                    "- UFSCustom Plotly charts: {}_ufscustom_*.html",
                    output_prefix
                );
                log!(
                    "- UFSCustom Matplotlib charts: {}_ufscustom_*.png",
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
) -> Result<TraceData, Box<dyn std::error::Error>> {
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
