use crate::models::{Block, UFS};
use plotly::common::{Mode, Title};
use plotly::{Plot, Scatter, Layout, Pie};
use plotly::layout::{Axis, Legend};
use std::path::Path;

/// UFS 데이터로부터 Plotly 차트를 생성합니다.
pub fn create_ufs_charts(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFS 데이터가 비어 있습니다.".to_string());
    }

    // 데이터를 시간 순서로 정렬
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    // 1. 시간에 따른 LBA 차트
    let mut lba_plot = Plot::new();
    let lba_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.lba as f64).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("LBA over Time");
    
    lba_plot.add_trace(lba_scatter);
    lba_plot.set_layout(Layout::new()
        .title(Title::from("LBA over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("LBA")))
    );
    
    let lba_chart_path = format!("{}_ufs_lba_time.html", output_prefix);
    lba_plot.write_html(Path::new(&lba_chart_path));
    println!("UFS LBA 차트 저장됨: {}", lba_chart_path);

    // 2. 시간에 따른 Queue Depth 차트
    let mut qd_plot = Plot::new();
    let qd_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.qd as f64).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Queue Depth over Time");
    
    qd_plot.add_trace(qd_scatter);
    qd_plot.set_layout(Layout::new()
        .title(Title::from("Queue Depth over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Queue Depth")))
    );
    
    let qd_chart_path = format!("{}_ufs_qd_time.html", output_prefix);
    qd_plot.write_html(Path::new(&qd_chart_path));
    println!("UFS Queue Depth 차트 저장됨: {}", qd_chart_path);

    // 3. 시간에 따른 Device to Complete Latency 차트
    let mut dtoc_plot = Plot::new();
    let dtoc_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.dtoc).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Device to Complete Latency over Time");
    
    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(Layout::new()
        .title(Title::from("Device to Complete Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Device to Complete Latency (ms)")))
    );
    
    let dtoc_chart_path = format!("{}_ufs_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!("UFS Device to Complete 차트 저장됨: {}", dtoc_chart_path);

    // 4. 시간에 따른 Complete to Device Latency 차트
    let mut ctod_plot = Plot::new();
    let ctod_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.ctod).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Complete to Device Latency over Time");
    
    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(Layout::new()
        .title(Title::from("Complete to Device Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Complete to Device Latency (ms)")))
    );
    
    let ctod_chart_path = format!("{}_ufs_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!("UFS Complete to Device 차트 저장됨: {}", ctod_chart_path);

    // 5. 시간에 따른 Complete to Complete Latency 차트
    let mut ctoc_plot = Plot::new();
    let ctoc_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.ctoc).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Complete to Complete Latency over Time");
    
    ctoc_plot.add_trace(ctoc_scatter);
    ctoc_plot.set_layout(Layout::new()
        .title(Title::from("Complete to Complete Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Complete to Complete Latency (ms)")))
    );
    
    let ctoc_chart_path = format!("{}_ufs_ctoc_time.html", output_prefix);
    ctoc_plot.write_html(Path::new(&ctoc_chart_path));
    println!("UFS Complete to Complete 차트 저장됨: {}", ctoc_chart_path);

    // 6. 연속성 (pie)
    let continuous_count = data.iter().filter(|d| d.continuous).count() as f64;
    let non_continuous_count = (data.len() as f64) - continuous_count;
    
    let mut continuous_plot = Plot::new();
    
    // Pie 차트 생성 및 설정 (labels, values 설정 방식 수정)
    let values = vec![continuous_count, non_continuous_count];
    let labels = vec!["연속".to_string(), "비연속".to_string()];
    let pie = Pie::new(values)
        .labels(labels)
        .name("연속성 분포");
    
    continuous_plot.add_trace(pie);
    continuous_plot.set_layout(Layout::new()
        .title(Title::from("UFS 연속성 분포"))
        .legend(Legend::new().title(Title::from("연속성")))
    );
    
    let continuous_chart_path = format!("{}_ufs_continuous.html", output_prefix);
    continuous_plot.write_html(Path::new(&continuous_chart_path));
    println!("UFS 연속성 파이 차트 저장됨: {}", continuous_chart_path);

    Ok(())
}

