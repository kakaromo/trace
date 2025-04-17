use crate::models::{UFS, Block};
use std::collections::HashMap;
use std::cmp::Ordering;

// 통계 계산을 위한 헬퍼 구조체
#[derive(Default)]
struct LatencyStats {
    values: Vec<f64>,
    sum: f64,
    min: f64,
    max: f64,
}

impl LatencyStats {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            sum: 0.0,
            min: f64::MAX,
            max: 0.0,
        }
    }

    fn add(&mut self, value: f64) {
        self.values.push(value);
        self.sum += value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    fn avg(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum / self.values.len() as f64
        }
    }

    fn median(&self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        
        // 값을 복사하여 정렬
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }

    fn std_dev(&self) -> f64 {
        if self.values.len() <= 1 {
            return 0.0;
        }
        let avg = self.avg();
        let variance = self.values.iter()
            .map(|&v| {
                let diff = avg - v;
                diff * diff
            })
            .sum::<f64>() / (self.values.len() - 1) as f64;
        variance.sqrt()
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        // 백분위수 계산을 위해 값을 복사하여 정렬
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
        
        let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx]
    }

    fn latency_ranges(&self) -> HashMap<String, usize> {
        // 지연 시간 범위 정의
        let latency_ranges = vec![
            (0.0, 0.1, "≤ 0.1ms"),
            (0.1, 0.5, "0.1ms < v ≤ 0.5ms"),
            (0.5, 1.0, "0.5ms < v ≤ 1ms"),
            (1.0, 5.0, "1ms < v ≤ 5ms"),
            (5.0, 10.0, "5ms < v ≤ 10ms"),
            (10.0, 50.0, "10ms < v ≤ 50ms"),
            (50.0, 100.0, "50ms < v ≤ 100ms"),
            (100.0, 500.0, "100ms < v ≤ 500ms"),
            (500.0, 1000.0, "500ms < v ≤ 1s"),
            (1000.0, 5000.0, "1s < v ≤ 5s"),
            (5000.0, 10000.0, "5s < v ≤ 10s"),
            (10000.0, 50000.0, "10s < v ≤ 50s"),
            (50000.0, 100000.0, "50s < v ≤ 100s"),
            (100000.0, 500000.0, "100s < v ≤ 500s"),
            (500000.0, 1000000.0, "500s < v ≤ 1000s"),
            (1000000.0, f64::MAX, "> 1000s"),
        ];

        let mut counts = HashMap::new();
        for &(_, _, label) in &latency_ranges {
            counts.insert(label.to_string(), 0);
        }

        for &value in &self.values {
            for &(min, max, label) in &latency_ranges {
                if value > min && value <= max {
                    *counts.get_mut(label).unwrap() += 1;
                    break;
                }
            }
        }

        counts
    }
}

// 사이즈 분포 계산을 위한 헬퍼 함수
fn count_sizes<T>(traces: &[&T], size_fn: impl Fn(&&T) -> u32) -> HashMap<u32, usize> {
    let mut size_counts = HashMap::new();
    for trace in traces {
        let size = size_fn(&trace);
        *size_counts.entry(size).or_insert(0) += 1;
    }
    size_counts
}

