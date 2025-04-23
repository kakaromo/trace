pub mod models;
pub mod output;
pub mod parsers;
pub mod processors;
pub mod utils;

// 주요 기능 재내보내기(re-exporting)
pub use models::{Block, TraceType, UFS};
pub use output::{
    generate_charts, print_block_statistics, print_ufs_statistics, read_block_from_parquet,
    read_ufs_from_parquet, save_to_parquet,
};
pub use parsers::log::parse_log_file;
pub use processors::{block_bottom_half_latency_process, ufs_bottom_half_latency_process};

// 새로운 트레이스 타입을 추가할 때는 다음과 같이 구성하면 됩니다:
// 1. models/ 디렉토리에 새 트레이스 타입 구조체 추가 (예: nvme.rs)
// 2. processors/ 디렉토리에 새 트레이스 처리 모듈 추가 (예: nvme.rs)
// 3. output/ 디렉토리에 새 트레이스 통계 및 차트 함수 추가
// 4. models/trace_type.rs에 TraceType 열거형에 새 트레이스 타입 추가
//
// 예시:
// pub use models::NVMe;
// pub use output::{print_nvme_statistics, read_nvme_from_parquet};
// pub use processors::nvme_bottom_half_latency_process;
