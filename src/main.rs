use std::env;
use std::io;
use std::time::Instant;
use trace::utils::{Logger, parse_latency_ranges, set_user_latency_ranges};
use trace::TraceType;
use trace::*;

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {} [options] <log_file> <output_prefix>                      - Parse log file and generate statistics", program);
    eprintln!("  {} [options] --parquet <type> <parquet_file> <output_prefix> - Read Parquet file and generate statistics", program);
    eprintln!("    where <type> is one of: 'ufs', 'block', 'ufscustom'");
    eprintln!("  {} [options] --ufscustom <custom_file> <output_prefix>       - Parse UFSCustom CSV file and generate statistics", program);
    eprintln!("\nOptions:");
    eprintln!("  -l <values>  - Custom latency ranges in ms (comma-separated). Example: -l 0.1,0.5,1,5,10,50,100");
    // 새 트레이스 타입이나 옵션이 추가되면 여기에 업데이트
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
    
    // 옵션 파싱
    let mut i = 1;
    let mut log_file_index = 0;
    let mut output_prefix_index = 0;
    let mut is_parquet_mode = false;
    let mut parquet_type_index = 0;
    let mut parquet_path_index = 0;
    let mut is_ufscustom_mode = false;
    let mut ufscustom_file_index = 0;
    
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
                    },
                    Err(e) => {
                        eprintln!("Error in latency ranges: {}", e);
                        print_usage(&args[0]);
                        return Ok(());
                    }
                }
                
                i += 2; // 옵션과 값을 건너뜀
            },
            "--parquet" => {
                is_parquet_mode = true;
                parquet_type_index = i + 1;
                parquet_path_index = i + 2;
                output_prefix_index = i + 3;
                i += 1;
            },
            "--ufscustom" => {
                is_ufscustom_mode = true;
                ufscustom_file_index = i + 1;
                output_prefix_index = i + 2;
                i += 1;
            },
            _ => {
                // 일반 위치 인수 처리
                if !is_parquet_mode && !is_ufscustom_mode {
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

    // 명령줄 인수 처리
    let result = if !is_parquet_mode && !is_ufscustom_mode && log_file_index > 0 && output_prefix_index > 0 {
        // 일반 로그 파싱 모드
        process_log_file(&args[log_file_index], &args[output_prefix_index])
    } else if is_parquet_mode && parquet_type_index > 0 && parquet_type_index < args.len() &&
              parquet_path_index > 0 && parquet_path_index < args.len() &&
              output_prefix_index > 0 && output_prefix_index < args.len() {
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
        process_single_parquet_file(trace_type, &args[parquet_path_index], &args[output_prefix_index])
    } else if is_ufscustom_mode && ufscustom_file_index > 0 && ufscustom_file_index < args.len() &&
              output_prefix_index > 0 && output_prefix_index < args.len() {
        // UFSCustom 파일 처리 모드
        process_ufscustom_file(&args[ufscustom_file_index], &args[output_prefix_index])
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
fn process_log_file(log_file_path: &str, output_prefix: &str) -> io::Result<()> {
    // Logger 초기화 - 로그 파일은 trace가 저장되는 경로와 동일하게 설정
    Logger::init(output_prefix);
    
    // 사용자 정의 레이턴시 범위가 있다면 로그에 기록
    if let Some(ranges) = trace::utils::get_user_latency_ranges() {
        log!("Using custom latency ranges: {:?} ms", ranges);
    }

    let total_start_time = Instant::now();
    log!("===== Starting Log File Processing =====");

    // Parse log file
    log!("\n[1/6] Parsing log file...");
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
    log!("\n[2/6] Post-processing data...");
    let process_start = Instant::now();

    // 발견된 각 트레이스 타입 처리
    let mut processed_traces = Vec::new();

    // 처리된 결과를 담을 변수들
    let processed_ufs = if has_ufs {
        log!("Post-processing UFS data...");
        let processed = ufs_bottom_half_latency_process(parse_result.0);
        processed_traces.push(("UFS", processed.len()));
        processed
    } else {
        Vec::new()
    };

    let processed_blocks = if has_block {
        log!("Post-processing Block I/O data...");
        let processed = block_bottom_half_latency_process(parse_result.1);
        processed_traces.push(("Block", processed.len()));
        processed
    } else {
        Vec::new()
    };

    let processed_ufscustom = if has_ufscustom {
        log!("Post-processing UFSCUSTOM data...");
        processed_traces.push(("UFSCUSTOM", parse_result.2.len()));
        parse_result.2
    } else {
        Vec::new()
    };

    log!(
        "Post-processing complete: Time taken: {:.2}s",
        process_start.elapsed().as_secs_f64()
    );

    // Output analysis results
    log!("\n[3/6] Calculating analysis results...");
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
    log!("\n[4/6] Saving to Parquet files...");
    let save_start = Instant::now();

    match save_to_parquet(&processed_ufs, &processed_blocks, &processed_ufscustom, output_prefix) {
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

    // Generate Plotly charts
    log!("\n[5/6] Generating Plotly charts...");
    let charts_start = Instant::now();

    match generate_charts(&processed_ufs, &processed_blocks, &processed_ufscustom, output_prefix) {
        Ok(()) => log!(
            "Plotly charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating Plotly charts: {}", e),
    }
    log!("\n[6/6] Generating Plotly charts...");
    if let Err(e) = generate_plotters_charts(&processed_ufs, &processed_blocks, &processed_ufscustom, output_prefix) {
        eprintln!("차트 생성 중 오류 발생: {}", e);
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
        log!("- UFS Plotly charts: {}_ufs_*.html", output_prefix);
        log!("- UFS Matplotlib charts: {}_ufs_*.png", output_prefix);
    }

    if has_block {
        log!("- Block I/O Parquet file: {}_block.parquet", output_prefix);
        log!("- Block I/O Plotly charts: {}_block_*.html", output_prefix);
        log!(
            "- Block I/O Matplotlib charts: {}_block_*.png",
            output_prefix
        );
    }

    if has_ufscustom {
        log!("- UFSCUSTOM Parquet file: {}_ufscustom.parquet", output_prefix);
        log!("- UFSCUSTOM Plotly charts: {}_ufscustom_*.html", output_prefix);
        log!("- UFSCUSTOM Matplotlib charts: {}_ufscustom_*.png", output_prefix);
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
    let trace_data = match load_trace_data(&trace_type, parquet_path) {
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

    // 2. 통계 계산 및 출력
    log!("\n[2/3] Calculating {} statistics...", data_label);
    let stats_start = Instant::now();
    log!("\n=== {} Analysis Results ===", data_label);

    trace_data.print_statistics();

    log!(
        "\nStatistics calculation complete (Time taken: {:.2}s)",
        stats_start.elapsed().as_secs_f64()
    );

    // 3. 차트 생성
    log!("\n[3/3] Generating {} Plotly charts...", data_label);
    let charts_start = Instant::now();

    match trace_data.generate_charts(output_prefix) {
        Ok(()) => log!(
            "{} Plotly charts generated successfully (Time taken: {:.2}s)",
            data_label,
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating {} Plotly charts: {}", data_label, e),
    }

    match trace_data.generate_plotters_charts(output_prefix) {
        Ok(()) => log!(
            "{} plotters charts generated successfully (Time taken: {:.2}s)",
            data_label,
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating {} plotters charts: {}", data_label, e),
    }

    

    

    // 4. 요약 정보 출력
    log!("\n===== {} Parquet Analysis Complete! =====", data_label);
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    trace_data.print_summary(output_prefix);

    log!("- Log file: {}_result.log", output_prefix);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}

// UFSCustom 파일 처리 로직
fn process_ufscustom_file(custom_file_path: &str, output_prefix: &str) -> io::Result<()> {
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

    let traces = match parse_ufscustom_file(custom_file_path) {
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

    match output::charts::generate_ufscustom_charts(&traces, output_prefix) {
        Ok(()) => log!(
            "UFSCustom Plotly charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating UFSCustom Plotly charts: {}", e),
    }

    if let Err(e) = generate_plotters_charts(&[], &[], &traces, output_prefix) {
        eprintln!("차트 생성 중 오류 발생: {}", e);
    }

    // 4. 요약 정보 출력
    log!("\n===== UFSCustom File Processing Complete! =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );

    log!("Total UFSCustom events analyzed: {}", traces.len());
    log!("Generated files:");
    log!("- UFSCustom Plotly charts: {}_ufscustom_*.html", output_prefix);
    log!("- UFSCustom Matplotlib charts: {}_ufscustom_*.png", output_prefix);
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
    fn generate_charts(&self, output_prefix: &str) -> Result<(), String> {
        match self {
            TraceData::UFS(traces) => generate_charts(traces, &[], &[], output_prefix),
            TraceData::Block(traces) => generate_charts(&[], traces, &[], output_prefix),
            TraceData::UFSCUSTOM(traces) => output::charts::generate_ufscustom_charts(traces, output_prefix),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    fn generate_plotters_charts(&self, output_prefix: &str) -> Result<(), String> {
        match self {
            TraceData::UFS(traces) => generate_plotters_charts(traces, &[], &[], output_prefix),
            TraceData::Block(traces) => generate_plotters_charts(&[], traces, &[], output_prefix),
            TraceData::UFSCUSTOM(traces) => generate_plotters_charts(&[], &[], traces, output_prefix),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
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
                log!("- UFSCustom Plotly charts: {}_ufscustom_*.html", output_prefix);
                log!("- UFSCustom Matplotlib charts: {}_ufscustom_*.png", output_prefix);
            }
            // 새 트레이스 타입 추가 시 여기에 추가
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
        }
        // 새 트레이스 타입 추가 시 여기에 추가
    }
}
