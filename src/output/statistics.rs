use crate::log;
use crate::models::{Block, TraceItem, UFS, UFSCUSTOM};
use crate::utils::get_user_latency_ranges;
use std::cmp::Ordering;
use std::collections::HashMap;

// UFS 타입에 대한 TraceItem 구현
impl TraceItem for UFS {
    fn get_type(&self) -> String {
        self.opcode.clone() // UFS는 opcode를 타입으로 사용
    }

    fn get_dtoc(&self) -> f64 {
        self.dtoc
    }

    fn get_ctoc(&self) -> f64 {
        self.ctoc
    }

    fn get_ctod(&self) -> f64 {
        self.ctod
    }

    fn get_size(&self) -> u32 {
        self.size
    }

    fn get_action(&self) -> &str {
        &self.action
    }

    fn is_continuous(&self) -> bool {
        self.continuous
    }

    fn get_qd(&self) -> u32 {
        self.qd
    }
}

// Block 타입에 대한 TraceItem 구현
impl TraceItem for Block {
    fn get_type(&self) -> String {
        // Block은 io_type의 첫 글자를 타입으로 사용
        self.io_type.chars().next().unwrap_or('?').to_string()
    }

    fn get_dtoc(&self) -> f64 {
        self.dtoc
    }

    fn get_ctoc(&self) -> f64 {
        self.ctoc
    }

    fn get_ctod(&self) -> f64 {
        self.ctod
    }

    fn get_size(&self) -> u32 {
        self.size
    }

    fn get_action(&self) -> &str {
        &self.action
    }

    fn is_continuous(&self) -> bool {
        self.continuous
    }

    fn get_qd(&self) -> u32 {
        self.qd
    }
}

// UFSCUSTOM 타입에 대한 TraceItem 구현
impl TraceItem for UFSCUSTOM {
    fn get_type(&self) -> String {
        self.opcode.clone() // UFSCUSTOM도 UFS와 같이 opcode를 타입으로 사용
    }

    fn get_dtoc(&self) -> f64 {
        self.dtoc
    }

    fn get_ctoc(&self) -> f64 {
        0.0 // UFSCUSTOM 모델에는 ctoc 필드가 없음
    }

    fn get_ctod(&self) -> f64 {
        0.0 // UFSCUSTOM 모델에는 ctod 필드가 없음
    }

    fn get_size(&self) -> u32 {
        self.size
    }

    fn get_action(&self) -> &str {
        "complete" // UFSCUSTOM은 완료된 IO만 기록하므로 항상 "complete"
    }

    fn is_continuous(&self) -> bool {
        false // UFSCUSTOM 모델에는 continuous 필드가 없음
    }

    fn get_qd(&self) -> u32 {
        0 // UFSCUSTOM 모델에는 qd 필드가 없음
    }
}

