use crate::log;
use crate::models::UFS;
use crate::utils::constants::MILLISECONDS;
use std::collections::{HashMap, HashSet};
use rayon::prelude::*;

pub fn ufs_bottom_half_latency_process(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    // 이벤트가 없으면 빈 벡터 반환
    if ufs_list.is_empty() {
        return ufs_list;
    }

    // 시작 시간 기록
    let start_time = std::time::Instant::now();
    log!(
        "Starting UFS Latency processing (event count: {})",
        ufs_list.len()
    );

    // time 기준으로 오름차순 정렬 - 병렬 정렬 사용
    log!("  Sorting UFS data by time...");
    ufs_list.par_sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 메모리 효율성을 위한 용량 최적화 - 더 큰 초기 용량 설정
    // 더 보수적인 예상을 위해 전체 크기의 1/3을 사용
    let estimated_capacity = ufs_list.len() / 3;
    let mut req_times: HashMap<(u32, String), f64> = HashMap::with_capacity(estimated_capacity);

    // 자주 사용되는 opcode 문자열을 저장하는 캐시 생성
    let mut opcode_cache: HashMap<String, String> = HashMap::with_capacity(32);

    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // 이전 send_req의 정보를 저장할 변수들
    let mut prev_send_req: Option<(u64, u32, String)> = None; // (lba, size, opcode)

    // send_req와 complete_rsp의 쌍을 추적하기 위한 집합
    let mut send_req_indices: HashSet<usize> = HashSet::new();
    let mut complete_rsp_indices: HashSet<usize> = HashSet::new();
    let mut paired_tags: HashMap<(u32, String), usize> = HashMap::new(); // (tag, opcode) -> send_req index
    
    log!("  Calculating UFS Latency and continuity...");

    // 첫 번째 패스: send_req와 complete_rsp의 쌍을 찾기 - 병렬 처리
    log!("  Finding send_req and complete_rsp pairs...");
    
    // 병렬로 send_req와 complete_rsp 인덱스 수집
    let (send_reqs, complete_rsps): (Vec<_>, Vec<_>) = ufs_list
        .par_iter()
        .enumerate()
        .filter_map(|(idx, ufs)| {
            match ufs.action.as_str() {
                "send_req" => Some((0, idx, ufs.tag, &ufs.opcode)),
                "complete_rsp" => Some((1, idx, ufs.tag, &ufs.opcode)),
                _ => None,
            }
        })
        .partition(|(action_type, _, _, _)| *action_type == 0);
    
    // send_req 맵 생성
    for (_, idx, tag, opcode) in send_reqs {
        let key = (tag, opcode.clone());
        paired_tags.insert(key, idx);
        send_req_indices.insert(idx);
    }
    
    // complete_rsp 처리 및 경고 출력
    for (_, idx, tag, opcode) in complete_rsps {
        let key = (tag, opcode.clone());
        if paired_tags.contains_key(&key) {
            complete_rsp_indices.insert(idx);
        } else {
            log!("  Warning: Found complete_rsp without matching send_req (tag: {}, opcode: {})", tag, opcode);
        }
    }

    // 쌍이 없는 send_req 찾기 - 병렬 처리
    let unpaired_send_req: Vec<usize> = paired_tags
        .par_iter()
        .filter_map(|(key, send_idx)| {
            let has_complete = complete_rsp_indices
                .par_iter()
                .any(|&idx| {
                    let ufs = &ufs_list[idx];
                    ufs.action == "complete_rsp" && ufs.tag == key.0 && ufs.opcode == key.1
                });
            
            if !has_complete {
                log!("  Warning: Found send_req without matching complete_rsp (tag: {}, opcode: {})", key.0, key.1);
                Some(*send_idx)
            } else {
                None
            }
        })
        .collect();

    // 유효한 인덱스만 유지 (쌍이 있는 것들만)
    let valid_indices: HashSet<usize> = send_req_indices.union(&complete_rsp_indices)
        .filter(|&&idx| !unpaired_send_req.contains(&idx))
        .copied()
        .collect();

    // 쌍이 없는 이벤트들 제거 - retain 사용으로 메모리 효율성 향상
    let original_len = ufs_list.len();
    ufs_list = ufs_list
        .into_iter()
        .enumerate()
        .filter_map(|(idx, ufs)| {
            if valid_indices.contains(&idx) || (ufs.action != "send_req" && ufs.action != "complete_rsp") {
                Some(ufs)
            } else {
                None
            }
        })
        .collect();
    
    let removed_count = original_len - ufs_list.len();
    if removed_count > 0 {
        log!("  Removed {} unpaired events", removed_count);
    }

    // 다시 정렬 (필터링 후) - 병렬 정렬
    ufs_list.par_sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 해시맵 다시 초기화
    req_times.clear();
    paired_tags.clear();
    send_req_indices.clear();
    complete_rsp_indices.clear();

    // 배치 처리를 더 큰 단위로 변경하여 오버헤드 감소
    let batch_size = 50000; // 더 큰 배치 크기로 변경
    
    // 프로그레스 카운터 업데이트 (필터링 후 새로운 크기 기준)
    let total_events = ufs_list.len();
    let report_interval = (total_events / 10).max(5000); // 10% 간격으로 변경
    let mut last_reported = 0;

    for batch_start in (0..ufs_list.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(ufs_list.len());

        // 인덱스 기반 반복문을 iterator + enumerate()로 변경
        for (idx, ufs) in ufs_list[batch_start..batch_end].iter_mut().enumerate() {
            let idx = batch_start + idx; // 전체 인덱스 계산

            // 진행 상황 보고 (10% 간격)
            if idx >= last_reported + report_interval {
                let progress = (idx * 100) / total_events;
                log!(
                    "  UFS processing progress: {}% ({}/{})",
                    progress,
                    idx,
                    total_events
                );
                last_reported = idx;
            }

            match ufs.action.as_str() {
                "send_req" => {
                    // 연속성 체크: 이전 send_req가 있는 경우
                    if let Some((prev_lba, prev_size, ref prev_opcode)) = prev_send_req {
                        let prev_end_addr = prev_lba + prev_size as u64;
                        // 현재 요청의 시작 주소가 이전 요청의 끝 주소와 같고, opcode가 같은 경우
                        ufs.continuous = ufs.lba == prev_end_addr && ufs.opcode == *prev_opcode;
                    } else {
                        ufs.continuous = false;
                    }

                    // 현재 send_req 정보 저장
                    prev_send_req = Some((ufs.lba, ufs.size, ufs.opcode.clone()));

                    // 해시맵에 삽입 (이제 쌍이 있는 것들만 처리되므로 안전)
                    req_times.insert((ufs.tag, ufs.opcode.clone()), ufs.time);

                    current_qd += 1;
                    if current_qd == 1 {
                        if let Some(t) = last_complete_qd0_time {
                            ufs.ctod = (ufs.time - t) * MILLISECONDS as f64;
                        }
                        first_c = true;
                        first_complete_time = ufs.time;
                    }
                }
                "complete_rsp" => {
                    // complete_rsp는 continuous 체크하지 않음
                    ufs.continuous = false;

                    current_qd = current_qd.saturating_sub(1);

                    // 성능 개선: 필요한 경우에만 복제
                    let key = (ufs.tag, ufs.opcode.clone());
                    if let Some(send_time) = req_times.remove(&key) {
                        ufs.dtoc = (ufs.time - send_time) * MILLISECONDS as f64;
                    }

                    match first_c {
                        true => {
                            ufs.ctoc = (ufs.time - first_complete_time) * MILLISECONDS as f64;
                            first_c = false;
                        }
                        false => {
                            if let Some(t) = last_complete_time {
                                ufs.ctoc = (ufs.time - t) * MILLISECONDS as f64;
                            }
                        }
                    }
                    if current_qd == 0 {
                        last_complete_qd0_time = Some(ufs.time);
                    }
                    last_complete_time = Some(ufs.time);
                }
                _ => {
                    ufs.continuous = false;
                }
            }
            ufs.qd = current_qd;
        }

        // 메모리 사용량 최적화를 위해 더 자주 해시맵 정리 (3배 간격으로)
        if batch_end % (batch_size * 3) == 0 {
            req_times.shrink_to_fit();
        }
    }

    // 모든 처리가 끝난 후 메모리 최적화
    req_times.clear(); // 먼저 내용 비우기
    req_times.shrink_to_fit();
    opcode_cache.clear();
    opcode_cache.shrink_to_fit();
    ufs_list.shrink_to_fit();

    // 불필요한 참조 명시적 해제
    drop(prev_send_req);
    drop(opcode_cache);
    drop(req_times);
    drop(send_req_indices);
    drop(complete_rsp_indices);
    drop(paired_tags);

    let elapsed = start_time.elapsed();
    log!(
        "UFS Latency processing completed: {:.2} seconds",
        elapsed.as_secs_f64()
    );

    ufs_list
}
