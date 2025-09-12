use crate::models::{Block, UFS, UFSCUSTOM, TraceItem};
use crate::log;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

/// 초당 성능 데이터 구조체
#[derive(Debug, Clone)]
pub struct PerSecondPerformance {
    pub second: u64,
    pub read_mibs: f64,    // Read MiB/s
    pub write_mibs: f64,   // Write MiB/s
    pub read_ops: u64,     // Read operations per second
    pub write_ops: u64,    // Write operations per second
}

/// Trace 타입별 성능 분석
pub fn analyze_performance<T: TraceItem>(traces: &[T], trace_type: &str) -> Vec<PerSecondPerformance> {
    if traces.is_empty() {
        return Vec::new();
    }

    log!("Analyzing {} performance for {} events...", trace_type, traces.len());

    // 시작 시간 기준으로 각 초별로 데이터 그룹화
    let start_time = traces.iter().map(|t| t.get_time()).fold(f64::INFINITY, f64::min);
    let mut per_second_data: HashMap<u64, (f64, f64, u64, u64)> = HashMap::new(); // (read_bytes, write_bytes, read_ops, write_ops)

    for trace in traces {
        let elapsed_seconds = ((trace.get_time() - start_time) as u64).max(0);
        
        // 타입별로 다른 단위 적용
        let size_bytes = match trace_type {
            "Block" | "UFSCustom" => {
                // Block과 UFSCustom은 sector 단위 (일반적으로 512 바이트)
                trace.get_size() as f64 * 512.0
            },
            "UFS" => {
                // UFS는 4KB 단위
                trace.get_size() as f64 * 4096.0
            },
            _ => trace.get_size() as f64, // 기본값
        };
        
        // I/O 타입 확인
        let is_read = match trace.get_io_type().to_uppercase().as_str() {
            s if s.starts_with('R') => true,
            s if s.starts_with('W') => false,
            _ => continue, // 알 수 없는 타입은 건너뜀
        };

        let entry = per_second_data.entry(elapsed_seconds).or_insert((0.0, 0.0, 0, 0));
        
        if is_read {
            entry.0 += size_bytes; // read_bytes
            entry.2 += 1;          // read_ops
        } else {
            entry.1 += size_bytes; // write_bytes
            entry.3 += 1;          // write_ops
        }
    }

    // HashMap을 Vec으로 변환하고 시간순 정렬
    let mut result: Vec<PerSecondPerformance> = per_second_data
        .into_iter()
        .map(|(second, (read_bytes, write_bytes, read_ops, write_ops))| {
            PerSecondPerformance {
                second,
                read_mibs: read_bytes / (1024.0 * 1024.0), // 바이트를 MiB로 변환
                write_mibs: write_bytes / (1024.0 * 1024.0),
                read_ops,
                write_ops,
            }
        })
        .collect();

    result.sort_by_key(|p| p.second);
    
    log!("{} performance analysis completed: {} seconds of data", trace_type, result.len());
    result
}

/// Block 트레이스 성능 분석
pub fn analyze_block_performance(blocks: &[Block]) -> Vec<PerSecondPerformance> {
    analyze_performance(blocks, "Block")
}

/// UFS 트레이스 성능 분석  
pub fn analyze_ufs_performance(ufs_traces: &[UFS]) -> Vec<PerSecondPerformance> {
    analyze_performance(ufs_traces, "UFS")
}

/// UFSCustom 트레이스 성능 분석
pub fn analyze_ufscustom_performance(ufscustom_traces: &[UFSCUSTOM]) -> Vec<PerSecondPerformance> {
    analyze_performance(ufscustom_traces, "UFSCustom")
}

/// 성능 데이터를 CSV 파일로 저장
pub fn save_performance_csv(
    output_prefix: &str,
    block_perf: &[PerSecondPerformance],
    ufs_perf: &[PerSecondPerformance], 
    ufscustom_perf: &[PerSecondPerformance],
) -> Result<(), Box<dyn std::error::Error>> {
    let csv_path = format!("{}_performance.csv", output_prefix);
    let mut file = File::create(&csv_path)?;

    // CSV 헤더 작성
    writeln!(file, "Type,Second,Read_MiB/s,Write_MiB/s,Read_OPS,Write_OPS")?;

    // Block 데이터 작성
    for perf in block_perf {
        writeln!(
            file,
            "Block,{},{:.3},{:.3},{},{}",
            perf.second,
            perf.read_mibs,
            perf.write_mibs,
            perf.read_ops,
            perf.write_ops
        )?;
    }

    // UFS 데이터 작성
    for perf in ufs_perf {
        writeln!(
            file,
            "UFS,{},{:.3},{:.3},{},{}",
            perf.second,
            perf.read_mibs,
            perf.write_mibs,
            perf.read_ops,
            perf.write_ops
        )?;
    }

    // UFSCustom 데이터 작성
    for perf in ufscustom_perf {
        writeln!(
            file,
            "UFSCustom,{},{:.3},{:.3},{},{}",
            perf.second,
            perf.read_mibs,
            perf.write_mibs,
            perf.read_ops,
            perf.write_ops
        )?;
    }

    log!("Performance CSV saved: {}", csv_path);
    Ok(())
}