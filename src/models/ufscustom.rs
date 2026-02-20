use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// UFS는 Universal Flash Storage의 약어이므로 UFs로 변경하지 않고 원래 이름 유지
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct UFSCUSTOM {
    pub opcode: Box<str>,
    pub lba: u64,
    pub size: u32,
    pub start_time: f64,
    pub end_time: f64,
    pub dtoc: f64,
    // 새로 추가할 필드들
    pub start_qd: u32,    // Queue Depth at request start
    pub end_qd: u32,      // Queue Depth at request end
    pub ctoc: f64,        // Complete to Complete latency (ms)
    pub ctod: f64,        // Complete to Dispatch latency (ms)
    pub continuous: bool, // 연속적인 요청 여부
    pub aligned: bool,    // LBA alignment check
}
