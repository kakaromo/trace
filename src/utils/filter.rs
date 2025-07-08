use std::io::{self, BufRead};

// 필터링 옵션을 저장할 구조체 정의
#[derive(Debug, Clone)]
pub struct FilterOptions {
    pub start_time: f64,   // 시작 시간 (ms)
    pub end_time: f64,     // 종료 시간 (ms)
    pub start_sector: u64, // 시작 섹터/LBA
    pub end_sector: u64,   // 종료 섹터/LBA
    // 레이턴시 필터링 옵션
    pub min_dtoc: f64,     // 최소 Device to Complete 레이턴시 (ms)
    pub max_dtoc: f64,     // 최대 Device to Complete 레이턴시 (ms)
    pub min_ctoc: f64,     // 최소 Complete to Complete 레이턴시 (ms)
    pub max_ctoc: f64,     // 최대 Complete to Complete 레이턴시 (ms)
    pub min_ctod: f64,     // 최소 Complete to Device 레이턴시 (ms)
    pub max_ctod: f64,     // 최대 Complete to Device 레이턴시 (ms)
    pub min_qd: u32,       // 최소 Queue Depth
    pub max_qd: u32,       // 최대 Queue Depth
}

impl Default for FilterOptions {
    fn default() -> Self {
        FilterOptions {
            start_time: 0.0,
            end_time: 0.0,
            start_sector: 0,
            end_sector: 0,
            min_dtoc: 0.0,
            max_dtoc: 0.0,
            min_ctoc: 0.0,
            max_ctoc: 0.0,
            min_ctod: 0.0,
            max_ctod: 0.0,
            min_qd: 0,
            max_qd: 0,
        }
    }
}

impl FilterOptions {
    // 필터 옵션이 활성화되어 있는지 확인
    pub fn is_time_filter_active(&self) -> bool {
        self.start_time > 0.0 || self.end_time > 0.0
    }

    pub fn is_sector_filter_active(&self) -> bool {
        self.start_sector > 0 || self.end_sector > 0
    }

    // 레이턴시 필터가 활성화되어 있는지 확인
    pub fn is_dtoc_filter_active(&self) -> bool {
        self.min_dtoc > 0.0 || self.max_dtoc > 0.0
    }

    pub fn is_ctoc_filter_active(&self) -> bool {
        self.min_ctoc > 0.0 || self.max_ctoc > 0.0
    }

    pub fn is_ctod_filter_active(&self) -> bool {
        self.min_ctod > 0.0 || self.max_ctod > 0.0
    }

    pub fn is_qd_filter_active(&self) -> bool {
        self.min_qd > 0 || self.max_qd > 0
    }

    // UFS LBA로 변환 (4KB = 8 섹터)
    pub fn to_ufs_lba(&self) -> FilterOptions {
        FilterOptions {
            start_time: self.start_time,
            end_time: self.end_time,
            start_sector: if self.start_sector > 0 {
                self.start_sector
            } else {
                0
            },
            end_sector: if self.end_sector > 0 {
                self.end_sector
            } else {
                0
            },
            min_dtoc: self.min_dtoc,
            max_dtoc: self.max_dtoc,
            min_ctoc: self.min_ctoc,
            max_ctoc: self.max_ctoc,
            min_ctod: self.min_ctod,
            max_ctod: self.max_ctod,
            min_qd: self.min_qd,
            max_qd: self.max_qd,
        }
    }
}

