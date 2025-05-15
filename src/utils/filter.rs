use std::io::{self, BufRead};

// 필터링 옵션을 저장할 구조체 정의
#[derive(Debug, Clone)]
pub struct FilterOptions {
    pub start_time: f64,   // 시작 시간 (ms)
    pub end_time: f64,     // 종료 시간 (ms)
    pub start_sector: u64, // 시작 섹터/LBA
    pub end_sector: u64,   // 종료 섹터/LBA
}

impl Default for FilterOptions {
    fn default() -> Self {
        FilterOptions {
            start_time: 0.0,
            end_time: 0.0,
            start_sector: 0,
            end_sector: 0,
        }
    }
}

impl FilterOptions {
    // 필터 옵션이 활성화되어 있는지 확인
    pub fn is_time_filter_active(&self) -> bool {
        self.start_time > 0.0 && self.end_time > 0.0
    }

    pub fn is_sector_filter_active(&self) -> bool {
        self.start_sector > 0 && self.end_sector > 0
    }

    // UFS LBA로 변환 (4KB = 8 섹터)
    pub fn to_ufs_lba(&self) -> FilterOptions {
        FilterOptions {
            start_time: self.start_time,
            end_time: self.end_time,
            start_sector: if self.start_sector > 0 {
                self.start_sector / 8
            } else {
                0
            },
            end_sector: if self.end_sector > 0 {
                self.end_sector / 8
            } else {
                0
            },
        }
    }
}

// 사용자로부터 필터 옵션을 입력받는 함수
pub fn read_filter_options() -> io::Result<FilterOptions> {
    let stdin = io::stdin();
    let mut filter = FilterOptions::default();

    // 시작 시간 입력
    println!("start time : ");
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.start_time = value;
        }
    }

    // 종료 시간 입력
    println!("end time : ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<f64>() {
        if value > 0.0 {
            filter.end_time = value;
        }
    }

    // 시작 섹터/LBA 입력
    println!("start sector/lba : ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u64>() {
        if value > 0 {
            filter.start_sector = value;
        }
    }

    // 종료 섹터/LBA 입력
    println!("end sector/lba : ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    if let Ok(value) = input.trim().parse::<u64>() {
        if value > 0 {
            filter.end_sector = value;
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
    if !filter.is_time_filter_active() && !filter.is_sector_filter_active() {
        return block_data;
    }

    block_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용
            let time_match = if filter.is_time_filter_active() {
                item.time >= filter.start_time && item.time <= filter.end_time
            } else {
                true
            };

            // 섹터 필터 적용
            let sector_match = if filter.is_sector_filter_active() {
                // 범위가 겹치는지 확인 (item.sector부터 item.sector + item.size까지)
                let item_end_sector = item.sector + item.size as u64 / 512;
                !(item_end_sector < filter.start_sector || item.sector > filter.end_sector)
            } else {
                true
            };

            // 두 필터 조건 모두 만족해야 함
            time_match && sector_match
        })
        .collect()
}

// UFS 데이터 필터링 함수 (4KB LBA로 변환 적용)
pub fn filter_ufs_data(ufs_data: Vec<crate::UFS>, filter: &FilterOptions) -> Vec<crate::UFS> {
    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active() && !filter.is_sector_filter_active() {
        return ufs_data;
    }

    // UFS LBA로 변환된 필터 옵션 사용
    let ufs_filter = filter.to_ufs_lba();

    ufs_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용
            let time_match = if filter.is_time_filter_active() {
                item.time >= filter.start_time && item.time <= filter.end_time
            } else {
                true
            };

            // LBA 필터 적용
            let lba_match = if ufs_filter.is_sector_filter_active() {
                // 범위가 겹치는지 확인 (item.lba부터 item.lba + item.size까지)
                let item_end_lba = item.lba + item.size as u64 / 4096;
                !(item_end_lba < ufs_filter.start_sector || item.lba > ufs_filter.end_sector)
            } else {
                true
            };

            // 두 필터 조건 모두 만족해야 함
            time_match && lba_match
        })
        .collect()
}

// UFSCUSTOM 데이터 필터링 함수 (start_lba 기준)
pub fn filter_ufscustom_data(
    ufscustom_data: Vec<crate::UFSCUSTOM>,
    filter: &FilterOptions,
) -> Vec<crate::UFSCUSTOM> {
    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active() && !filter.is_sector_filter_active() {
        return ufscustom_data;
    }

    // UFS LBA로 변환된 필터 옵션 사용
    let ufs_filter = filter.to_ufs_lba();

    ufscustom_data
        .into_iter()
        .filter(|item| {
            // 시간 필터 적용 (start_time과 end_time 사용)
            let time_match = if filter.is_time_filter_active() {
                // 시간 범위가 겹치는지 확인
                !(item.end_time < filter.start_time || item.start_time > filter.end_time)
            } else {
                true
            };

            // LBA 필터 적용 (start_lba 기준)
            let lba_match = if ufs_filter.is_sector_filter_active() {
                // 범위가 겹치는지 확인 (item.lba부터 item.lba + item.size까지)
                let item_end_lba = item.lba + item.size as u64 / 4096;
                !(item_end_lba < ufs_filter.start_sector || item.lba > ufs_filter.end_sector)
            } else {
                true
            };

            // 두 필터 조건 모두 만족해야 함
            time_match && lba_match
        })
        .collect()
}
