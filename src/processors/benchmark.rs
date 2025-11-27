use crate::log;
use crate::output::save_to_parquet;
use crate::parsers::{parse_log_file_high_perf, BenchmarkParser, LogLineType};
use crate::utils::{read_to_string_auto, IterationOutputManager};
use crate::TraceType;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufWriter, Write};

/// 벤치마크 로그 파일을 파싱하여 iteration별로 trace를 분류하고 저장
/// 기존 기능(전체 parquet, result.log)도 함께 수행
pub fn parse_benchmark_log(log_file: &str, output_prefix: &str) -> io::Result<()> {
    log!("Starting benchmark log parsing...");
    log!("Log file: {}", log_file);
    log!("Output prefix: {}", output_prefix);

    // 1. 먼저 전체 로그를 기존 방식으로 파싱 (전체 parquet 및 result.log 생성)
    log!("\n=== Phase 1: Parsing entire log file (standard mode) ===");
    let (all_ufs, all_block, all_ufscustom) = parse_log_file_high_perf(log_file)?;

    // 후처리 수행 (QD, DTOC, CTOD 등 계산)
    log!("Processing traces for latency calculations...");
    let processed_all_ufs = if !all_ufs.is_empty() {
        crate::processors::ufs_bottom_half_latency_process(all_ufs)
    } else {
        all_ufs
    };

    let processed_all_block = if !all_block.is_empty() {
        crate::processors::block_bottom_half_latency_process(all_block)
    } else {
        all_block
    };

    let processed_all_ufscustom = if !all_ufscustom.is_empty() {
        crate::processors::ufscustom_bottom_half_latency_process(all_ufscustom)
    } else {
        all_ufscustom
    };

    // 전체 parquet 파일 저장
    let base_parquet_path = format!("{output_prefix}trace_output.parquet");
    if !processed_all_ufs.is_empty()
        || !processed_all_block.is_empty()
        || !processed_all_ufscustom.is_empty()
    {
        log!("Saving combined parquet file: {}", base_parquet_path);
        if let Err(e) = save_to_parquet(
            &processed_all_ufs,
            &processed_all_block,
            &processed_all_ufscustom,
            &base_parquet_path,
            50000,
        ) {
            log!("Warning: Failed to save combined parquet: {}", e);
        }
    }

    // result.log 생성
    let result_log_path = format!("{output_prefix}result.log");
    save_result_log(
        &result_log_path,
        &processed_all_ufs,
        &processed_all_block,
        &processed_all_ufscustom,
    )?;

    // 2. 벤치마크 모드: iteration별 분류 및 저장
    log!("\n=== Phase 2: Parsing for benchmark iterations ===");

    // 로그 파일 읽기
    let log_content = read_to_string_auto(log_file)?;

    // 벤치마크 파서 생성
    let parser = BenchmarkParser::new();

    // Iteration 출력 관리자 생성
    let output_manager = IterationOutputManager::new(output_prefix)?;

    // Iteration별 trace 라인 수집
    let mut iteration_traces: HashMap<usize, IterationTraces> = HashMap::new();
    let mut current_iteration = 0;
    let mut benchmark_results = Vec::new();

    log!("Detecting iterations and collecting traces...");

    for line in log_content.lines() {
        let line_type = parser.detect_line_type(line, &mut current_iteration);

        match line_type {
            LogLineType::FioResult {
                iteration,
                test_type,
                bandwidth,
            } => {
                log!(
                    "Found FIO result - Iteration {}: {} = {} MiB/s",
                    iteration,
                    test_type,
                    bandwidth
                );
                benchmark_results.push((iteration, "FIO".to_string(), test_type, bandwidth));
            }
            LogLineType::TioTestResult {
                iteration,
                test_type,
                bandwidth,
            } => {
                log!(
                    "Found TIOtest result - Iteration {}: {} = {} MB/s",
                    iteration,
                    test_type,
                    bandwidth
                );
                benchmark_results.push((iteration, "TIOtest".to_string(), test_type, bandwidth));
            }
            LogLineType::IOzoneResult {
                iteration,
                test_type,
                bandwidth,
            } => {
                log!(
                    "Found IOzone result - Iteration {}: {} = {} MB/s",
                    iteration,
                    test_type,
                    bandwidth
                );
                benchmark_results.push((iteration, "IOzone".to_string(), test_type, bandwidth));
            }
            LogLineType::UfsTrace => {
                if current_iteration > 0 {
                    iteration_traces
                        .entry(current_iteration)
                        .or_insert_with(IterationTraces::new)
                        .ufs_traces
                        .push(line.to_string());
                }
            }
            LogLineType::BlockTrace => {
                if current_iteration > 0 {
                    iteration_traces
                        .entry(current_iteration)
                        .or_insert_with(IterationTraces::new)
                        .block_traces
                        .push(line.to_string());
                }
            }
            LogLineType::UfsCustomTrace => {
                if current_iteration > 0 {
                    iteration_traces
                        .entry(current_iteration)
                        .or_insert_with(IterationTraces::new)
                        .ufscustom_traces
                        .push(line.to_string());
                }
            }
            LogLineType::Other => {}
        }
    }

    log!("Found {} iterations", iteration_traces.len());

    // 벤치마크 결과를 CSV와 JSON으로 저장
    if !benchmark_results.is_empty() {
        save_benchmark_results_csv(&output_manager, &benchmark_results)?;
        save_benchmark_results_json(&output_manager, &benchmark_results)?;
    }

    // 각 iteration별로 trace 파싱 및 저장
    for (iteration, traces) in iteration_traces.iter() {
        log!("\nProcessing Iteration {}...", iteration);

        // UFS traces 처리
        if !traces.ufs_traces.is_empty() {
            log!("  Parsing {} UFS trace lines...", traces.ufs_traces.len());
            process_trace_type(
                &output_manager,
                *iteration,
                &traces.ufs_traces,
                TraceType::UFS,
                "ufs_trace.parquet",
            )?;
        }

        // Block traces 처리
        if !traces.block_traces.is_empty() {
            log!(
                "  Parsing {} Block trace lines...",
                traces.block_traces.len()
            );
            process_trace_type(
                &output_manager,
                *iteration,
                &traces.block_traces,
                TraceType::Block,
                "block_trace.parquet",
            )?;
        }

        // UFSCustom traces 처리
        if !traces.ufscustom_traces.is_empty() {
            log!(
                "  Parsing {} UFSCustom trace lines...",
                traces.ufscustom_traces.len()
            );
            process_trace_type(
                &output_manager,
                *iteration,
                &traces.ufscustom_traces,
                TraceType::UFSCUSTOM,
                "ufscustom_trace.parquet",
            )?;
        }
    }

    log!("\n=== Summary ===");
    log!("Combined parquet: {}", base_parquet_path);
    log!("Result log: {}", result_log_path);
    log!("Iteration folders: {}", output_manager.base_dir().display());
    log!("Benchmark log parsing completed successfully!");

    Ok(())
}