// Helper structure for statistical calculations
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

        // Copy and sort values
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
        let variance = self
            .values
            .iter()
            .map(|&v| {
                let diff = avg - v;
                diff * diff
            })
            .sum::<f64>()
            / (self.values.len() - 1) as f64;
        variance.sqrt()
    }

    fn percentile(&self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        // Copy and sort values for percentile calculation
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

        let idx = (p / 100.0 * (sorted.len() - 1) as f64).round() as usize;
        sorted[idx]
    }

    fn latency_ranges(&self) -> HashMap<String, usize> {
        // 사용자 정의 레이턴시 범위 또는 기본값 사용
        let user_ranges = get_user_latency_ranges();
        
        let latency_ranges: Vec<(f64, f64, String)> = if let Some(ranges) = user_ranges {
            // 사용자 정의 범위를 사용
            let mut result = Vec::with_capacity(ranges.len() + 1);
            
            // 첫 번째 범위 (0 ~ 첫번째 값)
            result.push((0.0, ranges[0], format!("≤ {}ms", ranges[0])));
            
            // 중간 범위들
            for i in 0..(ranges.len()-1) {
                result.push((
                    ranges[i], 
                    ranges[i+1], 
                    format!("{}ms < v ≤ {}ms", ranges[i], ranges[i+1])
                ));
            }
            
            // 마지막 범위 (마지막 값 이상)
            let last = ranges.last().unwrap();
            result.push((
                *last, 
                f64::MAX, 
                format!("> {}ms", last)
            ));
            
            result
        } else {
            // 기본 레이턴시 범위 사용
            vec![
                (0.0, 0.1, "≤ 0.1ms".to_string()),
                (0.1, 0.5, "0.1ms < v ≤ 0.5ms".to_string()),
                (0.5, 1.0, "0.5ms < v ≤ 1ms".to_string()),
                (1.0, 5.0, "1ms < v ≤ 5ms".to_string()),
                (5.0, 10.0, "5ms < v ≤ 10ms".to_string()),
                (10.0, 50.0, "10ms < v ≤ 50ms".to_string()),
                (50.0, 100.0, "50ms < v ≤ 100ms".to_string()),
                (100.0, 500.0, "100ms < v ≤ 500ms".to_string()),
                (500.0, 1000.0, "500ms < v ≤ 1s".to_string()),
                (1000.0, 5000.0, "1s < v ≤ 5s".to_string()),
                (5000.0, 10000.0, "5s < v ≤ 10s".to_string()),
                (10000.0, 50000.0, "10s < v ≤ 50s".to_string()),
                (50000.0, 100000.0, "50s < v ≤ 100s".to_string()),
                (100000.0, 500000.0, "100s < v ≤ 500s".to_string()),
                (500000.0, 1000000.0, "500s < v ≤ 1000s".to_string()),
                (1000000.0, f64::MAX, "> 1000s".to_string()),
            ]
        };

        let mut counts: HashMap<String, usize> = HashMap::new();
        for &(_, _, ref label) in &latency_ranges {
            counts.insert(label.clone(), 0);
        }

        for &value in &self.values {
            for &(min, max, ref label) in &latency_ranges {
                if value > min && value <= max {
                    *counts.get_mut(label).unwrap() += 1;
                    break;
                }
            }
        }

        counts
    }
}

// Helper function for size distribution calculations
fn count_sizes<T>(traces: &[&T], size_fn: impl Fn(&&T) -> u32) -> HashMap<u32, usize> {
    let mut size_counts = HashMap::new();
    for trace in traces {
        let size = size_fn(trace);
        *size_counts.entry(size).or_insert(0) += 1;
    }
    size_counts
}


// 모든 트레이스 타입에 공통으로 사용할 통계 처리 함수들

