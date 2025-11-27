use crate::models::{Block, UFS, UFSCUSTOM};
use plotters::prelude::*;
use std::collections::HashMap;
use std::error::Error;

/// Plotters 차트 생성을 위한 공통 구조체
#[derive(Clone)]
pub struct PlottersConfig {
    pub width: u32,
    pub height: u32,
    pub font_family: &'static str,
    pub title_font_size: u32,
    pub axis_label_font_size: u32,
    pub tick_label_font_size: u32,
    pub point_size: u32,
    pub legend_spacing: u32,
    pub y_axis_range: Option<(f64, f64)>, // y축 범위 고정 옵션 (min, max)
}

impl Default for PlottersConfig {
    fn default() -> Self {
        Self {
            width: 1800,
            height: 800,
            font_family: "D2Coding",
            title_font_size: 24,
            axis_label_font_size: 16,
            tick_label_font_size: 12,
            point_size: 2,
            legend_spacing: 25,
            y_axis_range: None, // 기본값은 자동 범위
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

/// Block I/O 타입에 따른 색상 매핑
fn get_color_for_io_type(io_type: &str) -> RGBColor {
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

/// 데이터 범위에 패딩을 추가하는 함수
fn add_padding_to_range(min: f64, max: f64, padding_percent: f64) -> (f64, f64) {
    let range = max - min;
    let padding = range * padding_percent;
    (min - padding, max + padding)
}

/// 레전드 그리기 함수
fn draw_legend(
    legend_area: &DrawingArea<BitMapBackend, plotters::coord::Shift>,
    legends: &[(String, RGBColor)],
    config: &PlottersConfig,
) -> Result<(), Box<dyn Error>> {
    legend_area.fill(&WHITE.mix(0.95))?;

    // 레전드 영역의 높이 가져오기
    let (_, legend_height) = legend_area.dim_in_pixel();
    let total_legend_height = legends.len() * config.legend_spacing as usize;

    // 세로 가운데 정렬을 위한 시작 위치 계산
    let start_y = ((legend_height as i32 - total_legend_height as i32) / 2).max(20);

    for (i, (name, color)) in legends.iter().enumerate() {
        let spacing = config.legend_spacing as usize;
        let y_pos = start_y + (i * spacing) as i32;

        // 직선으로 레전드 표시
        legend_area.draw(&PathElement::new(
            vec![(15_i32, y_pos), (45_i32, y_pos)],
            color.stroke_width(2),
        ))?;

        legend_area.draw(&Text::new(
            name.clone(),
            (50_i32, y_pos),
            (
                config.font_family,
                config.tick_label_font_size,
                FontStyle::Normal,
            ), // Bold 제거
        ))?;
    }

    Ok(())
}

/// UFS 색상 매핑 헬퍼 함수 - 주요 opcode 색상 고정
fn ufs_opcode_color_mapper(opcode: &str) -> RGBColor {
    match opcode {
        "0x28" => RGBColor(65, 105, 225), // READ_10 - 파란색 계열
        "0x2a" => RGBColor(220, 20, 60),  // WRITE_10 - 빨간색 계열
        "0x35" => RGBColor(255, 215, 0),  // SYNCHRONIZE_CACHE_10 - 노란색 계열
        "0x42" => RGBColor(138, 43, 226), // UNMAP/DISCARD - 보라색 계열
        _ => RGBColor(50, 50, 50),        // 기타 - 검은색 계열
    }
}

/// CPU 색상 매핑 헬퍼 함수 - CPU 번호에 따라 색상 고정
fn cpu_color_mapper(cpu: &str) -> RGBColor {
    match cpu {
        "0" => RGBColor(228, 26, 28),   // 빨강
        "1" => RGBColor(55, 126, 184),  // 파랑
        "2" => RGBColor(77, 175, 74),   // 초록
        "3" => RGBColor(152, 78, 163),  // 보라
        "4" => RGBColor(255, 127, 0),   // 주황
        "5" => RGBColor(255, 255, 51),  // 노랑
        "6" => RGBColor(166, 86, 40),   // 갈색
        "7" => RGBColor(247, 129, 191), // 분홍
        _ => RGBColor(50, 50, 50),      // 기타 (8 이상 또는 음수) - 검은색
    }
}

/// 일반적인 X대비 Y 그래프 생성을 위한 함수
/// T: 데이터 타입, F: X축 추출 함수, G: Y축 데이터 추출 함수, H: 필터 조건 함수
///
/// 이 함수는 ChartConfig를 사용하는 create_xy_scatter_chart_with_config의 래퍼 함수입니다.
#[allow(clippy::too_many_arguments)]
fn create_xy_scatter_chart<T, F, G, H>(
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
fn create_xy_scatter_chart_with_config<T, F, G, H>(
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

    // chart 영역과 legend 영역을 분리 (y축 label을 위한 공간 확보)
    let (chart_area, legend_area) = root.split_horizontally(1100);

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

    // y축 범위 설정 - 고정 범위가 설정되어 있으면 사용, 없으면 자동 계산
    let (min_y, max_y) = if let Some((fixed_min, fixed_max)) = chart_config.config.y_axis_range {
        (fixed_min, fixed_max)
    } else {
        let min_y = (min_y.max(0.0) - (max_y - min_y) * 0.05).max(0.0); // 0 아래로 내려가지 않게
        let max_y = max_y + (max_y - min_y) * 0.05;
        (min_y, max_y)
    };

    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption(
            chart_config.title,
            (
                chart_config.config.font_family,
                chart_config.config.title_font_size,
                FontStyle::Normal,
            ),
        )
        .margin(30)
        .x_label_area_size(70)
        .y_label_area_size(150)
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
            FontStyle::Normal, // Bold 제거, Normal 스타일 명시적 지정
        ))
        .label_style((
            chart_config.config.font_family,
            chart_config.config.tick_label_font_size,
            FontStyle::Normal,
        ))
        // .disable_mesh()  // 격자 비활성화
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

/// UFS 메트릭 정보를 담는 구조체
struct UfsMetricInfo<'a> {
    metric_name: &'a str,
    metric_label: &'a str,
    metric_extractor: fn(&UFS) -> f64,
    file_suffix: &'a str,
    require_positive: bool,
}

/// UFS CPU 차트 생성 함수
/// Send/Complete별로 Time vs CPU 또는 Time vs LBA 차트를 생성합니다
fn create_ufs_cpu_chart(
    data: &[UFS],
    output_prefix: &str,
    config: &PlottersConfig,
    action_filter: &str, // "send_req" 또는 "complete_rsp"
    chart_type: &str,    // "cpu" 또는 "address"
) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFS data available for generating CPU charts".to_string());
    }

    // CPU별로 데이터 그룹화
    let mut cpu_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        // 지정된 action만 포함
        if item.action == action_filter {
            cpu_groups
                .entry(item.cpu.to_string())
                .or_default()
                .push(item);
        }
    }

