use std::error::Error;
use tonic::Request;

use super::log_processor::{
    log_processor_client::LogProcessorClient, ConvertToCsvRequest, FilterOptions, ListFilesRequest,
    ProcessLogsRequest,
};

/// 필터 옵션 생성 헬퍼 함수
#[allow(clippy::too_many_arguments)]
pub fn create_filter(
    start_time: f64,
    end_time: f64,
    start_sector: u64,
    end_sector: u64,
    min_dtoc: f64,
    max_dtoc: f64,
    min_ctoc: f64,
    max_ctoc: f64,
    min_ctod: f64,
    max_ctod: f64,
    min_qd: u32,
    max_qd: u32,
    cpu_list: Vec<u32>,
) -> FilterOptions {
    FilterOptions {
        start_time,
        end_time,
        start_sector,
        end_sector,
        min_dtoc,
        max_dtoc,
        min_ctoc,
        max_ctoc,
        min_ctod,
        max_ctod,
        min_qd,
        max_qd,
        cpu_list,
    }
}

/// 로그 처리 요청
#[allow(clippy::too_many_arguments)]
pub async fn process_logs(
    server_addr: &str,
    source_bucket: &str,
    source_path: &str,
    target_bucket: &str,
    target_path: &str,
    log_type: &str,
    chunk_size: Option<i32>,
    filter: Option<FilterOptions>,
) -> Result<(), Box<dyn Error>> {
    let mut client = LogProcessorClient::connect(format!("http://{}", server_addr)).await?;

    let request = Request::new(ProcessLogsRequest {
        source_bucket: source_bucket.to_string(),
        source_path: source_path.to_string(),
        target_bucket: target_bucket.to_string(),
        target_path: target_path.to_string(),
        log_type: log_type.to_string(),
        chunk_size,
        filter: filter.clone(),
    });

    println!("\n===== Processing Logs =====");
    println!("Source: {}/{}", source_bucket, source_path);
    println!("Target: {}/{}", target_bucket, target_path);
    println!("Type: {}", log_type);
    if let Some(cs) = chunk_size {
        println!("Chunk Size: {}", cs);
    }
    if filter.is_some() {
        println!("Filter: Applied");
    }
    println!();

    let mut stream = client.process_logs(request).await?.into_inner();

    while let Some(progress) = stream.message().await? {
        let stage_name = match progress.stage {
            0 => "UNKNOWN",
            1 => "DOWNLOADING",
            2 => "PARSING",
            3 => "CONVERTING",
            4 => "UPLOADING",
            5 => "COMPLETED",
            6 => "FAILED",
            _ => "UNKNOWN",
        };

        print!(
            "[{:12}] {:3}% | {}",
            stage_name, progress.progress_percent, progress.message
        );

        if progress.records_processed > 0 {
            print!(" (Records: {})", progress.records_processed);
        }
        println!();

        if !progress.output_files.is_empty() {
            println!("\nGenerated files:");
            for file in &progress.output_files {
                println!("  - {}", file);
            }
        }

        if let Some(success) = progress.success {
            if success {
                println!("\n✓ Processing completed successfully");
            } else {
                println!("\n✗ Processing failed");
                if let Some(error) = progress.error {
                    println!("Error: {}", error);
                }
            }
        }
    }

    Ok(())
}

/// CSV 변환 요청
#[allow(clippy::too_many_arguments)]
pub async fn convert_to_csv(
    server_addr: &str,
    source_bucket: &str,
    source_parquet_path: &str,
    target_bucket: &str,
    target_csv_path: &str,
    csv_prefix: Option<String>,
    filter: Option<FilterOptions>,
) -> Result<(), Box<dyn Error>> {
    let mut client = LogProcessorClient::connect(format!("http://{}", server_addr)).await?;

    let request = Request::new(ConvertToCsvRequest {
        source_bucket: source_bucket.to_string(),
        source_parquet_path: source_parquet_path.to_string(),
        target_bucket: target_bucket.to_string(),
        target_csv_path: target_csv_path.to_string(),
        csv_prefix,
        filter: filter.clone(),
    });

    println!("\n===== Converting to CSV =====");
    println!("Source: {}/{}", source_bucket, source_parquet_path);
    println!("Target: {}/{}", target_bucket, target_csv_path);
    if filter.is_some() {
        println!("Filter: Applied");
    }
    println!();

    let mut stream = client.convert_to_csv(request).await?.into_inner();

    while let Some(progress) = stream.message().await? {
        let stage_name = match progress.stage {
            0 => "UNKNOWN",
            1 => "DOWNLOADING",
            2 => "CONVERTING",
            3 => "UPLOADING",
            4 => "COMPLETED",
            5 => "FAILED",
            _ => "UNKNOWN",
        };

        print!(
            "[{:12}] {:3}% | {}",
            stage_name, progress.progress_percent, progress.message
        );

        if progress.records_processed > 0 {
            print!(" (Records: {})", progress.records_processed);
        }
        println!();

        if !progress.csv_files.is_empty() {
            println!("\nGenerated CSV files:");
            for file in &progress.csv_files {
                println!("  - {}", file);
            }
        }

        if let Some(success) = progress.success {
            if success {
                println!("\n✓ CSV conversion completed successfully");
            } else {
                println!("\n✗ CSV conversion failed");
                if let Some(error) = progress.error {
                    println!("Error: {}", error);
                }
            }
        }
    }

    Ok(())
}

/// 파일 목록 조회
pub async fn list_files(
    server_addr: &str,
    bucket: &str,
    prefix: &str,
) -> Result<(), Box<dyn Error>> {
    let mut client = LogProcessorClient::connect(format!("http://{}", server_addr)).await?;

    let request = Request::new(ListFilesRequest {
        bucket: bucket.to_string(),
        prefix: prefix.to_string(),
    });

    println!("\n===== Listing Files =====");
    println!("Bucket: {}", bucket);
    println!("Prefix: {}", prefix);
    println!();

    let response = client.list_files(request).await?.into_inner();

    if response.files.is_empty() {
        println!("No files found");
    } else {
        println!("Found {} files:", response.files.len());
        for file in response.files {
            println!("  - {}", file);
        }
    }

    Ok(())
}

/// Parquet 파일 읽기 및 출력
pub async fn read_parquet(
    server_addr: &str,
    source_bucket: &str,
    source_parquet_path: &str,
    max_records: Option<i32>,
    filter: Option<FilterOptions>,
) -> Result<(), Box<dyn Error>> {
    let mut client = LogProcessorClient::connect(format!("http://{}", server_addr)).await?;

    let request = Request::new(super::log_processor::ReadParquetRequest {
        source_bucket: source_bucket.to_string(),
        source_parquet_path: source_parquet_path.to_string(),
        max_records,
        filter: filter.clone(),
    });

    println!("\n===== Reading Parquet =====");
    println!("Source: {}/{}", source_bucket, source_parquet_path);
    if let Some(max) = max_records {
        println!("Max Records: {}", max);
    }
    if filter.is_some() {
        println!("Filter: Applied");
    }
    println!();

    let mut stream = client.read_parquet(request).await?.into_inner();

    let mut record_count = 0;
    while let Some(response) = stream.message().await? {
        record_count += 1;
        println!("{}", response.record_json);
    }

    if record_count == 0 {
        println!("No records found");
    } else {
        println!("✓ Displayed {} records", record_count);
    }

    Ok(())
}