pub fn print_ufs_statistics(traces: &[UFS]) {
    println!("총 요청 수: {}", traces.len());
    println!("최대 큐 깊이: {}", traces.iter().map(|t| t.qd).max().unwrap_or(0));
    
    let complete_traces: Vec<_> = traces.iter().filter(|t| t.action == "complete_rsp").collect();
    
    if !complete_traces.is_empty() {
        let avg_dtoc = complete_traces.iter().map(|t| t.dtoc).sum::<f64>() / complete_traces.len() as f64;
        let avg_ctoc = complete_traces.iter().filter(|t| t.ctoc > 0.0).map(|t| t.ctoc).sum::<f64>() 
                      / complete_traces.iter().filter(|t| t.ctoc > 0.0).count() as f64;
        
        println!("평균 Device to Complete 지연: {:.3} ms", avg_dtoc);
        println!("평균 Complete to Complete 지연: {:.3} ms", avg_ctoc);
    }
    
    let send_traces: Vec<_> = traces.iter().filter(|t| t.action == "send_req").collect();
    if !send_traces.is_empty() {
        let avg_ctod = send_traces.iter().filter(|t| t.ctod > 0.0).map(|t| t.ctod).sum::<f64>() 
                      / send_traces.iter().filter(|t| t.ctod > 0.0).count() as f64;
        println!("평균 Complete to Device 지연: {:.3} ms", avg_ctod);
    }

    let continuous_reqs = traces.iter().filter(|t| t.continuous).count();
    println!("연속적 요청 비율: {:.1}%", (continuous_reqs as f64 / traces.len() as f64) * 100.0);

    // 지연 시간 통계 추가 (UFS opcode별)
    println!("\n[UFS 지연 시간 통계]");
    
    // dtoc, ctoc는 complete_rsp 이벤트에서 측정
    let mut complete_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.action == "complete_rsp") {
        complete_opcode_groups.entry(trace.opcode.clone())
            .or_insert_with(Vec::new)
            .push(trace);
    }
    
    // ctod는 send_req 이벤트에서 측정
    let mut send_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.action == "send_req") {
        send_opcode_groups.entry(trace.opcode.clone())
            .or_insert_with(Vec::new)
            .push(trace);
    }

    // 각 지연 시간 유형별로 통계 테이블 출력
    print_latency_stats_by_opcode(&complete_opcode_groups, "Device to Complete (dtoc)", |trace| trace.dtoc);
    print_latency_stats_by_opcode(&send_opcode_groups, "Complete to Device (ctod)", |trace| trace.ctod);
    print_latency_stats_by_opcode(&complete_opcode_groups, "Complete to Complete (ctoc)", |trace| trace.ctoc);
    
    // 지연 시간 범위별 분포 통계
    println!("\n[UFS Device to Complete (dtoc) 범위별 분포]");
    print_latency_ranges_by_opcode(&complete_opcode_groups, "Device to Complete (dtoc)", |trace| trace.dtoc);
    
    println!("\n[UFS Complete to Device (ctod) 범위별 분포]");
    print_latency_ranges_by_opcode(&send_opcode_groups, "Complete to Device (ctod)", |trace| trace.ctod);
    
    println!("\n[UFS Complete to Complete (ctoc) 범위별 분포]");
    print_latency_ranges_by_opcode(&complete_opcode_groups, "Complete to Complete (ctoc)", |trace| trace.ctoc);
    
    // 사이즈 분포 통계
    println!("\n[UFS 요청 크기 분포]");
    // opcode별로 사이즈 집계 (complete_opcode_groups 사용)
    for (opcode, traces) in complete_opcode_groups.iter() {
        let size_counts = count_sizes(traces, |trace| trace.size);
        println!("\nOpcode {} 크기 분포:", opcode);
        println!("Size\tCount");
        
        let mut sizes: Vec<_> = size_counts.keys().collect();
        sizes.sort();
        
        for &size in sizes {
            println!("{}\t{}", size, size_counts[&size]);
        }
    }
}