    if cpu_groups.is_empty() {
        return Err(format!(
            "No valid data for UFS {action_filter} {chart_type} chart"
        ));
    }

    let action_label = if action_filter == "send_req" {
        "Send"
    } else {
        "Complete"
    };

    let (title, y_label, png_path, y_range) = match chart_type {
        "cpu" => (
            format!("UFS {action_label} - CPU Allocation over Time"),
            "CPU",
            format!("{output_prefix}_ufs_{action_filter}_cpu_time_plotters.png"),
            Some((-1.0, 8.0)),
        ),
        "address" => (
            format!("UFS {action_label} - LBA Distribution over Time (by CPU)"),
            "LBA",
            format!("{output_prefix}_ufs_{action_filter}_lba_time_plotters.png"),
            None,
        ),
        _ => return Err(format!("Unknown chart type: {chart_type}")),
    };

    let y_extractor: fn(&UFS) -> f64 = match chart_type {
        "cpu" => |ufs: &UFS| ufs.cpu as f64,
        "address" => |ufs: &UFS| ufs.lba as f64,
        _ => return Err(format!("Unknown chart type: {chart_type}")),
    };

    // y축 범위 설정
    let mut chart_config = config.clone();
    if let Some(range) = y_range {
        chart_config.y_axis_range = Some(range);
    }

