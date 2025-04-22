use crate::models::UFS;
use crate::utils::constants::MILLISECONDS;
use std::collections::HashMap;

pub fn ufs_bottom_half_latency_process(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    // 이벤트가 없으면 빈 벡터 반환
    if ufs_list.is_empty() {
        return ufs_list;
    }

    // 시작 시간 기록
    let start_time = std::time::Instant::now();
    println!(
        "Starting UFS Latency processing (event count: {})",
        ufs_list.len()
    );

    // time 기준으로 오름차순 정렬
    println!("  Sorting UFS data by time...");
    ufs_list.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 메모리 효율성을 위한 용량 최적화
    let estimated_capacity = ufs_list.len() / 10;
    let mut req_times: HashMap<(u32, String), f64> = HashMap::with_capacity(estimated_capacity);

    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // 이전 send_req의 정보를 저장할 변수들
    let mut prev_send_req: Option<(u64, u32, String)> = None; // (lba, size, opcode)

    // 프로그레스 카운터
    let total_events = ufs_list.len();
    let report_interval = (total_events / 10).max(1); // 10% 간격으로 진행 상황 보고
    let mut last_reported = 0;

    println!("  Calculating UFS Latency and continuity...");

    for (idx, ufs) in ufs_list.iter_mut().enumerate() {
        // 진행 상황 보고 (10% 간격)
        if idx >= last_reported + report_interval {
            let progress = (idx * 100) / total_events;
            println!(
                "  UFS processing progress: {}% ({}/{})",
                progress, idx, total_events
            );
            last_reported = idx;
        }

        match ufs.action.as_str() {
            "send_req" => {
                // 연속성 체크: 이전 send_req가 있는 경우
                if let Some((prev_lba, prev_size, prev_opcode)) = prev_send_req {
                    let prev_end_addr = prev_lba + prev_size as u64;
                    // 현재 요청의 시작 주소가 이전 요청의 끝 주소와 같고, opcode가 같은 경우
                    ufs.continuous = ufs.lba == prev_end_addr && ufs.opcode == prev_opcode;
                } else {
                    ufs.continuous = false;
                }

                // 현재 send_req 정보 저장
                prev_send_req = Some((ufs.lba, ufs.size, ufs.opcode.clone()));

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
                if let Some(send_time) = req_times.remove(&(ufs.tag, ufs.opcode.clone())) {
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

    // 메모리 최적화를 위해 벡터 크기 조정
    ufs_list.shrink_to_fit();

    let elapsed = start_time.elapsed();
    println!(
        "UFS Latency processing completed: {:.2} seconds",
        elapsed.as_secs_f64()
    );

    ufs_list
}
