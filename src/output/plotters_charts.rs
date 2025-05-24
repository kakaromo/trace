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

/// 차트 생성을 위한 구성 정보를 포함하는 구조체
pub struct ChartConfig<'a, T, F, G, H> {
    pub data_groups: &'a HashMap<String, Vec<&'a T>>,
    pub output_path: &'a str,
    pub config: &'a PlottersConfig,
    pub title: &'a str,
    pub x_axis_label: &'a str,
    pub y_axis_label: &'a str,
    pub x_extractor: F,
    pub y_extractor: G,
    pub color_mapper: fn(&str) -> RGBColor,
    pub filter_condition: Option<H>,
}

/// UFS 명령어 타입에 따른 색상 매핑
pub fn get_color_for_ufs_opcode(opcode: &str) -> RGBColor {
    if opcode == "0x28" {
        RGBColor(65, 105, 225) // READ - 파란색 계열
    } else if opcode == "0x2a" {
        RGBColor(220, 20, 60) // WRITE - 빨간색 계열
    } else if opcode == "0x35" {
        RGBColor(255, 215, 0) // SYNC - 노란색 계열
    } else if opcode == "0x42" {
        RGBColor(138, 43, 226) // UNMAP/DISCARD - 보라색 계열
    } else {
        RGBColor(50, 50, 50) // 기타 - 검은색 계열
    }
}

/// Block I/O 타입에 따른 색상 매핑
pub fn get_color_for_io_type(io_type: &str) -> RGBColor {
    if let Some(first_char) = io_type.chars().next() {
        match first_char {
            'R' => RGBColor(65, 105, 225), // Read operations (R, RA, RAM, RS...) - 파란색 계열
            'W' => RGBColor(220, 20, 60),  // Write operations (W, WM, WS...) - 빨간색 계열
            'F' => RGBColor(255, 215, 0),  // Sync/Flush operations - 노란색 계열
            'D' => RGBColor(138, 43, 226), // Discard operations - 보라색 계열
            _ => RGBColor(50, 50, 50),     // 기타 - 검은색 계열
        }
    } else {
        RGBColor(50, 50, 50) // Empty string fallback
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
    config: &PlottersConfig,
) -> Result<(), Box<dyn Error>> {
    legend_area.fill(&WHITE.mix(0.95))?;

    for (i, (name, color)) in legends.iter().enumerate() {
        let spacing = config.legend_spacing as usize;
        let y_pos = (50 + i * spacing) as i32;

        // 직선으로 레전드 표시
        legend_area.draw(&PathElement::new(
            vec![(20_i32, y_pos), (50_i32, y_pos)],
            color.stroke_width(2),
        ))?;

        legend_area.draw(&Text::new(
            name.clone(),
            (60_i32, y_pos),
            (config.font_family, config.tick_label_font_size),
        ))?;
    }

    Ok(())
}

/// UFS 색상 매핑 헬퍼 함수
fn ufs_opcode_color_mapper(opcode: &str) -> RGBColor {
    if opcode == "READ_10" {
        RGBColor(65, 105, 225) // READ - 파란색 계열
    } else if opcode == "WRITE_10" {
        RGBColor(220, 20, 60) // WRITE - 빨간색 계열
    } else if opcode == "SYNCHRONIZE_CACHE_10" {
        RGBColor(255, 215, 0) // SYNC - 노란색 계열
    } else if opcode == "UNMAP" {
        RGBColor(138, 43, 226) // UNMAP/DISCARD - 보라색 계열
    } else {
        RGBColor(50, 50, 50) // 기타 - 검은색 계열
    }
}

/// 일반적인 X대비 Y 그래프 생성을 위한 함수
/// T: 데이터 타입, F: X축 추출 함수, G: Y축 데이터 추출 함수, H: 필터 조건 함수
/// 
/// 이 함수는 ChartConfig를 사용하는 create_xy_scatter_chart_with_config의 래퍼 함수입니다.
#[allow(clippy::too_many_arguments)]
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
    filter_condition: Option<H>,
) -> Result<(), String>
where
    F: Fn(&T) -> f64,
    G: Fn(&T) -> f64,
    H: Fn(&&T) -> bool,
{
    let chart_config = ChartConfig {
        data_groups,
        output_path,
        config,
        title,
        x_axis_label,
        y_axis_label,
        x_extractor,
        y_extractor,
        color_mapper,
        filter_condition,
    };

    create_xy_scatter_chart_with_config(chart_config)
}

