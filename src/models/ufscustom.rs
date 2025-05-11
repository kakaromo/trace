use serde::{Deserialize, Serialize};

// UFS는 Universal Flash Storage의 약어이므로 UFs로 변경하지 않고 원래 이름 유지
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UFSCUSTOM {
    pub opcode: String,
    pub lba: u64,
    pub size: u32,
    pub start_time: f64,
    pub end_time: f64,
    pub dtoc: f64,
}