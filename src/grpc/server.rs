use std::collections::HashMap;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{transport::Server, Request, Response, Status};
use uuid::Uuid;

use crate::parsers::parse_log_file_high_perf;
use crate::output::save_to_parquet;
use crate::storage::minio_client::{MinioAsyncClient, MinioConfig};
use crate::utils::compression::{extract_and_find_log, CompressionFormat};
use crate::TraceType;

pub mod log_processor {
    tonic::include_proto!("log_processor");
}

use log_processor::log_processor_server::{LogProcessor, LogProcessorServer};
use log_processor::{
    JobStatusRequest, JobStatusResponse, ListFilesRequest, ListFilesResponse,
    ProcessLogsProgress, ProcessLogsRequest, ProgressStage,
};

type JobMap = Arc<Mutex<HashMap<String, JobStatus>>>;

#[derive(Clone, Debug)]
struct JobStatus {
    job_id: String,
    stage: i32,
    message: String,
    progress_percent: i32,
    records_processed: i64,
    is_completed: bool,
    success: Option<bool>,
    error: Option<String>,
}

pub struct LogProcessorService {
    minio_config: MinioConfig,
    jobs: JobMap,
}

impl LogProcessorService {
    pub fn new(minio_config: MinioConfig) -> Self {
        Self {
            minio_config,
            jobs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn update_job_status(&self, job_id: &str, status: JobStatus) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.insert(job_id.to_string(), status);
    }

    async fn process_logs_internal(
        &self,
        job_id: String,
        request: ProcessLogsRequest,
        tx: mpsc::Sender<Result<ProcessLogsProgress, Status>>,
    ) {
        let result: Result<(), String> = async {
            // 1단계: MinIO에서 로그 파일 다운로드
            println!("[PROGRESS] Sending download start message (10%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageDownloading as i32,
                    message: format!("Downloading log file from {}/{}", request.source_bucket, request.source_path),
                    progress_percent: 10,
                    records_processed: 0,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send download start message - client disconnected");
                return Err("Client disconnected during download start".to_string());
            }
            println!("[PROGRESS] Download start message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // 스트림 플러시

            // 원본 파일 이름에서 확장자 추출
            let source_filename = request.source_path.split('/').next_back().unwrap_or("log");
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            let temp_log_path = format!("{}/trace_temp_log_{}_{}", home_dir, job_id, source_filename);

            // MinIO 클라이언트로 다운로드
            let source_config = MinioConfig {
                endpoint: self.minio_config.endpoint.clone(),
                access_key: self.minio_config.access_key.clone(),
                secret_key: self.minio_config.secret_key.clone(),
                bucket: request.source_bucket.clone(),
                region: self.minio_config.region.clone(),
            };

            let client = MinioAsyncClient::new(&source_config).map_err(|e| e.to_string())?;
            client.download_file(&request.source_path, &temp_log_path).await.map_err(|e| {
                let error_msg = format!("Failed to download file: {}", e);
                eprintln!("Download error: {}", error_msg);
                error_msg
            })?;

            println!("[PROGRESS] Sending download completed message (20%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageDownloading as i32,
                    message: "Download completed".to_string(),
                    progress_percent: 20,
                    records_processed: 0,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send download completed message - client disconnected");
                return Err("Client disconnected after download".to_string());
            }
            println!("[PROGRESS] Download completed message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            self.update_job_status(
                &job_id,
                JobStatus {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageDownloading as i32,
                    message: "Download completed".to_string(),
                    progress_percent: 20,
                    records_processed: 0,
                    is_completed: false,
                    success: None,
                    error: None,
                },
            );

            // 압축 파일 처리
            let downloaded_path = Path::new(&temp_log_path);
            let format = CompressionFormat::from_path(downloaded_path);
            
            let actual_log_path = if format != CompressionFormat::None {
                // 압축 파일인 경우 압축 해제
                println!("Compressed file detected: {:?}", format);
                
                println!("[PROGRESS] Sending extraction start message (25%)...");
                if tx
                    .send(Ok(ProcessLogsProgress {
                        job_id: job_id.clone(),
                        stage: ProgressStage::StageDownloading as i32,
                        message: format!("Extracting compressed file ({:?})...", format),
                        progress_percent: 25,
                        records_processed: 0,
                        success: None,
                        error: None,
                        output_files: vec![],
                    }))
                    .await.is_err() {
                    eprintln!("[ERROR] Failed to send extraction message - client disconnected");
                    return Err("Client disconnected during extraction".to_string());
                }
                println!("[PROGRESS] Extraction start message sent successfully");
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

                let extract_dir = format!("{}/trace_extract_{}", home_dir, job_id);
                println!("Starting extraction to: {}", extract_dir);
                let log_file = extract_and_find_log(downloaded_path, Path::new(&extract_dir))
                    .map_err(|e| {
                        let error_msg = format!("Failed to extract compressed file: {}", e);
                        eprintln!("Extraction error: {}", error_msg);
                        error_msg
                    })?;
                
                println!("Extraction completed: {}", log_file.display());
                
                // 압축 해제 완료 메시지 전송
                println!("[PROGRESS] Sending extraction completed message (28%)...");
                if tx
                    .send(Ok(ProcessLogsProgress {
                        job_id: job_id.clone(),
                        stage: ProgressStage::StageDownloading as i32,
                        message: "Extraction completed".to_string(),
                        progress_percent: 28,
                        records_processed: 0,
                        success: None,
                        error: None,
                        output_files: vec![],
                    }))
                    .await.is_err() {
                    eprintln!("[ERROR] Failed to send extraction completed message - client disconnected");
                    return Err("Client disconnected after extraction".to_string());
                }
                println!("[PROGRESS] Extraction completed message sent successfully");
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                
                log_file.to_string_lossy().to_string()
            } else {
                // 압축되지 않은 파일은 그대로 사용
                temp_log_path.clone()
            };

            // 2단계: 로그 파일 파싱
            println!("[PROGRESS] Sending parsing start message (30%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageParsing as i32,
                    message: "Parsing log file".to_string(),
                    progress_percent: 30,
                    records_processed: 0,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send parsing start message - client disconnected");
                return Err("Client disconnected before parsing".to_string());
            }
            println!("[PROGRESS] Parsing start message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // 로그 타입 결정 (Auto는 없으므로 UFS를 기본값으로)
            let _trace_type = match request.log_type.as_str() {
                "ufs" => TraceType::UFS,
                "block" => TraceType::Block,
                "ufscustom" => TraceType::UFSCUSTOM,
                _ => TraceType::UFS,  // 기본값
            };

            // 파싱 실행 (trace_type 파라미터가 없으므로 자동 감지)
            let (ufs_traces, block_traces, ufscustom_traces) =
                parse_log_file_high_perf(&actual_log_path)
                    .map_err(|e| {
                        let error_msg = format!("Failed to parse log file: {}", e);
                        eprintln!("Parse error: {}", error_msg);
                        error_msg
                    })?;

            let total_records = ufs_traces.len() + block_traces.len() + ufscustom_traces.len();

            println!("[PROGRESS] Sending parsing completed message (50%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageParsing as i32,
                    message: format!("Parsing completed: {} records", total_records),
                    progress_percent: 50,
                    records_processed: total_records as i64,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send parsing completed message - client disconnected");
                return Err("Client disconnected after parsing".to_string());
            }
            println!("[PROGRESS] Parsing completed message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            self.update_job_status(
                &job_id,
                JobStatus {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageParsing as i32,
                    message: "Parsing completed".to_string(),
                    progress_percent: 50,
                    records_processed: total_records as i64,
                    is_completed: false,
                    success: None,
                    error: None,
                },
            );

            // 3단계: Parquet으로 변환
            println!("[PROGRESS] Sending conversion start message (60%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageConverting as i32,
                    message: "Converting to Parquet format".to_string(),
                    progress_percent: 60,
                    records_processed: total_records as i64,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send conversion start message - client disconnected");
                return Err("Client disconnected before conversion".to_string());
            }
            println!("[PROGRESS] Conversion start message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Parquet 임시 파일 경로
            let temp_parquet_path = format!("{}/trace_temp_parquet_{}", home_dir, job_id);

            let chunk_size = request.chunk_size.unwrap_or(100000) as usize;
            save_to_parquet(
                &ufs_traces,
                &block_traces,
                &ufscustom_traces,
                &temp_parquet_path,
                chunk_size,
            ).map_err(|e| {
                let error_msg = format!("Failed to save parquet: {}", e);
                eprintln!("Parquet error: {}", error_msg);
                error_msg
            })?;

            println!("[PROGRESS] Sending conversion completed message (70%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageConverting as i32,
                    message: "Conversion completed".to_string(),
                    progress_percent: 70,
                    records_processed: total_records as i64,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send conversion completed message - client disconnected");
                return Err("Client disconnected after conversion".to_string());
            }
            println!("[PROGRESS] Conversion completed message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // 4단계: MinIO에 업로드
            println!("[PROGRESS] Sending upload start message (75%)...");
            if tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageUploading as i32,
                    message: "Uploading Parquet files to MinIO".to_string(),
                    progress_percent: 75,
                    records_processed: total_records as i64,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await.is_err() {
                eprintln!("[ERROR] Failed to send upload start message - client disconnected");
                return Err("Client disconnected before upload".to_string());
            }
            println!("[PROGRESS] Upload start message sent successfully");
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let target_config = MinioConfig {
                endpoint: self.minio_config.endpoint.clone(),
                access_key: self.minio_config.access_key.clone(),
                secret_key: self.minio_config.secret_key.clone(),
                bucket: request.target_bucket.clone(),
                region: self.minio_config.region.clone(),
            };

            let target_client = MinioAsyncClient::new(&target_config).map_err(|e| {
                let error_msg = format!("Failed to create target MinIO client: {}", e);
                eprintln!("MinIO client error: {}", error_msg);
                error_msg
            })?;
            let mut uploaded_files = Vec::new();

            let _ = tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageUploading as i32,
                    message: "Starting upload to MinIO".to_string(),
                    progress_percent: 80,
                    records_processed: total_records as i64,
                    success: None,
                    error: None,
                    output_files: vec![],
                }))
                .await;

            // 생성된 Parquet 파일들을 업로드
            let parquet_types = vec!["ufs", "block", "ufscustom"];
            for ptype in parquet_types {
                let local_file = format!("{}_{}.parquet", temp_parquet_path, ptype);
                if Path::new(&local_file).exists() {
                    let remote_file = format!("{}/{}.parquet", request.target_path.trim_end_matches('/'), ptype);
                    target_client.upload_file(&local_file, &remote_file).await.map_err(|e| {
                        let error_msg = format!("Failed to upload {}: {}", remote_file, e);
                        eprintln!("Upload error: {}", error_msg);
                        error_msg
                    })?;
                    uploaded_files.push(remote_file.clone());
                    
                    // 임시 파일 삭제
                    let _ = std::fs::remove_file(&local_file);
                }
            }

            // 임시 파일 및 디렉토리 정리
            let _ = std::fs::remove_file(&temp_log_path);
            
            // 압축 해제 디렉토리가 있으면 삭제
            let extract_dir = format!("{}/trace_extract_{}", home_dir, job_id);
            if Path::new(&extract_dir).exists() {
                let _ = std::fs::remove_dir_all(&extract_dir);
                println!("Cleaned up extraction directory: {}", extract_dir);
            }

            println!("Sending completion message to client...");
            let send_result = tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageCompleted as i32,
                    message: format!("Processing completed successfully. Uploaded {} files", uploaded_files.len()),
                    progress_percent: 100,
                    records_processed: total_records as i64,
                    success: Some(true),
                    error: None,
                    output_files: uploaded_files.clone(),
                }))
                .await;
            
            if send_result.is_err() {
                eprintln!("Failed to send completion message to client");
            } else {
                println!("Completion message sent successfully");
            }

            self.update_job_status(
                &job_id,
                JobStatus {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageCompleted as i32,
                    message: "Completed".to_string(),
                    progress_percent: 100,
                    records_processed: total_records as i64,
                    is_completed: true,
                    success: Some(true),
                    error: None,
                },
            );

            Ok(())
        }
        .await;
            