/// Iteration별 trace 라인 저장
struct IterationTraces {
    ufs_traces: Vec<String>,
    block_traces: Vec<String>,
    ufscustom_traces: Vec<String>,
}

impl IterationTraces {
    fn new() -> Self {
        Self {
            ufs_traces: Vec::new(),
            block_traces: Vec::new(),
            ufscustom_traces: Vec::new(),
        }
    }
}

/// 특정 trace 타입 처리
fn process_trace_type(
    output_manager: &IterationOutputManager,
    iteration: usize,
    trace_lines: &[String],
    trace_type: TraceType,
    filename: &str,
) -> io::Result<()> {
    // 임시 파일에 trace 라인 저장
    let temp_trace_file = format!("/tmp/trace_temp_{iteration}_{filename}.log");
    {
        let file = File::create(&temp_trace_file)?;
        let mut writer = BufWriter::new(file);
        for line in trace_lines {
            writeln!(writer, "{line}")?;
        }
    }

    // 기존 파서로 파싱
    let result = parse_log_file_high_perf(&temp_trace_file);

    // 임시 파일 삭제
    std::fs::remove_file(&temp_trace_file).ok();

    match result {
        Ok((ufs_data, block_data, ufscustom_data)) => {
            // 후처리 수행 (QD, DTOC, CTOD 등 계산)
            let processed_ufs = if !ufs_data.is_empty() {
                crate::processors::ufs_bottom_half_latency_process(ufs_data)
            } else {
                ufs_data
            };

            let processed_block = if !block_data.is_empty() {
                crate::processors::block_bottom_half_latency_process(block_data)
            } else {
                block_data
            };

            let processed_ufscustom = if !ufscustom_data.is_empty() {
                crate::processors::ufscustom_bottom_half_latency_process(ufscustom_data)
            } else {
                ufscustom_data
            };

            // Parquet 파일 저장
            let parquet_path = output_manager.get_file_path(iteration, filename)?;

            match trace_type {
                TraceType::UFS => {
                    if !processed_ufs.is_empty() {
                        if let Err(e) = save_to_parquet(
                            &processed_ufs,
                            &[],
                            &[],
                            parquet_path.to_str().unwrap(),
                            50000,
                        ) {
                            log!("    Warning: Failed to save UFS parquet: {}", e);
                        } else {
                            log!("    Saved UFS trace to: {}", parquet_path.display());
                        }
                    }
                }
                TraceType::Block => {
                    if !processed_block.is_empty() {
                        if let Err(e) = save_to_parquet(
                            &[],
                            &processed_block,
                            &[],
                            parquet_path.to_str().unwrap(),
                            50000,
                        ) {
                            log!("    Warning: Failed to save Block parquet: {}", e);
                        } else {
                            log!("    Saved Block trace to: {}", parquet_path.display());
                        }
                    }
                }
                TraceType::UFSCUSTOM => {
                    if !processed_ufscustom.is_empty() {
                        if let Err(e) = save_to_parquet(
                            &[],
                            &[],
                            &processed_ufscustom,
                            parquet_path.to_str().unwrap(),
                            50000,
                        ) {
                            log!("    Warning: Failed to save UFSCustom parquet: {}", e);
                        } else {
                            log!("    Saved UFSCustom trace to: {}", parquet_path.display());
                        }
                    }
                }
            }

            Ok(())
        }
        Err(e) => {
            log!("    Warning: Failed to parse trace: {}", e);
            Ok(())
        }
    }
}

