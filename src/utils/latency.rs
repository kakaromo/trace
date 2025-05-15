//! 사용자 정의 레이턴시 범위 처리를 위한 유틸리티 모듈

use std::sync::Mutex;
use std::sync::OnceLock;

/// 사용자 정의 레이턴시 범위를 저장하는 구조체
#[derive(Debug, Clone, Default)]
pub struct UserLatencyRanges {
    pub ranges: Vec<f64>,
}

// 전역 상태로 레이턴시 범위 저장 (thread-safe)
static LATENCY_RANGES: OnceLock<Mutex<Option<UserLatencyRanges>>> = OnceLock::new();

/// 사용자 정의 레이턴시 범위 설정
pub fn set_user_latency_ranges(ranges: Vec<f64>) {
    let mutex = LATENCY_RANGES.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = mutex.lock() {
        *guard = Some(UserLatencyRanges { ranges });
    }
}

/// 사용자 정의 레이턴시 범위 가져오기
pub fn get_user_latency_ranges() -> Option<Vec<f64>> {
    let mutex = LATENCY_RANGES.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = mutex.lock() {
        guard.as_ref().map(|ranges| ranges.ranges.clone())
    } else {
        None
    }
}

/// 입력 문자열에서 쉼표(,)로 구분된 레이턴시 범위를 파싱
pub fn parse_latency_ranges(value_str: &str) -> Result<Vec<f64>, String> {
    let mut ranges = Vec::new();

    for val in value_str.split(',') {
        match val.trim().parse::<f64>() {
            Ok(v) if v >= 0.0 => ranges.push(v),
            Ok(_) => return Err("Latency range values must be non-negative".to_string()),
            Err(_) => return Err(format!("Invalid latency range value: {}", val)),
        }
    }

    // 값이 오름차순인지 확인
    for i in 1..ranges.len() {
        if ranges[i] <= ranges[i - 1] {
            return Err("Latency range values must be in ascending order".to_string());
        }
    }

    if ranges.is_empty() {
        return Err("No valid latency range values provided".to_string());
    }

    Ok(ranges)
}
