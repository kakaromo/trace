use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Encode, Decode)]
pub struct Block {
    pub time: f64,
    pub process: Box<str>,
    pub cpu: u32,
    pub flags: Box<str>,
    pub action: Box<str>,
    pub devmajor: u32,
    pub devminor: u32,
    pub io_type: Box<str>,
    pub extra: u32,
    pub sector: u64,
    pub size: u32,
    pub comm: Box<str>,
    pub qd: u32,   // Queue Depth
    pub dtoc: f64, // Device to Complete latency
    pub ctoc: f64, // Complete to Complete latency
    pub ctod: f64, // Complete to Device latency
    pub continuous: bool,
    pub aligned: bool, // Sector alignment check
}
