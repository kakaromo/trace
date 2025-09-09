// 공통 트레이트 정의 - 모든 트레이스 타입이 구현해야 함
pub trait TraceItem {
    // 트레이스 항목의 타입을 반환 (UFS의 opcode나 Block의 io_type 등)
    fn get_type(&self) -> String;

    // 지연 시간 관련 메서드들
    fn get_dtoc(&self) -> f64; // Dispatch to Complete 지연 시간
    fn get_qtoc(&self) -> f64; // Queue to Complete 지연 시간
    fn get_ctoc(&self) -> f64; // Complete to Complete 지연 시간
    fn get_ctod(&self) -> f64; // Complete to Dispatch 지연 시간

    // 요청 크기
    fn get_size(&self) -> u32;

    // 액션 타입 (UFS의 send_req/complete_rsp, Block의 block_rq_issue/block_rq_complete)
    fn get_action(&self) -> &str;

    // continuous 여부
    fn is_continuous(&self) -> bool;

    // Queue Depth
    fn get_qd(&self) -> u32;
    
    // Queue Depth at start and end (UFSCUSTOM 전용, 다른 타입은 기본값 반환)
    fn get_start_qd(&self) -> u32 {
        self.get_qd() // 기본적으로 qd와 동일
    }
    
    fn get_end_qd(&self) -> u32 {
        self.get_qd() // 기본적으로 qd와 동일
    }
}