// 사용자로부터 필터 옵션을 입력받는 함수
pub fn read_filter_options() -> io::Result<FilterOptions> {
    let stdin = io::stdin();
    let mut filter = FilterOptions::default();

    // 시작 시간 입력
    println!("start time (ms, 0 to skip): ");
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.start_time = value;
        }
    }

    // 종료 시간 입력
    println!("end time (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.end_time = value;
        }
    }

    // 시작 섹터/LBA 입력
    println!("start sector/lba (0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u64>() {
        if value > 0 {
            filter.start_sector = value;
        }
    }

    // 종료 섹터/LBA 입력
    println!("end sector/lba (0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u64>() {
        if value > 0 {
            filter.end_sector = value;
        }
    }

    // DTOC 최소값 입력
    println!("min dtoc latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.min_dtoc = value;
        }
    }

    // DTOC 최대값 입력
    println!("max dtoc latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.max_dtoc = value;
        }
    }

    // CTOC 최소값 입력
    println!("min ctoc latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.min_ctoc = value;
        }
    }

    // CTOC 최대값 입력
    println!("max ctoc latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.max_ctoc = value;
        }
    }

    // CTOD 최소값 입력
    println!("min ctod latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.min_ctod = value;
        }
    }

    // CTOD 최대값 입력
    println!("max ctod latency (ms, 0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.max_ctod = value;
        }
    }

    // QD 최소값 입력
    println!("min queue depth (0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u32>() {
        if value > 0 {
            filter.min_qd = value;
        }
    }

    // QD 최대값 입력
    println!("max queue depth (0 to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u32>() {
        if value > 0 {
            filter.max_qd = value;
        }
    }

    Ok(filter)
}

// Block 데이터 필터링 함수
pub fn filter_block_data(
    block_data: Vec<crate::Block>,
    filter: &FilterOptions,
) -> Vec<crate::Block> {
    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active() 
        && !filter.is_sector_filter_active()
        && !filter.is_dtoc_filter_active()
        && !filter.is_ctoc_filter_active()
        && !filter.is_ctod_filter_active()
        && !filter.is_qd_filter_active() {
        return block_data;
    }

    block_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용
            let time_match = if filter.is_time_filter_active() {
                let start_check = filter.start_time > 0.0;
                let end_check = filter.end_time > 0.0;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    item.time >= filter.start_time
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    item.time >= 0.0 && item.time <= filter.end_time
                }
                // 둘 다 설정된 경우
                else if start_check && end_check {
                    item.time >= filter.start_time && item.time <= filter.end_time
                }
                else {
                    true
                }
            } else {
                true
            };

            // 섹터 필터 적용
            let sector_match = if filter.is_sector_filter_active() {
                let start_check = filter.start_sector > 0;
                let end_check = filter.end_sector > 0;
                let item_end_sector = item.sector + item.size as u64;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    // item의 섹터 범위가 filter.start_sector와 겹치는지 확인
                    item_end_sector >= filter.start_sector
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    // item의 섹터 범위가 0부터 filter.end_sector 사이에 있는지 확인
                    item.sector <= filter.end_sector
                }
                // 둘 다 설정된 경우 - 범위가 겹치는지 확인
                else if start_check && end_check {
                    !(item_end_sector < filter.start_sector || item.sector > filter.end_sector)
                }
                else {
                    true
                }
            } else {
                true
            };

            // DTOC 필터 적용
            let dtoc_match = if filter.is_dtoc_filter_active() {
                let min_check = filter.min_dtoc > 0.0;
                let max_check = filter.max_dtoc > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.dtoc >= filter.min_dtoc
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.dtoc >= 0.0 && item.dtoc <= filter.max_dtoc
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.dtoc >= filter.min_dtoc && item.dtoc <= filter.max_dtoc
                }
                else {
                    true
                }
            } else {
                true
            };

            // CTOC 필터 적용
            let ctoc_match = if filter.is_ctoc_filter_active() {
                let min_check = filter.min_ctoc > 0.0;
                let max_check = filter.max_ctoc > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.ctoc >= filter.min_ctoc
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.ctoc >= 0.0 && item.ctoc <= filter.max_ctoc
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.ctoc >= filter.min_ctoc && item.ctoc <= filter.max_ctoc
                }
                else {
                    true
                }
            } else {
                true
            };

            // CTOD 필터 적용
            let ctod_match = if filter.is_ctod_filter_active() {
                let min_check = filter.min_ctod > 0.0;
                let max_check = filter.max_ctod > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.ctod >= filter.min_ctod
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.ctod >= 0.0 && item.ctod <= filter.max_ctod
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.ctod >= filter.min_ctod && item.ctod <= filter.max_ctod
                }
                else {
                    true
                }
            } else {
                true
            };

            // QD 필터 적용
            let qd_match = if filter.is_qd_filter_active() {
                let min_check = filter.min_qd > 0;
                let max_check = filter.max_qd > 0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.qd >= filter.min_qd
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.qd <= filter.max_qd
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.qd >= filter.min_qd && item.qd <= filter.max_qd
                }
                else {
                    true
                }
            } else {
                true
            };

            // 모든 필터 조건을 만족해야 함
            time_match && sector_match && dtoc_match && ctoc_match && ctod_match && qd_match
        })
        .collect()
}

