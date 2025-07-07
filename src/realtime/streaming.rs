use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncSeekExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::models::{Block, UFS, UFSCUSTOM};
use crate::parsers::log_common;

pub struct StreamingProcessor {
    pub block_traces: Arc<Mutex<Vec<Block>>>,
    pub ufs_traces: Arc<Mutex<Vec<UFS>>>,
    pub ufscustom_traces: Arc<Mutex<Vec<UFSCUSTOM>>>,
    pub parsed_lines: Arc<Mutex<u64>>,
    pub last_processed_time: Arc<Mutex<Instant>>,
    pub output_prefix: Option<String>,
    pub initial_load_completed: Arc<Mutex<bool>>,
}

impl StreamingProcessor {
    pub fn new(output_prefix: Option<&str>) -> Self {
        Self {
            block_traces: Arc::new(Mutex::new(Vec::new())),
            ufs_traces: Arc::new(Mutex::new(Vec::new())),
            ufscustom_traces: Arc::new(Mutex::new(Vec::new())),
            parsed_lines: Arc::new(Mutex::new(0)),
            last_processed_time: Arc::new(Mutex::new(Instant::now())),
            output_prefix: output_prefix.map(|s| s.to_string()),
            initial_load_completed: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn start_streaming(&self, log_file: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::channel::<String>(10000);
        
        // 로그 파일 모니터링 태스크
        let log_file_path = log_file.to_string();
        let tx_clone = tx.clone();
        let initial_load_completed = Arc::clone(&self.initial_load_completed);
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_log_file(&log_file_path, tx_clone, initial_load_completed).await {
                eprintln!("로그 파일 모니터링 오류: {}", e);
            }
        });

        // 로그 파싱 태스크
        let processor = self.clone();
        tokio::spawn(async move {
            let mut batch = Vec::new();
            const BATCH_SIZE: usize = 1000;
            
            while let Some(line) = rx.recv().await {
                batch.push(line);
                
                if batch.len() >= BATCH_SIZE {
                    processor.process_batch(&batch).await;
                    batch.clear();
                }
            }
            
            // 남은 배치 처리
            if !batch.is_empty() {
                processor.process_batch(&batch).await;
            }
        });

        // 주기적 후처리 태스크 (1초마다 체크)
        let processor_clone = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                processor_clone.check_and_process().await;
            }
        });

        // 통계 태스크 (10초마다)
        let processor_clone = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                processor_clone.print_statistics().await;
            }
        });

        // 메인 루프
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    async fn monitor_log_file(
        log_file: &str,
        tx: mpsc::Sender<String>,
        initial_load_completed: Arc<Mutex<bool>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = tokio::fs::File::open(log_file).await?;
        let mut last_pos;
        
        // 처음 실행 시 전체 파일을 읽음
        let is_initial_load = !*initial_load_completed.lock().unwrap();
        
        if is_initial_load {
            println!("🔄 초기 로그 파일 전체 읽기 시작: {}", log_file);
            let start_time = Instant::now();
            
            let mut reader = BufReader::new(&mut file);
            let mut line = String::new();
            let mut line_count = 0u64;
            
            while reader.read_line(&mut line).await? > 0 {
                if (tx.send(line.trim().to_string()).await).is_err() {
                    break; // 수신자가 종료됨
                }
                line.clear();
                line_count += 1;
                
                // 진행 상황 출력 (100,000 라인마다)
                if line_count % 100_000 == 0 {
                    println!("📖 초기 로딩 진행: {} 라인 처리됨", line_count);
                }
            }
            
            last_pos = file.stream_position().await?;
            *initial_load_completed.lock().unwrap() = true;
            
            let elapsed = start_time.elapsed();
            println!("✅ 초기 로그 파일 읽기 완료: {} 라인, 소요시간: {:?}", 
                     line_count, elapsed);
        } else {
            // 파일 끝으로 이동
            let metadata = file.metadata().await?;
            last_pos = metadata.len();
            file.seek(tokio::io::SeekFrom::Start(last_pos)).await?;
        }
        
        // 이후 1초마다 새로 추가된 로그만 읽음
        let mut interval = interval(Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // 파일 크기 확인
            let metadata = file.metadata().await?;
            let current_size = metadata.len();
            
            if current_size > last_pos {
                println!("📨 새로운 로그 감지: {} 바이트 추가됨", current_size - last_pos);
                
                // 새로운 데이터가 있음
                file.seek(tokio::io::SeekFrom::Start(last_pos)).await?;
                let mut reader = BufReader::new(&mut file);
                let mut line = String::new();
                let mut new_line_count = 0;
                
                while reader.read_line(&mut line).await? > 0 {
                    if (tx.send(line.trim().to_string()).await).is_err() {
                        break; // 수신자가 종료됨
                    }
                    line.clear();
                    new_line_count += 1;
                }
                
                last_pos = file.stream_position().await?;
                if new_line_count > 0 {
                    println!("📋 새로운 {} 라인 처리됨", new_line_count);
                }
            }
        }
    }

    async fn process_batch(&self, lines: &[String]) {
        let mut ufs_batch = Vec::new();
        let mut block_batch = Vec::new();
        let mut ufscustom_batch = Vec::new();
        
        for line in lines {
            if let Some((ufs, block, ufscustom)) = log_common::process_line(line) {
                if let Some(ufs) = ufs {
                    ufs_batch.push(ufs);
                }
                if let Some(block) = block {
                    block_batch.push(block);
                }
                if let Some(ufscustom) = ufscustom {
                    ufscustom_batch.push(ufscustom);
                }
            }
        }
        
        // 배치 처리로 락 시간 최소화
        if !ufs_batch.is_empty() {
            let mut traces = self.ufs_traces.lock().unwrap();
            traces.extend(ufs_batch);
        }
        
        if !block_batch.is_empty() {
            let mut traces = self.block_traces.lock().unwrap();
            traces.extend(block_batch);
        }
        
        if !ufscustom_batch.is_empty() {
            let mut traces = self.ufscustom_traces.lock().unwrap();
            traces.extend(ufscustom_batch);
        }
        
        // 파싱된 라인 카운트 증가
        {
            let mut parsed_lines = self.parsed_lines.lock().unwrap();
            *parsed_lines += lines.len() as u64;
        }
        
        // 배치 처리 후 메모리 관리 (용량이 많이 증가했을 때)
        let total_traces = {
            let block_count = self.block_traces.lock().unwrap().len();
            let ufs_count = self.ufs_traces.lock().unwrap().len();
            let ufscustom_count = self.ufscustom_traces.lock().unwrap().len();
            block_count + ufs_count + ufscustom_count
        };
        
        if total_traces > 25_000 {  // 임계치를 넘으면 즉시 메모리 정리
            self.manage_memory().await;
        }
    }

    #[allow(dead_code)]
    async fn process_line(&self, line: &str) {
        // 실제 로그 파싱 구현
        if let Some((ufs, block, ufscustom)) = log_common::process_line(line) {
            // UFS 트레이스 처리
            if let Some(ufs) = ufs {
                let mut traces = self.ufs_traces.lock().unwrap();
                traces.push(ufs);
            }
            
            // Block 트레이스 처리
            if let Some(block) = block {
                let mut traces = self.block_traces.lock().unwrap();
                traces.push(block);
            }
            
            // UFSCUSTOM 트레이스 처리
            if let Some(ufscustom) = ufscustom {
                let mut traces = self.ufscustom_traces.lock().unwrap();
                traces.push(ufscustom);
            }
        }
        
        // 파싱된 라인 카운트 증가
        let mut parsed_lines = self.parsed_lines.lock().unwrap();
        *parsed_lines += 1;
    }

    async fn check_and_process(&self) {
        let parsed_lines = *self.parsed_lines.lock().unwrap();
        
        // 새로운 로그가 있을 때만 처리
        if parsed_lines > 0 {
            let now = Instant::now();
            let should_process = {
                let last_processed_time = self.last_processed_time.lock().unwrap();
                parsed_lines >= 100_000 || now.duration_since(*last_processed_time) >= Duration::from_secs(5)
            };

            if should_process {
                println!("🔄 누적된 {} 라인 처리 중...", parsed_lines);
                self.process_accumulated_data().await;
                *self.last_processed_time.lock().unwrap() = now;
                *self.parsed_lines.lock().unwrap() = 0;
            }
        }
    }

    async fn process_accumulated_data(&self) {
        // 현재 데이터 수량 확인
        let block_count = self.block_traces.lock().unwrap().len();
        let ufs_count = self.ufs_traces.lock().unwrap().len();
        let ufscustom_count = self.ufscustom_traces.lock().unwrap().len();
        
        // 새로운 데이터가 있을 때만 Parquet 저장
        if block_count > 0 || ufs_count > 0 || ufscustom_count > 0 {
            if let Some(ref prefix) = self.output_prefix {
                if let Err(e) = self.save_to_parquet(prefix).await {
                    eprintln!("Parquet 저장 오류: {}", e);
                }
            }
        }

        // 메모리 사용량 관리
        self.manage_memory().await;
    }

    async fn manage_memory(&self) {
        const MAX_TRACES: usize = 20_000;  // 최대 개수를 줄임
        const TARGET_TRACES: usize = 15_000;  // 정리 후 유지할 개수

        // Block 트레이스 메모리 관리
        {
            let mut block_traces = self.block_traces.lock().unwrap();
            if block_traces.len() > MAX_TRACES {
                let remove_count = block_traces.len() - TARGET_TRACES;
                block_traces.drain(0..remove_count);
                println!("🧹 Block 트레이스 메모리 정리: {} 개 제거, 현재 {} 개", 
                         remove_count, block_traces.len());
            }
        }

        // UFS 트레이스 메모리 관리
        {
            let mut ufs_traces = self.ufs_traces.lock().unwrap();
            if ufs_traces.len() > MAX_TRACES {
                let remove_count = ufs_traces.len() - TARGET_TRACES;
                ufs_traces.drain(0..remove_count);
                println!("🧹 UFS 트레이스 메모리 정리: {} 개 제거, 현재 {} 개", 
                         remove_count, ufs_traces.len());
            }
        }

        // UFSCUSTOM 트레이스 메모리 관리
        {
            let mut ufscustom_traces = self.ufscustom_traces.lock().unwrap();
            if ufscustom_traces.len() > MAX_TRACES {
                let remove_count = ufscustom_traces.len() - TARGET_TRACES;
                ufscustom_traces.drain(0..remove_count);
                println!("🧹 UFSCUSTOM 트레이스 메모리 정리: {} 개 제거, 현재 {} 개", 
                         remove_count, ufscustom_traces.len());
            }
        }
    }

    async fn print_statistics(&self) {
        let block_count = self.block_traces.lock().unwrap().len();
        let ufs_count = self.ufs_traces.lock().unwrap().len();
        let ufscustom_count = self.ufscustom_traces.lock().unwrap().len();
        let parsed_lines = *self.parsed_lines.lock().unwrap();
        let is_initial_completed = *self.initial_load_completed.lock().unwrap();
        
        let status = if is_initial_completed { 
            if parsed_lines > 0 {
                "실시간 모니터링 (처리 대기 중)"
            } else {
                "실시간 모니터링 (대기 중)"
            }
        } else { 
            "초기 로딩 중" 
        };
        
        if parsed_lines > 0 || !is_initial_completed {
            println!("📊 [{status}] 현재 통계 - Block: {}, UFS: {}, UFSCUSTOM: {}, 파싱 대기: {}", 
                     block_count, ufs_count, ufscustom_count, parsed_lines);
        } else {
            println!("💤 [{}] 새로운 로그 대기 중... (Block: {}, UFS: {}, UFSCUSTOM: {})", 
                     status, block_count, ufs_count, ufscustom_count);
        }
    }

    // 웹 대시보드용 데이터 접근 메서드
    pub fn get_current_data(&self) -> (Vec<Block>, Vec<UFS>, Vec<UFSCUSTOM>) {
        let block_traces = self.block_traces.lock().unwrap().clone();
        let ufs_traces = self.ufs_traces.lock().unwrap().clone();
        let ufscustom_traces = self.ufscustom_traces.lock().unwrap().clone();
        
        (block_traces, ufs_traces, ufscustom_traces)
    }

    pub fn get_parsed_lines(&self) -> u64 {
        *self.parsed_lines.lock().unwrap()
    }

    async fn save_to_parquet(&self, prefix: &str) -> Result<(), String> {
        // 현재 데이터 가져오기
        let (block_traces, ufs_traces, ufscustom_traces) = self.get_current_data();
        
        // 데이터가 있는 경우만 저장
        if !block_traces.is_empty() || !ufs_traces.is_empty() || !ufscustom_traces.is_empty() {
            // async context에서 blocking 작업 실행
            let prefix = prefix.to_string();
            let block_traces = block_traces.clone();
            let ufs_traces = ufs_traces.clone();
            let ufscustom_traces = ufscustom_traces.clone();
            
            tokio::task::spawn_blocking(move || {
                crate::output::parquet::append_to_parquet(
                    &ufs_traces,
                    &block_traces,
                    &ufscustom_traces,
                    &prefix,
                    10000, // chunk_size
                ).map_err(|e| e.to_string())
            }).await.map_err(|e| e.to_string())??;
        }
        
        Ok(())
    }
}

impl Clone for StreamingProcessor {
    fn clone(&self) -> Self {
        Self {
            block_traces: Arc::clone(&self.block_traces),
            ufs_traces: Arc::clone(&self.ufs_traces),
            ufscustom_traces: Arc::clone(&self.ufscustom_traces),
            parsed_lines: Arc::clone(&self.parsed_lines),
            last_processed_time: Arc::clone(&self.last_processed_time),
            output_prefix: self.output_prefix.clone(),
            initial_load_completed: Arc::clone(&self.initial_load_completed),
        }
    }
}
