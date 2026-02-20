use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

// UFS는 Universal Flash Storage의 약어이므로 UFs로 변경하지 않고 원래 이름 유지
#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct UFS {
    pub time: f64,
    pub process: Box<str>,
    pub cpu: u32,
    pub action: Box<str>,
    pub tag: u32,
    pub opcode: Box<str>,
    pub lba: u64,
    pub size: u32,
    pub groupid: u32,
    pub hwqid: u32,
    pub qd: u32,   // Queue Depth
    pub dtoc: f64, // Device to Complete latency
    pub ctoc: f64, // Complete to Complete latency
    pub ctod: f64, // Complete to Device latency
    pub continuous: bool,
    pub aligned: bool, // LBA alignment check
}
