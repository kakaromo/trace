use crate::log;
use crate::models::{Block, TraceItem, UFS, UFSCUSTOM};
use crate::utils::get_user_latency_ranges;
use rayon::prelude::*;
use std::cmp::Ordering;
use std::collections::HashMap;

// UFS 타입에 대한 TraceItem 구현
impl TraceItem for UFS {
    fn get_type(&self) -> String {
        self.opcode.clone() // UFS는 opcode를 타입으로 사용
    }

    fn get_time(&self) -> f64 {
        self.time
    }

    fn get_io_type(&self) -> String {
        // UFS의 opcode에서 Read/Write 판단
        if self.opcode.to_lowercase().contains("0x28") {
            "R".to_string()
        } else if self.opcode.to_lowercase().contains("0x2a") {
            "W".to_string()
        } else {
            "O".to_string() // Other
        }
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

    fn is_aligned(&self) -> bool {
        self.aligned
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

    fn get_time(&self) -> f64 {
        self.time
    }

    fn get_io_type(&self) -> String {
        // Block은 io_type을 직접 사용
        self.io_type.clone()
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

    fn is_aligned(&self) -> bool {
        self.aligned
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

    fn get_time(&self) -> f64 {
        self.end_time // 완료 시간을 기본 시간으로 사용
    }

    fn get_io_type(&self) -> String {
        // UFSCUSTOM의 opcode에서 Read/Write 판단
        if self.opcode.to_lowercase().contains("read") {
            "R".to_string()
        } else if self.opcode.to_lowercase().contains("write") {
            "W".to_string()
        } else {
            "O".to_string() // Other
        }
    }

    fn get_dtoc(&self) -> f64 {
        self.dtoc
    }

    fn get_ctoc(&self) -> f64 {
        self.ctoc // 이제 실제 값 반환
    }

    fn get_ctod(&self) -> f64 {
        self.ctod // 이제 실제 값 반환
    }

    fn get_size(&self) -> u32 {
        self.size
    }

    fn get_action(&self) -> &str {
        "complete" // UFSCUSTOM은 완료된 IO만 기록하므로 항상 "complete"
    }

    fn is_continuous(&self) -> bool {
        self.continuous // 이제 실제 값 반환
    }

    fn is_aligned(&self) -> bool {
        self.aligned
    }

    fn get_qd(&self) -> u32 {
        self.start_qd // start_qd를 기본 qd로 사용
    }
    
    fn get_start_qd(&self) -> u32 {
        self.start_qd
    }
    
    fn get_end_qd(&self) -> u32 {
        self.end_qd
    }
}

// Helper structure for statistical calculations
#[derive(Default)]
struct LatencyStats {
    values: Vec<f64>,
    sorted_values: Option<Vec<f64>>, // 정렬된 값을 캐시
    sum: f64,
    min: f64,
    max: f64,
}

impl LatencyStats {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            sorted_values: None,
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
        // 값이 추가되면 캐시 무효화
        self.sorted_values = None;
    }
    
    // 정렬된 값을 가져오거나 생성
    fn get_sorted(&mut self) -> &[f64] {
        if self.sorted_values.is_none() {
            let mut sorted = self.values.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
            self.sorted_values = Some(sorted);
        }
        self.sorted_values.as_ref().unwrap()
    }

    fn avg(&self) -> f64 {
        if self.values.is_empty() {
            0.0
        } else {
            self.sum / self.values.len() as f64
        }
    }

    fn median(&mut self) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }

        // 정렬된 값 재사용
        let sorted = self.get_sorted();
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

    fn percentile(&mut self, p: f64) -> f64 {
        if self.values.is_empty() {
            return 0.0;
        }
        // 정렬된 값 재사용
        let sorted = self.get_sorted();
        let n = sorted.len();

        // 선형 보간법 (Linear Interpolation) 사용
        // 표준적인 percentile 계산 방법
        let index = (p / 100.0) * (n - 1) as f64;
        let lower_index = index.floor() as usize;
        let upper_index = index.ceil() as usize;

        // 인덱스가 같으면 (정수 인덱스인 경우) 해당 값 반환
        if lower_index == upper_index {
            return sorted[lower_index];
        }

        // 선형 보간
        let weight = index - lower_index as f64;
        sorted[lower_index] * (1.0 - weight) + sorted[upper_index] * weight
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
            for i in 0..(ranges.len() - 1) {
                result.push((
                    ranges[i],
                    ranges[i + 1],
                    format!("{}ms < v ≤ {}ms", ranges[i], ranges[i + 1]),
                ));
            }

            // 마지막 범위 (마지막 값 이상)
            let last = ranges.last().unwrap();
            result.push((*last, f64::MAX, format!("> {}ms", last)));

            result
        } else {
            // 기본 레이턴시 범위 사용 (ms 단위 먼저, 그 다음 s 단위)
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
        for (_, _, label) in &latency_ranges {
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
pub fn print_trace_statistics<T: TraceItem + Sync>(traces: &[T], trace_type_name: &str) {
    if traces.is_empty() {
        log!("{} 트레이스가 비어 있습니다.", trace_type_name);
        return;
    }
    
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
    } else if trace_type_name == "Block" {
        // Block I/O는 두 가지 형태를 지원:
        // 1. ftrace 형태: block_rq_complete
        // 2. blktrace CSV 형태: C
        // 실제 데이터에서 어떤 액션이 사용되는지 확인
        if traces.iter().any(|t| t.get_action() == "block_rq_complete") {
            "block_rq_complete" // ftrace 형태
        } else {
            "C" // blktrace CSV 형태
        }
    } else {
        "block_rq_complete" // 기본값
    };

    // Request 액션 타입 결정
    let request_action = if trace_type_name == "UFS" {
        "send_req"
    } else if trace_type_name == "Block" {
        // Block I/O는 두 가지 형태를 지원:
        // 1. ftrace 형태: block_rq_issue
        // 2. blktrace CSV 형태: Q
        if traces.iter().any(|t| t.get_action() == "block_rq_issue") {
            "block_rq_issue" // ftrace 형태
        } else {
            "Q" // blktrace CSV 형태
        }
    } else {
        "block_rq_issue" // 기본값
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

    let aligned_reqs = traces.iter().filter(|t| t.is_aligned()).count();
    log!(
        "Aligned request ratio: {:.1}%",
        (aligned_reqs as f64 / traces.len() as f64) * 100.0
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
    } else {
        // UFSCUSTOM의 경우에도 CTOC, CTOD 분포 출력
        log!(
            "\n[{} Complete to Dispatch (ctod) Distribution by Range]",
            trace_type_name
        );
        print_generic_latency_ranges_by_type(
            &complete_type_groups, // UFSCUSTOM은 모두 complete이므로
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
fn print_generic_latency_stats_by_type<T: TraceItem + Sync>(
    type_groups: &HashMap<String, Vec<&T>>,
    stat_name: &str,
    latency_fn: impl Fn(&&T) -> f64 + Sync + Send,
) {
    log!("\n{} Statistics:", stat_name);
    log!("Type,Avg,Min,Median,Max,Std,99th,99.9th,99.99th,99.999th,99.9999th");

    // 정렬된 타입 목록
    let mut types: Vec<&String> = type_groups.keys().collect();
    types.sort();

    // 타입별 통계를 병렬로 계산
    let results: Vec<(String, String)> = types
        .par_iter()
        .filter_map(|&type_name| {
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
                // 모든 통계 값을 먼저 계산 (mutable borrow 문제 해결)
                let avg = stats.avg();
                let min = stats.min;
                let median = stats.median();
                let max = stats.max;
                let std_dev = stats.std_dev();
                let p99 = stats.percentile(99.0);
                let p999 = stats.percentile(99.9);
                let p9999 = stats.percentile(99.99);
                let p99999 = stats.percentile(99.999);
                let p999999 = stats.percentile(99.9999);
                
                let result = format!(
                    "{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
                    type_name,
                    avg,
                    min,
                    median,
                    max,
                    std_dev,
                    p99,
                    p999,
                    p9999,
                    p99999,
                    p999999
                );
                Some((type_name.clone(), result))
            } else {
                None
            }
        })
        .collect();

    // 타입 이름 순서대로 출력 (병렬 계산 후 순서 보장)
    for type_name in types {
        if let Some((_, result)) = results.iter().find(|(name, _)| name == type_name) {
            log!("{}", result);
        }
    }
}

// 제네릭 지연 시간 범위 분포 출력 함수
fn print_generic_latency_ranges_by_type<T: TraceItem + Sync>(
    type_groups: &HashMap<String, Vec<&T>>,
    stat_name: &str,
    latency_fn: impl Fn(&&T) -> f64 + Sync + Send,
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

    // 각 타입에 대한 지연 시간 통계를 병렬로 계산
    let all_stats: HashMap<&String, LatencyStats> = types
        .par_iter()
        .map(|&type_name| {
            let traces = &type_groups[type_name];
            let mut stats = LatencyStats::new();

            for &trace in traces {
                let latency = latency_fn(&trace);
                if latency > 0.0 {
                    stats.add(latency);
                }
            }

            (type_name, stats)
        })
        .collect();

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
                // "숫자ms" 또는 "숫자s" 패턴의 숫자 부분을 추출하고 단위를 고려
                fn extract_lower_bound_ms(s: &str) -> f64 {
                    // "X < v ≤ Y" 패턴에서 X 부분을 추출
                    if let Some(start) = s.find(" < v ≤ ") {
                        let left_part = &s[..start];
                        
                        // "숫자ms" 또는 "숫자s" 패턴에서 숫자 추출
                        if left_part.ends_with("ms") {
                            if let Ok(val) = left_part.replace("ms", "").parse::<f64>() {
                                return val; // ms는 그대로
                            }
                        } else if left_part.ends_with("s") {
                            if let Ok(val) = left_part.replace("s", "").parse::<f64>() {
                                return val * 1000.0; // s를 ms로 변환
                            }
                        }
                    }
                    0.0 // 기본값
                }

                let a_val = extract_lower_bound_ms(a);
                let b_val = extract_lower_bound_ms(b);

                a_val
                    .partial_cmp(&b_val)
                    .unwrap_or(std::cmp::Ordering::Equal)
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
    if ufs_traces.is_empty() {
        log!("UFS 트레이스가 비어 있습니다.");
        return;
    }
    print_trace_statistics(ufs_traces, "UFS");
}

pub fn print_block_statistics(block_traces: &[Block]) {
    if block_traces.is_empty() {
        log!("Block 트레이스가 비어 있습니다.");
        return;
    }
    print_trace_statistics(block_traces, "Block");
}

pub fn print_ufscustom_statistics(ufscustom_traces: &[UFSCUSTOM]) {
    // UFSCustom 데이터가 비어 있으면 바로 리턴
    if ufscustom_traces.is_empty() {
        log!("UFSCustom 트레이스가 비어 있습니다.");
        return;
    }

    // 기본 통계는 기존 함수 사용
    print_ufscustom_specific_statistics(ufscustom_traces);

    // 상세 통계는 generic 함수 사용 (UFS, Block와 동일한 형태)
    print_trace_statistics(ufscustom_traces, "UFSCustom");
}

/// UFSCUSTOM 전용 통계 함수
fn print_ufscustom_specific_statistics(traces: &[UFSCUSTOM]) {
    log!("Total UFSCustom requests: {}", traces.len());
    
    // Queue Depth 통계
    let max_qd = traces.iter().map(|t| t.start_qd).max().unwrap_or(0);
    log!("Maximum queue depth: {}", max_qd);
    
    // 평균 Queue Depth 계산
    let avg_qd = traces.iter().map(|t| t.start_qd as f64).sum::<f64>() / traces.len() as f64;
    log!("Average queue depth: {:.2}", avg_qd);

    // DTOC (Dispatch to Complete) 통계
    if !traces.is_empty() {
        let avg_dtoc = traces.iter().map(|t| t.dtoc).sum::<f64>() / traces.len() as f64;
        log!("Average Dispatch to Complete latency: {:.3} ms", avg_dtoc);
    }

    // CTOC (Complete to Complete) 통계
    let ctoc_traces: Vec<_> = traces.iter().filter(|t| t.ctoc > 0.0).collect();
    if !ctoc_traces.is_empty() {
        let avg_ctoc = ctoc_traces.iter().map(|t| t.ctoc).sum::<f64>() / ctoc_traces.len() as f64;
        log!("Average Complete to Complete latency: {:.3} ms", avg_ctoc);
    } else {
        log!("Average Complete to Complete latency: N/A ms");
    }

    // CTOD (Complete to Dispatch) 통계
    let ctod_traces: Vec<_> = traces.iter().filter(|t| t.ctod > 0.0).collect();
    if !ctod_traces.is_empty() {
        let avg_ctod = ctod_traces.iter().map(|t| t.ctod).sum::<f64>() / ctod_traces.len() as f64;
        log!("Average Complete to Dispatch latency: {:.3} ms", avg_ctod);
    } else {
        log!("Average Complete to Dispatch latency: N/A ms");
    }

    // Continuous 요청 비율
    let continuous_reqs = traces.iter().filter(|t| t.continuous).count();
    log!(
        "Continuous request ratio: {:.1}%",
        (continuous_reqs as f64 / traces.len() as f64) * 100.0
    );

    // Aligned 요청 비율
    let aligned_reqs = traces.iter().filter(|t| t.aligned).count();
    log!(
        "Aligned request ratio: {:.1}%",
        (aligned_reqs as f64 / traces.len() as f64) * 100.0
    );

    // 타입별 요청 수 집계
    let mut type_groups: HashMap<String, usize> = HashMap::new();
    for trace in traces {
        *type_groups.entry(trace.opcode.clone()).or_insert(0) += 1;
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
}