// 제네릭 통계 출력 함수
pub fn print_trace_statistics<T: TraceItem>(traces: &[T], trace_type_name: &str) {
    log!("Total {} requests: {}", trace_type_name, traces.len());
    log!(
        "Maximum queue depth: {}",
        traces.iter().map(|t| t.get_qd()).max().unwrap_or(0)
    );

    // Complete 액션 타입 결정
    let complete_action = if trace_type_name == "UFS" {
        "complete_rsp"
    } else if trace_type_name == "UFSCustom" {
        "complete" // UFSCustom는 항상 complete
    } else {
        "block_rq_complete"
    };

    // Request 액션 타입 결정
    let request_action = if trace_type_name == "UFS" {
        "send_req"
    } else {
        "block_rq_issue"
    };

    let complete_traces: Vec<_> = traces
        .iter()
        .filter(|t| t.get_action() == complete_action)
        .collect();

    if !complete_traces.is_empty() {
        let avg_dtoc = complete_traces.iter().map(|t| t.get_dtoc()).sum::<f64>()
            / complete_traces.len() as f64;
        let avg_ctoc = complete_traces
            .iter()
            .filter(|t| t.get_ctoc() > 0.0)
            .map(|t| t.get_ctoc())
            .sum::<f64>()
            / complete_traces
                .iter()
                .filter(|t| t.get_ctoc() > 0.0)
                .count() as f64;

        log!("Average Dispatch to Complete latency: {:.3} ms", avg_dtoc);
        log!("Average Complete to Complete latency: {:.3} ms", avg_ctoc);
    }

    let send_traces: Vec<_> = traces
        .iter()
        .filter(|t| t.get_action() == request_action)
        .collect();
    if !send_traces.is_empty() {
        let avg_ctod = send_traces
            .iter()
            .filter(|t| t.get_ctod() > 0.0)
            .map(|t| t.get_ctod())
            .sum::<f64>()
            / send_traces.iter().filter(|t| t.get_ctod() > 0.0).count() as f64;
        log!("Average Complete to Dispatch latency: {:.3} ms", avg_ctod);
    }

    let continuous_reqs = traces.iter().filter(|t| t.is_continuous()).count();
    log!(
        "Continuous request ratio: {:.1}%",
        (continuous_reqs as f64 / traces.len() as f64) * 100.0
    );

    // 타입별 요청 수 집계
    let mut type_groups: HashMap<String, usize> = HashMap::new();
    for trace in traces {
        let type_name = trace.get_type();
        *type_groups.entry(type_name).or_insert(0) += 1;
    }

    // 타입별 비율 출력
    for (type_name, count) in type_groups.iter() {
        log!(
            "{} requests: {} ({:.1}%)",
            type_name,
            count,
            (*count as f64 / traces.len() as f64) * 100.0
        );
    }

    // 추가 지연 시간 통계
    log!("\n[{} Latency Statistics]", trace_type_name);

    // 타입별로 그룹화
    let mut complete_type_groups: HashMap<String, Vec<&T>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.get_action() == complete_action) {
        complete_type_groups
            .entry(trace.get_type())
            .or_default()
            .push(trace);
    }

    let mut request_type_groups: HashMap<String, Vec<&T>> = HashMap::new();
    for trace in traces.iter().filter(|t| t.get_action() == request_action) {
        request_type_groups
            .entry(trace.get_type())
            .or_default()
            .push(trace);
    }

    // 각 지연 시간 유형에 대한 통계 테이블 출력
    print_generic_latency_stats_by_type(
        &complete_type_groups,
        "Dispatch to Complete (dtoc)",
        |trace| trace.get_dtoc(),
    );
    if trace_type_name != "UFSCustom" {
        print_generic_latency_stats_by_type(
            &request_type_groups,
            "Complete to Dispatch (ctod)",
            |trace| trace.get_ctod(),
        );

        print_generic_latency_stats_by_type(
            &complete_type_groups,
            "Complete to Complete (ctoc)",
            |trace| trace.get_ctoc(),
        );
    }

    // 범위별 지연 시간 분포
    log!(
        "\n[{} Dispatch to Complete (dtoc) Distribution by Range]",
        trace_type_name
    );
    print_generic_latency_ranges_by_type(
        &complete_type_groups,
        "Dispatch to Complete (dtoc)",
        |trace| trace.get_dtoc(),
    );
    if trace_type_name != "UFSCustom" {
        log!(
            "\n[{} Complete to Dispatch (ctod) Distribution by Range]",
            trace_type_name
        );
        print_generic_latency_ranges_by_type(
            &request_type_groups,
            "Complete to Dispatch (ctod)",
            |trace| trace.get_ctod(),
        );
    
        log!(
            "\n[{} Complete to Complete (ctoc) Distribution by Range]",
            trace_type_name
        );
        print_generic_latency_ranges_by_type(
            &complete_type_groups,
            "Complete to Complete (ctoc)",
            |trace| trace.get_ctoc(),
        );
    }
    

    // 크기 분포 통계
    log!("\n[{} Request Size Distribution]", trace_type_name);
    // 타입별로 크기 집계
    for (type_name, traces) in complete_type_groups.iter() {
        let size_counts = count_sizes(traces, |trace| trace.get_size());
        log!("\nType {} Size Distribution:", type_name);
        log!("Size,Count");

        let mut sizes: Vec<_> = size_counts.keys().collect();
        sizes.sort();

        for &size in sizes {
            log!("{},{}", size, size_counts[&size]);
        }
    }
}

