use std::collections::HashMap;
use crate::models::UFS;
use crate::utils::constants::MILLISECONDS;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub fn ufs_bottom_half_latency_process(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    let ufs_count = ufs_list.len();
    println!("UFS 후처리 시작: {} 이벤트", ufs_count);

    // time 기준으로 오름차순 정렬 (병렬 정렬 사용)
    ufs_list.par_sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    println!("UFS 정렬 완료: {:.2}초", start_time.elapsed().as_secs_f64());

    // 파일 크기에 따라 처리 방식 결정
    if ufs_count > 1_000_000 {
        // 대용량 처리: 청크 분할 후 병렬 처리 후 통합
        chunk_based_processing(ufs_list)
    } else {
        // 소량 처리: 기존 방식
        sequential_processing(ufs_list)
    }
}

// 대용량 데이터를 위한 청크 기반 병렬 처리
fn chunk_based_processing(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    let ufs_count = ufs_list.len();
    
    // 데이터를 그룹화하여 병렬 처리 가능하게 함
    // Tag별로 그룹화하여 독립적으로 처리
    let mut tag_groups: HashMap<u32, Vec<usize>> = HashMap::new();
    
    // 첫 번째 단계: 인덱스와 태그 정보 수집
    for (idx, ufs) in ufs_list.iter().enumerate() {
        tag_groups.entry(ufs.tag).or_default().push(idx);
    }
    
    println!("UFS 태그 그룹화 완료: {} 그룹, {:.2}초", 
             tag_groups.len(), start_time.elapsed().as_secs_f64());

    // 두 번째 단계: 태그 그룹별로 병렬 처리
    let processed_indices = Arc::new(Mutex::new(HashMap::new()));
    
    tag_groups.par_iter().for_each(|(_tag, indices)| {
        if indices.len() < 2 {
            return; // 단일 이벤트는 처리할 필요 없음
        }
        
        let mut req_times: HashMap<(u32, String), f64> = HashMap::new();
        let mut current_qd: u32 = 0;
        let mut last_complete_time: Option<f64> = None;
        let mut last_complete_qd0_time: Option<f64> = None;
        let mut prev_send_req: Option<(u64, u32, String)> = None;
        let mut first_c: bool = false;
        let mut first_complete_time: f64 = 0.0;
        
        // 각 태그 그룹의 모든 이벤트 처리
        let mut tag_results = HashMap::new();
        
        for &idx in indices {
            let ufs = &ufs_list[idx];
            let mut result_ufs = ufs.clone(); // 수정할 복사본 생성
            
            match ufs.action.as_str() {
                "send_req" => {
                    // 연속성 체크: 이전 send_req가 있는 경우
                    if let Some((prev_lba, prev_size, prev_opcode)) = &prev_send_req {
                        let prev_end_addr = prev_lba + *prev_size as u64;
                        // 현재 요청의 시작 주소가 이전 요청의 끝 주소와 같고, opcode가 같은 경우
                        result_ufs.continuous = ufs.lba == prev_end_addr && &ufs.opcode == prev_opcode;
                    } else {
                        result_ufs.continuous = false;
                    }

                    // 현재 send_req 정보 저장
                    prev_send_req = Some((ufs.lba, ufs.size, ufs.opcode.clone()));
                    req_times.insert((ufs.tag, ufs.opcode.clone()), ufs.time);
                    current_qd += 1;
                    
                    // ctod는 send_req(Device)에서 계산 - 마지막 complete에서 현재 device까지
                    if let Some(t) = last_complete_qd0_time {
                        result_ufs.ctod = (ufs.time - t) * MILLISECONDS as f64;
                    }
                    
                    if current_qd == 1 {
                        first_c = true;
                        first_complete_time = ufs.time;
                    }
                }
                "complete_rsp" => {
                    // complete_rsp는 continuous 체크하지 않음
                    result_ufs.continuous = false;

                    current_qd = current_qd.saturating_sub(1);
                    if let Some(send_time) = req_times.remove(&(ufs.tag, ufs.opcode.clone())) {
                        result_ufs.dtoc = (ufs.time - send_time) * MILLISECONDS as f64;
                    }
                    
                    match first_c {
                        true => {
                            result_ufs.ctoc = (ufs.time - first_complete_time) * MILLISECONDS as f64;
                            first_c = false;
                        }
                        false => {
                            if let Some(t) = last_complete_time {
                                result_ufs.ctoc = (ufs.time - t) * MILLISECONDS as f64;
                            }
                        }
                    }
                    if current_qd == 0 {
                        last_complete_qd0_time = Some(ufs.time);
                    }
                    last_complete_time = Some(ufs.time);
                }
                _ => {
                    result_ufs.continuous = false;
                }
            }
            
            result_ufs.qd = current_qd;
            tag_results.insert(idx, result_ufs);
        }
        
        // 처리 결과 저장
        let mut processed = processed_indices.lock().unwrap();
        for (idx, result) in tag_results {
            processed.insert(idx, result);
        }
    });
    
    // 세 번째 단계: 처리된 결과를 원본 벡터에 적용
    let processed = processed_indices.lock().unwrap();
    for (idx, result) in processed.iter() {
        ufs_list[*idx] = result.clone();
    }
    
    println!("UFS 병렬 처리 완료: {} 이벤트, {:.2}초", 
             ufs_count, start_time.elapsed().as_secs_f64());
    
    ufs_list
}

// 기존 순차 처리 방식
fn sequential_processing(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    
    let mut req_times: HashMap<(u32, String), f64> = HashMap::new();
    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // 이전 send_req의 정보를 저장할 변수들
    let mut prev_send_req: Option<(u64, u32, String)> = None; // (lba, size, opcode)

    for ufs in ufs_list.iter_mut() {
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
                
                // ctod는 send_req(Device)에서 계산 - 마지막 complete에서 현재 device까지
                if let Some(t) = last_complete_qd0_time {
                    ufs.ctod = (ufs.time - t) * MILLISECONDS as f64;
                }
                
                if current_qd == 1 {
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
    
    println!("UFS 순차 처리 완료: {:.2}초", start_time.elapsed().as_secs_f64());
    
    ufs_list
}