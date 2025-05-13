use crate::models::{Block, UFS, UFSCUSTOM};
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;

/// Plotters 차트 생성을 위한 공통 구조체
pub struct PlottersConfig {
    pub width: u32,
    pub height: u32,
    pub font_family: &'static str,
    pub title_font_size: u32,
    pub axis_label_font_size: u32,
    pub tick_label_font_size: u32,
    pub point_size: u32,
    pub legend_spacing: u32,
}

impl Default for PlottersConfig {
    fn default() -> Self {
        Self {
            width: 1000,
            height: 800,
            font_family: "sans-serif",
            title_font_size: 30,
            axis_label_font_size: 20,
            tick_label_font_size: 15,
            point_size: 2,
            legend_spacing: 30,
        }
    }
}

/// UFS 명령어 타입에 따른 색상 매핑
pub fn get_color_for_ufs_opcode(opcode: &str) -> RGBColor {
    match opcode {
        "0x28" => RGBColor(65, 105, 225),    // READ - 파란색 계열
        "0x2a" => RGBColor(220, 20, 60),     // WRITE - 빨간색 계열
        "0x35" => RGBColor(255, 215, 0),     // SYNC - 노란색 계열
        "0x42" => RGBColor(138, 43, 226),    // UNMAP/DISCARD - 보라색 계열
        _ => RGBColor(50, 50, 50),           // 기타 - 검은색 계열
    }
}

/// Block I/O 타입에 따른 색상 매핑
pub fn get_color_for_io_type(io_type: &str) -> RGBColor {
    if let Some(first_char) = io_type.chars().next() {
        match first_char {
            'R' => RGBColor(65, 105, 225),   // Read operations (R, RA, RAM, RS...) - 파란색 계열
            'W' => RGBColor(220, 20, 60),    // Write operations (W, WM, WS...) - 빨간색 계열
            'F' => RGBColor(255, 215, 0),    // Sync/Flush operations - 노란색 계열
            'D' => RGBColor(138, 43, 226),   // Discard operations - 보라색 계열
            _ => RGBColor(50, 50, 50),       // 기타 - 검은색 계열
        }
    } else {
        RGBColor(50, 50, 50)  // Empty string fallback
    }
}

/// UFS 명령어 이름 매핑 함수
pub fn get_ufs_opcode_name(opcode: &str) -> String {
    match opcode {
        "0x28" => "READ_10".to_string(),
        "0x2a" => "WRITE_10".to_string(),
        "0x35" => "SYNCHRONIZE_CACHE_10".to_string(),
        "0x42" => "UNMAP".to_string(),
        _ => opcode.to_string(),
    }
}

/// 데이터 범위에 패딩을 추가하는 함수
pub fn add_padding_to_range(min: f64, max: f64, padding_percent: f64) -> (f64, f64) {
    let range = max - min;
    let padding = range * padding_percent;
    (min - padding, max + padding)
}

/// 레전드 그리기 함수
pub fn draw_legend(
    legend_area: &DrawingArea<BitMapBackend, plotters::coord::Shift>,
    legends: &[(String, RGBColor)],
    config: &PlottersConfig
) -> Result<(), Box<dyn Error>> {
    legend_area.fill(&WHITE.mix(0.95))?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        let spacing = config.legend_spacing as usize;
        let y_pos = (50 + i * spacing) as i32;
        
        // 직선으로 레전드 표시
        legend_area.draw(&PathElement::new(
            vec![(20_i32, y_pos), (50_i32, y_pos)],
            color.stroke_width(2)
        ))?;
        
        legend_area.draw(&Text::new(
            name.clone(),
            (60_i32, y_pos),
            (config.font_family, config.tick_label_font_size)
        ))?;
    }
    
    Ok(())
}