        // 에러 처리
        if let Err(e) = result {
            let error_msg = format!("Processing failed: {}", e);
            eprintln!("ERROR: {}", error_msg);
            let _ = tx
                .send(Ok(ProcessLogsProgress {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageFailed as i32,
                    message: error_msg.clone(),
                    progress_percent: 0,
                    records_processed: 0,
                    success: Some(false),
                    error: Some(error_msg.clone()),
                    output_files: vec![],
                }))
                .await;

            self.update_job_status(
                &job_id,
                JobStatus {
                    job_id: job_id.clone(),
                    stage: ProgressStage::StageFailed as i32,
                    message: "Failed".to_string(),
                    progress_percent: 0,
                    records_processed: 0,
                    is_completed: true,
                    success: Some(false),
                    error: Some(error_msg),
                },
            );
        }
    }
}

#[tonic::async_trait]
impl LogProcessor for LogProcessorService {
    type ProcessLogsStream =
        Pin<Box<dyn Stream<Item = Result<ProcessLogsProgress, Status>> + Send>>;

    async fn process_logs(
        &self,
        request: Request<ProcessLogsRequest>,
    ) -> Result<Response<Self::ProcessLogsStream>, Status> {
        let req = request.into_inner();

        // 작업 ID 생성
        let job_id = Uuid::new_v4().to_string();

        // 채널 생성 (버퍼 크기 1로 설정하여 실시간 스트리밍)
        let (tx, rx) = mpsc::channel(1);

        // 초기 상태 저장
        self.update_job_status(
            &job_id,
            JobStatus {
                job_id: job_id.clone(),
                stage: ProgressStage::StageDownloading as i32,
                message: "Starting".to_string(),
                progress_percent: 0,
                records_processed: 0,
                is_completed: false,
                success: None,
                error: None,
            },
        );

        // 백그라운드에서 처리
        let service = self.clone();
        let job_id_clone = job_id.clone();
        tokio::spawn(async move {
            service.process_logs_internal(job_id_clone, req, tx).await;
        });

        // 스트림 반환
        let output_stream = ReceiverStream::new(rx);
        Ok(Response::new(
            Box::pin(output_stream) as Self::ProcessLogsStream
        ))
    }

