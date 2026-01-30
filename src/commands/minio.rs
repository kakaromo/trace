use crate::output::{generate_charts, generate_charts_with_config, save_to_csv, save_to_parquet};
use crate::parsers::parse_log_file_high_perf;
use crate::processors;
use crate::storage::minio_client::{
    download_log_from_minio, download_parquet_from_minio, upload_parquet_to_minio, MinioConfig,
};
use crate::utils::filter::FilterOptions;
use crate::TraceType;
use crate::{print_block_statistics, print_ufs_statistics, print_ufscustom_statistics};
use crate::{read_block_from_parquet, read_ufs_from_parquet, read_ufscustom_from_parquet};
use std::collections::HashMap;
use std::fs;
use std::time::Instant;

/// MinIO에서 로그를 읽어서 Parquet로 변환하고 MinIO에 저장 (통계/차트 생성 안함)
pub fn handle_minio_log_to_parquet(
    remote_log_path: &str,
    remote_output_path: &str,
    chunk_size: usize,
    filter_options: Option<FilterOptions>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n===== Starting MinIO Log to Parquet =====\n");
    let total_start = Instant::now();

    // MinIO 설정 로드
    let minio_config = MinioConfig::from_env().map_err(|e| {
        format!("Failed to load MinIO configuration. Please set environment variables.\nError: {e}")
    })?;

    println!("[1/4] Downloading log file from MinIO...");
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    // 원본 파일명 유지 (압축 형식 감지를 위해)
    let file_name = remote_log_path.split('/').next_back().unwrap_or("log_file");
    let temp_log_file = format!("{}/{}", home_dir, file_name);
    let actual_log_path = download_log_from_minio(&minio_config, remote_log_path, &temp_log_file)?;
    println!("Log file downloaded: {remote_log_path}");

    println!("\n[2/4] Parsing log file...");
    let parse_start = Instant::now();
    let (ufs_traces, block_traces, ufscustom_traces) = parse_log_file_high_perf(&actual_log_path)?;

    // 트레이스 타입 자동 감지
    let detected_trace_type = if !ufs_traces.is_empty() {
        TraceType::UFS
    } else if !block_traces.is_empty() {
        TraceType::Block
    } else if !ufscustom_traces.is_empty() {
        TraceType::UFSCUSTOM
    } else {
        TraceType::UFS // 기본값
    };

    println!(
        "Parsing completed in {:.2}s (Type: {:?})",
        parse_start.elapsed().as_secs_f64(),
        detected_trace_type
    );
    println!(
        "  UFS: {}, Block: {}, UFSCUSTOM: {}",
        ufs_traces.len(),
        block_traces.len(),
        ufscustom_traces.len()
    );

    // 필터 적용
    let (ufs_traces, block_traces, ufscustom_traces) = if let Some(ref filter) = filter_options {
        println!("\n[Applying Filters]...");
        use crate::utils::filter::{filter_ufs_data, filter_block_data, filter_ufscustom_data};
        let filtered_ufs = filter_ufs_data(ufs_traces, filter);
        let filtered_block = filter_block_data(block_traces, filter);
        let filtered_ufscustom = filter_ufscustom_data(ufscustom_traces, filter);
        println!(
            "  After filtering - UFS: {}, Block: {}, UFSCUSTOM: {}",
            filtered_ufs.len(),
            filtered_block.len(),
            filtered_ufscustom.len()
        );
        (filtered_ufs, filtered_block, filtered_ufscustom)
    } else {
        (ufs_traces, block_traces, ufscustom_traces)
    };

    println!("\n[3/4] Processing bottom-half latencies...");
    let process_start = Instant::now();

    let ufs_traces_processed = if !ufs_traces.is_empty() {
        processors::ufs_bottom_half_latency_process(ufs_traces)
    } else {
        ufs_traces
    };

    let block_traces_processed = if !block_traces.is_empty() {
        processors::block_bottom_half_latency_process(block_traces)
    } else {
        block_traces
    };

    println!(
        "Processing completed in {:.2}s",
        process_start.elapsed().as_secs_f64()
    );

    println!("\n[4/4] Saving to Parquet and uploading to MinIO...");
    let temp_output_prefix = "/tmp/trace_temp_output";
    save_to_parquet(
        &ufs_traces_processed,
        &block_traces_processed,
        &ufscustom_traces,
        temp_output_prefix,
        chunk_size,
    )?;

    // Parquet 파일들을 MinIO에 업로드 (간단한 파일명 사용)
    let parquet_files = vec![
        (format!("{temp_output_prefix}_ufs.parquet"), "ufs.parquet"),
        (
            format!("{temp_output_prefix}_block.parquet"),
            "block.parquet",
        ),
        (
            format!("{temp_output_prefix}_ufscustom.parquet"),
            "ufscustom.parquet",
        ),
    ];

    for (local_parquet, remote_filename) in &parquet_files {
        if std::path::Path::new(local_parquet).exists() {
            let remote_parquet = format!(
                "{}/{}",
                remote_output_path.trim_end_matches('/'),
                remote_filename
            );

            upload_parquet_to_minio(&minio_config, local_parquet, &remote_parquet)?;

            // 로컬 임시 파일 삭제
            if let Err(e) = fs::remove_file(local_parquet) {
                eprintln!(
                    "Warning: failed to remove local temporary parquet file '{}': {}",
                    local_parquet, e
                );
            }
        }
    }

    // 로컬 임시 파일 및 압축 해제 디렉토리 정리
    if let Err(e) = fs::remove_file(&temp_log_file) {
        eprintln!(
            "Warning: failed to remove local temporary log file '{}': {}",
            temp_log_file, e
        );
    }
    if let Err(e) = fs::remove_file(&actual_log_path) {
        eprintln!(
            "Warning: failed to remove extracted log file '{}': {}",
            actual_log_path, e
        );
    }
    // 압축 해제 디렉토리도 삭제 시도
    if actual_log_path != temp_log_file {
        if let Some(parent) = std::path::Path::new(&actual_log_path).parent() {
            let _ = fs::remove_dir_all(parent);
        }
    }

    println!(
        "\n===== MinIO Log to Parquet Complete! =====\nTotal time: {:.2}s",
        total_start.elapsed().as_secs_f64()
    );

    Ok(())
}

