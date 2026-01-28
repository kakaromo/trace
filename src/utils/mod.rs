pub mod compression;
pub mod constants;
pub mod encoding;
pub mod filter;
pub mod iteration;
pub mod latency;
pub mod logger;
pub mod performance;

use std::sync::OnceLock;

/// Alignment configuration
#[derive(Debug, Clone)]
pub struct AlignmentConfig {
    pub alignment_size_kb: u64, // Alignment size in KB
}

impl Default for AlignmentConfig {
    fn default() -> Self {
        Self {
            alignment_size_kb: 64, // 64KB default
        }
    }
}

static ALIGNMENT_CONFIG: OnceLock<AlignmentConfig> = OnceLock::new();

/// Set alignment configuration
pub fn set_alignment_config(config: AlignmentConfig) {
    ALIGNMENT_CONFIG.set(config).ok();
}

/// Get alignment configuration
pub fn get_alignment_config() -> &'static AlignmentConfig {
    ALIGNMENT_CONFIG.get_or_init(AlignmentConfig::default)
}

/// Check if LBA/sector is aligned for UFS (4KB units)
pub fn is_ufs_aligned(lba: u64) -> bool {
    let config = get_alignment_config();
    let alignment_units = config.alignment_size_kb / 4; // Convert KB to 4KB units
    lba.is_multiple_of(alignment_units)
}

/// Check if sector is aligned for Block (512-byte sectors)
pub fn is_block_aligned(sector: u64) -> bool {
    let config = get_alignment_config();
    let alignment_sectors = (config.alignment_size_kb * 1024) / 512; // Convert KB to sectors
    sector.is_multiple_of(alignment_sectors)
}

pub use self::encoding::{open_encoded_reader, read_to_string_auto, EncodedBufReader};
pub use self::filter::{
    filter_block_data, filter_ufs_data, filter_ufscustom_data, read_filter_options, FilterOptions,
};
pub use self::iteration::IterationOutputManager;
pub use self::latency::{
    get_user_latency_ranges, parse_latency_ranges, set_user_latency_ranges, UserLatencyRanges,
};
pub use self::logger::Logger;
pub use self::performance::{
    calculate_optimal_chunk_size, MemoryMonitor, PerformanceMetrics, PerformanceProfiler,
    SystemMemoryInfo,
};