/// 일반적인 X대비 Y 그래프 생성을 위한 함수 (ChartConfig 사용)
pub fn create_xy_scatter_chart_with_config<T, F, G, H>(
    chart_config: ChartConfig<'_, T, F, G, H>,
) -> Result<(), String>
where
    F: Fn(&T) -> f64,
    G: Fn(&T) -> f64,
    H: Fn(&&T) -> bool,
{
    // Create the drawing area
    let root = BitMapBackend::new(
        chart_config.output_path,
        (chart_config.config.width, chart_config.config.height),
    )
    .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;

    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);

    // Find min and max values for axes
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for events in chart_config.data_groups.values() {
        for event in events {
            // 필터 조건이 있다면 적용
            if let Some(ref cond) = chart_config.filter_condition {
                if !cond(event) {
                    continue;
                }
            }

            let x_value = (chart_config.x_extractor)(event);
            let y_value = (chart_config.y_extractor)(event);

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
        .caption(
            chart_config.title,
            (
                chart_config.config.font_family,
                chart_config.config.title_font_size,
            )
                .into_font(),
        )
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_x..max_x, min_y..max_y)
        .map_err(|e| e.to_string())?;

    // Configure the chart
    chart
        .configure_mesh()
        .x_desc(chart_config.x_axis_label)
        .y_desc(chart_config.y_axis_label)
        .axis_desc_style((
            chart_config.config.font_family,
            chart_config.config.axis_label_font_size,
        ))
        .label_style((
            chart_config.config.font_family,
            chart_config.config.tick_label_font_size,
        ))
        .draw()
        .map_err(|e| e.to_string())?;

    // Add each group as a series
    let mut legends = Vec::new();

    for (group_name, events) in chart_config.data_groups {
        // 그룹에 따라 색상 지정
        let color = (chart_config.color_mapper)(group_name);

        let filtered_events: Vec<&T> = if let Some(ref cond) = chart_config.filter_condition {
            events.iter().filter(|e| cond(e)).cloned().collect()
        } else {
            events.clone()
        };

        if !filtered_events.is_empty() {
            legends.push((group_name.clone(), color));

            // 산점도 포인트 그리기
            chart
                .draw_series(filtered_events.iter().map(|event| {
                    Circle::new(
                        (
                            (chart_config.x_extractor)(event),
                            (chart_config.y_extractor)(event),
                        ),
                        chart_config.config.point_size,
                        color.filled(),
                    )
                }))
                .map_err(|e| e.to_string())?;
        }
    }

    // 레전드 영역 그리기
    draw_legend(&legend_area, &legends, chart_config.config).map_err(|e| e.to_string())?;

    root.present().map_err(|e| e.to_string())?;
    println!("Chart saved to: {}", chart_config.output_path);

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
        // UFS lba 차트
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "lba") {
            Ok(_) => {
                println!("UFS lba trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!(
                    "Error generating UFS lba trend PNG chart with Plotters: {}",
                    e
                );
            }
        }
        // UFS DTOC (Dispatch to Complete) 차트
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "dtoc") {
            Ok(_) => {
                println!("UFS latency trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!(
                    "Error generating UFS latency trend PNG chart with Plotters: {}",
                    e
                );
            }
        }

        // UFS CTOC (Complete to Complete) 차트
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "ctoc") {
            Ok(_) => {
                println!("UFS complete-to-complete trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!(
                    "Error generating UFS complete-to-complete trend PNG chart: {}",
                    e
                );
            }
        }

        // UFS CTOD (Complete to Dispatch) 차트
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "ctod") {
            Ok(_) => {
                println!("UFS complete-to-dispatch trend PNG chart generated with Plotters.");
            }
            Err(e) => {
                eprintln!(
                    "Error generating UFS complete-to-dispatch trend PNG chart: {}",
                    e
                );
            }
        }

        // UFS Queue Depth 차트
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "qd") {
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

/// UFS 메트릭 정보를 담는 구조체
struct UfsMetricInfo<'a> {
    metric_name: &'a str,
    metric_label: &'a str,
    metric_extractor: fn(&UFS) -> f64,
    file_suffix: &'a str,
    require_positive: bool,
}