// 제네릭 지연 시간 통계 출력 함수
fn print_generic_latency_stats_by_type<T: TraceItem>(
    type_groups: &HashMap<String, Vec<&T>>,
    stat_name: &str,
    latency_fn: impl Fn(&&T) -> f64,
) {
    log!("\n{} Statistics:", stat_name);
    log!("Type,Avg,Min,Median,Max,Std,99th,99.9th,99.99th,99.999th,99.9999th");

    // 정렬된 타입 목록
    let mut types: Vec<&String> = type_groups.keys().collect();
    types.sort();

    for &type_name in &types {
        let traces = &type_groups[type_name];

        // 지연 시간 통계 계산
        let mut stats = LatencyStats::new();
        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {
                // 유효한 지연 시간만 처리
                stats.add(latency);
            }
        }

        if !stats.values.is_empty() {
            log!(
                "{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
                type_name,
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

// 제네릭 지연 시간 범위 분포 출력 함수
fn print_generic_latency_ranges_by_type<T: TraceItem>(
    type_groups: &HashMap<String, Vec<&T>>,
    stat_name: &str,
    latency_fn: impl Fn(&&T) -> f64,
) {
    log!("\n{} Distribution by Range:", stat_name);

    // 정렬된 타입 목록
    let mut types: Vec<&String> = type_groups.keys().collect();
    types.sort();

    // 헤더 출력
    let mut header = String::from("Range,");
    for &type_name in &types {
        header.push_str(&format!("{},", type_name));
    }
    log!("{}", header);

    // 각 타입에 대한 지연 시간 통계 계산
    let mut all_stats = HashMap::new();
    for &type_name in &types {
        let traces = &type_groups[type_name];
        let mut stats = LatencyStats::new();

        for &trace in traces {
            let latency = latency_fn(&trace);
            if latency > 0.0 {
                stats.add(latency);
            }
        }

        all_stats.insert(type_name, stats);
    }

    // 지연 시간 범위를 동적으로 가져오기
    // 첫 번째 통계 객체에서 범위를 가져옴 (비어 있지 않다면)
    let mut range_labels: Vec<String> = Vec::new();
    
    if let Some(&first_type) = types.first() {
        if let Some(stats) = all_stats.get(first_type) {
            let range_counts = stats.latency_ranges();
            range_labels = range_counts.keys().cloned().collect();
            
            // 범위 레이블을 순서대로 정렬하는 완전히 새로운 방식
            range_labels.sort_by(|a, b| {
                // 특별 케이스: "≤" 패턴은 항상 가장 먼저
                if a.starts_with("≤") {
                    return std::cmp::Ordering::Less;
                }
                if b.starts_with("≤") {
                    return std::cmp::Ordering::Greater;
                }
                
                // 특별 케이스: ">" 패턴은 항상 가장 마지막
                if a.starts_with(">") {
                    return std::cmp::Ordering::Greater;
                }
                if b.starts_with(">") {
                    return std::cmp::Ordering::Less;
                }
                
                // 나머지 "X < v ≤ Y" 패턴: 하한값 X를 추출하여 비교
                // "숫자ms" 또는 "숫자s" 패턴의 숫자 부분을 추출
                fn extract_lower_bound(s: &str) -> f64 {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if parts.len() >= 1 {
                        // 첫 번째 부분에서 숫자만 추출
                        if let Ok(val) = parts[0].replace("ms", "")
                                                 .replace("s", "")
                                                 .parse::<f64>() {
                            return val;
                        }
                    }
                    0.0 // 기본값
                }
                
                let a_val = extract_lower_bound(a);
                let b_val = extract_lower_bound(b);
                
                a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    // 각 범위에 대한 개수 출력
    for range in range_labels {
        let mut row = format!("{},", range);

        for &type_name in &types {
            if let Some(stats) = all_stats.get(type_name) {
                let range_counts = stats.latency_ranges();
                row.push_str(&format!("{},", range_counts.get(&range).unwrap_or(&0)));
            } else {
                row.push_str("0,");
            }
        }
        log!("{}", row);
    }
}

// 기존 개별 타입 통계 함수를 제네릭 함수를 사용하는 간단한 래퍼로 변경
pub fn print_ufs_statistics(ufs_traces: &[UFS]) {
    print_trace_statistics(ufs_traces, "UFS");
}

pub fn print_block_statistics(block_traces: &[Block]) {
    print_trace_statistics(block_traces, "Block");
}

pub fn print_ufscustom_statistics(ufscustom_traces: &[UFSCUSTOM]) {
    // UFSCustom 데이터는 단순한 구조이므로 비어 있으면 바로 리턴
    if ufscustom_traces.is_empty() {
        log!("UFSCustom 트레이스가 비어 있습니다.");
        return;
    }
    
    // 모든 통계 처리는 이제 print_trace_statistics 함수에서 수행
    print_trace_statistics(ufscustom_traces, "UFSCustom");
}