/// MinIO에서 Parquet를 다운로드하여 분석하고 차트 생성
pub fn handle_minio_parquet_analysis(
    remote_parquet_path: &str,
    local_output_prefix: &str,
    y_axis_ranges: Option<HashMap<String, (f64, f64)>>,
    filter_options: Option<FilterOptions>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n===== Starting MinIO Parquet Analysis =====\n");
    let total_start = Instant::now();

    // MinIO 설정 로드
    let minio_config = MinioConfig::from_env().map_err(|e| {
        format!("Failed to load MinIO configuration. Please set environment variables.\nError: {e}")
    })?;

    // Parquet 타입 감지 (파일명에서)
    let trace_type = if remote_parquet_path.contains("ufs.parquet") {
        "ufs"
    } else if remote_parquet_path.contains("block.parquet") {
        "block"
    } else if remote_parquet_path.contains("ufscustom.parquet") {
        "ufscustom"
    } else {
        return Err("Cannot detect trace type from file name. Use 'ufs.parquet', 'block.parquet', or 'ufscustom.parquet' in the file name.".into());
    };

    println!("[1/3] Downloading Parquet file from MinIO...");
    let temp_parquet_file = format!("/tmp/trace_temp_{trace_type}.parquet");
    download_parquet_from_minio(&minio_config, remote_parquet_path, &temp_parquet_file)?;
    println!("Parquet file downloaded: {remote_parquet_path}");

    println!("\n[2/3] Loading Parquet data...");
    let load_start = Instant::now();

    match trace_type {
        "ufs" => {
            let mut ufs_traces = read_ufs_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read UFS parquet: {}", e))?;
            println!(
                "UFS Parquet loaded: {} events (Time: {:.2}s)",
                ufs_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_ufs_data;
                ufs_traces = filter_ufs_data(ufs_traces, filter);
                println!("  After filtering - UFS: {} events", ufs_traces.len());
            }

            println!("\n[3/3] Generating statistics and charts...");
            let stats_start = Instant::now();
            print_ufs_statistics(&ufs_traces);
            println!(
                "Statistics complete (Time: {:.2}s)",
                stats_start.elapsed().as_secs_f64()
            );

            let chart_start = Instant::now();
            if let Some(ranges) = y_axis_ranges {
                generate_charts_with_config(
                    &ufs_traces,
                    &[],
                    &[],
                    local_output_prefix,
                    Some(&ranges),
                )?;
            } else {
                generate_charts(&ufs_traces, &[], &[], local_output_prefix)?;
            }
            println!(
                "Charts generated (Time: {:.2}s)",
                chart_start.elapsed().as_secs_f64()
            );
        }
        "block" => {
            let mut block_traces = read_block_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read Block parquet: {}", e))?;
            println!(
                "Block Parquet loaded: {} events (Time: {:.2}s)",
                block_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_block_data;
                block_traces = filter_block_data(block_traces, filter);
                println!("  After filtering - Block: {} events", block_traces.len());
            }

            println!("\n[3/3] Generating statistics and charts...");
            let stats_start = Instant::now();
            print_block_statistics(&block_traces);
            println!(
                "Statistics complete (Time: {:.2}s)",
                stats_start.elapsed().as_secs_f64()
            );

            let chart_start = Instant::now();
            if let Some(ranges) = y_axis_ranges {
                generate_charts_with_config(
                    &[],
                    &block_traces,
                    &[],
                    local_output_prefix,
                    Some(&ranges),
                )?;
            } else {
                generate_charts(&[], &block_traces, &[], local_output_prefix)?;
            }
            println!(
                "Charts generated (Time: {:.2}s)",
                chart_start.elapsed().as_secs_f64()
            );
        }
        "ufscustom" => {
            let mut ufscustom_traces = read_ufscustom_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read UFSCUSTOM parquet: {}", e))?;
            println!(
                "UFSCUSTOM Parquet loaded: {} events (Time: {:.2}s)",
                ufscustom_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_ufscustom_data;
                ufscustom_traces = filter_ufscustom_data(ufscustom_traces, filter);
                println!("  After filtering - UFSCUSTOM: {} events", ufscustom_traces.len());
            }

            println!("\n[3/3] Generating statistics and charts...");
            let stats_start = Instant::now();
            print_ufscustom_statistics(&ufscustom_traces);
            println!(
                "Statistics complete (Time: {:.2}s)",
                stats_start.elapsed().as_secs_f64()
            );

            let chart_start = Instant::now();
            if let Some(ranges) = y_axis_ranges {
                generate_charts_with_config(
                    &[],
                    &[],
                    &ufscustom_traces,
                    local_output_prefix,
                    Some(&ranges),
                )?;
            } else {
                generate_charts(&[], &[], &ufscustom_traces, local_output_prefix)?;
            }
            println!(
                "Charts generated (Time: {:.2}s)",
                chart_start.elapsed().as_secs_f64()
            );
        }
        _ => unreachable!(),
    }

    // 로컬 임시 파일 삭제
    if let Err(e) = fs::remove_file(&temp_parquet_file) {
        eprintln!(
            "Warning: failed to remove local temporary parquet file '{}': {}",
            temp_parquet_file, e
        );
    }

    println!(
        "\n===== MinIO Parquet Analysis Complete! =====\nTotal time: {:.2}s",
        total_start.elapsed().as_secs_f64()
    );

    Ok(())
}

