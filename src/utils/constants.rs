// 밀리초로 변환하기 위한 상수 (초에서 밀리초로)
pub const MILLISECONDS: u64 = 1000;

// UFS 로그에서 사용되는 디버그용 LBA 값(2^61 - 1)
pub const UFS_DEBUG_LBA: u64 = 2305843009213693951;

// 터무니없는 LBA 판정을 위한 최대 허용 값 (약 2^48)
pub const MAX_VALID_UFS_LBA: u64 = 1u64 << 48;