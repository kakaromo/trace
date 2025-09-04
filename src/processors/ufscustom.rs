use crate::models::ufscustom::UFSCUSTOM;
use rayon::prelude::*;
use std::time::Instant;

const MILLISECONDS: u32 = 1000;

/// UFSCUSTOM 데이터에 대한 Queue Depth 및 Latency 후처리
pub fn ufscustom_bottom_half_latency_process(mut ufscustom_list: Vec<UFSCUSTOM>) -> Vec<UFSCUSTOM> {
    if ufscustom_list.is_empty() {
        return ufscustom_list;
    }

    let start_time = Instant::now();
    println!("Starting UFSCUSTOM bottom half latency processing for {} items", ufscustom_list.len());

    // 시간순으로 정렬 (start_time 기준)
    ufscustom_list.par_sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

    // 이벤트 기반 QD 계산을 위한 구조체
    #[derive(Debug, Clone)]
    struct Event {
        time: f64,
        event_type: EventType,
        request_idx: usize,
    }

    #[derive(Debug, Clone)]
    enum EventType {
        Start,
        Complete,
    }

    // 모든 요청에 대한 이벤트 생성
    let mut events = Vec::new();
    for (idx, ufscustom) in ufscustom_list.iter().enumerate() {
        events.push(Event {
            time: ufscustom.start_time,
            event_type: EventType::Start,
            request_idx: idx,
        });
        events.push(Event {
            time: ufscustom.end_time,
            event_type: EventType::Complete,
            request_idx: idx,
        });
    }

    // 시간순으로 이벤트 정렬
    events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    // 이벤트 처리하여 각 요청의 start_qd, end_qd 계산
    let mut current_qd = 0u32;
    let mut qd_values = vec![(0u32, 0u32); ufscustom_list.len()]; // (start_qd, end_qd)

    for event in events {
        match event.event_type {
            EventType::Start => {
                current_qd += 1;
                qd_values[event.request_idx].0 = current_qd; // start_qd 설정 (1부터 시작)
            }
            EventType::Complete => {
                current_qd = current_qd.saturating_sub(1);
                qd_values[event.request_idx].1 = current_qd; // end_qd 설정
            }
        }
    }

    // QD 값들을 실제 구조체에 설정
    for (idx, ufscustom) in ufscustom_list.iter_mut().enumerate() {
        ufscustom.start_qd = qd_values[idx].0;
        ufscustom.end_qd = qd_values[idx].1;
    }

    // CTOC, CTOD, continuous 계산
    let mut prev_request: Option<(u64, u32, String)> = None;
    let mut last_complete_time: Option<f64> = None;
    let mut last_qd_zero_complete_time: Option<f64> = None; // QD가 0이 될 때의 완료 시간
    
    let batch_size = 10000;
    let total_items = ufscustom_list.len();
    
    for (i, ufscustom) in ufscustom_list.iter_mut().enumerate() {
        // 배치별 진행률 출력
        if i % batch_size == 0 {
            println!("Processing UFSCUSTOM item {}/{}", i + 1, total_items);
        }

        // continuous 요청 판단
        if let Some((prev_lba, prev_size, prev_opcode)) = &prev_request {
            // let expected_lba = prev_lba + (*prev_size as u64 / 512); // 섹터 단위로 변환
            ufscustom.continuous = ufscustom.lba == *prev_lba + *prev_size as u64
                && ufscustom.opcode == *prev_opcode;
        } else {
            ufscustom.continuous = false;
        }

        // CTOC 계산 (Complete to Complete) - 이전 완료에서 현재 완료까지
        if let Some(prev_complete) = last_complete_time {
            let time_diff = ufscustom.end_time - prev_complete;
            ufscustom.ctoc = if time_diff >= 0.0 { time_diff * MILLISECONDS as f64 } else { 0.0 };
        } else {
            ufscustom.ctoc = 0.0; // 첫 번째 요청
        }

        // CTOD 계산 (Complete to Dispatch)
        // start_qd가 0인 경우: 이전 QD=0 완료에서 현재 시작까지
        // start_qd가 0이 아닌 경우: 이전 완료에서 현재 시작까지
        if ufscustom.start_qd == 0 {
            if let Some(prev_qd_zero_complete) = last_qd_zero_complete_time {
                let time_diff = ufscustom.start_time - prev_qd_zero_complete;
                ufscustom.ctod = if time_diff >= 0.0 { time_diff * MILLISECONDS as f64 } else { 0.0 };
            } else {
                ufscustom.ctod = 0.0; // 첫 번째 idle 시작 요청
            }
        } else if let Some(prev_complete) = last_complete_time {
            let time_diff = ufscustom.start_time - prev_complete;
            ufscustom.ctod = if time_diff >= 0.0 { time_diff * MILLISECONDS as f64 } else { 0.0 };
        } else {
            ufscustom.ctod = 0.0; // 첫 번째 요청
        }

        // 완료 시간 업데이트
        last_complete_time = Some(ufscustom.end_time);
        
        // QD가 0이 되는 완료 시간 업데이트
        if ufscustom.end_qd == 0 {
            last_qd_zero_complete_time = Some(ufscustom.end_time);
        }

        // 현재 요청 정보 저장
        prev_request = Some((ufscustom.lba, ufscustom.size, ufscustom.opcode.clone()));
    }

    // 메모리 최적화
    ufscustom_list.shrink_to_fit();
    drop(prev_request);

    let elapsed = start_time.elapsed();
    println!("UFSCUSTOM bottom half processing completed in {:.2} seconds", elapsed.as_secs_f64());

    ufscustom_list
}