pub fn print_block_statistics(traces: &[Block]) {
    println!("총 요청 수: {}", traces.len());
    println!("최대 큐 깊이: {}", traces.iter().map(|t| t.qd).max().unwrap_or(0));
    
    let complete_traces: Vec<_> = traces.iter().filter(|t| t.action == "block_rq_complete").collect();
    
    if !complete_traces.is_empty() {
        let avg_dtoc = complete_traces.iter().map(|t| t.dtoc).sum::<f64>() / complete_traces.len() as f64;
        let avg_ctoc = complete_traces.iter().filter(|t| t.ctoc > 0.0).map(|t| t.ctoc).sum::<f64>() 
                      / complete_traces.iter().filter(|t| t.ctoc > 0.0).count() as f64;
        
        println!("평균 Device to Complete 지연: {:.3} ms", avg_dtoc);
        println!("평균 Complete to Complete 지연: {:.3} ms", avg_ctoc);
    }

    let issue_traces: Vec<_> = traces.iter().filter(|t| t.action == "block_rq_issue").collect();
    if !issue_traces.is_empty() {
        let avg_ctod = issue_traces.iter().filter(|t| t.ctod > 0.0).map(|t| t.ctod).sum::<f64>() 
                      / issue_traces.iter().filter(|t| t.ctod > 0.0).count() as f64;
        println!("평균 Complete to Device 지연: {:.3} ms", avg_ctod);
    }

    let continuous_reqs = traces.iter().filter(|t| t.continuous).count();
    println!("연속적 요청 비율: {:.1}%", (continuous_reqs as f64 / traces.len() as f64) * 100.0);
    
    // I/O 타입별 통계
    let reads = traces.iter().filter(|t| t.io_type.starts_with('R')).count();
    let writes = traces.iter().filter(|t| t.io_type.starts_with('W')).count();
    println!("읽기 요청: {} ({:.1}%)", reads, (reads as f64 / traces.len() as f64) * 100.0);
    println!("쓰기 요청: {} ({:.1}%)", writes, (writes as f64 / traces.len() as f64) * 100.0);

    // 지연 시간 통계 추가 (Block I/O 타입별)
    println!("\n[Block I/O 지연 시간 통계]");
    
    // dtoc, ctoc는 block_rq_complete 이벤트에서 측정
    let mut complete_iotype_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.action == "block_rq_complete") {
        complete_iotype_groups.entry(trace.io_type.clone())
            .or_insert_with(Vec::new)
            .push(trace);
    }

    // ctod는 block_rq_issue 이벤트에서 측정
    let mut issue_iotype_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.action == "block_rq_issue") {
        issue_iotype_groups.entry(trace.io_type.clone())
            .or_insert_with(Vec::new)
            .push(trace);
    }

    // 각 지연 시간 유형별로 통계 테이블 출력
    print_latency_stats_by_iotype(&complete_iotype_groups, "Device to Complete (dtoc)", |trace| trace.dtoc);
    print_latency_stats_by_iotype(&issue_iotype_groups, "Complete to Device (ctod)", |trace| trace.ctod);
    print_latency_stats_by_iotype(&complete_iotype_groups, "Complete to Complete (ctoc)", |trace| trace.ctoc);
    
    // 지연 시간 범위별 분포 통계
    println!("\n[Block I/O Device to Complete (dtoc) 범위별 분포]");
    print_latency_ranges_by_iotype(&complete_iotype_groups, "Device to Complete (dtoc)", |trace| trace.dtoc);
    
    println!("\n[Block I/O Complete to Device (ctod) 범위별 분포]");
    print_latency_ranges_by_iotype(&issue_iotype_groups, "Complete to Device (ctod)", |trace| trace.ctod);
    
    println!("\n[Block I/O Complete to Complete (ctoc) 범위별 분포]");
    print_latency_ranges_by_iotype(&complete_iotype_groups, "Complete to Complete (ctoc)", |trace| trace.ctoc);
    
    // 사이즈 분포 통계
    println!("\n[Block I/O 요청 크기 분포]");
    // I/O 타입별로 사이즈 집계 (complete_iotype_groups 사용)
    for (io_type, traces) in complete_iotype_groups.iter() {
        let size_counts = count_sizes(traces, |trace| trace.size);
        println!("\nI/O 타입 {} 크기 분포:", io_type);
        println!("Size\tCount");
        
        let mut sizes: Vec<_> = size_counts.keys().collect();
        sizes.sort();
        
        for &size in sizes {
            println!("{}\t{}", size, size_counts[&size]);
        }
    }
}

// UFS opcode별 지연 시간 통계 출력 함수
fn print_latency_stats_by_opcode(
    opcode_groups: &HashMap<String, Vec<&UFS>>, 
    stat_name: &str,
    latency_fn: impl Fn(&&UFS) -> f64
) {
    println!("\n{} 통계:", stat_name);
    println!("Type\tAvg\tMin\tMedian\tMax\tStd\t99th\t99.9th\t99.99th\t99.999th\t99.9999th");
    
    // 정렬된 opcodes
    let mut opcodes: Vec<&String> = opcode_groups.keys().collect();
    opcodes.sort();
    
    for &opcode in &opcodes {
        let traces = &opcode_groups[opcode];
        
        // 지연 시간 통계 계산
        let mut stats = LatencyStats::new();
        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {  // 유효한 지연 시간만 처리
                stats.add(latency);
            }
        }
        
        if !stats.values.is_empty() {
            println!("{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
                opcode,
                stats.avg(),
                stats.min,
                stats.median(),
                stats.max,
                stats.std_dev(),
                stats.percentile(99.0),
                stats.percentile(99.9),
                stats.percentile(99.99),
                stats.percentile(99.999),
                stats.percentile(99.9999)
            );
        }
    }
}

