use regex::Regex;

/// 벤치마크 로그 라인의 타입
#[derive(Debug, Clone, PartialEq)]
pub enum LogLineType {
    /// FIO 성능 결과 라인
    FioResult { iteration: usize, test_type: String, bandwidth: f64 },
    /// TIOtest 성능 결과 라인
    TioTestResult { iteration: usize, test_type: String, bandwidth: f64 },
    /// IOzone 성능 결과 라인
    IOzoneResult { iteration: usize, test_type: String, bandwidth: f64 },
    /// UFS Trace 라인
    UfsTrace,
    /// Block Trace 라인
    BlockTrace,
    /// UFSCustom Trace 라인
    UfsCustomTrace,
    /// 일반 라인 (무시)
    Other,
}

/// 벤치마크 로그 파서
pub struct BenchmarkParser {
    fio_iteration_regex: Regex,
    fio_result_regex: Regex,
    tiotest_result_regex: Regex,
    iozone_seq_regex: Regex,
    iozone_rand_regex: Regex,
    ufs_trace_regex: Regex,
    block_trace_regex: Regex,
    ufscustom_trace_regex: Regex,
}

impl BenchmarkParser {
    pub fn new() -> Self {
        Self {
            // FIO iteration 감지: "--- FIO 1GB Sequential Write Test (Iteration 1) ---"
            fio_iteration_regex: Regex::new(r"---\s+FIO.*\(Iteration\s+(\d+)\)\s+---").unwrap(),
            // FIO 결과 감지: "WRITE: bw=604MiB/s" 또는 "READ: bw=3190MiB/s"
            fio_result_regex: Regex::new(r"(WRITE|READ):\s+bw=(\d+\.?\d*)MiB/s").unwrap(),
            // TIOtest 결과 감지: "| Write        1024 MBs |    0.5 s | 1938.124 MB/s |"
            tiotest_result_regex: Regex::new(r"\|\s+(Write|Read|Random Write|Random Read)\s+\d+\s+MBs\s+\|.*\|\s+(\d+\.?\d*)\s+MB/s\s+\|").unwrap(),
            // IOzone sequential 결과 감지: "         1048576    1024   2226634         0   9045852"
            iozone_seq_regex: Regex::new(r"^\s+\d+\s+\d+\s+(\d+)\s+\d+\s+(\d+)").unwrap(),
            // IOzone random 결과 감지: "Parent sees throughput for 8 random readers = 257470.02 kB/sec"
            iozone_rand_regex: Regex::new(r"Parent sees throughput for \d+ random (readers|writers)\s+=\s+(\d+\.?\d*)\s+kB/sec").unwrap(),
            // UFS trace 감지: "ufshcd_command:"
            ufs_trace_regex: Regex::new(r"ufshcd_command:").unwrap(),
            // Block trace 감지: "block_rq_"
            block_trace_regex: Regex::new(r"block_rq_").unwrap(),
            // UFSCustom trace 감지 (예시 패턴, 실제 패턴에 맞게 수정 필요)
            ufscustom_trace_regex: Regex::new(r"ufscustom_").unwrap(),
        }
    }

