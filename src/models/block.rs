use serde::{Deserialize, Serialize};
use bincode::{Encode, Decode};

#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct Block {
    pub time: f64,
    pub process: String,
    pub cpu: u32,
    pub flags: String,
    pub action: String,
    pub devmajor: u32,
    pub devminor: u32,
    pub io_type: String,
    pub extra: u32,
    pub sector: u64,
    pub size: u32,
    pub comm: String,
    pub qd: u32,   // Queue Depth
    pub dtoc: f64, // Device to Complete latency
    pub ctoc: f64, // Complete to Complete latency
    pub ctod: f64, // Complete to Device latency
    pub continuous: bool,
}