/// MinIO에서 Parquet 파일을 다운로드하여 CSV로 변환하고 MinIO에 업로드
pub fn handle_minio_parquet_to_csv(
    remote_parquet_path: &str,
    remote_csv_path: &str,
    filter_options: Option<FilterOptions>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n===== Starting MinIO Parquet to CSV =====\n");
    let total_start = Instant::now();

    // MinIO 설정 로드
    let minio_config = MinioConfig::from_env().map_err(|e| {
        format!("Failed to load MinIO configuration. Please set environment variables.\nError: {e}")
    })?;

    // Parquet 타입 감지 (파일명에서)
    let trace_type = if remote_parquet_path.contains("ufs.parquet") {
        "ufs"
    } else if remote_parquet_path.contains("block.parquet") {
        "block"
    } else if remote_parquet_path.contains("ufscustom.parquet") {
        "ufscustom"
    } else {
        return Err("Cannot detect trace type from file name. Use 'ufs.parquet', 'block.parquet', or 'ufscustom.parquet' in the file name.".into());
    };

    println!("Detected trace type: {}", trace_type);

    println!("[1/3] Downloading Parquet file from MinIO...");
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let temp_parquet_file = format!("{}/trace_temp.parquet", home_dir);

    // MinIO에서 Parquet 다운로드
    download_parquet_from_minio(&minio_config, remote_parquet_path, &temp_parquet_file)?;
    println!("Download completed");

    println!("\n[2/3] Converting Parquet to CSV...");
    let load_start = Instant::now();

    // CSV 파일을 저장할 임시 prefix (trace_type을 포함하여 ufs_*.csv, block_*.csv 형태로 생성)
    let temp_csv_prefix = format!("{}/{}", home_dir, trace_type);

    match trace_type {
        "ufs" => {
            let mut ufs_traces = read_ufs_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read UFS parquet: {e}"))?;

            println!(
                "Loaded {} UFS records (Time: {:.2}s)",
                ufs_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_ufs_data;
                ufs_traces = filter_ufs_data(ufs_traces, filter);
                println!("  After filtering - UFS: {} records", ufs_traces.len());
            }

            // CSV 저장
            save_to_csv(&ufs_traces, &[], &[], &temp_csv_prefix)?;
            println!("CSV conversion completed");
        }
        "block" => {
            let mut block_traces = read_block_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read Block parquet: {e}"))?;

            println!(
                "Loaded {} Block records (Time: {:.2}s)",
                block_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_block_data;
                block_traces = filter_block_data(block_traces, filter);
                println!("  After filtering - Block: {} records", block_traces.len());
            }

            // CSV 저장
            save_to_csv(&[], &block_traces, &[], &temp_csv_prefix)?;
            println!("CSV conversion completed");
        }
        "ufscustom" => {
            let mut ufscustom_traces = read_ufscustom_from_parquet(&temp_parquet_file)
                .map_err(|e| format!("Failed to read UFSCUSTOM parquet: {e}"))?;

            println!(
                "Loaded {} UFSCUSTOM records (Time: {:.2}s)",
                ufscustom_traces.len(),
                load_start.elapsed().as_secs_f64()
            );

            // 필터 적용
            if let Some(ref filter) = filter_options {
                println!("\n[Applying Filters]...");
                use crate::utils::filter::filter_ufscustom_data;
                ufscustom_traces = filter_ufscustom_data(ufscustom_traces, filter);
                println!("  After filtering - UFSCUSTOM: {} records", ufscustom_traces.len());
            }

            // CSV 저장
            save_to_csv(&[], &[], &ufscustom_traces, &temp_csv_prefix)?;
            println!("CSV conversion completed");
        }
        _ => {
            return Err(format!("Unsupported trace type: {trace_type}").into());
        }
    }

    println!("\n[3/3] Uploading CSV files to MinIO...");
    let upload_start = Instant::now();

    // 생성된 CSV 파일들 찾기 (type_*.csv 패턴으로 검색)
    let csv_pattern = format!("{}/{}_*.csv", home_dir, trace_type);
    let mut uploaded_count = 0;

    // glob 패턴으로 파일 찾기
    for entry in
        glob::glob(&csv_pattern).map_err(|e| format!("Failed to parse glob pattern: {e}"))?
    {
        match entry {
            Ok(local_csv_path) => {
                let filename = local_csv_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("Invalid filename")?;

                // MinIO 경로 생성 (파일명 그대로 사용)
                let remote_csv = format!("{}/{}", remote_csv_path.trim_end_matches('/'), filename);

                // MinIO에 업로드
                let client = crate::storage::minio_client::MinioClient::new(&minio_config)?;
                client.upload_file(local_csv_path.to_str().unwrap(), &remote_csv)?;

                println!("  Uploaded: {}", remote_csv);
                uploaded_count += 1;

                // 로컬 임시 파일 삭제
                if let Err(e) = fs::remove_file(&local_csv_path) {
                    eprintln!(
                        "Warning: failed to remove local CSV file '{}': {}",
                        local_csv_path.display(),
                        e
                    );
                }
            }
            Err(e) => eprintln!("Error processing file: {}", e),
        }
    }

    println!(
        "Upload completed: {} CSV files (Time: {:.2}s)",
        uploaded_count,
        upload_start.elapsed().as_secs_f64()
    );

    // 임시 Parquet 파일 삭제
    if let Err(e) = fs::remove_file(&temp_parquet_file) {
        eprintln!(
            "Warning: failed to remove local temporary parquet file '{}': {}",
            temp_parquet_file, e
        );
    }

    println!(
        "\n===== MinIO Parquet to CSV Complete! =====\nTotal time: {:.2}s",
        total_start.elapsed().as_secs_f64()
    );

    Ok(())
}
