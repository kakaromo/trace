use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::io::{self, BufRead};

/// 필터링 가능한 트레이스 데이터를 위한 트레이트
pub trait Filterable {
    /// 시간 필터 매칭 (단일 시점 또는 범위)
    fn matches_time(&self, filter: &FilterOptions) -> bool;
    /// 섹터/LBA 필터 매칭
    fn matches_sector(&self, filter: &FilterOptions) -> bool;
    fn dtoc(&self) -> f64;
    fn ctoc(&self) -> f64;
    fn ctod(&self) -> f64;
    fn qd(&self) -> u32;
    /// CPU 번호 (없는 경우 None 반환)
    fn cpu(&self) -> Option<u32>;
}

impl Filterable for crate::Block {
    fn matches_time(&self, filter: &FilterOptions) -> bool {
        match_time_single(self.time, filter)
    }
    fn matches_sector(&self, filter: &FilterOptions) -> bool {
        match_sector_range(self.sector, self.sector + self.size as u64, filter)
    }
    fn dtoc(&self) -> f64 { self.dtoc }
    fn ctoc(&self) -> f64 { self.ctoc }
    fn ctod(&self) -> f64 { self.ctod }
    fn qd(&self) -> u32 { self.qd }
    fn cpu(&self) -> Option<u32> { Some(self.cpu) }
}

impl Filterable for crate::UFS {
    fn matches_time(&self, filter: &FilterOptions) -> bool {
        match_time_single(self.time, filter)
    }
    fn matches_sector(&self, filter: &FilterOptions) -> bool {
        let ufs_filter = filter.to_ufs_lba();
        match_sector_range(self.lba, self.lba + self.size as u64 / 4096, &ufs_filter)
    }
    fn dtoc(&self) -> f64 { self.dtoc }
    fn ctoc(&self) -> f64 { self.ctoc }
    fn ctod(&self) -> f64 { self.ctod }
    fn qd(&self) -> u32 { self.qd }
    fn cpu(&self) -> Option<u32> { Some(self.cpu) }
}

impl Filterable for crate::UFSCUSTOM {
    fn matches_time(&self, filter: &FilterOptions) -> bool {
        match_time_range(self.start_time, self.end_time, filter)
    }
    fn matches_sector(&self, filter: &FilterOptions) -> bool {
        let ufs_filter = filter.to_ufs_lba();
        match_sector_range(self.lba, self.lba + self.size as u64 / 4096, &ufs_filter)
    }
    fn dtoc(&self) -> f64 { self.dtoc }
    fn ctoc(&self) -> f64 { self.ctoc }
    fn ctod(&self) -> f64 { self.ctod }
    fn qd(&self) -> u32 { self.start_qd }
    fn cpu(&self) -> Option<u32> { None }
}

/// 단일 시점 time에 대한 시간 필터 매칭
#[inline]
fn match_time_single(time: f64, filter: &FilterOptions) -> bool {
    if !filter.is_time_filter_active() {
        return true;
    }
    let start_check = filter.start_time > 0.0;
    let end_check = filter.end_time > 0.0;
    if start_check && !end_check {
        time >= filter.start_time
    } else if !start_check && end_check {
        time >= 0.0 && time <= filter.end_time
    } else if start_check && end_check {
        time >= filter.start_time && time <= filter.end_time
    } else {
        true
    }
}

/// 시간 범위(start_time~end_time)에 대한 시간 필터 매칭
#[inline]
fn match_time_range(start_time: f64, end_time: f64, filter: &FilterOptions) -> bool {
    if !filter.is_time_filter_active() {
        return true;
    }
    let start_check = filter.start_time > 0.0;
    let end_check = filter.end_time > 0.0;
    if start_check && !end_check {
        end_time >= filter.start_time
    } else if !start_check && end_check {
        start_time <= filter.end_time
    } else if start_check && end_check {
        !(end_time < filter.start_time || start_time > filter.end_time)
    } else {
        true
    }
}

/// 섹터/LBA 범위에 대한 필터 매칭
#[inline]
fn match_sector_range(start: u64, end: u64, filter: &FilterOptions) -> bool {
    if !filter.is_sector_filter_active() {
        return true;
    }
    let start_check = filter.start_sector > 0;
    let end_check = filter.end_sector > 0;
    if start_check && !end_check {
        end >= filter.start_sector
    } else if !start_check && end_check {
        start <= filter.end_sector
    } else if start_check && end_check {
        !(end < filter.start_sector || start > filter.end_sector)
    } else {
        true
    }
}

/// min/max 범위 필터 헬퍼
#[inline]
fn match_range_f64(value: f64, min: f64, max: f64) -> bool {
    let min_check = min > 0.0;
    let max_check = max > 0.0;
    if min_check && !max_check {
        value >= min
    } else if !min_check && max_check {
        value >= 0.0 && value <= max
    } else if min_check && max_check {
        value >= min && value <= max
    } else {
        true
    }
}

#[inline]
fn match_range_u32(value: u32, min: u32, max: u32) -> bool {
    let min_check = min > 0;
    let max_check = max > 0;
    if min_check && !max_check {
        value >= min
    } else if !min_check && max_check {
        value <= max
    } else if min_check && max_check {
        value >= min && value <= max
    } else {
        true
    }
}