    create_xy_scatter_chart(
        &cpu_groups,
        &png_path,
        &chart_config,
        &title,
        "Time (s)",
        y_label,
        |ufs| ufs.time,
        y_extractor,
        cpu_color_mapper,
        None::<fn(&&UFS) -> bool>,
    )?;

    println!("UFS {action_filter} {chart_type} chart saved to: {png_path}");

    Ok(())
}

/// 통합된 UFS 지표 차트 생성 함수
/// 매개변수로 받은 metric에 따라 다양한 UFS 차트를 생성합니다
fn create_ufs_metric_chart(
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
            file_suffix: "dtoc",
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
        _ => return Err(format!("Unknown metric: {metric}")),
    };

    // opcode별로 데이터 그룹화 (opcode 값 그대로 사용)
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for item in data {
        // LBA와 QD 메트릭의 경우 send_req만 포함
        let include_item = if metric == "lba" || metric == "qd" {
            item.action == "send_req"
        } else {
            // 다른 메트릭은 기존 로직 유지
            !metric_info.require_positive || (metric_info.metric_extractor)(item) > 0.0
        };

        if include_item {
            opcode_groups
                .entry(item.opcode.clone()) // opcode 값 그대로 사용
                .or_default()
                .push(item);
        }
    }

    if opcode_groups.is_empty() {
        return Err(format!(
            "No valid data for UFS {} chart",
            metric_info.metric_name
        ));
    }

    // PNG 파일 경로 생성
    let png_path = format!(
        "{}_ufs_{}_plotters.png",
        output_prefix, metric_info.file_suffix
    );

    // 필터 조건 생성
    let filter_condition = if metric_info.require_positive {
        Some(move |ufs: &&UFS| (metric_info.metric_extractor)(ufs) > 0.0)
    } else {
        None
    };

    create_xy_scatter_chart(
        &opcode_groups,
        &png_path,
        config,
        &format!("UFS {} by Operation Code", metric_info.metric_name),
        "Time (s)",
        metric_info.metric_label,
        |ufs| ufs.time,
        metric_info.metric_extractor,
        ufs_opcode_color_mapper, // 주요 opcode 색상 고정
        filter_condition,
    )?;

    println!(
        "UFS {} PNG chart saved to: {}",
        metric_info.metric_name, png_path
    );

    Ok(())
}

/// Block I/O 메트릭 정보를 담는 구조체
struct BlockMetricInfo<'a> {
    metric_name: &'a str,
    metric_label: &'a str,
    metric_extractor: fn(&Block) -> f64,
    file_suffix: &'a str,
    require_positive: bool,
}

/// Block CPU 차트 생성 함수
/// Send/Complete별로 Time vs CPU 또는 Time vs Sector 차트를 생성합니다
fn create_block_cpu_chart(
    data: &[Block],
    output_prefix: &str,
    config: &PlottersConfig,
    action_filter: &str, // "issue" 또는 "complete"
    chart_type: &str,    // "cpu" 또는 "address"
) -> Result<(), String> {
    if data.is_empty() {
        return Err("No Block data available for generating CPU charts".to_string());
    }

    // CPU별로 데이터 그룹화
    let mut cpu_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for item in data {
        // action_filter에 따라 필터링
        let is_match = match action_filter {
            "issue" => item.action == "block_rq_issue" || item.action == "Q",
            "complete" => item.action == "block_rq_complete" || item.action == "C",
            _ => false,
        };

        if is_match {
            cpu_groups
                .entry(item.cpu.to_string())
                .or_default()
                .push(item);
        }
    }

    if cpu_groups.is_empty() {
        return Err(format!(
            "No valid data for Block {action_filter} {chart_type} chart"
        ));
    }

    let action_label = if action_filter == "issue" {
        "Send"
    } else {
        "Complete"
    };

    let (title, y_label, png_path, y_range) = match chart_type {
        "cpu" => (
            format!("Block I/O {action_label} - CPU Allocation over Time"),
            "CPU",
            format!("{output_prefix}_block_{action_filter}_cpu_time_plotters.png"),
            Some((-1.0, 8.0)),
        ),
        "address" => (
            format!("Block I/O {action_label} - Sector Distribution over Time (by CPU)"),
            "Sector",
            format!("{output_prefix}_block_{action_filter}_sector_time_plotters.png"),
            None,
        ),
        _ => return Err(format!("Unknown chart type: {chart_type}")),
    };

    let y_extractor: fn(&Block) -> f64 = match chart_type {
        "cpu" => |block: &Block| block.cpu as f64,
        "address" => |block: &Block| block.sector as f64,
        _ => return Err(format!("Unknown chart type: {chart_type}")),
    };

    // y축 범위 설정
    let mut chart_config = config.clone();
    if let Some(range) = y_range {
        chart_config.y_axis_range = Some(range);
    }

    create_xy_scatter_chart(
        &cpu_groups,
        &png_path,
        &chart_config,
        &title,
        "Time (s)",
        y_label,
        |block| block.time,
        y_extractor,
        cpu_color_mapper,
        None::<fn(&&Block) -> bool>,
    )?;

    println!("Block {action_filter} {chart_type} chart saved to: {png_path}");

    Ok(())
}