// UFS opcode별 지연 시간 범위 분포 출력 함수
fn print_latency_ranges_by_opcode(
    opcode_groups: &HashMap<String, Vec<&UFS>>, 
    stat_name: &str,
    latency_fn: impl Fn(&&UFS) -> f64
) {
    println!("\n{} 범위별 분포:", stat_name);
    
    // 먼저 모든 opcode 목록 정렬
    let mut opcodes: Vec<&String> = opcode_groups.keys().collect();
    opcodes.sort();
    
    // 헤더 출력
    print!("Range\t");
    for &opcode in &opcodes {
        print!("{}\t", opcode);
    }
    println!();
    
    // 각 opcode별 latency 통계 계산
    let mut all_stats = HashMap::new();
    for &opcode in &opcodes {
        let traces = &opcode_groups[opcode];
        let mut stats = LatencyStats::new();
        
        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {
                stats.add(latency);
            }
        }
        
        all_stats.insert(opcode, stats);
    }
    
    // 각 범위별 카운트 출력
    let ranges = vec![
        "≤ 0.1ms", "0.1ms < v ≤ 0.5ms", "0.5ms < v ≤ 1ms", "1ms < v ≤ 5ms",
        "5ms < v ≤ 10ms", "10ms < v ≤ 50ms", "50ms < v ≤ 100ms", "100ms < v ≤ 500ms",
        "500ms < v ≤ 1s", "1s < v ≤ 5s", "5s < v ≤ 10s", "10s < v ≤ 50s",
        "50s < v ≤ 100s", "100s < v ≤ 500s", "500s < v ≤ 1000s", "> 1000s"
    ];
    
    for range in ranges {
        print!("{}\t", range);
        
        for &opcode in &opcodes {
            if let Some(stats) = all_stats.get(opcode) {
                let range_counts = stats.latency_ranges();
                print!("{}\t", range_counts.get(range).unwrap_or(&0));
            } else {
                print!("0\t");
            }
        }
        println!();
    }
}

// Block I/O 타입별 지연 시간 통계 출력 함수
fn print_latency_stats_by_iotype(
    iotype_groups: &HashMap<String, Vec<&Block>>, 
    stat_name: &str,
    latency_fn: impl Fn(&&Block) -> f64
) {
    println!("\n{} 통계:", stat_name);
    println!("Type\tAvg\tMin\tMedian\tMax\tStd\t99th\t99.9th\t99.99th\t99.999th\t99.9999th");
    
    // 정렬된 iotypes
    let mut iotypes: Vec<&String> = iotype_groups.keys().collect();
    iotypes.sort();
    
    for &iotype in &iotypes {
        let traces = &iotype_groups[iotype];
        
        // 지연 시간 통계 계산
        let mut stats = LatencyStats::new();
        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {  // 유효한 지연 시간만 처리
                stats.add(latency);
            }
        }
        
        if !stats.values.is_empty() {
            println!("{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}",
                iotype,
                stats.avg(),
                stats.min,
                stats.median(),
                stats.max,
                stats.std_dev(),
                stats.percentile(99.0),
                stats.percentile(99.9),
                stats.percentile(99.99),
                stats.percentile(99.999),
                stats.percentile(99.9999)
            );
        }
    }
}

// Block I/O 타입별 지연 시간 범위 분포 출력 함수
fn print_latency_ranges_by_iotype(
    iotype_groups: &HashMap<String, Vec<&Block>>, 
    stat_name: &str,
    latency_fn: impl Fn(&&Block) -> f64
) {
    println!("\n{} 범위별 분포:", stat_name);
    
    // 먼저 모든 I/O 타입 목록 정렬
    let mut iotypes: Vec<&String> = iotype_groups.keys().collect();
    iotypes.sort();
    
    // 헤더 출력
    print!("Range\t");
    for &iotype in &iotypes {
        print!("{}\t", iotype);
    }
    println!();
    
    // 각 I/O 타입별 latency 통계 계산
    let mut all_stats = HashMap::new();
    for &iotype in &iotypes {
        let traces = &iotype_groups[iotype];
        let mut stats = LatencyStats::new();
        
        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {
                stats.add(latency);
            }
        }
        
        all_stats.insert(iotype, stats);
    }
    
    // 각 범위별 카운트 출력
    let ranges = vec![
        "≤ 0.1ms", "0.1ms < v ≤ 0.5ms", "0.5ms < v ≤ 1ms", "1ms < v ≤ 5ms",
        "5ms < v ≤ 10ms", "10ms < v ≤ 50ms", "50ms < v ≤ 100ms", "100ms < v ≤ 500ms",
        "500ms < v ≤ 1s", "1s < v ≤ 5s", "5s < v ≤ 10s", "10s < v ≤ 50s",
        "50s < v ≤ 100s", "100s < v ≤ 500s", "500s < v ≤ 1000s", "> 1000s"
    ];
    
    for range in ranges {
        print!("{}\t", range);
        
        for &iotype in &iotypes {
            if let Some(stats) = all_stats.get(iotype) {
                let range_counts = stats.latency_ranges();
                print!("{}\t", range_counts.get(range).unwrap_or(&0));
            } else {
                print!("0\t");
            }
        }
        println!();
    }
}