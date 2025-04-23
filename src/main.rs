use std::env;
use std::io;
use std::time::Instant;
use trace::utils::Logger;
use trace::TraceType;
use trace::*;

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {} <log_file> <output_prefix>                      - Parse log file and generate statistics", program);
    eprintln!("  {} --parquet <type> <parquet_file> <output_prefix> - Read Parquet file and generate statistics", program);
    eprintln!("    where <type> is one of: 'ufs', 'block'");
    // 새 트레이스 타입이 추가되면 여기에 업데이트
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

    // 명령줄 인수 처리 - 로그 파싱 모드와 Parquet 분석 모드 구분
    let result = if args.len() == 3 {
        // 기존 로그 파싱 모드
        process_log_file(&args[1], &args[2])
    } else if args.len() == 5 && args[1] == "--parquet" {
        // Parquet 분석 모드 (단일 파일)
        let trace_type = match args[2].parse::<TraceType>() {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Error: {}", e);
                eprintln!("Supported types: 'ufs', 'block'"); // 새 타입 추가 시 업데이트
                print_usage(&args[0]);
                return Ok(());
            }
        };
        process_single_parquet_file(trace_type, &args[3], &args[4])
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

    let total_start_time = Instant::now();
    log!("===== Starting Log File Processing =====");

    // Parse log file
    log!("\n[1/6] Parsing log file...");
    let parse_start = Instant::now();

    // 로그 파일 파싱 - 향후 더 많은 트레이스 타입 지원
    let parse_result = match parse_log_file(log_file_path) {
        Ok(result) => result,
        Err(e) => {
            log_error!("File parsing error: {}", e);
            (Vec::new(), Vec::new()) // Return empty vectors on error
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

    if has_ufs {
        trace_counts.push(format!("UFS={}", parse_result.0.len()));
    }
    if has_block {
        trace_counts.push(format!("Block={}", parse_result.1.len()));
    }
    // 새 트레이스 타입이 추가되면 여기에 조건 추가

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
    // 새 트레이스 타입이 추가되면 여기에 조건 추가

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
    // 새 트레이스 타입이 추가되면 여기에 조건 추가

    log!(
        "\nAnalysis complete: Time taken: {:.2}s",
        analysis_start.elapsed().as_secs_f64()
    );

    // Save to Parquet files
    log!("\n[4/6] Saving to Parquet files...");
    let save_start = Instant::now();

    match save_to_parquet(&processed_ufs, &processed_blocks, output_prefix) {
        Ok(()) => {
            let mut saved_files = Vec::new();
            if has_ufs {
                saved_files.push(format!("{}_ufs.parquet", output_prefix));
            }
            if has_block {
                saved_files.push(format!("{}_block.parquet", output_prefix));
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

    match generate_charts(&processed_ufs, &processed_blocks, output_prefix) {
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

    // 새 트레이스 타입이 추가되면 여기에 조건 추가

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

// TraceData 열거형 정의 - 각 트레이스 타입에 대한 데이터를 담습니다
#[allow(clippy::upper_case_acronyms)]
enum TraceData {
    UFS(Vec<UFS>),
    Block(Vec<Block>),
    // 새 트레이스 타입 추가 시 여기에 추가
}

impl TraceData {
    // 데이터 개수 반환
    fn count(&self) -> usize {
        match self {
            TraceData::UFS(traces) => traces.len(),
            TraceData::Block(traces) => traces.len(),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // 통계 출력
    fn print_statistics(&self) {
        match self {
            TraceData::UFS(traces) => print_ufs_statistics(traces),
            TraceData::Block(traces) => print_block_statistics(traces),
            // 새 트레이스 타입 추가 시 여기에 추가
        }
    }

    // 차트 생성
    fn generate_charts(&self, output_prefix: &str) -> Result<(), String> {
        match self {
            TraceData::UFS(traces) => generate_charts(traces, &[], output_prefix),
            TraceData::Block(traces) => generate_charts(&[], traces, output_prefix),
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
        // 새 트레이스 타입 추가 시 여기에 추가
    }
}