/// 통합된 Block I/O 지표 차트 생성 함수
/// 매개변수로 받은 metric에 따라 다양한 Block I/O 차트를 생성합니다
fn create_block_metric_chart(
    data: &[Block],
    output_prefix: &str,
    config: &PlottersConfig,
    metric: &str,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("No Block I/O data available for generating charts".to_string());
    }

    // 메트릭 이름과 값 추출기를 매핑
    let metric_info = match metric {
        "dtoc" => BlockMetricInfo {
            metric_name: "Latency",
            metric_label: "Latency (ms)",
            metric_extractor: |block| block.dtoc,
            file_suffix: "dtoc",
            require_positive: true,
        },
        "ctoc" => BlockMetricInfo {
            metric_name: "Complete to Complete Time",
            metric_label: "Complete to Complete (ms)",
            metric_extractor: |block| block.ctoc,
            file_suffix: "ctoc",
            require_positive: true,
        },
        "ctod" => BlockMetricInfo {
            metric_name: "Complete to Dispatch Time",
            metric_label: "Complete to Dispatch (ms)",
            metric_extractor: |block| block.ctod,
            file_suffix: "ctod",
            require_positive: true,
        },
        "qd" => BlockMetricInfo {
            metric_name: "Queue Depth",
            metric_label: "Queue Depth",
            metric_extractor: |block| block.qd as f64,
            file_suffix: "qd",
            require_positive: false,
        },
        "lba" => BlockMetricInfo {
            metric_name: "Sector",
            metric_label: "Sector",
            metric_extractor: |block| block.sector as f64,
            file_suffix: "lba",
            require_positive: false,
        },
        _ => return Err(format!("Unknown metric: {metric}")),
    };

    // I/O 타입별로 데이터 그룹화
    let mut io_type_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in data {
        // LBA와 QD 메트릭의 경우 issue만 포함 (block_rq_issue 또는 Q)
        let include_item = if metric == "lba" || metric == "qd" {
            block.action == "block_rq_issue" || block.action == "Q"
        } else {
            // 다른 메트릭은 기존 로직 유지
            !metric_info.require_positive || (metric_info.metric_extractor)(block) > 0.0
        };

        if include_item {
            io_type_groups
                .entry(block.io_type.clone())
                .or_default()
                .push(block);
        }
    }

    if io_type_groups.is_empty() {
        return Err(format!(
            "No valid data for Block I/O {} chart",
            metric_info.metric_name
        ));
    }

    // PNG 파일 경로 생성
    let png_path = format!(
        "{}_block_{}_plotters.png",
        output_prefix, metric_info.file_suffix
    );

    // 필터 조건 생성
    let filter_condition = if metric_info.require_positive {
        Some(move |block: &&Block| (metric_info.metric_extractor)(block) > 0.0)
    } else {
        None
    };

    create_xy_scatter_chart(
        &io_type_groups,
        &png_path,
        config,
        &format!("Block I/O {} by I/O Type", metric_info.metric_name),
        "Time (s)",
        metric_info.metric_label,
        |block| block.time,
        metric_info.metric_extractor,
        get_color_for_io_type,
        filter_condition,
    )?;

    println!(
        "Block I/O {} PNG chart saved to: {}",
        metric_info.metric_name, png_path
    );

    Ok(())
}