/// Block I/O 데이터로부터 Plotly 차트를 생성합니다.
pub fn create_block_charts(data: &[Block], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O 데이터가 비어 있습니다.".to_string());
    }

    // 데이터를 시간 순서로 정렬
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));

    // 1. 시간에 따른 Sector 차트
    let mut sector_plot = Plot::new();
    let sector_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.sector as f64).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Sector over Time");
    
    sector_plot.add_trace(sector_scatter);
    sector_plot.set_layout(Layout::new()
        .title(Title::from("Sector over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Sector")))
    );
    
    let sector_chart_path = format!("{}_block_sector_time.html", output_prefix);
    sector_plot.write_html(Path::new(&sector_chart_path));
    println!("Block Sector 차트 저장됨: {}", sector_chart_path);

    // 2. 시간에 따른 Queue Depth 차트
    let mut qd_plot = Plot::new();
    let qd_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.qd as f64).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Queue Depth over Time");
    
    qd_plot.add_trace(qd_scatter);
    qd_plot.set_layout(Layout::new()
        .title(Title::from("Queue Depth over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Queue Depth")))
    );
    
    let qd_chart_path = format!("{}_block_qd_time.html", output_prefix);
    qd_plot.write_html(Path::new(&qd_chart_path));
    println!("Block Queue Depth 차트 저장됨: {}", qd_chart_path);

    // 3. 시간에 따른 Device to Complete Latency 차트
    let mut dtoc_plot = Plot::new();
    let dtoc_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.dtoc).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Device to Complete Latency over Time");
    
    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(Layout::new()
        .title(Title::from("Device to Complete Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Device to Complete Latency (ms)")))
    );
    
    let dtoc_chart_path = format!("{}_block_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!("Block Device to Complete 차트 저장됨: {}", dtoc_chart_path);

    // 4. 시간에 따른 Complete to Device Latency 차트
    let mut ctod_plot = Plot::new();
    let ctod_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.ctod).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Complete to Device Latency over Time");
    
    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(Layout::new()
        .title(Title::from("Complete to Device Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Complete to Device Latency (ms)")))
    );
    
    let ctod_chart_path = format!("{}_block_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!("Block Complete to Device 차트 저장됨: {}", ctod_chart_path);

    // 5. 시간에 따른 Complete to Complete Latency 차트
    let mut ctoc_plot = Plot::new();
    let ctoc_scatter = Scatter::new(
        time_sorted_data.iter().map(|d| d.time).collect::<Vec<f64>>(),
        time_sorted_data.iter().map(|d| d.ctoc).collect::<Vec<f64>>()
    )
    .mode(Mode::Markers)
    .name("Complete to Complete Latency over Time");
    
    ctoc_plot.add_trace(ctoc_scatter);
    ctoc_plot.set_layout(Layout::new()
        .title(Title::from("Complete to Complete Latency over Time"))
        .x_axis(Axis::new().title(Title::from("Time (s)")))
        .y_axis(Axis::new().title(Title::from("Complete to Complete Latency (ms)")))
    );
    
    let ctoc_chart_path = format!("{}_block_ctoc_time.html", output_prefix);
    ctoc_plot.write_html(Path::new(&ctoc_chart_path));
    println!("Block Complete to Complete 차트 저장됨: {}", ctoc_chart_path);

    // 6. 연속성 (pie)
    let continuous_count = data.iter().filter(|d| d.continuous).count() as f64;
    let non_continuous_count = (data.len() as f64) - continuous_count;
    
    let mut continuous_plot = Plot::new();
    
    // Pie 차트 생성 및 설정 (labels, values 설정 방식 수정)
    let values = vec![continuous_count, non_continuous_count];
    let labels = vec!["연속".to_string(), "비연속".to_string()];
    let pie = Pie::new(values)
        .labels(labels)
        .name("연속성 분포");
    
    continuous_plot.add_trace(pie);
    continuous_plot.set_layout(Layout::new()
        .title(Title::from("Block I/O 연속성 분포"))
        .legend(Legend::new().title(Title::from("연속성")))
    );
    
    let continuous_chart_path = format!("{}_block_continuous.html", output_prefix);
    continuous_plot.write_html(Path::new(&continuous_chart_path));
    println!("Block 연속성 파이 차트 저장됨: {}", continuous_chart_path);

    Ok(())
}

/// 차트 생성 및 통계 데이터를 저장합니다.
pub fn generate_charts(processed_ufs: &[UFS], processed_blocks: &[Block], output_prefix: &str) -> Result<(), String> {
    // UFS 차트 생성
    match create_ufs_charts(processed_ufs, output_prefix) {
        Ok(_) => {
            println!("UFS 차트가 생성되었습니다.");
        },
        Err(e) => {
            eprintln!("UFS 차트 생성 중 오류 발생: {}", e);
        }
    }

    // Block I/O 차트 생성
    match create_block_charts(processed_blocks, output_prefix) {
        Ok(_) => {
            println!("Block I/O 차트가 생성되었습니다.");
        },
        Err(e) => {
            eprintln!("Block I/O 차트 생성 중 오류 발생: {}", e);
        }
    }

    Ok(())
}