// UFS 데이터 필터링 함수 (4KB LBA로 변환 적용)
pub fn filter_ufs_data(ufs_data: Vec<crate::UFS>, filter: &FilterOptions) -> Vec<crate::UFS> {
    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active() 
        && !filter.is_sector_filter_active()
        && !filter.is_dtoc_filter_active()
        && !filter.is_ctoc_filter_active()
        && !filter.is_ctod_filter_active()
        && !filter.is_qd_filter_active() {
        return ufs_data;
    }

    // UFS LBA로 변환된 필터 옵션 사용
    let ufs_filter = filter.to_ufs_lba();

    ufs_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용
            let time_match = if filter.is_time_filter_active() {
                let start_check = filter.start_time > 0.0;
                let end_check = filter.end_time > 0.0;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    item.time >= filter.start_time
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    item.time >= 0.0 && item.time <= filter.end_time
                }
                // 둘 다 설정된 경우
                else if start_check && end_check {
                    item.time >= filter.start_time && item.time <= filter.end_time
                }
                else {
                    true
                }
            } else {
                true
            };

            // LBA 필터 적용
            let lba_match = if ufs_filter.is_sector_filter_active() {
                let start_check = ufs_filter.start_sector > 0;
                let end_check = ufs_filter.end_sector > 0;
                let item_end_lba = item.lba + item.size as u64 / 4096;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    // item의 LBA 범위가 filter.start_sector와 겹치는지 확인
                    item_end_lba >= ufs_filter.start_sector
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    // item의 LBA 범위가 0부터 filter.end_sector 사이에 있는지 확인
                    item.lba <= ufs_filter.end_sector
                }
                // 둘 다 설정된 경우 - 범위가 겹치는지 확인
                else if start_check && end_check {
                    !(item_end_lba < ufs_filter.start_sector || item.lba > ufs_filter.end_sector)
                }
                else {
                    true
                }
            } else {
                true
            };

            // DTOC 필터 적용
            let dtoc_match = if filter.is_dtoc_filter_active() {
                let min_check = filter.min_dtoc > 0.0;
                let max_check = filter.max_dtoc > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.dtoc >= filter.min_dtoc
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.dtoc >= 0.0 && item.dtoc <= filter.max_dtoc
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.dtoc >= filter.min_dtoc && item.dtoc <= filter.max_dtoc
                }
                else {
                    true
                }
            } else {
                true
            };

            // CTOC 필터 적용
            let ctoc_match = if filter.is_ctoc_filter_active() {
                let min_check = filter.min_ctoc > 0.0;
                let max_check = filter.max_ctoc > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.ctoc >= filter.min_ctoc
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.ctoc >= 0.0 && item.ctoc <= filter.max_ctoc
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.ctoc >= filter.min_ctoc && item.ctoc <= filter.max_ctoc
                }
                else {
                    true
                }
            } else {
                true
            };

            // CTOD 필터 적용
            let ctod_match = if filter.is_ctod_filter_active() {
                let min_check = filter.min_ctod > 0.0;
                let max_check = filter.max_ctod > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.ctod >= filter.min_ctod
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.ctod >= 0.0 && item.ctod <= filter.max_ctod
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.ctod >= filter.min_ctod && item.ctod <= filter.max_ctod
                }
                else {
                    true
                }
            } else {
                true
            };

            // QD 필터 적용
            let qd_match = if filter.is_qd_filter_active() {
                let min_check = filter.min_qd > 0;
                let max_check = filter.max_qd > 0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.qd >= filter.min_qd
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.qd <= filter.max_qd
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.qd >= filter.min_qd && item.qd <= filter.max_qd
                }
                else {
                    true
                }
            } else {
                true
            };

            // 모든 필터 조건을 만족해야 함
            time_match && lba_match && dtoc_match && ctoc_match && ctod_match && qd_match
        })
        .collect()
}

