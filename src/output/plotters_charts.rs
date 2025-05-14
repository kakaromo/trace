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
    if opcode == "0x28" {
        RGBColor(65, 105, 225)      // READ - 파란색 계열
    } else if opcode == "0x2a" {
        RGBColor(220, 20, 60)       // WRITE - 빨간색 계열
    } else if opcode == "0x35" {
        RGBColor(255, 215, 0)       // SYNC - 노란색 계열
    } else if opcode == "0x42" {
        RGBColor(138, 43, 226)      // UNMAP/DISCARD - 보라색 계열
    } else {
        RGBColor(50, 50, 50)        // 기타 - 검은색 계열
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

/// UFS 색상 매핑 헬퍼 함수
fn ufs_opcode_color_mapper(opcode: &str) -> RGBColor {
    if opcode == "READ_10" {
        RGBColor(65, 105, 225)      // READ - 파란색 계열
    } else if opcode == "WRITE_10" {
        RGBColor(220, 20, 60)       // WRITE - 빨간색 계열
    } else if opcode == "SYNCHRONIZE_CACHE_10" {
        RGBColor(255, 215, 0)       // SYNC - 노란색 계열
    } else if opcode == "UNMAP" {
        RGBColor(138, 43, 226)      // UNMAP/DISCARD - 보라색 계열
    } else {
        RGBColor(50, 50, 50)        // 기타 - 검은색 계열
    }
}

/// 일반적인 X대비 Y 그래프 생성을 위한 함수
/// T: 데이터 타입, F: X축 추출 함수, G: Y축 데이터 추출 함수, H: 필터 조건 함수
pub fn create_xy_scatter_chart<T, F, G, H>(
    data_groups: &HashMap<String, Vec<&T>>,
    output_path: &str,
    config: &PlottersConfig,
    title: &str,
    x_axis_label: &str,
    y_axis_label: &str,
    x_extractor: F,
    y_extractor: G,
    color_mapper: fn(&str) -> RGBColor,
    filter_condition: Option<H>
) -> Result<(), String>
where
    F: Fn(&T) -> f64,
    G: Fn(&T) -> f64,
    H: Fn(&&T) -> bool,
{
    // Create the drawing area
    let root = BitMapBackend::new(output_path, (config.width, config.height))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Find min and max values for axes
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;
    
    for (_, events) in data_groups {
        for event in events {
            // 필터 조건이 있다면 적용
            if let Some(ref cond) = filter_condition {
                if !cond(event) {
                    continue;
                }
            }
            
            let x_value = x_extractor(event);
            let y_value = y_extractor(event);
            
            min_x = min_x.min(x_value);
            max_x = max_x.max(x_value);
            min_y = min_y.min(y_value);
            max_y = max_y.max(y_value);
        }
    }
    
    // Add padding
    let (min_x, max_x) = add_padding_to_range(min_x, max_x, 0.05);
    let min_y = (min_y.max(0.0) - (max_y - min_y) * 0.05).max(0.0); // 0 아래로 내려가지 않게
    let max_y = max_y + (max_y - min_y) * 0.05;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption(title, (config.font_family, config.title_font_size).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc(x_axis_label)
        .y_desc(y_axis_label)
        .axis_desc_style((config.font_family, config.axis_label_font_size))
        .label_style((config.font_family, config.tick_label_font_size))
        .draw()
        .map_err(|e| e.to_string())?;
    
    // Add each group as a series
    let mut legends = Vec::new();
    
    for (group_name, events) in data_groups {
        // 그룹에 따라 색상 지정
        let color = color_mapper(group_name);
        
        let filtered_events: Vec<&T> = if let Some(ref cond) = filter_condition {
            events.iter().filter(|e| cond(e)).cloned().collect()
        } else {
            events.clone()
        };
        
        if !filtered_events.is_empty() {
            legends.push((group_name.clone(), color));
            
            // 산점도 포인트 그리기
            chart.draw_series(
                filtered_events.iter().map(|event| {
                    Circle::new((x_extractor(event), y_extractor(event)), config.point_size, color.filled())
                })
            )
            .map_err(|e| e.to_string())?;
        }
    }
    
    // 레전드 영역 그리기
    draw_legend(&legend_area, &legends, config).map_err(|e| e.to_string())?;
    
    root.present().map_err(|e| e.to_string())?;
    println!("Chart saved to: {}", output_path);
    
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
        // UFS DTOC (Dispatch to Complete) 차트
        match create_ufs_latency_trend_plotters(processed_ufs, output_prefix, &config) {
            Ok(_) => {
                println!("UFS latency trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFS latency trend PNG chart with Plotters: {}", e);
            }
        }
        
        // UFS CTOC (Complete to Complete) 차트
        match create_ufs_ctoc_chart(processed_ufs, output_prefix, &config) {
            Ok(_) => {
                println!("UFS complete-to-complete trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete-to-complete trend PNG chart: {}", e);
            }
        }
        
        // UFS CTOD (Complete to Dispatch) 차트
        match create_ufs_ctod_chart(processed_ufs, output_prefix, &config) {
            Ok(_) => {
                println!("UFS complete-to-dispatch trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete-to-dispatch trend PNG chart: {}", e);
            }
        }
        
        // UFS Queue Depth 차트
        match create_ufs_qd_chart(processed_ufs, output_prefix, &config) {
            Ok(_) => {
                println!("UFS queue depth trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!("Error generating UFS queue depth trend PNG chart: {}", e);
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
    
    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        if item.dtoc > 0.0 {
            opcode_groups.entry(item.opcode.clone()).or_default().push(item);
        }
    }
    
    if opcode_groups.is_empty() {
        return Err("No valid data for UFS latency trend chart".to_string());
    }
    
    // 명령어 이름 변환 및 색상 매핑을 위한 새로운 그룹 생성
    let mut named_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for (opcode, events) in opcode_groups {
        let opcode_name = get_ufs_opcode_name(&opcode);
        named_opcode_groups.insert(opcode_name, events);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_latency_trend_plotters.png", output_prefix);
    
    create_xy_scatter_chart(
        &named_opcode_groups,
        &png_path,
        config,
        "UFS Latency Trend by Operation Code",
        "Time (s)",
        "Latency (ms)",
        |ufs| ufs.time,
        |ufs| ufs.dtoc,
        ufs_opcode_color_mapper,
        Some(|ufs: &&UFS| ufs.dtoc > 0.0)
    )?;
    
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
        let png_path = format!("{}_block_io_analysis_plotters.png", output_prefix);
        
        create_xy_scatter_chart(
            &io_type_groups,
            &png_path,
            config,
            "Block I/O Latency over Time by I/O Type",
            "Time (s)",
            "Latency (ms)",
            |block| block.time,
            |block| block.dtoc,
            get_color_for_io_type,
            Some(|block: &&Block| block.dtoc > 0.0)
        )?;
        
        println!("Block I/O analysis PNG chart saved to: {}", png_path);
    }
    
    // LBA vs Latency 스캐터 플롯
    {
        let png_path = format!("{}_block_lba_latency_plotters.png", output_prefix);
        
        create_xy_scatter_chart(
            &io_type_groups,
            &png_path,
            config,
            "Block I/O Sector/LBA vs Latency by I/O Type",
            "Sector/LBA",
            "Latency (ms)",
            |block| block.sector as f64,
            |block| block.dtoc,
            get_color_for_io_type,
            Some(|block: &&Block| block.dtoc > 0.0)
        )?;
        
        println!("Block I/O LBA vs Latency PNG chart saved to: {}", png_path);
    }
    
    Ok(())
}

/// 추가적인 차트 생성을 위한 헬퍼 함수 - UFS CTOC(Complete to Complete) 지표 생성
pub fn create_ufs_ctoc_chart(data: &[UFS], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating charts".to_string());
    }
    
    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        if item.ctoc > 0.0 {
            opcode_groups.entry(item.opcode.clone()).or_default().push(item);
        }
    }
    
    if opcode_groups.is_empty() {
        return Err("No valid CTOC data for UFS chart".to_string());
    }
    
    // 명령어 이름 변환 및 색상 매핑을 위한 새로운 그룹 생성
    let mut named_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for (opcode, events) in opcode_groups {
        let opcode_name = get_ufs_opcode_name(&opcode);
        named_opcode_groups.insert(opcode_name, events);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_ctoc_time_plotters.png", output_prefix);
    
    create_xy_scatter_chart(
        &named_opcode_groups,
        &png_path,
        config,
        "UFS Complete to Complete Time by Operation Code",
        "Time (s)",
        "Complete to Complete (ms)",
        |ufs| ufs.time,
        |ufs| ufs.ctoc,
        ufs_opcode_color_mapper,
        Some(|ufs: &&UFS| ufs.ctoc > 0.0)
    )?;
    
    println!("UFS Complete to Complete PNG chart saved to: {}", png_path);
    
    Ok(())
}

/// 추가적인 차트 생성을 위한 헬퍼 함수 - UFS CTOD(Complete to Dispatch) 지표 생성
pub fn create_ufs_ctod_chart(data: &[UFS], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating charts".to_string());
    }
    
    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        if item.ctod > 0.0 {
            opcode_groups.entry(item.opcode.clone()).or_default().push(item);
        }
    }
    
    if opcode_groups.is_empty() {
        return Err("No valid CTOD data for UFS chart".to_string());
    }
    
    // 명령어 이름 변환 및 색상 매핑을 위한 새로운 그룹 생성
    let mut named_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for (opcode, events) in opcode_groups {
        let opcode_name = get_ufs_opcode_name(&opcode);
        named_opcode_groups.insert(opcode_name, events);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_ctod_time_plotters.png", output_prefix);
    
    create_xy_scatter_chart(
        &named_opcode_groups,
        &png_path,
        config,
        "UFS Complete to Dispatch Time by Operation Code",
        "Time (s)",
        "Complete to Dispatch (ms)",
        |ufs| ufs.time,
        |ufs| ufs.ctod,
        ufs_opcode_color_mapper,
        Some(|ufs: &&UFS| ufs.ctod > 0.0)
    )?;
    
    println!("UFS Complete to Dispatch PNG chart saved to: {}", png_path);
    
    Ok(())
}

/// 추가적인 차트 생성을 위한 헬퍼 함수 - UFS Queue Depth 지표 생성
pub fn create_ufs_qd_chart(data: &[UFS], output_prefix: &str, config: &PlottersConfig) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating charts".to_string());
    }
    
    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        opcode_groups.entry(item.opcode.clone()).or_default().push(item);
    }
    
    if opcode_groups.is_empty() {
        return Err("No valid QD data for UFS chart".to_string());
    }
    
    // 명령어 이름 변환 및 색상 매핑을 위한 새로운 그룹 생성
    let mut named_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for (opcode, events) in opcode_groups {
        let opcode_name = get_ufs_opcode_name(&opcode);
        named_opcode_groups.insert(opcode_name, events);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_qd_time_plotters.png", output_prefix);
    
    create_xy_scatter_chart(
        &named_opcode_groups,
        &png_path,
        config,
        "UFS Queue Depth over Time by Operation Code",
        "Time (s)",
        "Queue Depth",
        |ufs| ufs.time,
        |ufs| ufs.qd as f64,
        ufs_opcode_color_mapper,
        Option::<fn(&&UFS) -> bool>::None
    )?;
    
    println!("UFS Queue Depth PNG chart saved to: {}", png_path);
    
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
        let png_path = format!("{}_ufscustom_lba_time_plotters.png", output_prefix);
        
        create_xy_scatter_chart(
            &command_groups,
            &png_path,
            config,
            "UFSCUSTOM LBA over Time by Opcode",
            "Time (s)",
            "LBA",
            |event| event.start_time,
            |event| event.lba as f64,
            get_color_for_ufs_opcode,
            Option::<fn(&&UFSCUSTOM) -> bool>::None
        )?;
        
        println!("UFSCUSTOM LBA over Time PNG chart saved to: {}", png_path);
    }
    
    // DTOC vs Time 스캐터 플롯 생성
    {
        let png_path = format!("{}_ufscustom_dtoc_time_plotters.png", output_prefix);
        
        create_xy_scatter_chart(
            &command_groups,
            &png_path,
            config,
            "UFSCUSTOM Latency over Time by Command",
            "Time (s)",
            "Latency (ms)",
            |event| event.start_time,
            |event| event.dtoc,
            get_color_for_ufs_opcode,
            Some(|event: &&UFSCUSTOM| event.dtoc > 0.0)
        )?;
        
        println!("UFSCUSTOM Latency over Time PNG chart saved to: {}", png_path);
    }
    
    Ok(())
}