/// 제네릭 필터 함수: Filterable 트레이트를 구현하는 모든 타입에 적용
pub fn filter_data<T: Filterable>(data: Vec<T>, filter: &FilterOptions) -> Vec<T> {
    // cpu_set 캐시가 없으면 구축
    let filter = if filter.is_cpu_filter_active() && filter.cpu_set.is_none() {
        let mut f = filter.clone();
        f.build_cpu_set();
        std::borrow::Cow::Owned(f)
    } else {
        std::borrow::Cow::Borrowed(filter)
    };
    let filter = filter.as_ref();

    // 필터가 활성화되지 않은 경우 원본 데이터 반환
    if !filter.is_time_filter_active()
        && !filter.is_sector_filter_active()
        && !filter.is_dtoc_filter_active()
        && !filter.is_ctoc_filter_active()
        && !filter.is_ctod_filter_active()
        && !filter.is_qd_filter_active()
        && !filter.is_cpu_filter_active()
    {
        return data;
    }

    data.into_iter()
        .filter(|item| {
            item.matches_time(filter)
                && item.matches_sector(filter)
                && (!filter.is_dtoc_filter_active()
                    || match_range_f64(item.dtoc(), filter.min_dtoc, filter.max_dtoc))
                && (!filter.is_ctoc_filter_active()
                    || match_range_f64(item.ctoc(), filter.min_ctoc, filter.max_ctoc))
                && (!filter.is_ctod_filter_active()
                    || match_range_f64(item.ctod(), filter.min_ctod, filter.max_ctod))
                && (!filter.is_qd_filter_active()
                    || match_range_u32(item.qd(), filter.min_qd, filter.max_qd))
                && (!filter.is_cpu_filter_active()
                    || item.cpu().is_none_or(|cpu| filter.cpu_matches(cpu)))
        })
        .collect()
}

// 필터링 옵션을 저장할 구조체 정의
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOptions {
    pub start_time: f64,   // 시작 시간 (ms)
    pub end_time: f64,     // 종료 시간 (ms)
    pub start_sector: u64, // 시작 섹터/LBA
    pub end_sector: u64,   // 종료 섹터/LBA
    // 레이턴시 필터링 옵션
    pub min_dtoc: f64,      // 최소 Device to Complete 레이턴시 (ms)
    pub max_dtoc: f64,      // 최대 Device to Complete 레이턴시 (ms)
    pub min_ctoc: f64,      // 최소 Complete to Complete 레이턴시 (ms)
    pub max_ctoc: f64,      // 최대 Complete to Complete 레이턴시 (ms)
    pub min_ctod: f64,      // 최소 Complete to Device 레이턴시 (ms)
    pub max_ctod: f64,      // 최대 Complete to Device 레이턴시 (ms)
    pub min_qd: u32,        // 최소 Queue Depth
    pub max_qd: u32,        // 최대 Queue Depth
    pub cpu_list: Vec<u32>, // 필터링할 CPU 번호 목록
    #[serde(skip)]
    pub cpu_set: Option<HashSet<u32>>, // cpu_list의 O(1) 조회용 캐시
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
            cpu_list: Vec::new(),
            cpu_set: None,
        }
    }
}

impl FilterOptions {
    // 필터 옵션이 활성화되어 있는지 확인
    pub fn is_time_filter_active(&self) -> bool {
        self.end_time > self.start_time
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

    pub fn is_cpu_filter_active(&self) -> bool {
        !self.cpu_list.is_empty()
    }

    /// cpu_list에서 HashSet 캐시를 구축 (필터링 전 한 번 호출)
    pub fn build_cpu_set(&mut self) {
        if !self.cpu_list.is_empty() {
            self.cpu_set = Some(self.cpu_list.iter().copied().collect());
        } else {
            self.cpu_set = None;
        }
    }

    /// CPU 번호가 필터에 매칭되는지 O(1)로 확인
    #[inline]
    pub fn cpu_matches(&self, cpu: u32) -> bool {
        match &self.cpu_set {
            Some(set) => set.contains(&cpu),
            None => self.cpu_list.contains(&cpu), // 캐시 미구축 시 fallback
        }
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
            cpu_list: self.cpu_list.clone(),
            cpu_set: self.cpu_set.clone(),
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

    // CPU 목록 입력
    println!("cpu numbers (comma-separated, e.g., 0,1,2,3 or press enter to skip): ");
    input.clear();
    stdin.lock().read_line(&mut input)?;
    let cpu_input = input.trim();
    if !cpu_input.is_empty() {
        let cpus: Result<Vec<u32>, _> = cpu_input
            .split(',')
            .map(|s| s.trim().parse::<u32>())
            .collect();
        if let Ok(cpu_vec) = cpus {
            filter.cpu_list = cpu_vec;
        }
    }

    Ok(filter)
}

// Block 데이터 필터링 함수
pub fn filter_block_data(
    block_data: Vec<crate::Block>,
    filter: &FilterOptions,
) -> Vec<crate::Block> {
    filter_data(block_data, filter)
}

// UFS 데이터 필터링 함수
pub fn filter_ufs_data(ufs_data: Vec<crate::UFS>, filter: &FilterOptions) -> Vec<crate::UFS> {
    filter_data(ufs_data, filter)
}

// UFSCUSTOM 데이터 필터링 함수
pub fn filter_ufscustom_data(
    ufscustom_data: Vec<crate::UFSCUSTOM>,
    filter: &FilterOptions,
) -> Vec<crate::UFSCUSTOM> {
    filter_data(ufscustom_data, filter)
}