/// Generate charts using plotters library and save as PNG
pub fn generate_plotters_charts(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), String> {
    // 기본 차트 구성 생성
    let config = PlottersConfig::default();
    
    // UFS 차트 생성
    if !processed_ufs.is_empty() {
        match create_ufs_latency_trend_plotters(processed_ufs, output_prefix, &config) {
            Ok(_) => {
                println!("UFS latency trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFS latency trend PNG chart with Plotters: {}", e);
            }
        }
    }

    // Block I/O 차트 생성
    if !processed_blocks.is_empty() {
        match create_block_io_plotters(processed_blocks, output_prefix, &config) {
            Ok(_) => {
                println!("Block I/O PNG charts generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O PNG charts with Plotters: {}", e);
            }
        }
    }

    // UFSCUSTOM 차트 생성
    if !processed_ufscustom.is_empty() {
        match create_ufscustom_plotters(processed_ufscustom, output_prefix, &config) {
            Ok(_) => {
                println!("UFSCUSTOM PNG charts generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM PNG charts with Plotters: {}", e);
            }
        }
    }

    println!("Plotters charts generated successfully.");

    Ok(())
}

/// Create UFS latency trend chart using Plotters library and save as PNG
pub fn create_ufs_latency_trend_plotters(data: &[UFS], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating charts".to_string());
    }
    
    // Sort data by time
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| {
        a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // 명령어별로 데이터 그룹화
    let mut chart_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    
    for item in &time_sorted_data {
        if item.dtoc > 0.0 {
            chart_data
                .entry(item.opcode.clone())
                .or_insert_with(Vec::new)
                .push((item.time, item.dtoc));
        }
    }
    
    if chart_data.is_empty() {
        return Err("No valid data for UFS latency trend chart".to_string());
    }
    
    // X축과 Y축의 범위를 계산
    let mut min_time = f64::MAX;
    let mut max_time = f64::MIN;
    let mut min_latency = f64::MAX;
    let mut max_latency = f64::MIN;
    
    for (_opcode, points) in &chart_data {
        for &(time, latency) in points {
            min_time = min_time.min(time);
            max_time = max_time.max(time);
            min_latency = min_latency.min(latency);
            max_latency = max_latency.max(latency);
        }
    }
    
    // 축에 패딩 추가
    let (min_time, max_time) = add_padding_to_range(min_time, max_time, 0.05);
    let min_latency = (min_latency.max(0.0) - (max_latency - min_latency) * 0.05).max(0.0); // 0 아래로 내려가지 않게
    let max_latency = max_latency + (max_latency - min_latency) * 0.05;
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_latency_trend_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (config.width, config.height))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption("UFS Latency Trend by Operation Code", (config.font_family, config.title_font_size).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_time..max_time, min_latency..max_latency)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Latency (ms)")
        .axis_desc_style((config.font_family, config.axis_label_font_size))
        .label_style((config.font_family, config.tick_label_font_size))
        .draw()
        .map_err(|e| e.to_string())?;
    
    // 명령어별 스캐터 플롯 추가
    let mut legends = Vec::new();
    
    for (opcode, points) in &chart_data {
        // 명령어 이름과 색상 매핑
        let opcode_name = get_ufs_opcode_name(opcode);
        let color = get_color_for_ufs_opcode(opcode);
        
        legends.push((opcode_name, color));
        
        // 스캐터 플롯 그리기
        chart.draw_series(
            points.iter().map(|&(x, y)| Circle::new((x, y), config.point_size, color.filled())),
        )
        .map_err(|e| e.to_string())?;
    }
    
    // 레전드 그리기
    draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
    
    // 차트 표시
    root.present().map_err(|e| e.to_string())?;
    println!("UFS latency trend PNG chart saved to: {}", png_path);
    
    Ok(())
}

