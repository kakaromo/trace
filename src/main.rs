use std::env;
use std::io;
use std::time::Instant;
use trace::*;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <log_file> <output_prefix>", args[0]);
        std::process::exit(1);
    }

    let total_start_time = Instant::now();
    println!("===== 대용량 로그 파일 처리 시작 =====");

    // 로그 파일 파싱
    println!("\n[1/4] 로그 파일 파싱 중...");
    let parse_start = Instant::now();
    let (ufs_traces, block_traces) = parse_log_file(&args[1])?;
    println!("로그 파싱 완료: UFS={}, Block={}, 소요 시간: {:.2}초", 
             ufs_traces.len(), block_traces.len(), parse_start.elapsed().as_secs_f64());

    // 후처리 수행 (병렬 처리)
    println!("\n[2/4] 데이터 후처리 중...");
    let process_start = Instant::now();
    
    println!("UFS 데이터 후처리 중...");
    let processed_ufs = ufs_bottom_half_latency_process(ufs_traces);
    
    println!("Block I/O 데이터 후처리 중...");
    let processed_blocks = block_bottom_half_latency_process(block_traces);
    
    println!("후처리 완료: 소요 시간: {:.2}초", process_start.elapsed().as_secs_f64());

    // 분석 결과 출력
    println!("\n[3/4] 분석 결과 계산 중...");
    let analysis_start = Instant::now();
    
    println!("\n=== UFS 분석 결과 ===");
    print_ufs_statistics(&processed_ufs);

    println!("\n=== Block I/O 분석 결과 ===");
    print_block_statistics(&processed_blocks);
    
    println!("\n분석 완료: 소요 시간: {:.2}초", analysis_start.elapsed().as_secs_f64());

    // Parquet 파일로 저장
    println!("\n[4/4] Parquet 파일 저장 중...");
    let save_start = Instant::now();
    
    match save_to_parquet(&processed_ufs, &processed_blocks, &args[2]) {
        Ok(()) => println!(
            "Parquet 파일 저장 완료 (소요 시간: {:.2}초):\n{}_ufs.parquet\n{}_block.parquet", 
            save_start.elapsed().as_secs_f64(), args[2], args[2]
        ),
        Err(e) => eprintln!("Parquet 파일 저장 중 오류 발생: {}", e),
    }

    println!("\n===== 모든 처리 완료! =====");
    println!("총 소요 시간: {:.2}초", total_start_time.elapsed().as_secs_f64());
    println!("처리된 UFS 이벤트: {}, Block I/O 이벤트: {}", processed_ufs.len(), processed_blocks.len());

    Ok(())
}