/// UFSCUSTOM 메트릭 정보를 담는 구조체
struct UfscustomMetricInfo<'a> {
    metric_name: &'a str,
    metric_label: &'a str,
    metric_extractor: fn(&UFSCUSTOM) -> f64,
    file_suffix: &'a str,
    require_positive: bool,
}

/// 통합된 UFSCUSTOM 지표 차트 생성 함수
/// 매개변수로 받은 metric에 따라 다양한 UFSCUSTOM 차트를 생성합니다
fn create_ufscustom_metric_chart(
    data: &[UFSCUSTOM],
    output_prefix: &str,
    config: &PlottersConfig,
    metric: &str,
) -> Result<(), String> {
    if data.is_empty() {
        return Err("No UFSCUSTOM data available for generating charts".to_string());
    }

    // 메트릭 이름과 값 추출기를 매핑
    let metric_info = match metric {
        "dtoc" => UfscustomMetricInfo {
            metric_name: "Latency",
            metric_label: "Latency (ms)",
            metric_extractor: |ufscustom| ufscustom.dtoc,
            file_suffix: "dtoc",
            require_positive: true,
        },
        "ctoc" => UfscustomMetricInfo {
            metric_name: "Complete to Complete Time",
            metric_label: "Complete to Complete (ms)",
            metric_extractor: |ufscustom| ufscustom.ctoc,
            file_suffix: "ctoc",
            require_positive: true,
        },
        "ctod" => UfscustomMetricInfo {
            metric_name: "Complete to Dispatch Time",
            metric_label: "Complete to Dispatch (ms)",
            metric_extractor: |ufscustom| ufscustom.ctod,
            file_suffix: "ctod",
            require_positive: true,
        },
        "start_qd" => UfscustomMetricInfo {
            metric_name: "Start Queue Depth",
            metric_label: "Start Queue Depth",
            metric_extractor: |ufscustom| ufscustom.start_qd as f64,
            file_suffix: "start_qd",
            require_positive: false,
        },
        "end_qd" => UfscustomMetricInfo {
            metric_name: "End Queue Depth",
            metric_label: "End Queue Depth",
            metric_extractor: |ufscustom| ufscustom.end_qd as f64,
            file_suffix: "end_qd",
            require_positive: false,
        },
        "lba" => UfscustomMetricInfo {
            metric_name: "LBA",
            metric_label: "LBA",
            metric_extractor: |ufscustom| ufscustom.lba as f64,
            file_suffix: "lba",
            require_positive: false,
        },
        _ => return Err(format!("Unknown metric: {metric}")),
    };

    // opcode별로 데이터 그룹화 (opcode 값 그대로 사용)
    let mut opcode_groups: HashMap<String, Vec<&UFSCUSTOM>> = HashMap::new();
    for item in data {
        // 양수 값이 필요한 메트릭은 필터링
        if !metric_info.require_positive || (metric_info.metric_extractor)(item) > 0.0 {
            opcode_groups
                .entry(item.opcode.clone()) // opcode 값 그대로 사용
                .or_default()
                .push(item);
        }
    }

    if opcode_groups.is_empty() {
        return Err(format!(
            "No valid data for UFSCUSTOM {} chart",
            metric_info.metric_name
        ));
    }

    // PNG 파일 경로 생성
    let png_path = format!(
        "{}_ufscustom_{}_plotters.png",
        output_prefix, metric_info.file_suffix
    );

    // 필터 조건 생성
    let filter_condition = if metric_info.require_positive {
        Some(move |ufscustom: &&UFSCUSTOM| (metric_info.metric_extractor)(ufscustom) > 0.0)
    } else {
        None
    };

    create_xy_scatter_chart(
        &opcode_groups,
        &png_path,
        config,
        &format!("UFSCUSTOM {} by Operation Code", metric_info.metric_name),
        "Time (s)",
        metric_info.metric_label,
        |ufscustom| ufscustom.start_time,
        metric_info.metric_extractor,
        ufs_opcode_color_mapper, // 주요 opcode 색상 고정
        filter_condition,
    )?;

    println!(
        "UFSCUSTOM {} PNG chart saved to: {}",
        metric_info.metric_name, png_path
    );

    Ok(())
}

