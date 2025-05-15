use crate::log;
use crate::models::Block;
use crate::utils::constants::MILLISECONDS;
use std::collections::{HashMap, HashSet};

// Block latency post-processing function
pub fn block_bottom_half_latency_process(block_list: Vec<Block>) -> Vec<Block> {
    // Return an empty vector if there are no events
    if block_list.is_empty() {
        return block_list;
    }

    // Record start time
    let start_time = std::time::Instant::now();
    log!(
        "Starting Block Latency processing (event count: {})",
        block_list.len()
    );

    // 1. Sort by timestamp
    log!("  Sorting Block data by timestamp...");
    let mut sorted_blocks = block_list;
    sorted_blocks.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 미리 IO 타입 문자열 생성 (매번 생성하지 않고 재사용)
    let read_str = String::from("read");
    let write_str = String::from("write");
    let discard_str = String::from("discard");
    let other_str = String::from("other");

    // 2. Remove duplicate block_rq_issue (pre-processing)
    log!("  Filtering duplicate events...");
    // 더 크고 효율적인 초기 용량 설정 (1/3)
    let estimated_capacity = sorted_blocks.len() / 3;
    let mut processed_issues = HashSet::with_capacity(estimated_capacity);
    let mut deduplicated_blocks = Vec::with_capacity(sorted_blocks.len());

    // Progress counter - 보고 간격 조정 (5%)
    let total_blocks = sorted_blocks.len();
    let report_interval = (total_blocks / 20).max(1000); // 5% 간격으로 보고 (최소 1000건마다)
    let mut last_reported = 0;

    // 배치 처리 도입 - 시스템 메모리에 따라 조정 가능
    let batch_size = 10000; // 한 번에 처리할 항목 수

    for batch_start in (0..sorted_blocks.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(sorted_blocks.len());
        let batch = &sorted_blocks[batch_start..batch_end];

        for (local_idx, block) in batch.iter().enumerate() {
            let idx = batch_start + local_idx;

            // Report progress (5% intervals)
            if idx >= last_reported + report_interval {
                let progress = (idx * 100) / total_blocks;
                log!(
                    "  Duplicate removal progress: {}% ({}/{})",
                    progress,
                    idx,
                    total_blocks
                );
                last_reported = idx;
            }

            if block.action == "block_rq_issue" {
                // 문자열 참조 사용하여 불필요한 할당 제거
                let io_operation = if block.io_type.starts_with('R') {
                    &read_str
                } else if block.io_type.starts_with('W') {
                    &write_str
                } else if block.io_type.starts_with('D') {
                    &discard_str
                } else {
                    &other_str
                };

                // Extend the key to (sector, io_operation, size)
                let key = (block.sector, io_operation.clone(), block.size);

                if processed_issues.contains(&key) {
                    continue;
                }

                processed_issues.insert(key);
                deduplicated_blocks.push(block.clone());
            } else if block.action == "block_rq_complete" {
                // Remove from duplicate check list for complete
                let io_operation = if block.io_type.starts_with('R') {
                    &read_str
                } else if block.io_type.starts_with('W') {
                    &write_str
                } else if block.io_type.starts_with('D') {
                    &discard_str
                } else {
                    &other_str
                };

                // If write and size is 0, Flush is marked twice (remove duplicates) FF->WS can occur
                if block.io_type.starts_with('W') && block.size == 0 {
                    continue;
                }

                let key = (block.sector, io_operation.clone(), block.size);
                // 키가 존재하는 경우에만 제거 시도
                if processed_issues.contains(&key) {
                    processed_issues.remove(&key);
                }
                deduplicated_blocks.push(block.clone());
            } else {
                deduplicated_blocks.push(block.clone());
            }
        }

        // 주기적으로 메모리 최적화 - 더 자주 수행
        if batch_end % (batch_size * 3) == 0 {
            processed_issues.shrink_to_fit();
        }
    }

    log!(
        "  Number of events after duplicate removal: {}",
        deduplicated_blocks.len()
    );

    // Adjust capacity for memory optimization
    processed_issues.clear();
    processed_issues.shrink_to_fit();

    // 3. Post-process the deduplicated data
    // (Continuity, latency, etc.)
    log!("  Calculating Block latency and continuity...");
    let mut filtered_blocks = Vec::with_capacity(deduplicated_blocks.len());
    // 더 큰 초기 용량으로 해시맵 생성 (1/3)
    let mut req_times: HashMap<(u64, String), f64> = HashMap::with_capacity(estimated_capacity);
    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut prev_end_sector: Option<u64> = None;
    let mut prev_io_type: Option<String> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // Progress counter - 보고 간격 조정 (5%)
    let total_dedup = deduplicated_blocks.len();
    let report_interval_2 = (total_dedup / 20).max(1000);
    let mut last_reported_2 = 0;

    // 배치 처리로 변경
    for batch_start in (0..deduplicated_blocks.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(deduplicated_blocks.len());

        // 임시 버퍼 사용으로 메모리 할당 감소
        let mut temp_blocks = Vec::with_capacity(batch_end - batch_start);

        // 인덱스 기반 반복문을 iterator + enumerate()로 변경
        for (local_idx, block_orig) in deduplicated_blocks[batch_start..batch_end]
            .iter()
            .enumerate()
        {
            let idx = batch_start + local_idx; // 전체 인덱스 계산
            let mut block = block_orig.clone();

            // Report progress (5% intervals)
            if idx >= last_reported_2 + report_interval_2 {
                let progress = (idx * 100) / total_dedup;
                log!(
                    "  Latency calculation progress: {}% ({}/{})",
                    progress,
                    idx,
                    total_dedup
                );
                last_reported_2 = idx;
            }

            // Set continuous to false by default
            block.continuous = false;

            // 문자열 참조 사용하여 불필요한 할당 제거
            let io_operation = if block.io_type.starts_with('R') {
                &read_str
            } else if block.io_type.starts_with('W') {
                &write_str
            } else if block.io_type.starts_with('D') {
                &discard_str
            } else {
                &other_str
            };

            let key = (block.sector, io_operation.clone());

            match block.action.as_str() {
                "block_rq_issue" => {
                    // Check continuity
                    if io_operation != &other_str {
                        if let (Some(end_sector), Some(prev_type)) =
                            (prev_end_sector, prev_io_type.as_ref())
                        {
                            if block.sector == end_sector && io_operation == prev_type {
                                block.continuous = true;
                            }
                        }

                        // Update the end sector and io_type of the current request
                        prev_end_sector = Some(block.sector + block.size as u64);
                        prev_io_type = Some(io_operation.clone());
                    }

                    // Record request time and update QD
                    req_times.insert(key, block.time);
                    current_qd += 1;

                    // ctod is calculated in block_rq_issue(Device) - from the last complete to the current device
                    if let Some(t) = last_complete_qd0_time {
                        block.ctod = (block.time - t) * MILLISECONDS as f64;
                    }

                    if current_qd == 1 {
                        first_c = true;
                        first_complete_time = block.time;
                    }
                }
                "block_rq_complete" => {
                    // complete is always continuous = false
                    if let Some(first_issue_time) = req_times.remove(&key) {
                        block.dtoc = (block.time - first_issue_time) * MILLISECONDS as f64;
                    }

                    match first_c {
                        true => {
                            block.ctoc = (block.time - first_complete_time) * MILLISECONDS as f64;
                            first_c = false;
                        }
                        false => {
                            if let Some(t) = last_complete_time {
                                block.ctoc = (block.time - t) * MILLISECONDS as f64;
                            }
                        }
                    }

                    current_qd = current_qd.saturating_sub(1);
                    if current_qd == 0 {
                        last_complete_qd0_time = Some(block.time);
                    }
                    last_complete_time = Some(block.time);
                }
                _ => {}
            }

            block.qd = current_qd;
            temp_blocks.push(block);
        }

        // 임시 버퍼에서 결과 버퍼로 이동
        filtered_blocks.append(&mut temp_blocks);

        // 주기적으로 메모리 최적화 - 더 자주 수행
        if batch_end % (batch_size * 3) == 0 {
            req_times.shrink_to_fit();
        }
    }

    // 메모리 사용량 최적화 - 마지막 정리
    req_times.clear();
    req_times.shrink_to_fit();
    filtered_blocks.shrink_to_fit();

    // 중간 데이터 정리
    deduplicated_blocks.clear();
    deduplicated_blocks.shrink_to_fit();

    // 강제 GC 트리거
    drop(sorted_blocks);
    drop(processed_issues);

    let elapsed = start_time.elapsed();
    log!(
        "Block Latency processing completed: {:.2} seconds",
        elapsed.as_secs_f64()
    );

    filtered_blocks
}
