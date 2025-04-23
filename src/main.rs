use std::env;
use std::io;
use std::io::{BufRead, Write};
use std::time::Instant;
use trace::*;
use trace::utils::Logger;

fn main() -> io::Result<()> {
    // Get command line arguments or prompt for them if not provided
    let args: Vec<String> = if env::args().len() > 1 {
        env::args().collect()
    } else {
        let mut input_args = vec![String::from("trace")]; // Program name

        print!("Enter log file path: ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        input_args.push(input.trim().to_string());

        print!("Enter output file prefix: ");
        io::stdout().flush()?;
        input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        input_args.push(input.trim().to_string());

        input_args
    };

    if args.len() != 3 {
        eprintln!("Usage: {} <log_file> <output_prefix>", args[0]);
        return Ok(());  // 프로그램 종료        
    }

    // Logger 초기화 - 로그 파일은 trace가 저장되는 경로와 동일하게 설정
    Logger::init(&args[2]);

    let total_start_time = Instant::now();
    log!("===== Starting Large Log File Processing =====");

    // Parse log file
    log!("\n[1/6] Parsing log file...");
    let parse_start = Instant::now();
    let (ufs_traces, block_traces) = match parse_log_file(&args[1]) {
        Ok(result) => result,
        Err(e) => {
            log_error!("File parsing error: {}", e);
            (Vec::new(), Vec::new()) // Return empty vectors on error
        }
    };
    log!(
        "Log parsing complete: UFS={}, Block={}, Time taken: {:.2}s",
        ufs_traces.len(),
        block_traces.len(),
        parse_start.elapsed().as_secs_f64()
    );

    // Post-processing (parallel processing)
    log!("\n[2/6] Post-processing data...");
    let process_start = Instant::now();

    log!("Post-processing UFS data...");
    let processed_ufs = ufs_bottom_half_latency_process(ufs_traces);

    log!("Post-processing Block I/O data...");
    let processed_blocks = block_bottom_half_latency_process(block_traces);

    log!(
        "Post-processing complete: Time taken: {:.2}s",
        process_start.elapsed().as_secs_f64()
    );

    // Output analysis results
    log!("\n[3/6] Calculating analysis results...");
    let analysis_start = Instant::now();

    log!("\n=== UFS Analysis Results ===");
    print_ufs_statistics(&processed_ufs);

    log!("\n=== Block I/O Analysis Results ===");
    print_block_statistics(&processed_blocks);

    log!(
        "\nAnalysis complete: Time taken: {:.2}s",
        analysis_start.elapsed().as_secs_f64()
    );

    // Save to Parquet files
    log!("\n[4/6] Saving to Parquet files...");
    let save_start = Instant::now();

    match save_to_parquet(&processed_ufs, &processed_blocks, &args[2]) {
        Ok(()) => log!(
            "Parquet files saved successfully (Time taken: {:.2}s):\n{}_ufs.parquet\n{}_block.parquet", 
            save_start.elapsed().as_secs_f64(), args[2], args[2]
        ),
        Err(e) => log_error!("Error while saving Parquet files: {}", e),
    }

    // Generate Plotly charts
    log!("\n[5/6] Generating Plotly charts...");
    let charts_start = Instant::now();

    match generate_charts(&processed_ufs, &processed_blocks, &args[2]) {
        Ok(()) => log!(
            "Plotly charts generated successfully (Time taken: {:.2}s)",
            charts_start.elapsed().as_secs_f64()
        ),
        Err(e) => log_error!("Error while generating Plotly charts: {}", e),
    }

    log!("\n===== All Processing Complete! =====");
    log!(
        "Total time taken: {:.2}s",
        total_start_time.elapsed().as_secs_f64()
    );
    log!(
        "Processed UFS events: {}, Block I/O events: {}",
        processed_ufs.len(),
        processed_blocks.len()
    );
    log!("Generated files:");
    log!(
        "- Parquet files: {}_ufs.parquet, {}_block.parquet",
        args[2], args[2]
    );
    log!("- UFS Plotly charts: {}_ufs_*.html", args[2]);
    log!("- Block I/O Plotly charts: {}_block_*.html", args[2]);
    log!("- UFS Matplotlib charts: {}_ufs_*.png", args[2]);
    log!("- Block I/O Matplotlib charts: {}_block_*.png", args[2]);
    log!("- Log file: {}_result.log", args[2]);

    // 로그 파일 버퍼 비우기
    let _ = Logger::flush();

    Ok(())
}