/// Creates Block I/O charts using Plotters library
pub fn create_block_io_plotters(data: &[Block], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O data is empty.".to_string());
    }
    
    // I/O 타입별로 데이터 그룹화
    let mut io_type_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in data {
        io_type_groups.entry(block.io_type.clone()).or_default().push(block);
    }
    
    // Block I/O Latency over Time 차트
    {
        // PNG 파일 경로 생성
        let png_path = format!("{}_block_io_analysis_plotters.png", output_prefix);
        
        // Create the drawing area
        let root = BitMapBackend::new(&png_path, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;
        
        // 차트 영역과 레전드 영역을 분리
        let (chart_area, legend_area) = root.split_horizontally(800);
        
        // Find min and max values for axes
        let min_time = data.iter().map(|e| e.time).fold(f64::MAX, |a, b| a.min(b));
        let max_time = data.iter().map(|e| e.time).fold(f64::MIN, |a, b| a.max(b));
        
        let min_latency = data.iter()
            .filter(|b| b.dtoc > 0.0)
            .map(|b| b.dtoc)
            .fold(f64::MAX, |a, b| a.min(b));
        
        let max_latency = data.iter()
            .map(|b| b.dtoc)
            .fold(0.0_f64, |a: f64, b| a.max(b));
        
        // Add padding
        let (min_time, max_time) = add_padding_to_range(min_time, max_time, 0.05);
        let min_latency = min_latency.max(0.0) - (max_latency - min_latency) * 0.05;
        let max_latency = max_latency + (max_latency - min_latency) * 0.05;
        
        // Create the chart
        let mut chart = ChartBuilder::on(&chart_area)
            .caption("Block I/O Latency over Time by I/O Type", (config.font_family, config.title_font_size).into_font())
            .margin(10)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_time..max_time, min_latency..max_latency)
            .map_err(|e| e.to_string())?;
        
        // Configure the chart
        chart.configure_mesh()
            .x_desc("Time (s)")
            .y_desc("Latency (ms)")
            .axis_desc_style((config.font_family, config.axis_label_font_size))
            .label_style((config.font_family, config.tick_label_font_size))
            .draw()
            .map_err(|e| e.to_string())?;
        
        // I/O 타입별 스캐터 플롯 추가
        let mut legends = Vec::new();
        
        for (io_type, events) in &io_type_groups {
            let color = get_color_for_io_type(io_type);
            let filtered_events: Vec<&Block> = events.iter()
                .filter(|e| e.dtoc > 0.0)
                .cloned()
                .collect();
            
            if !filtered_events.is_empty() {
                legends.push((io_type.clone(), color));
                
                // 스캐터 플롯 그리기
                chart.draw_series(
                    filtered_events.iter().map(|event| 
                        Circle::new((event.time, event.dtoc), config.point_size, color.filled())
                    )
                )
                .map_err(|e| e.to_string())?;
            }
        }
        
        // 레전드 영역 그리기
        draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
        
        root.present().map_err(|e| e.to_string())?;
        println!("Block I/O analysis PNG chart saved to: {}", png_path);
    }
    
    // LBA vs Latency 스캐터 플롯
    {
        // PNG 파일 경로 생성
        let png_path = format!("{}_block_lba_latency_plotters.png", output_prefix);
        
        // Create the drawing area
        let root = BitMapBackend::new(&png_path, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;
        
        // 차트 영역과 레전드 영역을 분리
        let (chart_area, legend_area) = root.split_horizontally(800);
        
        // Find min and max values for LBA axis
        let min_sector = data.iter().map(|e| e.sector as f64).fold(f64::MAX, |a, b| a.min(b));
        let max_sector = data.iter().map(|e| e.sector as f64).fold(f64::MIN, |a, b| a.max(b));
        
        let min_latency = data.iter()
            .filter(|b| b.dtoc > 0.0)
            .map(|b| b.dtoc)
            .fold(f64::MAX, |a, b| a.min(b));
        
        let max_latency = data.iter()
            .map(|b| b.dtoc)
            .fold(0.0_f64, |a: f64, b| a.max(b));
        
        // Add padding
        let (min_sector, max_sector) = add_padding_to_range(min_sector, max_sector, 0.05);
        let min_latency = min_latency.max(0.0) - (max_latency - min_latency) * 0.05;
        let max_latency = max_latency + (max_latency - min_latency) * 0.05;
        
        // Create the chart
        let mut chart = ChartBuilder::on(&chart_area)
            .caption("Block I/O Sector/LBA vs Latency by I/O Type", (config.font_family, config.title_font_size).into_font())
            .margin(10)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_sector..max_sector, min_latency..max_latency)
            .map_err(|e| e.to_string())?;
        
        // Configure the chart
        chart.configure_mesh()
            .x_desc("Sector/LBA")
            .y_desc("Latency (ms)")
            .axis_desc_style((config.font_family, config.axis_label_font_size))
            .label_style((config.font_family, config.tick_label_font_size))
            .draw()
            .map_err(|e| e.to_string())?;
        
        // I/O 타입별 스캐터 플롯 추가
        let mut legends = Vec::new();
        
        for (io_type, events) in &io_type_groups {
            let color = get_color_for_io_type(io_type);
            let filtered_events: Vec<&Block> = events.iter()
                .filter(|e| e.dtoc > 0.0)
                .cloned()
                .collect();
            
            if !filtered_events.is_empty() {
                legends.push((io_type.clone(), color));
                
                // 스캐터 플롯 그리기
                chart.draw_series(
                    filtered_events.iter().map(|event| 
                        Circle::new((event.sector as f64, event.dtoc), config.point_size, color.filled())
                    )
                )
                .map_err(|e| e.to_string())?;
            }
        }
        
        // 레전드 영역 그리기
        draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
        
        root.present().map_err(|e| e.to_string())?;
        println!("Block I/O LBA vs Latency PNG chart saved to: {}", png_path);
    }
    
    Ok(())
}