/// 벤치마크 결과를 CSV로 저장
fn save_benchmark_results_csv(
    output_manager: &IterationOutputManager,
    results: &[(usize, String, String, f64)],
) -> io::Result<()> {
    let csv_path = output_manager.base_dir().join("benchmark_results.csv");
    let file = File::create(&csv_path)?;
    let mut writer = BufWriter::new(file);

    // Iteration과 Tool별로 그룹화
    let mut grouped: HashMap<(usize, String), (Option<f64>, Option<f64>)> = HashMap::new();

    for (iteration, tool, test_type, bandwidth) in results {
        let key = (*iteration, tool.clone());
        let entry = grouped.entry(key).or_insert((None, None));

        if test_type.contains("WRITE") {
            entry.0 = Some(*bandwidth);
        } else if test_type.contains("READ") {
            entry.1 = Some(*bandwidth);
        }
    }

    // CSV 헤더
    writeln!(writer, "Iteration,Tool,Write,Read")?;

    // Iteration과 Tool 순서로 정렬
    let mut sorted_keys: Vec<_> = grouped.keys().collect();
    sorted_keys.sort_by_key(|(iteration, tool)| (*iteration, tool.clone()));

    // 데이터 작성
    for (iteration, tool) in sorted_keys {
        let (write, read) = grouped.get(&(*iteration, tool.clone())).unwrap();
        let write_str = write
            .map(|w| format!("{w:.2}"))
            .unwrap_or_else(|| "".to_string());
        let read_str = read
            .map(|r| format!("{r:.2}"))
            .unwrap_or_else(|| "".to_string());
        writeln!(writer, "{iteration},{tool},{write_str},{read_str}")?;
    }

    log!("Saved benchmark results to: {}", csv_path.display());
    Ok(())
}

