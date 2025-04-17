use std::collections::{HashMap, HashSet};
use crate::models::Block;
use crate::utils::constants::MILLISECONDS;

// Block 레이턴시 후처리 함수
pub fn block_bottom_half_latency_process(block_list: Vec<Block>) -> Vec<Block> {
    // 이벤트가 없으면 빈 벡터 반환
    if block_list.is_empty() {
        return block_list;
    }
    
    // 시작 시간 기록
    let start_time = std::time::Instant::now();
    println!("Block 지연 시간 처리 시작 (이벤트 수: {})", block_list.len());
    
    // 1. 시간순 정렬
    println!("  Block 데이터 시간순 정렬 중...");
    let mut sorted_blocks = block_list;
    sorted_blocks.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    // 2. 중복 block_rq_issue 제거 (사전 작업)
    println!("  중복 이벤트 필터링 중...");
    // 키를 (sector, io_type, size)로 확장하여 동일 크기의 요청만 중복으로 처리
    let mut processed_issues = HashSet::with_capacity(sorted_blocks.len() / 5);
    let mut deduplicated_blocks = Vec::with_capacity(sorted_blocks.len());

    // 프로그레스 카운터 - 중복 제거 단계
    let total_blocks = sorted_blocks.len();
    let report_interval = (total_blocks / 10).max(1); // 10% 간격으로 진행 상황 보고
    let mut last_reported = 0;
    
    for (idx, block) in sorted_blocks.into_iter().enumerate() {
        // 진행 상황 보고 (10% 간격)
        if idx >= last_reported + report_interval {
            let progress = (idx * 100) / total_blocks;
            println!("  중복 제거 진행률: {}% ({}/{})", progress, idx, total_blocks);
            last_reported = idx;
        }
        
        if block.action == "block_rq_issue" {
            let io_operation = if block.io_type.starts_with('R') {
                "read"
            } else if block.io_type.starts_with('W') {
                "write"
            } else if block.io_type.starts_with('D') {
                "discard"
            } else {
                "other"
            };

            // 키를 (sector, io_operation, size)로 확장
            let key = (block.sector, io_operation.to_string(), block.size);

            if processed_issues.contains(&key) {
                continue;
            }

            processed_issues.insert(key);
        } else if block.action == "block_rq_complete" {
            // complete일 경우 중복 체크 목록에서 제거
            let io_operation = if block.io_type.starts_with('R') {
                "read"
            } else if block.io_type.starts_with('W') {
                "write"
            } else if block.io_type.starts_with('D') {
                "discard"
            } else {
                "other"
            };

            // write 이고 size가 0인 경우에 Flush 표시가 2번 발생 (중복 제거) FF->WS 이런식으로 들어올 수 있음
            if block.io_type.starts_with('W') && block.size == 0 {
                continue;
            }

            let key = (block.sector, io_operation.to_string(), block.size);
            processed_issues.remove(&key);
        }

        deduplicated_blocks.push(block);
    }

    println!("  중복 제거 후 이벤트 수: {}", deduplicated_blocks.len());
    
    // 메모리 최적화를 위한 용량 조절
    processed_issues.clear();
    processed_issues.shrink_to_fit();
    
    // 3. 중복이 제거된 데이터에 대해 후처리 진행
    // (연속성, 지연 시간 등 처리)
    println!("  Block 지연 시간 및 연속성 계산 중...");
    let mut filtered_blocks = Vec::with_capacity(deduplicated_blocks.len());
    let mut req_times: HashMap<(u64, String), f64> = HashMap::with_capacity(deduplicated_blocks.len() / 5);
    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut prev_end_sector: Option<u64> = None;
    let mut prev_io_type: Option<String> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // 프로그레스 카운터 - 지연 시간 계산 단계
    let total_dedup = deduplicated_blocks.len();
    let report_interval_2 = (total_dedup / 10).max(1); 
    let mut last_reported_2 = 0;
    
    for (idx, mut block) in deduplicated_blocks.into_iter().enumerate() {
        // 진행 상황 보고 (10% 간격)
        if idx >= last_reported_2 + report_interval_2 {
            let progress = (idx * 100) / total_dedup;
            println!("  지연 시간 계산 진행률: {}% ({}/{})", progress, idx, total_dedup);
            last_reported_2 = idx;
        }
        
        // 기본적으로 continuous를 false로 설정
        block.continuous = false;

        let io_operation = if block.io_type.starts_with('R') {
            "read"
        } else if block.io_type.starts_with('W') {
            "write"
        } else if block.io_type.starts_with('D') {
            "discard"
        } else {
            "other"
        };

        let key = (block.sector, io_operation.to_string());

        match block.action.as_str() {
            "block_rq_issue" => {
                // 연속성 체크
                if io_operation != "other" {
                    if let (Some(end_sector), Some(prev_type)) =
                        (prev_end_sector, prev_io_type.as_ref())
                    {
                        if block.sector == end_sector && io_operation == prev_type {
                            block.continuous = true;
                        }
                    }

                    // 현재 요청의 끝 sector 및 io_type 업데이트
                    prev_end_sector = Some(block.sector + block.size as u64);
                    prev_io_type = Some(io_operation.to_string());
                }

                // 요청 시간 기록 및 QD 업데이트
                req_times.insert(key, block.time);
                current_qd += 1;

                // ctod는 block_rq_issue(Device)에서 계산 - 마지막 complete에서 현재 device까지
                if let Some(t) = last_complete_qd0_time {
                    block.ctod = (block.time - t) * MILLISECONDS as f64;
                }

                if current_qd == 1 {
                    first_c = true;
                    first_complete_time = block.time;
                }
            }
            "block_rq_complete" => {
                // complete는 항상 continuous = false
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
        filtered_blocks.push(block);
    }

    // 메모리 최적화를 위해 벡터 크기 조정
    filtered_blocks.shrink_to_fit();
    
    let elapsed = start_time.elapsed();
    println!("Block 지연 시간 처리 완료: {:.2}초", elapsed.as_secs_f64());
    
    filtered_blocks
}