    /// 로그 라인 타입 감지
    pub fn detect_line_type(&self, line: &str, current_iteration: &mut usize) -> LogLineType {
        // FIO iteration 감지
        if let Some(caps) = self.fio_iteration_regex.captures(line) {
            if let Ok(iter) = caps[1].parse::<usize>() {
                *current_iteration = iter;
            }
            return LogLineType::Other;
        }

        // FIO 결과 감지
        if let Some(caps) = self.fio_result_regex.captures(line) {
            let test_type = caps[1].to_string();
            if let Ok(bandwidth) = caps[2].parse::<f64>() {
                return LogLineType::FioResult {
                    iteration: *current_iteration,
                    test_type,
                    bandwidth,
                };
            }
        }

        // TIOtest 결과 감지
        if let Some(caps) = self.tiotest_result_regex.captures(line) {
            let test_type = caps[1].to_string();
            if let Ok(bandwidth) = caps[2].parse::<f64>() {
                // TIOtest는 모든 테스트를 순차적으로 수행하므로
                // Write -> Read -> Random Write -> Random Read 순서로 iteration 증가
                if test_type.contains("Write") && !test_type.contains("Random") {
                    *current_iteration += 1;
                }
                return LogLineType::TioTestResult {
                    iteration: *current_iteration,
                    test_type,
                    bandwidth,
                };
            }
        }

        // IOzone sequential 결과 감지
        if let Some(caps) = self.iozone_seq_regex.captures(line) {
            if let (Ok(write_bw), Ok(_read_bw)) = (caps[1].parse::<f64>(), caps[2].parse::<f64>()) {
                *current_iteration += 1;
                // Sequential write와 read를 하나의 iteration으로 처리
                return LogLineType::IOzoneResult {
                    iteration: *current_iteration,
                    test_type: "Sequential".to_string(),
                    bandwidth: write_bw, // write 성능만 저장 (필요시 둘 다 저장 가능)
                };
            }
        }

        // IOzone random 결과 감지
        if let Some(caps) = self.iozone_rand_regex.captures(line) {
            let test_type = if caps[1].to_string() == "readers" { "Random Read" } else { "Random Write" };
            if let Ok(bandwidth) = caps[2].parse::<f64>() {
                return LogLineType::IOzoneResult {
                    iteration: *current_iteration,
                    test_type: test_type.to_string(),
                    bandwidth: bandwidth / 1024.0, // kB/sec to MB/sec
                };
            }
        }

        // Trace 라인 감지
        if self.ufs_trace_regex.is_match(line) {
            return LogLineType::UfsTrace;
        }

        if self.block_trace_regex.is_match(line) {
            return LogLineType::BlockTrace;
        }

        if self.ufscustom_trace_regex.is_match(line) {
            return LogLineType::UfsCustomTrace;
        }

        LogLineType::Other
    }

    /// 벤치마크 결과를 CSV 형식으로 추출
    pub fn extract_benchmark_results(&self, log_content: &str) -> Vec<BenchmarkResult> {
        let mut results = Vec::new();
        let mut current_iteration = 0;

        for line in log_content.lines() {
            match self.detect_line_type(line, &mut current_iteration) {
                LogLineType::FioResult { iteration, test_type, bandwidth } => {
                    results.push(BenchmarkResult {
                        tool: "FIO".to_string(),
                        iteration,
                        test_type,
                        bandwidth,
                    });
                }
                LogLineType::TioTestResult { iteration, test_type, bandwidth } => {
                    results.push(BenchmarkResult {
                        tool: "TIOtest".to_string(),
                        iteration,
                        test_type,
                        bandwidth,
                    });
                }
                LogLineType::IOzoneResult { iteration, test_type, bandwidth } => {
                    results.push(BenchmarkResult {
                        tool: "IOzone".to_string(),
                        iteration,
                        test_type,
                        bandwidth,
                    });
                }
                _ => {}
            }
        }

        results
    }
}

impl Default for BenchmarkParser {
    fn default() -> Self {
        Self::new()
    }
}

/// 벤치마크 결과 구조체
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub tool: String,
    pub iteration: usize,
    pub test_type: String,
    pub bandwidth: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fio_iteration_detection() {
        let parser = BenchmarkParser::new();
        let mut current_iter = 0;
        
        let line = "--- FIO 1GB Sequential Write Test (Iteration 1) ---";
        parser.detect_line_type(line, &mut current_iter);
        assert_eq!(current_iter, 1);
    }

    #[test]
    fn test_fio_result_detection() {
        let parser = BenchmarkParser::new();
        let mut current_iter = 1;
        
        let line = "  WRITE: bw=604MiB/s (633MB/s), 604MiB/s-604MiB/s (633MB/s-633MB/s), io=1024MiB (1074MB), run=1695-1695msec";
        let result = parser.detect_line_type(line, &mut current_iter);
        
        match result {
            LogLineType::FioResult { iteration, test_type, bandwidth } => {
                assert_eq!(iteration, 1);
                assert_eq!(test_type, "WRITE");
                assert_eq!(bandwidth, 604.0);
            }
            _ => panic!("Expected FioResult"),
        }
    }

    #[test]
    fn test_trace_detection() {
        let parser = BenchmarkParser::new();
        let mut current_iter = 1;
        
        let ufs_line = "    kworker/1:1H-175     [001] ..... 22218.735851: ufshcd_command: send_req:";
        let result = parser.detect_line_type(ufs_line, &mut current_iter);
        assert_eq!(result, LogLineType::UfsTrace);
    }
}
