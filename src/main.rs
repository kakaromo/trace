use std::env;
use std::io;
use std::io::{Write, BufRead};
use std::time::Instant;
use trace::*;

fn main() -> io::Result<()> {
    let mut continue_loop = true;

    while continue_loop {
        // Get command line arguments or prompt for them if not provided
        let args: Vec<String> = if env::args().len() > 1 {
            env::args().collect()
        } else {
            let mut input_args = vec![String::from("trace")]; // Program name
            
            print!("로그 파일 경로를 입력해주세요: ");
            io::stdout().flush()?;
            let mut input = String::new();
            io::stdin().lock().read_line(&mut input)?;
            input_args.push(input.trim().to_string());
            
            print!("출력 파일 접두사를 입력해주세요: ");
            io::stdout().flush()?;
            input = String::new();
            io::stdin().lock().read_line(&mut input)?;
            input_args.push(input.trim().to_string());
            
            input_args
        };

        if args.len() != 3 {
            eprintln!("Usage: {} <log_file> <output_prefix>", args[0]);
            if ask_continue()? {
                continue;
            } else {
                break;
            }
        }

        let total_start_time = Instant::now();
        println!("===== Starting Large Log File Processing =====");

        // Parse log file
        println!("\n[1/5] Parsing log file...");
        let parse_start = Instant::now();
        let (ufs_traces, block_traces) = match parse_log_file(&args[1]) {
            Ok(result) => result,
            Err(e) => {
                eprintln!("파일 파싱 오류: {}", e);
                if ask_continue()? {
                    continue;
                } else {
                    break;
                }
            }
        };
        println!("Log parsing complete: UFS={}, Block={}, Time taken: {:.2}s", 
                ufs_traces.len(), block_traces.len(), parse_start.elapsed().as_secs_f64());

        // Post-processing (parallel processing)
        println!("\n[2/5] Post-processing data...");
        let process_start = Instant::now();
        
        println!("Post-processing UFS data...");
        let processed_ufs = ufs_bottom_half_latency_process(ufs_traces);
        
        println!("Post-processing Block I/O data...");
        let processed_blocks = block_bottom_half_latency_process(block_traces);
        
        println!("Post-processing complete: Time taken: {:.2}s", process_start.elapsed().as_secs_f64());

        // Output analysis results
        println!("\n[3/5] Calculating analysis results...");
        let analysis_start = Instant::now();
        
        println!("\n=== UFS Analysis Results ===");
        print_ufs_statistics(&processed_ufs);

        println!("\n=== Block I/O Analysis Results ===");
        print_block_statistics(&processed_blocks);
        
        println!("\nAnalysis complete: Time taken: {:.2}s", analysis_start.elapsed().as_secs_f64());

        // Save to Parquet files
        println!("\n[4/5] Saving to Parquet files...");
        let save_start = Instant::now();
        
        match save_to_parquet(&processed_ufs, &processed_blocks, &args[2]) {
            Ok(()) => println!(
                "Parquet files saved successfully (Time taken: {:.2}s):\n{}_ufs.parquet\n{}_block.parquet", 
                save_start.elapsed().as_secs_f64(), args[2], args[2]
            ),
            Err(e) => eprintln!("Error while saving Parquet files: {}", e),
        }
        
        // Generate Plotly charts
        println!("\n[5/5] Generating Plotly charts...");
        let charts_start = Instant::now();
        
        match generate_charts(&processed_ufs, &processed_blocks, &args[2]) {
            Ok(()) => println!(
                "Plotly charts generated successfully (Time taken: {:.2}s)", 
                charts_start.elapsed().as_secs_f64()
            ),
            Err(e) => eprintln!("Error while generating Plotly charts: {}", e),
        }

        println!("\n===== All Processing Complete! =====");
        println!("Total time taken: {:.2}s", total_start_time.elapsed().as_secs_f64());
        println!("Processed UFS events: {}, Block I/O events: {}", processed_ufs.len(), processed_blocks.len());
        println!("Generated files:");
        println!("- Parquet files: {}_ufs.parquet, {}_block.parquet", args[2], args[2]);
        println!("- UFS Plotly charts: {}_ufs_*.html", args[2]);
        println!("- Block I/O Plotly charts: {}_block_*.html", args[2]);
        println!("- UFS graphs: {}_ufs_*.png", args[2]);
        println!("- Block I/O graphs: {}_block_*.png", args[2]);

        // Ask if user wants to continue with another analysis
        continue_loop = ask_continue()?;
    }

    Ok(())
}

/// 사용자에게 계속 진행할지 묻는 함수
fn ask_continue() -> io::Result<bool> {
    loop {
        print!("계속 반복하시겠습니까? (Y/N): ");
        io::stdout().flush()?;
        
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        
        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            _ => println!("잘못된 입력입니다. Y 또는 N으로 대답해주세요."),
        }
    }
}