    async fn get_job_status(
        &self,
        request: Request<JobStatusRequest>,
    ) -> Result<Response<JobStatusResponse>, Status> {
        let req = request.into_inner();
        let jobs = self.jobs.lock().unwrap();

        if let Some(status) = jobs.get(&req.job_id) {
            Ok(Response::new(JobStatusResponse {
                job_id: status.job_id.clone(),
                stage: status.stage,
                message: status.message.clone(),
                progress_percent: status.progress_percent,
                records_processed: status.records_processed,
                is_completed: status.is_completed,
                success: status.success,
                error: status.error.clone(),
            }))
        } else {
            Err(Status::not_found(format!(
                "Job not found: {}",
                req.job_id
            )))
        }
    }

    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();

        let config = MinioConfig {
            endpoint: self.minio_config.endpoint.clone(),
            access_key: self.minio_config.access_key.clone(),
            secret_key: self.minio_config.secret_key.clone(),
            bucket: req.bucket.clone(),
            region: self.minio_config.region.clone(),
        };

        let client = MinioAsyncClient::new(&config).map_err(|e| {
            Status::internal(format!("Failed to create MinIO client: {}", e))
        })?;

        let files = client.list_files(&req.prefix).await.map_err(|e| {
            Status::internal(format!("Failed to list files: {}", e))
        })?;

        Ok(Response::new(ListFilesResponse { files }))
    }
}

impl Clone for LogProcessorService {
    fn clone(&self) -> Self {
        Self {
            minio_config: self.minio_config.clone(),
            jobs: Arc::clone(&self.jobs),
        }
    }
}

pub async fn run_grpc_server(
    addr: String,
    minio_config: MinioConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = LogProcessorService::new(minio_config);

    println!("🚀 gRPC server listening on {}", addr);

    Server::builder()
        .add_service(LogProcessorServer::new(service))
        .serve(addr.parse()?)
        .await?;

    Ok(())
}