/// 벤치마크 결과를 JSON으로 저장
fn save_benchmark_results_json(
    output_manager: &IterationOutputManager,
    results: &[(usize, String, String, f64)],
) -> io::Result<()> {
    let json_path = output_manager.base_dir().join("benchmark_results.json");
    let file = File::create(&json_path)?;
    let mut writer = BufWriter::new(file);

    // 도구별로 그룹화 (read/write 배열 생성)
    let mut tool_data: HashMap<String, (Vec<f64>, Vec<f64>)> = HashMap::new();

    // Iteration 순서대로 정렬된 결과 수집
    let mut sorted_results = results.to_vec();
    sorted_results.sort_by_key(|(iteration, _, _, _)| *iteration);

    // 각 iteration의 read/write 값을 도구별로 수집
    let max_iteration = sorted_results
        .iter()
        .map(|(i, _, _, _)| *i)
        .max()
        .unwrap_or(0);

    for iteration in 1..=max_iteration {
        for (iter, tool, test_type, bandwidth) in &sorted_results {
            if *iter == iteration {
                let entry = tool_data
                    .entry(tool.clone())
                    .or_insert((Vec::new(), Vec::new()));

                if test_type.contains("READ") {
                    entry.0.push(*bandwidth);
                } else if test_type.contains("WRITE") {
                    entry.1.push(*bandwidth);
                }
            }
        }
    }

    // JSON 시작
    writeln!(writer, "{{")?;

    let mut tool_keys: Vec<_> = tool_data.keys().collect();
    tool_keys.sort();

    for (tool_idx, tool) in tool_keys.iter().enumerate() {
        let (read_values, write_values) = tool_data.get(*tool).unwrap();
        writeln!(writer, "  \"{}\": {{", tool.to_lowercase())?;

        // READ 배열
        if !read_values.is_empty() {
            writeln!(writer, "    \"read\": [")?;
            for (idx, value) in read_values.iter().enumerate() {
                if idx > 0 {
                    writeln!(writer, ", ")?;
                }
                write!(writer, "        {value:.2}")?;
            }
            writeln!(writer, "\n    ],")?;
        } else {
            writeln!(writer, "    \"read\": [],")?;
        }

        // WRITE 배열
        if !write_values.is_empty() {
            writeln!(writer, "    \"write\": [")?;
            for (idx, value) in write_values.iter().enumerate() {
                if idx > 0 {
                    writeln!(writer, ", ")?;
                }
                write!(writer, "        {value:.2}")?;
            }
            writeln!(writer, "\n    ]")?;
        } else {
            writeln!(writer, "    \"write\": []")?;
        }

        let comma = if tool_idx < tool_keys.len() - 1 {
            ","
        } else {
            ""
        };
        writeln!(writer, "  }}{comma}")?;
    }

    writeln!(writer, "}}")?;

    log!("Saved benchmark results to: {}", json_path.display());
    Ok(())
}

/// result.log 파일 생성 (기존 기능 유지)
fn save_result_log(
    result_log_path: &str,
    ufs_data: &[crate::models::UFS],
    block_data: &[crate::models::Block],
    ufscustom_data: &[crate::models::UFSCUSTOM],
) -> io::Result<()> {
    let file = File::create(result_log_path)?;
    let mut writer = BufWriter::new(file);

    writeln!(writer, "=== Trace Analysis Summary ===")?;
    writeln!(writer)?;
    writeln!(writer, "UFS Traces: {} items", ufs_data.len())?;
    writeln!(writer, "Block Traces: {} items", block_data.len())?;
    writeln!(writer, "UFSCustom Traces: {} items", ufscustom_data.len())?;
    writeln!(writer)?;

    if !ufs_data.is_empty() {
        writeln!(writer, "=== UFS Trace Statistics ===")?;
        let read_count = ufs_data
            .iter()
            .filter(|u| u.opcode.contains("READ"))
            .count();
        let write_count = ufs_data
            .iter()
            .filter(|u| u.opcode.contains("WRITE"))
            .count();
        let sync_count = ufs_data
            .iter()
            .filter(|u| u.opcode.contains("SYNC"))
            .count();

        writeln!(writer, "READ operations: {read_count}")?;
        writeln!(writer, "WRITE operations: {write_count}")?;
        writeln!(writer, "SYNC operations: {sync_count}")?;

        if let Some(first) = ufs_data.first() {
            if let Some(last) = ufs_data.last() {
                writeln!(
                    writer,
                    "Time range: {:.3} - {:.3} ms",
                    first.time, last.time
                )?;
            }
        }
        writeln!(writer)?;
    }

    if !block_data.is_empty() {
        writeln!(writer, "=== Block Trace Statistics ===")?;
        let read_count = block_data
            .iter()
            .filter(|b| b.io_type.contains("R"))
            .count();
        let write_count = block_data
            .iter()
            .filter(|b| b.io_type.contains("W"))
            .count();

        writeln!(writer, "READ operations: {read_count}")?;
        writeln!(writer, "WRITE operations: {write_count}")?;

        if let Some(first) = block_data.first() {
            if let Some(last) = block_data.last() {
                writeln!(
                    writer,
                    "Time range: {:.3} - {:.3} ms",
                    first.time, last.time
                )?;
            }
        }
        writeln!(writer)?;
    }

    if !ufscustom_data.is_empty() {
        writeln!(writer, "=== UFSCustom Trace Statistics ===")?;
        writeln!(writer, "Total custom traces: {}", ufscustom_data.len())?;

        if let Some(first) = ufscustom_data.first() {
            if let Some(last) = ufscustom_data.last() {
                writeln!(
                    writer,
                    "Time range: {:.3} - {:.3} ms",
                    first.start_time, last.start_time
                )?;
            }
        }
        writeln!(writer)?;
    }

    log!("Saved result log to: {}", result_log_path);
    Ok(())
}