// UFSCUSTOM 데이터 필터링 함수 (start_lba 기준, dtoc만 적용)
pub fn filter_ufscustom_data(
    ufscustom_data: Vec<crate::UFSCUSTOM>,
    filter: &FilterOptions,
) -> Vec<crate::UFSCUSTOM> {
    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active() 
        && !filter.is_sector_filter_active()
        && !filter.is_dtoc_filter_active() {
        return ufscustom_data;
    }

    // UFS LBA로 변환된 필터 옵션 사용
    let ufs_filter = filter.to_ufs_lba();

    ufscustom_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용 (start_time과 end_time 사용)
            let time_match = if filter.is_time_filter_active() {
                let start_check = filter.start_time > 0.0;
                let end_check = filter.end_time > 0.0;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    // item.end_time이 filter.start_time보다 크거나 같아야 함
                    item.end_time >= filter.start_time
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    // item.start_time이 filter.end_time보다 작거나 같아야 함
                    item.start_time <= filter.end_time
                }
                // 둘 다 설정된 경우 - 범위가 겹치는지 확인
                else if start_check && end_check {
                    !(item.end_time < filter.start_time || item.start_time > filter.end_time)
                }
                else {
                    true
                }
            } else {
                true
            };

            // LBA 필터 적용 (start_lba 기준)
            let lba_match = if ufs_filter.is_sector_filter_active() {
                let start_check = ufs_filter.start_sector > 0;
                let end_check = ufs_filter.end_sector > 0;
                let item_end_lba = item.lba + item.size as u64 / 4096;
                
                // start만 설정된 경우
                if start_check && !end_check {
                    // item의 LBA 범위가 filter.start_sector와 겹치는지 확인
                    item_end_lba >= ufs_filter.start_sector
                }
                // end만 설정된 경우: 0부터 end까지 허용
                else if !start_check && end_check {
                    // item의 LBA 범위가 0부터 filter.end_sector 사이에 있는지 확인
                    item.lba <= ufs_filter.end_sector
                }
                // 둘 다 설정된 경우 - 범위가 겹치는지 확인
                else if start_check && end_check {
                    !(item_end_lba < ufs_filter.start_sector || item.lba > ufs_filter.end_sector)
                }
                else {
                    true
                }
            } else {
                true
            };

            // DTOC 필터 적용 (UFSCUSTOM은 dtoc만 가지고 있음)
            let dtoc_match = if filter.is_dtoc_filter_active() {
                let min_check = filter.min_dtoc > 0.0;
                let max_check = filter.max_dtoc > 0.0;
                
                // min만 설정된 경우
                if min_check && !max_check {
                    item.dtoc >= filter.min_dtoc
                }
                // max만 설정된 경우: 0부터 max까지 허용
                else if !min_check && max_check {
                    item.dtoc >= 0.0 && item.dtoc <= filter.max_dtoc
                }
                // 둘 다 설정된 경우
                else if min_check && max_check {
                    item.dtoc >= filter.min_dtoc && item.dtoc <= filter.max_dtoc
                }
                else {
                    true
                }
            } else {
                true
            };

            // 모든 필터 조건을 만족해야 함 (ctoc, ctod, qd는 UFSCUSTOM에 없으므로 제외)
            time_match && lba_match && dtoc_match
        })
        .collect()
}
