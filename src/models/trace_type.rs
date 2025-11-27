// TraceType 열거형 정의
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
pub enum TraceType {
    UFS,
    Block,
    UFSCUSTOM,
    // 여기에 새로운 트레이스 타입 추가 가능
    // 예: NVMe, F2FS, EXT4 등
}

use std::str::FromStr;

// FromStr 트레이트 구현
impl FromStr for TraceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ufs" => Ok(TraceType::UFS),
            "block" => Ok(TraceType::Block),
            "ufscustom" => Ok(TraceType::UFSCUSTOM),
            // 여기에 새 트레이스 타입 매칭 추가
            _ => Err(format!("Unknown trace type: {s}")),
        }
    }
}

impl TraceType {
    // 이전의 from_str 메서드를 parse_str로 이름 변경
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ufs" => Some(TraceType::UFS),
            "block" => Some(TraceType::Block),
            "ufscustom" => Some(TraceType::UFSCUSTOM),
            // 여기에 새 트레이스 타입 매칭 추가
            _ => None,
        }
    }

    // 표시 이름 반환
    pub fn display_name(&self) -> &'static str {
        match self {
            TraceType::UFS => "UFS",
            TraceType::Block => "Block I/O",
            TraceType::UFSCUSTOM => "UFSCustom",
            // 여기에 새 트레이스 타입 표시 이름 추가
        }
    }
}
