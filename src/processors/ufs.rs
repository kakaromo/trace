use std::collections::HashMap;
use crate::models::UFS;
use crate::utils::constants::MILLISECONDS;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub fn ufs_bottom_half_latency_process(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    let ufs_count = ufs_list.len();
    println!("Starting UFS post-processing: {} events", ufs_count);

    // Sort by time in ascending order (using parallel sort)
    ufs_list.par_sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap());
    println!("UFS sorting complete: {:.2}s", start_time.elapsed().as_secs_f64());

    // Determine processing method based on file size
    if ufs_count > 1_000_000 {
        // Large-scale processing: Split into chunks, process in parallel, then merge
        chunk_based_processing(ufs_list)
    } else {
        // Small-scale processing: Use existing sequential method
        sequential_processing(ufs_list)
    }
}

// Chunk-based parallel processing for large datasets
fn chunk_based_processing(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    let ufs_count = ufs_list.len();
    
    // Group data for parallel processing
    // Group by tag for independent processing
    let mut tag_groups: HashMap<u32, Vec<usize>> = HashMap::new();
    
    // First step: Collect index and tag information
    for (idx, ufs) in ufs_list.iter().enumerate() {
        tag_groups.entry(ufs.tag).or_default().push(idx);
    }
    
    println!("UFS tag grouping complete: {} groups, {:.2}s", 
             tag_groups.len(), start_time.elapsed().as_secs_f64());

    // Second step: Process each tag group in parallel
    let processed_indices = Arc::new(Mutex::new(HashMap::new()));
    
    tag_groups.par_iter().for_each(|(_tag, indices)| {
        if indices.len() < 2 {
            return; // No need to process single events
        }
        
        let mut req_times: HashMap<(u32, String), f64> = HashMap::new();
        let mut current_qd: u32 = 0;
        let mut last_complete_time: Option<f64> = None;
        let mut last_complete_qd0_time: Option<f64> = None;
        let mut prev_send_req: Option<(u64, u32, String)> = None;
        let mut first_c: bool = false;
        let mut first_complete_time: f64 = 0.0;
        
        // Process all events in each tag group
        let mut tag_results = HashMap::new();
        
        for &idx in indices {
            let ufs = &ufs_list[idx];
            let mut result_ufs = ufs.clone(); // Create a copy to modify
            
            match ufs.action.as_str() {
                "send_req" => {
                    // Check continuity: if previous send_req exists
                    if let Some((prev_lba, prev_size, prev_opcode)) = &prev_send_req {
                        let prev_end_addr = prev_lba + *prev_size as u64;
                        // Current request's start address equals previous request's end address and opcode is the same
                        result_ufs.continuous = ufs.lba == prev_end_addr && &ufs.opcode == prev_opcode;
                    } else {
                        result_ufs.continuous = false;
                    }

                    // Store current send_req information
                    prev_send_req = Some((ufs.lba, ufs.size, ufs.opcode.clone()));
                    req_times.insert((ufs.tag, ufs.opcode.clone()), ufs.time);
                    current_qd += 1;
                    
                    // ctod is calculated at send_req(Device) - from last complete to current device
                    if let Some(t) = last_complete_qd0_time {
                        result_ufs.ctod = (ufs.time - t) * MILLISECONDS as f64;
                    }
                    
                    if current_qd == 1 {
                        first_c = true;
                        first_complete_time = ufs.time;
                    }
                }
                "complete_rsp" => {
                    // complete_rsp doesn't check continuity
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
        
        // Store processing results
        let mut processed = processed_indices.lock().unwrap();
        for (idx, result) in tag_results {
            processed.insert(idx, result);
        }
    });
    
    // Third step: Apply processed results to the original vector
    let processed = processed_indices.lock().unwrap();
    for (idx, result) in processed.iter() {
        ufs_list[*idx] = result.clone();
    }
    
    println!("UFS parallel processing complete: {} events, {:.2}s", 
             ufs_count, start_time.elapsed().as_secs_f64());
    
    ufs_list
}

// Traditional sequential processing method
fn sequential_processing(mut ufs_list: Vec<UFS>) -> Vec<UFS> {
    let start_time = Instant::now();
    
    let mut req_times: HashMap<(u32, String), f64> = HashMap::new();
    let mut current_qd: u32 = 0;
    let mut last_complete_time: Option<f64> = None;
    let mut last_complete_qd0_time: Option<f64> = None;
    let mut first_c: bool = false;
    let mut first_complete_time: f64 = 0.0;

    // Variables to store previous send_req information
    let mut prev_send_req: Option<(u64, u32, String)> = None; // (lba, size, opcode)

    for ufs in ufs_list.iter_mut() {
        match ufs.action.as_str() {
            "send_req" => {
                // Check continuity: if previous send_req exists
                if let Some((prev_lba, prev_size, prev_opcode)) = prev_send_req {
                    let prev_end_addr = prev_lba + prev_size as u64;
                    // Current request's start address equals previous request's end address and opcode is the same
                    ufs.continuous = ufs.lba == prev_end_addr && ufs.opcode == prev_opcode;
                } else {
                    ufs.continuous = false;
                }

                // Store current send_req information
                prev_send_req = Some((ufs.lba, ufs.size, ufs.opcode.clone()));
                req_times.insert((ufs.tag, ufs.opcode.clone()), ufs.time);
                current_qd += 1;
                
                // ctod is calculated at send_req(Device) - from last complete to current device
                if let Some(t) = last_complete_qd0_time {
                    ufs.ctod = (ufs.time - t) * MILLISECONDS as f64;
                }
                
                if current_qd == 1 {
                    first_c = true;
                    first_complete_time = ufs.time;
                }
            }
            "complete_rsp" => {
                // complete_rsp doesn't check continuity
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
    
    println!("UFS sequential processing complete: {:.2}s", start_time.elapsed().as_secs_f64());
    
    ufs_list
}