/// Generate charts and save statistics data.
pub fn generate_charts(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), String> {
    generate_charts_with_config(
        processed_ufs,
        processed_blocks,
        processed_ufscustom,
        output_prefix,
        None,
    )
}

/// Generate charts with custom y-axis ranges for different metrics.
pub fn generate_charts_with_config(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
    y_axis_ranges: Option<&std::collections::HashMap<String, (f64, f64)>>,
) -> Result<(), String> {
    println!("\nGenerating charts...");

    // 메트릭별 y축 범위를 가져오는 헬퍼 함수
    let get_y_range_for_metric = |metric: &str| -> Option<(f64, f64)> {
        y_axis_ranges.and_then(|ranges| ranges.get(metric).copied())
    };

    // UFS 차트 생성
    if !processed_ufs.is_empty() {
        let config = PlottersConfig::default();

        // UFS Send CPU Time 차트
        match create_ufs_cpu_chart(processed_ufs, output_prefix, &config, "send_req", "cpu") {
            Ok(_) => {
                println!("UFS send_req CPU time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS send_req CPU time chart: {e}");
            }
        }

        // UFS Send LBA Time 차트 (CPU별 색상)
        match create_ufs_cpu_chart(processed_ufs, output_prefix, &config, "send_req", "address") {
            Ok(_) => {
                println!("UFS send_req LBA time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS send_req LBA time chart: {e}");
            }
        }

        // UFS Complete CPU Time 차트
        match create_ufs_cpu_chart(processed_ufs, output_prefix, &config, "complete_rsp", "cpu") {
            Ok(_) => {
                println!("UFS complete_rsp CPU time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete_rsp CPU time chart: {e}");
            }
        }

        // UFS Complete LBA Time 차트 (CPU별 색상)
        match create_ufs_cpu_chart(
            processed_ufs,
            output_prefix,
            &config,
            "complete_rsp",
            "address",
        ) {
            Ok(_) => {
                println!("UFS complete_rsp LBA time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete_rsp LBA time chart: {e}");
            }
        }

        // UFS lba 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufs_lba"),
            ..Default::default()
        };
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "lba") {
            Ok(_) => {
                println!("UFS lba trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS lba trend PNG chart: {e}");
            }
        }

        // UFS DTOC (Dispatch to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufs_dtoc"),
            ..Default::default()
        };
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "dtoc") {
            Ok(_) => {
                println!("UFS dtoc trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS dtoc trend PNG chart: {e}");
            }
        }

        // UFS CTOC (Complete to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufs_ctoc"),
            ..Default::default()
        };
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "ctoc") {
            Ok(_) => {
                println!("UFS complete-to-complete trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete-to-complete trend PNG chart: {e}");
            }
        }

        // UFS CTOD (Complete to Dispatch) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufs_ctod"),
            ..Default::default()
        };
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "ctod") {
            Ok(_) => {
                println!("UFS complete-to-dispatch trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS complete-to-dispatch trend PNG chart: {e}");
            }
        }

        // UFS Queue Depth 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufs_qd"),
            ..Default::default()
        };
        match create_ufs_metric_chart(processed_ufs, output_prefix, &config, "qd") {
            Ok(_) => {
                println!("UFS queue depth trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS queue depth trend PNG chart: {e}");
            }
        }
    }

    // Block I/O 차트 생성
    if !processed_blocks.is_empty() {
        let config = PlottersConfig::default();

        // Block Send CPU Time 차트
        match create_block_cpu_chart(processed_blocks, output_prefix, &config, "issue", "cpu") {
            Ok(_) => {
                println!("Block I/O issue CPU time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O issue CPU time chart: {e}");
            }
        }

        // Block Send Sector Time 차트 (CPU별 색상)
        match create_block_cpu_chart(processed_blocks, output_prefix, &config, "issue", "address") {
            Ok(_) => {
                println!("Block I/O issue sector time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O issue sector time chart: {e}");
            }
        }

        // Block Complete CPU Time 차트
        match create_block_cpu_chart(processed_blocks, output_prefix, &config, "complete", "cpu") {
            Ok(_) => {
                println!("Block I/O complete CPU time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O complete CPU time chart: {e}");
            }
        }

        // Block Complete Sector Time 차트 (CPU별 색상)
        match create_block_cpu_chart(
            processed_blocks,
            output_prefix,
            &config,
            "complete",
            "address",
        ) {
            Ok(_) => {
                println!("Block I/O complete sector time chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O complete sector time chart: {e}");
            }
        }

        // Block I/O lba 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("block_lba"),
            ..Default::default()
        };
        match create_block_metric_chart(processed_blocks, output_prefix, &config, "lba") {
            Ok(_) => {
                println!("Block I/O lba trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O lba trend PNG chart: {e}");
            }
        }

        // Block I/O DTOC (Dispatch to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("block_dtoc"),
            ..Default::default()
        };
        match create_block_metric_chart(processed_blocks, output_prefix, &config, "dtoc") {
            Ok(_) => {
                println!("Block I/O dtoc trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O dtoc trend PNG chart: {e}");
            }
        }

        // Block I/O CTOC (Complete to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("block_ctoc"),
            ..Default::default()
        };
        match create_block_metric_chart(processed_blocks, output_prefix, &config, "ctoc") {
            Ok(_) => {
                println!("Block I/O complete-to-complete trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O complete-to-complete trend PNG chart: {e}");
            }
        }

        // Block I/O CTOD (Complete to Dispatch) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("block_ctod"),
            ..Default::default()
        };
        match create_block_metric_chart(processed_blocks, output_prefix, &config, "ctod") {
            Ok(_) => {
                println!("Block I/O complete-to-dispatch trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O complete-to-dispatch trend PNG chart: {e}");
            }
        }

        // Block I/O Queue Depth 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("block_qd"),
            ..Default::default()
        };
        match create_block_metric_chart(processed_blocks, output_prefix, &config, "qd") {
            Ok(_) => {
                println!("Block I/O queue depth trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O queue depth trend PNG chart: {e}");
            }
        }
    }

    // UFSCUSTOM 차트 생성
    if !processed_ufscustom.is_empty() {
        // UFSCUSTOM lba 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_lba"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "lba") {
            Ok(_) => {
                println!("UFSCUSTOM lba trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM lba trend PNG chart: {e}");
            }
        }

        // UFSCUSTOM DTOC (Dispatch to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_dtoc"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "dtoc") {
            Ok(_) => {
                println!("UFSCUSTOM dtoc trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM dtoc trend PNG chart: {e}");
            }
        }

        // UFSCUSTOM CTOC (Complete to Complete) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_ctoc"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "ctoc") {
            Ok(_) => {
                println!("UFSCUSTOM complete-to-complete trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM complete-to-complete trend PNG chart: {e}");
            }
        }

        // UFSCUSTOM CTOD (Complete to Dispatch) 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_ctod"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "ctod") {
            Ok(_) => {
                println!("UFSCUSTOM complete-to-dispatch trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM complete-to-dispatch trend PNG chart: {e}");
            }
        }

        // UFSCUSTOM Start Queue Depth 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_start_qd"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "start_qd")
        {
            Ok(_) => {
                println!("UFSCUSTOM start queue depth trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM start queue depth trend PNG chart: {e}");
            }
        }

        // UFSCUSTOM End Queue Depth 차트
        let config = PlottersConfig {
            y_axis_range: get_y_range_for_metric("ufscustom_end_qd"),
            ..Default::default()
        };
        match create_ufscustom_metric_chart(processed_ufscustom, output_prefix, &config, "end_qd") {
            Ok(_) => {
                println!("UFSCUSTOM end queue depth trend PNG chart generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM end queue depth trend PNG chart: {e}");
            }
        }
    }

    println!("Charts generated successfully.");

    Ok(())
}