/// Create UFSCUSTOM charts using Plotters library
pub fn create_ufscustom_plotters(data: &[UFSCUSTOM], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFSCUSTOM data is empty.".to_string());
    }
    
    // 명령어별로 데이터 그룹화
    let mut command_groups: HashMap<String, Vec<&UFSCUSTOM>> = HashMap::new();
    for event in data {
        command_groups.entry(event.opcode.clone()).or_default().push(event);
    }
    
    // LBA vs Time 스캐터 플롯 생성
    {
        // PNG 파일 경로 생성
        let png_path = format!("{}_ufscustom_lba_time_plotters.png", output_prefix);
        
        // Create the drawing area
        let root = BitMapBackend::new(&png_path, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;
        
        // Find min and max values for axes
        let min_time = data.iter().map(|e| e.start_time).fold(f64::MAX, |a, b| a.min(b));
        let max_time = data.iter().map(|e| e.start_time).fold(f64::MIN, |a, b| a.max(b));
        
        let min_lba = data.iter().map(|e| e.lba as f64).fold(f64::MAX, |a, b| a.min(b));
        let max_lba = data.iter().map(|e| e.lba as f64).fold(f64::MIN, |a, b| a.max(b));
        
        // Add padding
        let (min_time, max_time) = add_padding_to_range(min_time, max_time, 0.05);
        let (min_lba, max_lba) = add_padding_to_range(min_lba, max_lba, 0.05);
        
        // 차트 영역과 레전드 영역을 분리
        let (chart_area, legend_area) = root.split_horizontally(800);
        
        // Create the chart
        let mut chart = ChartBuilder::on(&chart_area)
            .caption("UFSCUSTOM LBA over Time by Opcode", (config.font_family, config.title_font_size).into_font())
            .margin(10)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_time..max_time, min_lba..max_lba)
            .map_err(|e| e.to_string())?;
        
        // Configure the chart
        chart.configure_mesh()
            .x_desc("Time (s)")
            .y_desc("LBA")
            .axis_desc_style((config.font_family, config.axis_label_font_size))
            .label_style((config.font_family, config.tick_label_font_size))
            .draw()
            .map_err(|e| e.to_string())?;
        
        // Add each command as a series
        let mut legends = Vec::new();
        
        for (command, events) in &command_groups {
            // 명령어 타입에 따라 색상 지정
            let color = get_color_for_ufs_opcode(command);
            
            legends.push((command.clone(), color));
            
            // 산점도 포인트 그리기
            chart.draw_series(events.iter().map(|event| {
                Circle::new((event.start_time, event.lba as f64), config.point_size, color.filled())
            }))
            .map_err(|e| e.to_string())?;
        }
        
        // 레전드 영역을 오른쪽에 그리기
        draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
        
        root.present().map_err(|e| e.to_string())?;
        println!("UFSCUSTOM LBA over Time PNG chart saved to: {}", png_path);
    }
    
    // DTOC vs Time 스캐터 플롯 생성
    {
        // PNG 파일 경로 생성
        let png_path = format!("{}_ufscustom_dtoc_time_plotters.png", output_prefix);
        
        // Create the drawing area
        let root = BitMapBackend::new(&png_path, (config.width, config.height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| e.to_string())?;
        
        // Find min and max values for axes
        let min_time = data.iter().map(|e| e.start_time).fold(f64::MAX, |a, b| a.min(b));
        let max_time = data.iter().map(|e| e.start_time).fold(f64::MIN, |a, b| a.max(b));
        
        let min_dtoc = data.iter()
            .filter(|e| e.dtoc > 0.0)
            .map(|e| e.dtoc)
            .fold(f64::MAX, |a, b| a.min(b));
        
        let max_dtoc = data.iter()
            .map(|e| e.dtoc)
            .fold(f64::MIN, |a, b| a.max(b));
        
        // Add padding for latency axis
        let (min_time, max_time) = add_padding_to_range(min_time, max_time, 0.05);
        let min_dtoc = min_dtoc.max(0.0) - (max_dtoc - min_dtoc) * 0.05;
        let max_dtoc = max_dtoc + (max_dtoc - min_dtoc) * 0.05;
        
        // 차트 영역과 레전드 영역을 분리
        let (chart_area, legend_area) = root.split_horizontally(800);
        
        // Create the chart
        let mut chart = ChartBuilder::on(&chart_area)
            .caption("UFSCUSTOM Latency over Time by Command", (config.font_family, config.title_font_size).into_font())
            .margin(10)
            .x_label_area_size(50)
            .y_label_area_size(60)
            .build_cartesian_2d(min_time..max_time, min_dtoc..max_dtoc)
            .map_err(|e| e.to_string())?;
        
        // Configure the chart
        chart.configure_mesh()
            .x_desc("Time (s)")
            .y_desc("Latency (ms)")
            .axis_desc_style((config.font_family, config.axis_label_font_size))
            .label_style((config.font_family, config.tick_label_font_size))
            .draw()
            .map_err(|e| e.to_string())?;
        
        // Add each command as a series
        let mut legends = Vec::new();
        
        for (command, events) in &command_groups {
            // 명령어 타입에 따라 색상 지정
            let color = get_color_for_ufs_opcode(command);
            
            let filtered_events: Vec<&UFSCUSTOM> = events.iter()
                .filter(|e| e.dtoc > 0.0)
                .cloned()
                .collect();
            
            if !filtered_events.is_empty() {
                legends.push((command.clone(), color));
                
                // 산점도 포인트 그리기
                chart.draw_series(filtered_events.iter().map(|event| {
                    Circle::new((event.start_time, event.dtoc), config.point_size, color.filled())
                }))
                .map_err(|e| e.to_string())?;
            }
        }
        
        // 레전드 영역을 오른쪽에 그리기
        draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
        
        root.present().map_err(|e| e.to_string())?;
        println!("UFSCUSTOM Latency over Time PNG chart saved to: {}", png_path);
    }
    
    Ok(())
}