/// 통합된 UFS 지표 차트 생성 함수
/// 매개변수로 받은 metric에 따라 다양한 UFS 차트를 생성합니다
pub fn create_ufs_metric_chart(
    data: &[UFS],
    output_prefix: &str,
    config: &PlottersConfig,
    metric: &str,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating charts".to_string());
    }

    // 메트릭 이름과 값 추출기를 매핑
    let metric_info = match metric {
        "dtoc" => UfsMetricInfo {
            metric_name: "Latency",
            metric_label: "Latency (ms)",
            metric_extractor: |ufs| ufs.dtoc,
            file_suffix: "latency",
            require_positive: true,
        },
        "ctoc" => UfsMetricInfo {
            metric_name: "Complete to Complete Time",
            metric_label: "Complete to Complete (ms)",
            metric_extractor: |ufs| ufs.ctoc,
            file_suffix: "ctoc",
            require_positive: true,
        },
        "ctod" => UfsMetricInfo {
            metric_name: "Complete to Dispatch Time",
            metric_label: "Complete to Dispatch (ms)",
            metric_extractor: |ufs| ufs.ctod,
            file_suffix: "ctod",
            require_positive: true,
        },
        "qd" => UfsMetricInfo {
            metric_name: "Queue Depth",
            metric_label: "Queue Depth",
            metric_extractor: |ufs| ufs.qd as f64,
            file_suffix: "qd",
            require_positive: false,
        },
        "lba" => UfsMetricInfo {
            metric_name: "LBA",
            metric_label: "LBA",
            metric_extractor: |ufs| ufs.lba as f64,
            file_suffix: "lba",
            require_positive: false,
        },
        _ => return Err(format!("Unknown metric: {}", metric)),
    };

    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        // 양수 값이 필요한 메트릭은 필터링
        if !metric_info.require_positive || (metric_info.metric_extractor)(item) > 0.0 {
            opcode_groups
                .entry(item.opcode.clone())
                .or_default()
                .push(item);
        }
    }

    if opcode_groups.is_empty() {
        return Err(format!("No valid data for UFS {} chart", metric_info.metric_name));
    }

    // 명령어 이름 변환 및 색상 매핑을 위한 새로운 그룹 생성
    let mut named_opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for (opcode, events) in opcode_groups {
        let opcode_name = get_ufs_opcode_name(&opcode);
        named_opcode_groups.insert(opcode_name, events);
    }

    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_{}_plotters.png", output_prefix, metric_info.file_suffix);

    // 필터 조건 생성
    let filter_condition = if metric_info.require_positive {
        Some(move |ufs: &&UFS| (metric_info.metric_extractor)(ufs) > 0.0)
    } else {
        None
    };

    create_xy_scatter_chart(
        &named_opcode_groups,
        &png_path,
        config,
        &format!("UFS {} by Operation Code", metric_info.metric_name),
        "Time (s)",
        metric_info.metric_label,
        |ufs| ufs.time,
        metric_info.metric_extractor,
        ufs_opcode_color_mapper,
        filter_condition,
    )?;

    println!("UFS {} PNG chart saved to: {}", metric_info.metric_name, png_path);

    Ok(())
}

/// Creates Block I/O charts using Plotters library
pub fn create_block_io_plotters(
    data: &[Block],
    output_prefix: &str,
    config: &PlottersConfig,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O data is empty.".to_string());
    }

    // I/O 타입별로 데이터 그룹화
    let mut io_type_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in data {
        io_type_groups
            .entry(block.io_type.clone())
            .or_default()
            .push(block);
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
            Some(|block: &&Block| block.dtoc > 0.0),
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
            Some(|block: &&Block| block.dtoc > 0.0),
        )?;

        println!("Block I/O LBA vs Latency PNG chart saved to: {}", png_path);
    }

    Ok(())
}

// 중복된 UFS 관련 차트 함수들은 create_ufs_metric_chart 함수로 통합되었습니다.

/// Create UFSCUSTOM charts using Plotters library
pub fn create_ufscustom_plotters(
    data: &[UFSCUSTOM],
    output_prefix: &str,
    config: &PlottersConfig,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFSCUSTOM data is empty.".to_string());
    }

    // 명령어별로 데이터 그룹화
    let mut command_groups: HashMap<String, Vec<&UFSCUSTOM>> = HashMap::new();
    for event in data {
        command_groups
            .entry(event.opcode.clone())
            .or_default()
            .push(event);
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
            Option::<fn(&&UFSCUSTOM) -> bool>::None,
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
            Some(|event: &&UFSCUSTOM| event.dtoc > 0.0),
        )?;

        println!(
            "UFSCUSTOM Latency over Time PNG chart saved to: {}",
            png_path
        );
    }

    Ok(())
}
