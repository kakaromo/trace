use crate::models::{Block, UFS, UFSCUSTOM};
use std::collections::HashMap;
use std::path::Path;
use charming::Chart;
use charming::component::{Title as CharmingTitle, Legend as CharmingLegend, Grid, Axis as CharmingAxis};
use charming::element::{AxisType, ItemStyle, NameLocation, Orient, Tooltip, Trigger};
use charming::series::{Line, Bar, Pie as CharmingPie, EffectScatter, Scatter as CharmingScatter};
use charming::renderer::{HtmlRenderer, ImageRenderer, ImageFormat};
use charming::theme::Theme;

// plotters 의존성 추가
use plotters::prelude::*;
use std::error::Error;

/// Generate charming-based interactive charts for trace data
pub fn generate_charming_charts(
    processed_ufs: &[UFS], 
    processed_blocks: &[Block], 
    output_prefix: &str
) -> Result<(), String> {
    if processed_ufs.is_empty() && processed_blocks.is_empty() {
        return Err("No data available for generating charts".to_string());
    }
    
    // UFS Latency Trend Chart
    if !processed_ufs.is_empty() {
        match create_ufs_latency_trend_chart(processed_ufs, output_prefix) {
            Ok(_) => println!("UFS latency trend chart generated with Charming"),
            Err(e) => eprintln!("Failed to generate UFS latency trend chart: {}", e),
        }
    }
    
    // Block I/O Analysis Chart
    if !processed_blocks.is_empty() {
        match create_block_operation_chart(processed_blocks, output_prefix) {
            Ok(_) => println!("Block I/O operation chart generated with Charming"),
            Err(e) => eprintln!("Failed to generate Block I/O operation chart: {}", e),
        }
    }
    
    // Performance Comparison Chart
    if !processed_ufs.is_empty() && !processed_blocks.is_empty() {
        match create_performance_comparison_chart(processed_ufs, processed_blocks, output_prefix) {
            Ok(_) => println!("Performance comparison chart generated with Charming"),
            Err(e) => eprintln!("Failed to generate performance comparison chart: {}", e),
        }
    }
    
    // Operation Distribution Pie Chart
    if !processed_ufs.is_empty() {
        match create_operation_distribution_chart(processed_ufs, output_prefix) {
            Ok(_) => println!("UFS operation distribution chart generated with Charming"),
            Err(e) => eprintln!("Failed to generate UFS operation distribution chart: {}", e),
        }
    }
    
    // Scatter Plot of LBA vs Latency
    if !processed_blocks.is_empty() {
        match create_lba_latency_scatter(processed_blocks, output_prefix) {
            Ok(_) => println!("LBA vs Latency scatter plot generated with Charming"),
            Err(e) => eprintln!("Failed to generate LBA vs Latency scatter plot: {}", e),
        }
    }
    
    Ok(())
}

/// Create UFS latency trend chart using Charming
fn create_ufs_latency_trend_chart(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    // Sort data by time
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| {
        a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Group data by opcode and calculate average latency over time windows
    let opcodes: Vec<String> = time_sorted_data
        .iter()
        .map(|d| d.opcode.clone())
        .collect::<std::collections::HashSet<String>>()
        .into_iter()
        .collect();
    
    let window_size = 20; // Aggregate data points in windows for smoother line
    let mut chart_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    
    for opcode in &opcodes {
        let mut window_times = Vec::new();
        let mut window_latencies = Vec::new();
        
        for item in &time_sorted_data {
            if &item.opcode == opcode && item.dtoc > 0.0 {
                window_times.push(item.time);
                window_latencies.push(item.dtoc);
                
                if window_times.len() >= window_size {
                    let avg_time = window_times.iter().sum::<f64>() / window_times.len() as f64;
                    let avg_latency = window_latencies.iter().sum::<f64>() / window_latencies.len() as f64;
                    
                    chart_data
                        .entry(opcode.clone())
                        .or_insert_with(Vec::new)
                        .push((avg_time, avg_latency));
                    
                    window_times.clear();
                    window_latencies.clear();
                }
            }
        }
        
        // Process any remaining data points
        if !window_times.is_empty() {
            let avg_time = window_times.iter().sum::<f64>() / window_times.len() as f64;
            let avg_latency = window_latencies.iter().sum::<f64>() / window_latencies.len() as f64;
            
            chart_data
                .entry(opcode.clone())
                .or_insert_with(Vec::new)
                .push((avg_time, avg_latency));
        }
    }
    
    // Create the chart
    if chart_data.is_empty() {
        return Err("No valid data for UFS latency trend chart".to_string());
    }
    
    let mut chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Latency Trend by Operation Code"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    let color_map = [
        "#5470c6", "#91cc75", "#fac858", "#ee6666", 
        "#73c0de", "#3ba272", "#fc8452", "#9a60b4"
    ];
    
    let mut color_idx = 0;
    for (opcode, points) in &chart_data {
        let mut x_values = Vec::new();
        let mut y_values = Vec::new();
        
        for (time, latency) in points {
            x_values.push(*time);
            y_values.push(*latency);
        }
        
        let opcode_name = match opcode.as_str() {
            "0x28" => "READ_10",
            "0x2a" => "WRITE_10",
            "0x35" => "SYNCHRONIZE_CACHE_10",
            _ => opcode.as_str(),
        };
        
        let line_color = color_map[color_idx % color_map.len()];
        color_idx += 1;
        
        chart = chart.series(
            Line::new()
                .name(opcode_name)
                .data(x_values.iter().zip(y_values.iter()).map(|(x, y)| vec![*x, *y]).collect::<Vec<Vec<f64>>>())
                .symbol_size(8)
                .item_style(ItemStyle::new().color(line_color))
        );
    }
    
    // Save as HTML
    let html_output_path = format!("{}_ufs_latency_trend_charming.html", output_prefix);
    let mut htmlrenderer = HtmlRenderer::new("UFS Latency Trend", 1000, 800);
    htmlrenderer.save(&chart, &html_output_path).map_err(|e| e.to_string())?;
    println!("UFS latency trend HTML chart saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_ufs_latency_trend_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("UFS latency trend SVG chart saved to: {}", svg_output_path);
    
    // Save as PNG
    let png_output_path = format!("{}_ufs_latency_trend_charming.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save(&chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("UFS latency trend PNG chart saved to: {}", png_output_path);
    
    // Save as JPEG
    let jpg_output_path = format!("{}_ufs_latency_trend_charming.jpg", output_prefix);
    let mut jpg_renderer = ImageRenderer::new(1000, 800);
    jpg_renderer.save(&chart, &jpg_output_path).map_err(|e| e.to_string())?;
    println!("UFS latency trend JPEG chart saved to: {}", jpg_output_path);
    
    Ok(())
}

/// Create Block I/O operation analysis chart using Charming
fn create_block_operation_chart(data: &[Block], output_prefix: &str) -> Result<(), String> {
    // Extract data for IO types
    let mut io_types: HashMap<String, Vec<Block>> = HashMap::new();
    for block in data {
        io_types.entry(block.io_type.clone()).or_default().push(block.clone());
    }
    
    // Prepare data for the bar chart
    let io_type_labels: Vec<String> = io_types.keys().cloned().collect();
    let mut read_dtoc = Vec::new();
    let mut write_dtoc = Vec::new();
    
    for io_type in &io_type_labels {
        let blocks = io_types.get(io_type).unwrap();
        let avg_latency = blocks.iter().map(|b| b.dtoc).sum::<f64>() / blocks.len() as f64;
        
        if io_type == "READ" {
            read_dtoc.push(avg_latency);
            write_dtoc.push(0.0);
        } else if io_type == "WRITE" {
            read_dtoc.push(0.0);
            write_dtoc.push(avg_latency);
        } else {
            read_dtoc.push(0.0);
            write_dtoc.push(0.0);
        }
    }
    
    // Create the chart
    let mut chart = Chart::new()
        .title(CharmingTitle::new().text("Block I/O Operation Latency Analysis"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().data(vec!["READ Latency", "WRITE Latency"]))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Category)
                .data(io_type_labels.clone())
                .name("I/O Type")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Avg Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));
    
    chart = chart
        .series(
            Bar::new()
                .name("READ Latency")
                .data(read_dtoc.clone())
                .item_style(ItemStyle::new().color("#5470c6"))
        )
        .series(
            Bar::new()
                .name("WRITE Latency")
                .data(write_dtoc.clone())
                .item_style(ItemStyle::new().color("#91cc75"))
        );
    
    // Save as HTML
    let html_output_path = format!("{}_block_io_analysis_charming.html", output_prefix);
    let mut htmlrenderer = HtmlRenderer::new("Block I/O Analysis", 1000, 800);
    htmlrenderer.save(&chart, &html_output_path).map_err(|e| e.to_string())?;
    println!("Block I/O operation HTML chart saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_block_io_analysis_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("Block I/O operation SVG chart saved to: {}", svg_output_path);
    
    // Save as PNG
    let png_output_path = format!("{}_block_io_analysis_charming.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save(&chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("Block I/O operation PNG chart saved to: {}", png_output_path);
    
    // Save as JPEG
    let jpg_output_path = format!("{}_block_io_analysis_charming.jpg", output_prefix);
    let mut jpg_renderer = ImageRenderer::new(1000, 800);
    jpg_renderer.save(&chart, &jpg_output_path).map_err(|e| e.to_string())?;
    println!("Block I/O operation JPEG chart saved to: {}", jpg_output_path);
    
    Ok(())
}

/// Create performance comparison chart between UFS and Block I/O using Charming
fn create_performance_comparison_chart(ufs_data: &[UFS], block_data: &[Block], output_prefix: &str) -> Result<(), String> {
    // Calculate average latencies
    let ufs_read_latency = ufs_data
        .iter()
        .filter(|u| u.opcode == "0x28") // READ_10
        .map(|u| u.dtoc)
        .sum::<f64>() / ufs_data.iter().filter(|u| u.opcode == "0x28").count().max(1) as f64;
    
    let ufs_write_latency = ufs_data
        .iter()
        .filter(|u| u.opcode == "0x2a") // WRITE_10
        .map(|u| u.dtoc)
        .sum::<f64>() / ufs_data.iter().filter(|u| u.opcode == "0x2a").count().max(1) as f64;
    
    let block_read_latency = block_data
        .iter()
        .filter(|b| b.io_type == "READ")
        .map(|b| b.dtoc)
        .sum::<f64>() / block_data.iter().filter(|b| b.io_type == "READ").count().max(1) as f64;
    
    let block_write_latency = block_data
        .iter()
        .filter(|b| b.io_type == "WRITE")
        .map(|b| b.dtoc)
        .sum::<f64>() / block_data.iter().filter(|b| b.io_type == "WRITE").count().max(1) as f64;
    
    // Create the chart
    let mut chart = Chart::new()
        .title(CharmingTitle::new().text("UFS vs Block I/O Performance Comparison"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .legend(CharmingLegend::new().data(vec!["UFS", "Block I/O"]))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Category)
                .data(vec!["READ", "WRITE"])
                .name("Operation Type")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Avg Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));
    
    chart = chart
        .series(
            Bar::new()
                .name("UFS")
                .data(vec![ufs_read_latency, ufs_write_latency])
                .item_style(ItemStyle::new().color("#5470c6"))
        )
        .series(
            Bar::new()
                .name("Block I/O")
                .data(vec![block_read_latency, block_write_latency])
                .item_style(ItemStyle::new().color("#91cc75"))
        );
    
    // Save as HTML
    let html_output_path = format!("{}_performance_comparison_charming.html", output_prefix);    
    let mut htmlrenderer = HtmlRenderer::new("Performance Comparison", 1000, 800);
    htmlrenderer.save(&chart, &html_output_path).map_err(|e| e.to_string())?;
    println!("Performance comparison HTML chart saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_performance_comparison_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("Performance comparison SVG chart saved to: {}", svg_output_path);
    
    // Save as PNG
    let png_output_path = format!("{}_performance_comparison_charming.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save(&chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("Performance comparison PNG chart saved to: {}", png_output_path);
    
    // Save as JPEG
    let jpg_output_path = format!("{}_performance_comparison_charming.jpg", output_prefix);
    let mut jpg_renderer = ImageRenderer::new(1000, 800);
    jpg_renderer.save(&chart, &jpg_output_path).map_err(|e| e.to_string())?;
    println!("Performance comparison JPEG chart saved to: {}", jpg_output_path);
    
    Ok(())
}

/// Create UFS operation distribution pie chart using Charming
fn create_operation_distribution_chart(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    // Count operations by opcode
    let mut opcode_counts: HashMap<String, usize> = HashMap::new();
    for event in data {
        *opcode_counts.entry(event.opcode.clone()).or_insert(0) += 1;
    }
    
    // Prepare data for the pie chart
    let mut series_data = Vec::new();
    for (opcode, count) in &opcode_counts {
        let opcode_name = match opcode.as_str() {
            "0x28" => "READ_10",
            "0x2a" => "WRITE_10",
            "0x35" => "SYNCHRONIZE_CACHE_10",
            _ => opcode.as_str(),
        };
        
        let item = vec![opcode_name.to_string(), count.to_string()];
        series_data.push(item);
    }
    
    // Create the chart
    let mut chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Operation Distribution"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().orient(Orient::Vertical).left("left"))
        .series(
            CharmingPie::new()
                .name("Operation")
                .radius(vec!["50%", "70%"])
                .data(series_data)
        );
    
    // Save as HTML
    let html_output_path = format!("{}_ufs_operation_distribution_charming.html", output_prefix);
    std::fs::write(&html_output_path, chart.to_string()).map_err(|e| e.to_string())?;
    println!("UFS operation distribution HTML chart saved: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_ufs_operation_distribution_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("UFS operation distribution SVG chart saved to: {}", svg_output_path);
    
    // Save as PNG
    let png_output_path = format!("{}_ufs_operation_distribution_charming.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save(&chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("UFS operation distribution PNG chart saved to: {}", png_output_path);
    
    // Save as JPEG
    let jpg_output_path = format!("{}_ufs_operation_distribution_charming.jpg", output_prefix);
    let mut jpg_renderer = ImageRenderer::new(1000, 800);
    jpg_renderer.save(&chart, &jpg_output_path).map_err(|e| e.to_string())?;
    println!("UFS operation distribution JPEG chart saved to: {}", jpg_output_path);
    
    Ok(())
}

/// Create LBA vs Latency scatter plot using Charming
fn create_lba_latency_scatter(data: &[Block], output_prefix: &str) -> Result<(), String> {
    // Prepare data for the scatter plot
    let mut read_data = Vec::new();
    let mut write_data = Vec::new();
    
    for block in data {
        if block.dtoc > 0.0 {
            if block.io_type == "READ" {
                read_data.push(vec![block.sector as f64, block.dtoc]);
            } else if block.io_type == "WRITE" {
                write_data.push(vec![block.sector as f64, block.dtoc]);
            }
        }
    }
    
    // Create the chart
    let mut chart = Chart::new()
        .title(CharmingTitle::new().text("LBA vs Latency Scatter Plot"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().data(vec!["READ", "WRITE"]))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Sector/LBA")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));
    
    if !read_data.is_empty() {
        chart = chart.series(
            EffectScatter::new()
                .name("READ")
                .data(read_data)
                .symbol_size(8)
                .item_style(ItemStyle::new().color("#5470c6"))
        );
    }
    
    if !write_data.is_empty() {
        chart = chart.series(
            EffectScatter::new()
                .name("WRITE")
                .data(write_data)
                .symbol_size(8)
                .item_style(ItemStyle::new().color("#91cc75"))
        );
    }
    
    // Save as HTML
    let html_output_path = format!("{}_lba_latency_scatter_charming.html", output_prefix);
    let mut htmlrenderer = HtmlRenderer::new("LBA vs Latency Scatter", 1000, 800);
    htmlrenderer.save(&chart, &html_output_path).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter HTML plot saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_lba_latency_scatter_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter SVG plot saved to: {}", svg_output_path);
    
    // Save as PNG
    let png_output_path = format!("{}_lba_latency_scatter_charming.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save(&chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter PNG plot saved to: {}", png_output_path);
    
    // Save as JPEG
    let jpg_output_path = format!("{}_lba_latency_scatter_charming.jpg", output_prefix);
    let mut jpg_renderer = ImageRenderer::new(1000, 800);
    jpg_renderer.save(&chart, &jpg_output_path).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter JPEG plot saved to: {}", jpg_output_path);
    
    Ok(())
}

/// Generate charts and save statistics data.
pub fn generate_charts(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), String> {
    // UFS 차트 생성
    if !processed_ufs.is_empty() {
        match create_ufs_charts(processed_ufs, output_prefix) {
            Ok(_) => {
                println!("UFS charts have been generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFS charts: {}", e);
            }
        }
    }

    // Block I/O 차트 생성
    if !processed_blocks.is_empty() {
        match create_block_charts(processed_blocks, output_prefix) {
            Ok(_) => {
                println!("Block I/O charts have been generated.");
            }
            Err(e) => {
                eprintln!("Error generating Block I/O charts: {}", e);
            }
        }
    }

    // UFSCUSTOM 차트 생성
    if !processed_ufscustom.is_empty() {
        match create_ufscustom_charts(processed_ufscustom, output_prefix) {
            Ok(_) => {
                println!("UFSCUSTOM charts have been generated.");
            }
            Err(e) => {
                eprintln!("Error generating UFSCUSTOM charts: {}", e);
            }
        }
    }

    println!("\nGenerating advanced diagrams...");

    // UFS와 Block I/O 데이터가 있는 경우 추가 다이어그램 생성
    if !processed_ufs.is_empty() || !processed_blocks.is_empty() {
        println!("\nGenerating Charming-based interactive charts...");
        match generate_charming_charts(processed_ufs, processed_blocks, output_prefix) {
            Ok(_) => println!("Charming interactive charts have been generated."),
            Err(e) => eprintln!("Error generating Charming charts: {}", e),
        }
    }

    println!("\nGenerating Plotters-based charts...");
    match generate_plotters_charts(processed_ufs, processed_blocks, processed_ufscustom, output_prefix) {
        Ok(_) => println!("Plotters-based charts have been generated."),
        Err(e) => eprintln!("Error generating Plotters-based charts: {}", e),
    }

    Ok(())
}

/// Creates UFSCUSTOM charts using Charming library
pub fn create_ufscustom_charts(data: &[UFSCUSTOM], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFSCUSTOM data is empty.".to_string());
    }

    // 명령어별로 데이터 그룹화 (UFSCUSTOM은 command 필드 사용)
    let mut command_groups: HashMap<String, Vec<&UFSCUSTOM>> = HashMap::new();
    for event in data {
        command_groups.entry(event.opcode.clone()).or_default().push(event);
    }

    // 색상 맵
    let color_map = [
        "#5470c6", "#91cc75", "#fac858", "#ee6666", 
        "#73c0de", "#3ba272", "#fc8452", "#9a60b4"
    ];

    // 1. LBA over Time chart with command-based legend
    let mut lba_chart = Chart::new()
        .title(CharmingTitle::new().text("UFSCUSTOM LBA over Time by Opcode"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Category)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("LBA")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    // 범례 데이터 준비
    let mut legend_data: Vec<String> = command_groups.keys().cloned().collect();    
    legend_data.sort();
    lba_chart = lba_chart.legend(CharmingLegend::new().orient(Orient::Vertical).bottom("bottom").data(legend_data.clone()));    

    // command별 시리즈 추가
    let mut color_idx = 0;
    for (command, events) in &command_groups {
        let lba_data = events.iter()
            .map(|e| vec![e.start_time, e.lba as f64])
            .collect::<Vec<Vec<f64>>>();

        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !lba_data.is_empty() {
            lba_chart = lba_chart.series(
                CharmingScatter::new()
                    .name(command)
                    .data(lba_data)
                    .symbol_size(2)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let lba_chart_path = format!("{}_ufscustom_lba_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFSCUSTOM LBA over Time by Command", 1000, 800);
    html_renderer.save(&lba_chart, &lba_chart_path).map_err(|e| e.to_string())?;
    println!("UFSCUSTOM LBA chart saved: {}", lba_chart_path);

    // Save as PNG
    let png_output_path = format!("{}_ufscustom_lba_time.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save_format(ImageFormat::Png, &lba_chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("UFSCUSTOM LBA chart PNG chart saved to: {}", png_output_path);

    // 2. Latency (dtoc) over Time chart with command-based legend
    let mut dtoc_chart = Chart::new()
        .title(CharmingTitle::new().text("UFSCUSTOM Latency over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Category)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    dtoc_chart = dtoc_chart.legend(CharmingLegend::new().orient(Orient::Vertical).right("right").data(legend_data.clone()));

    // command별 시리즈 추가
    color_idx = 0;
    for (command, events) in &command_groups {
        let dtoc_data = events.iter()
            .filter(|e| e.dtoc > 0.0)
            .map(|e| vec![e.start_time, e.dtoc])
            .collect::<Vec<Vec<f64>>>();

        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !dtoc_data.is_empty() {
            dtoc_chart = dtoc_chart.series(
                CharmingScatter::new()
                    .name(command)
                    .data(dtoc_data)
                    .symbol_size(2)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let dtoc_chart_path = format!("{}_ufscustom_dtoc_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFSCUSTOM Latency over Time by Command", 1000, 800);
    html_renderer.save(&dtoc_chart, &dtoc_chart_path).map_err(|e| e.to_string())?;
    println!("UFSCUSTOM Latency chart saved: {}", dtoc_chart_path);

    // Save as PNG
    let png_output_path = format!("{}_ufscustom_dtoc_time.png", output_prefix);
    let mut png_renderer = ImageRenderer::new(1000, 800);
    png_renderer.save_format(ImageFormat::Png, &dtoc_chart, &png_output_path).map_err(|e| e.to_string())?;
    println!("UFSCUSTOM latency PNG chart saved to: {}", png_output_path);

    Ok(())
}

/// Generate UFSCUSTOM charts
pub fn generate_ufscustom_charts(
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), String> {
    match create_ufscustom_charts(processed_ufscustom, output_prefix) {
        Ok(_) => {
            println!("UFSCUSTOM charts have been generated.");
        }
        Err(e) => {
            eprintln!("Error generating UFSCUSTOM charts: {}", e);
        }
    }
    
    Ok(())
}

/// Creates UFS charts using Charming library with command-based legends
pub fn create_ufs_charts(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFS data is empty.".to_string());
    }

    // 명령어별로 데이터 그룹화
    let mut opcode_groups: HashMap<String, Vec<&UFS>> = HashMap::new();
    for event in data {
        opcode_groups.entry(event.opcode.clone()).or_default().push(event);
    }

    // opcode 매핑 함수
    let get_opcode_name = |opcode: &str| -> String {
        match opcode {
            "0x28" => "READ_10".to_string(),
            "0x2a" => "WRITE_10".to_string(),
            "0x35" => "SYNCHRONIZE_CACHE_10".to_string(),
            _ => opcode.to_string(),
        }
    };

    // 색상 맵
    let color_map = [
        "#5470c6", "#91cc75", "#fac858", "#ee6666", 
        "#73c0de", "#3ba272", "#fc8452", "#9a60b4"
    ];

    // 1. LBA over Time chart with command-based legend
    let mut lba_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS LBA over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("LBA")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    // 범례 데이터 준비
    let mut legend_data: Vec<String> = opcode_groups.keys()
        .map(|opcode| get_opcode_name(opcode).to_string())
        .collect();
    legend_data.sort();
    lba_chart = lba_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // opcode별 시리즈 추가
    let mut color_idx = 0;
    for (opcode, events) in &opcode_groups {
        let lba_data = events.iter()
            .map(|e| vec![e.time, e.lba as f64])
            .collect::<Vec<Vec<f64>>>();

        let opcode_name = get_opcode_name(opcode);
        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !lba_data.is_empty() {
            lba_chart = lba_chart.series(
                CharmingScatter::new()
                    .name(opcode_name)
                    .data(lba_data)
                    .symbol_size(2)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let lba_chart_path = format!("{}_ufs_lba_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS LBA over Time by Command", 1000, 800);
    html_renderer.save(&lba_chart, &lba_chart_path).map_err(|e| e.to_string())?;
    println!("UFS LBA chart saved: {}", lba_chart_path);

    // 2. Queue Depth over Time chart with command-based legend
    let mut qd_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Queue Depth over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Queue Depth")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    qd_chart = qd_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // opcode별 시리즈 추가
    color_idx = 0;
    for (opcode, events) in &opcode_groups {
        let qd_data = events.iter()
            .map(|e| vec![e.time, e.qd as f64])
            .collect::<Vec<Vec<f64>>>();

        let opcode_name = get_opcode_name(opcode);
        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !qd_data.is_empty() {
            qd_chart = qd_chart.series(
                CharmingScatter::new()
                    .name(opcode_name)
                    .data(qd_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let qd_chart_path = format!("{}_ufs_qd_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS Queue Depth over Time by Command", 1000, 800);
    html_renderer.save(&qd_chart, &qd_chart_path).map_err(|e| e.to_string())?;
    println!("UFS Queue Depth chart saved: {}", qd_chart_path);

    // 3. Dispatch to Complete Latency over Time chart with command-based legend
    let mut dtoc_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Dispatch to Complete Latency over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Dispatch to Complete Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    dtoc_chart = dtoc_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // opcode별 시리즈 추가
    color_idx = 0;
    for (opcode, events) in &opcode_groups {
        let dtoc_data = events.iter()
            .filter(|e| e.dtoc > 0.0)
            .map(|e| vec![e.time, e.dtoc])
            .collect::<Vec<Vec<f64>>>();

        let opcode_name = get_opcode_name(opcode);
        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !dtoc_data.is_empty() {
            dtoc_chart = dtoc_chart.series(
                CharmingScatter::new()
                    .name(opcode_name)
                    .data(dtoc_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let dtoc_chart_path = format!("{}_ufs_dtoc_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS Dispatch to Complete Latency by Command", 1000, 800);
    html_renderer.save(&dtoc_chart, &dtoc_chart_path).map_err(|e| e.to_string())?;
    println!("UFS Dispatch to Complete chart saved: {}", dtoc_chart_path);

    // 4. Complete to Dispatch Latency over Time chart with command-based legend
    let mut ctod_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Complete to Dispatch Latency over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Complete to Dispatch Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    ctod_chart = ctod_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // opcode별 시리즈 추가
    color_idx = 0;
    for (opcode, events) in &opcode_groups {
        let ctod_data = events.iter()
            .filter(|e| e.ctod > 0.0)
            .map(|e| vec![e.time, e.ctod])
            .collect::<Vec<Vec<f64>>>();

        let opcode_name = get_opcode_name(opcode);
        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !ctod_data.is_empty() {
            ctod_chart = ctod_chart.series(
                CharmingScatter::new()
                    .name(opcode_name)
                    .data(ctod_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let ctod_chart_path = format!("{}_ufs_ctod_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS Complete to Dispatch Latency by Command", 1000, 800);
    html_renderer.save(&ctod_chart, &ctod_chart_path).map_err(|e| e.to_string())?;
    println!("UFS Complete to Dispatch chart saved: {}", ctod_chart_path);

    // 5. Complete to Complete Latency over Time chart with command-based legend
    let mut ctoc_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Complete to Complete Latency over Time by Command"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Complete to Complete Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    ctoc_chart = ctoc_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // opcode별 시리즈 추가
    color_idx = 0;
    for (opcode, events) in &opcode_groups {
        let ctoc_data = events.iter()
            .filter(|e| e.ctoc > 0.0)
            .map(|e| vec![e.time, e.ctoc])
            .collect::<Vec<Vec<f64>>>();

        let opcode_name = get_opcode_name(opcode);
        let color = color_map[color_idx % color_map.len()];
        color_idx += 1;

        if !ctoc_data.is_empty() {
            ctoc_chart = ctoc_chart.series(
                CharmingScatter::new()
                    .name(opcode_name)
                    .data(ctoc_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let ctoc_chart_path = format!("{}_ufs_ctoc_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS Complete to Complete Latency by Command", 1000, 800);
    html_renderer.save(&ctoc_chart, &ctoc_chart_path).map_err(|e| e.to_string())?;
    println!("UFS Complete to Complete chart saved: {}", ctoc_chart_path);

    // 6. Continuity pie chart
    let continuous_count = data.iter().filter(|d| d.continuous).count();
    let non_continuous_count = data.len() - continuous_count;

    let mut pie_chart = Chart::new()
        .title(CharmingTitle::new().text("UFS Continuity Distribution"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().orient(Orient::Vertical).left("left"));

    let series_data = vec![
        vec!["Continuous".to_string(), continuous_count.to_string()],
        vec!["Non-continuous".to_string(), non_continuous_count.to_string()],
    ];

    pie_chart = pie_chart.series(
        CharmingPie::new()
            .name("Continuity Distribution")
            .radius(vec!["50%", "70%"])
            .data(series_data)
    );

    let continuous_chart_path = format!("{}_ufs_continuous.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("UFS Continuity Distribution", 800, 600);
    html_renderer.save(&pie_chart, &continuous_chart_path).map_err(|e| e.to_string())?;
    println!("UFS Continuity pie chart saved: {}", continuous_chart_path);

    Ok(())
}

/// Creates Block I/O charts using Charming library with I/O type-based legends
pub fn create_block_charts(data: &[Block], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O data is empty.".to_string());
    }

    // I/O 유형별로 데이터 그룹화
    let mut io_type_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for event in data {
        io_type_groups.entry(event.io_type.clone()).or_default().push(event);
    }

    // 색상 맵
    let color_map = [
        "#5470c6", "#91cc75", "#fac858", "#ee6666", 
        "#73c0de", "#3ba272", "#fc8452", "#9a60b4"
    ];

    // 1. Sector/LBA over Time chart with I/O type-based legend
    let mut sector_chart = Chart::new()
        .title(CharmingTitle::new().text("Block Sector/LBA over Time by I/O Type"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Sector/LBA")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    // 범례 데이터 준비
    let mut legend_data: Vec<String> = io_type_groups.keys().cloned().collect();
    legend_data.sort();
    sector_chart = sector_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // I/O 타입별 시리즈 추가
    let mut color_idx = 0;
    for (io_type, events) in &io_type_groups {
        let sector_data = events.iter()
            .map(|e| vec![e.time, e.sector as f64])
            .collect::<Vec<Vec<f64>>>();

        let color = match io_type.as_str() {
            "READ" => "#5470c6",
            "WRITE" => "#91cc75",
            _ => color_map[color_idx % color_map.len()],
        };
        color_idx += 1;

        if !sector_data.is_empty() {
            sector_chart = sector_chart.series(
                CharmingScatter::new()
                    .name(io_type)
                    .data(sector_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let sector_chart_path = format!("{}_block_sector_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block Sector/LBA over Time by I/O Type", 1000, 800);
    html_renderer.save(&sector_chart, &sector_chart_path).map_err(|e| e.to_string())?;
    println!("Block Sector/LBA chart saved: {}", sector_chart_path);

    // 2. Queue Depth over Time chart with I/O type-based legend
    let mut qd_chart = Chart::new()
        .title(CharmingTitle::new().text("Block Queue Depth over Time by I/O Type"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Queue Depth")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    qd_chart = qd_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // I/O 타입별 시리즈 추가
    color_idx = 0;
    for (io_type, events) in &io_type_groups {
        let qd_data = events.iter()
            .map(|e| vec![e.time, e.qd as f64])
            .collect::<Vec<Vec<f64>>>();

        let color = match io_type.as_str() {
            "READ" => "#5470c6",
            "WRITE" => "#91cc75",
            _ => color_map[color_idx % color_map.len()],
        };
        color_idx += 1;

        if !qd_data.is_empty() {
            qd_chart = qd_chart.series(
                CharmingScatter::new()
                    .name(io_type)
                    .data(qd_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let qd_chart_path = format!("{}_block_qd_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block Queue Depth over Time by I/O Type", 1000, 800);
    html_renderer.save(&qd_chart, &qd_chart_path).map_err(|e| e.to_string())?;
    println!("Block Queue Depth chart saved: {}", qd_chart_path);

    // 3. Dispatch to Complete Latency over Time chart with I/O type-based legend
    let mut dtoc_chart = Chart::new()
        .title(CharmingTitle::new().text("Block Dispatch to Complete Latency over Time by I/O Type"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Dispatch to Complete Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    dtoc_chart = dtoc_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // I/O 타입별 시리즈 추가
    color_idx = 0;
    for (io_type, events) in &io_type_groups {
        let dtoc_data = events.iter()
            .filter(|e| e.dtoc > 0.0)
            .map(|e| vec![e.time, e.dtoc])
            .collect::<Vec<Vec<f64>>>();

        let color = match io_type.as_str() {
            "READ" => "#5470c6",
            "WRITE" => "#91cc75",
            _ => color_map[color_idx % color_map.len()],
        };
        color_idx += 1;

        if !dtoc_data.is_empty() {
            dtoc_chart = dtoc_chart.series(
                CharmingScatter::new()
                    .name(io_type)
                    .data(dtoc_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let dtoc_chart_path = format!("{}_block_dtoc_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block Dispatch to Complete Latency by I/O Type", 1000, 800);
    html_renderer.save(&dtoc_chart, &dtoc_chart_path).map_err(|e| e.to_string())?;
    println!("Block Dispatch to Complete chart saved: {}", dtoc_chart_path);

    // 4. Complete to Dispatch Latency over Time chart with I/O type-based legend
    let mut ctod_chart = Chart::new()
        .title(CharmingTitle::new().text("Block Complete to Dispatch Latency over Time by I/O Type"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Complete to Dispatch Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    ctod_chart = ctod_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // I/O 타입별 시리즈 추가
    color_idx = 0;
    for (io_type, events) in &io_type_groups {
        let ctod_data = events.iter()
            .filter(|e| e.ctod > 0.0)
            .map(|e| vec![e.time, e.ctod])
            .collect::<Vec<Vec<f64>>>();

        let color = match io_type.as_str() {
            "READ" => "#5470c6",
            "WRITE" => "#91cc75",
            _ => color_map[color_idx % color_map.len()],
        };
        color_idx += 1;

        if !ctod_data.is_empty() {
            ctod_chart = ctod_chart.series(
                CharmingScatter::new()
                    .name(io_type)
                    .data(ctod_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let ctod_chart_path = format!("{}_block_ctod_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block Complete to Dispatch Latency by I/O Type", 1000, 800);
    html_renderer.save(&ctod_chart, &ctod_chart_path).map_err(|e| e.to_string())?;
    println!("Block Complete to Dispatch chart saved: {}", ctod_chart_path);

    // 5. Complete to Complete Latency over Time chart with I/O type-based legend
    let mut ctoc_chart = Chart::new()
        .title(CharmingTitle::new().text("Block Complete to Complete Latency over Time by I/O Type"))
        .tooltip(Tooltip::new().trigger(Trigger::Axis))
        .x_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Time (s)")
                .name_location(NameLocation::Middle)
                .name_gap(30)
        )
        .y_axis(
            CharmingAxis::new()
                .type_(AxisType::Value)
                .name("Complete to Complete Latency (ms)")
                .name_location(NameLocation::Middle)
                .name_gap(45)
        )
        .grid(Grid::new().right("5%").bottom("10%").left("5%").top("15%"));

    ctoc_chart = ctoc_chart.legend(CharmingLegend::new().data(legend_data.clone()));

    // I/O 타입별 시리즈 추가
    color_idx = 0;
    for (io_type, events) in &io_type_groups {
        let ctoc_data = events.iter()
            .filter(|e| e.ctoc > 0.0)
            .map(|e| vec![e.time, e.ctoc])
            .collect::<Vec<Vec<f64>>>();

        let color = match io_type.as_str() {
            "READ" => "#5470c6",
            "WRITE" => "#91cc75",
            _ => color_map[color_idx % color_map.len()],
        };
        color_idx += 1;

        if !ctoc_data.is_empty() {
            ctoc_chart = ctoc_chart.series(
                CharmingScatter::new()
                    .name(io_type)
                    .data(ctoc_data)
                    .symbol_size(8)
                    .item_style(ItemStyle::new().color(color))
            );
        }
    }

    let ctoc_chart_path = format!("{}_block_ctoc_time.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block Complete to Complete Latency by I/O Type", 1000, 800);
    html_renderer.save(&ctoc_chart, &ctoc_chart_path).map_err(|e| e.to_string())?;
    println!("Block Complete to Complete chart saved: {}", ctoc_chart_path);

    // 6. Continuity pie chart
    let continuous_count = data.iter().filter(|d| d.continuous).count();
    let non_continuous_count = data.len() - continuous_count;

    let mut pie_chart = Chart::new()
        .title(CharmingTitle::new().text("Block I/O Continuity Distribution"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().orient(Orient::Vertical).left("left"));

    let series_data = vec![
        vec!["Continuous".to_string(), continuous_count.to_string()],
        vec!["Non-continuous".to_string(), non_continuous_count.to_string()],
    ];

    pie_chart = pie_chart.series(
        CharmingPie::new()
            .name("Continuity Distribution")
            .radius(vec!["50%", "70%"])
            .data(series_data)
    );

    let continuous_chart_path = format!("{}_block_continuous.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block I/O Continuity Distribution", 800, 600);
    html_renderer.save(&pie_chart, &continuous_chart_path).map_err(|e| e.to_string())?;
    println!("Block Continuity pie chart saved: {}", continuous_chart_path);

    // 7. I/O Type Distribution chart
    let mut io_type_counts: HashMap<String, usize> = HashMap::new();
    for event in data {
        *io_type_counts.entry(event.io_type.clone()).or_insert(0) += 1;
    }

    let mut io_type_chart = Chart::new()
        .title(CharmingTitle::new().text("Block I/O Type Distribution"))
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(CharmingLegend::new().orient(Orient::Vertical).left("left"));

    let mut io_series_data = Vec::new();
    for (io_type, count) in &io_type_counts {
        io_series_data.push(vec![io_type.clone(), count.to_string()]);
    }

    io_type_chart = io_type_chart.series(
        CharmingPie::new()
            .name("I/O Type")
            .radius(vec!["50%", "70%"])
            .data(io_series_data)
    );

    let io_type_chart_path = format!("{}_block_io_type.html", output_prefix);
    let mut html_renderer = HtmlRenderer::new("Block I/O Type Distribution", 800, 600);
    html_renderer.save(&io_type_chart, &io_type_chart_path).map_err(|e| e.to_string())?;
    println!("Block I/O Type pie chart saved: {}", io_type_chart_path);

    Ok(())
}

/// Generate charts using plotters library and save as PNG
pub fn generate_plotters_charts(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    processed_ufscustom: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), String> {
    // UFS 차트 생성
    if !processed_ufs.is_empty() {
        match create_ufs_latency_trend_plotters(processed_ufs, output_prefix) {
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
        match create_block_io_plotters(processed_blocks, output_prefix) {
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
        match create_ufscustom_plotters(processed_ufscustom, output_prefix) {
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
fn create_ufs_latency_trend_plotters(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    // Sort data by time
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| {
        a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal)
    });
    
    // Group data by opcode
    let opcodes: Vec<String> = time_sorted_data
        .iter()
        .map(|d| d.opcode.clone())
        .collect::<std::collections::HashSet<String>>()
        .into_iter()
        .collect();
    
    // Aggregate data for smoother line
    let window_size = 20;
    let mut chart_data: HashMap<String, Vec<(f64, f64)>> = HashMap::new();
    
    for opcode in &opcodes {
        let mut window_times = Vec::new();
        let mut window_latencies = Vec::new();
        
        for item in &time_sorted_data {
            if &item.opcode == opcode && item.dtoc > 0.0 {
                window_times.push(item.time);
                window_latencies.push(item.dtoc);
                
                if window_times.len() >= window_size {
                    let avg_time = window_times.iter().sum::<f64>() / window_times.len() as f64;
                    let avg_latency = window_latencies.iter().sum::<f64>() / window_latencies.len() as f64;
                    
                    chart_data
                        .entry(opcode.clone())
                        .or_insert_with(Vec::new)
                        .push((avg_time, avg_latency));
                    
                    window_times.clear();
                    window_latencies.clear();
                }
            }
        }
        
        // Process any remaining data points
        if !window_times.is_empty() {
            let avg_time = window_times.iter().sum::<f64>() / window_times.len() as f64;
            let avg_latency = window_latencies.iter().sum::<f64>() / window_latencies.len() as f64;
            
            chart_data
                .entry(opcode.clone())
                .or_insert_with(Vec::new)
                .push((avg_time, avg_latency));
        }
    }
    
    if chart_data.is_empty() {
        return Err("No valid data for UFS latency trend chart".to_string());
    }
    
    // Define colors for different opcodes
    let colors = [
        RGBColor(84, 112, 198),   // blue - #5470c6
        RGBColor(145, 204, 117),  // green - #91cc75
        RGBColor(250, 200, 88),   // yellow - #fac858
        RGBColor(238, 102, 102),  // red - #ee6666
        RGBColor(115, 192, 222),  // light blue - #73c0de
        RGBColor(59, 162, 114),   // dark green - #3ba272
        RGBColor(252, 132, 82),   // orange - #fc8452
        RGBColor(154, 96, 180),   // purple - #9a60b4
    ];
    
    // Find min and max values for axes
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
    
    // Add some padding to the axes
    let x_range = max_time - min_time;
    let y_range = max_latency - min_latency;
    let time_padding = x_range * 0.05;
    let latency_padding = y_range * 0.05;
    
    min_time -= time_padding;
    max_time += time_padding;
    min_latency = min_latency.max(0.0) - latency_padding.max(0.0);  // Don't go below 0
    max_latency += latency_padding;
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufs_latency_trend_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("UFS Latency Trend by Operation Code", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_time..max_time, min_latency..max_latency)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Latency (ms)")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 15))
        .draw()
        .map_err(|e| e.to_string())?;
    
    // Add each opcode as a series
    let mut color_idx = 0;
    for (opcode, points) in &chart_data {
        // Map opcode to a readable name
        let opcode_name = match opcode.as_str() {
            "0x28" => "READ_10",
            "0x2a" => "WRITE_10",
            "0x35" => "SYNCHRONIZE_CACHE_10",
            _ => opcode.as_str(),
        };
        
        let color = colors[color_idx % colors.len()];
        color_idx += 1;
        
        if points.len() > 1 {
            chart.draw_series(LineSeries::new(
                points.iter().map(|&(x, y)| (x, y)),
                color.stroke_width(2),
            ))
            .map_err(|e| e.to_string())?
            .label(opcode_name)
            .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(2)));
        } else if points.len() == 1 {
            // For single points, use a scatter plot
            chart.draw_series(
                points.iter().map(|&(x, y)| Circle::new((x, y), 3, color.filled())),
            )
            .map_err(|e| e.to_string())?
            .label(opcode_name)
            .legend(move |(x, y)| Circle::new((x + 10, y), 3, color.filled()));
        }
    }
    
    // Draw the legend
    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .legend_area_size(22)
        .draw()
        .map_err(|e| e.to_string())?;
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFS latency trend PNG chart saved to: {}", png_path);
    
    Ok(())
}

/// Create Block I/O charts using Plotters library
fn create_block_io_plotters(data: &[Block], output_prefix: &str) -> Result<(), String> {
    // 가장 기본적인 I/O 타입별 레이턴시 비교 차트 생성
    let mut io_types: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in data {
        io_types.entry(block.io_type.clone()).or_default().push(block);
    }
    
    let io_type_labels: Vec<String> = io_types.keys().cloned().collect();
    let mut avg_latency_data: HashMap<String, f64> = HashMap::new();
    
    for io_type in &io_type_labels {
        let blocks = io_types.get(io_type).unwrap();
        let avg_latency = blocks.iter()
            .filter(|b| b.dtoc > 0.0)
            .map(|b| b.dtoc)
            .sum::<f64>() / blocks.iter().filter(|b| b.dtoc > 0.0).count().max(1) as f64;
        avg_latency_data.insert(io_type.clone(), avg_latency);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_block_io_analysis_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // 막대 차트 범위 계산 - 타입을 명시적으로 지정
    let max_latency = avg_latency_data.values().fold(0.0f64, |acc: f64, &v| if acc > v { acc } else { v }) * 1.1;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("Block I/O Operation Latency Analysis", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(
            0i32..(io_type_labels.len() as i32),  // i32로 타입 변환
            0.0..max_latency
        )
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .y_desc("Avg Latency (ms)")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 15))
        .x_labels(io_type_labels.len())
        .x_label_formatter(&|idx| {
            let idx_usize = *idx as usize;  // i32를 usize로 변환
            if idx_usize < io_type_labels.len() {
                io_type_labels[idx_usize].clone()
            } else {
                String::new()
            }
        })
        .draw()
        .map_err(|e| e.to_string())?;
    
    // 막대 차트 그리기
    let bars: Vec<(i32, f64, RGBColor)> = io_type_labels.iter().enumerate()  // i32로 변환
        .map(|(idx, io_type)| {
            let latency = *avg_latency_data.get(io_type).unwrap_or(&0.0);
            let color = match io_type.as_str() {
                "READ" => RGBColor(84, 112, 198),   // blue
                "WRITE" => RGBColor(145, 204, 117), // green
                _ => RGBColor(250, 200, 88),        // yellow
            };
            
            (idx as i32, latency, color)  // usize를 i32로 변환
        })
        .collect();
    
    chart.draw_series(
        bars.iter().map(|(idx, latency, color)| {
            let bar_width = 0.7; // 막대 너비 (0.0 - 1.0)
            let start_x = *idx as f32 - bar_width / 2.0;
            let end_x = *idx as f32 + bar_width / 2.0;
            
            Rectangle::new(
                [(start_x as i32, 0.0), (end_x as i32, *latency)],
                color.filled(),
            )
        })
    )
    .map_err(|e| e.to_string())?;
    
    // 범례 그리기
    let mut legends = Vec::new();
    for (idx, io_type) in io_type_labels.iter().enumerate() {
        let color = match io_type.as_str() {
            "READ" => RGBColor(84, 112, 198),
            "WRITE" => RGBColor(145, 204, 117),
            _ => RGBColor(250, 200, 88),
        };
        
        legends.push((io_type.clone(), color));
    }
    
    // Legend area 생성
    let legend_area = root.titled("Legend", ("sans-serif", 15))
        .map_err(|e| e.to_string())?;
    
    for (idx, (label, color)) in legends.iter().enumerate() {
        let x: i32 = 100;
        let y: i32 = idx as i32 * 25 + 30;
        
        legend_area.draw(&Rectangle::new(
            [(x - 10, y - 10), (x + 10, y + 10)],
            color.filled(),
        ))
        .map_err(|e| e.to_string())?;
        
        legend_area.draw(&Text::new(
            label.clone(),
            (x + 20, y),
            ("sans-serif", 15),
        ))
        .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("Block I/O analysis PNG chart saved to: {}", png_path);
    
    Ok(())
}

/// Create UFSCUSTOM charts using Plotters library
fn create_ufscustom_plotters(data: &[UFSCUSTOM], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFSCUSTOM data is empty.".to_string());
    }
    
    // 명령어별로 데이터 그룹화
    let mut command_groups: HashMap<String, Vec<&UFSCUSTOM>> = HashMap::new();
    for event in data {
        command_groups.entry(event.opcode.clone()).or_default().push(event);
    }
    
    // LBA vs Time 스캐터 플롯 생성
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufscustom_lba_time_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // Find min and max values for axes
    let min_time = data.iter().map(|e| e.start_time).fold(f64::MAX, |a, b| a.min(b));
    let max_time = data.iter().map(|e| e.start_time).fold(f64::MIN, |a, b| a.max(b));
    
    let min_lba = data.iter().map(|e| e.lba as f64).fold(f64::MAX, |a, b| a.min(b));
    let max_lba = data.iter().map(|e| e.lba as f64).fold(f64::MIN, |a, b| a.max(b));
    
    // Add padding
    let x_range = max_time - min_time;
    let y_range = max_lba - min_lba;
    let time_padding = x_range * 0.05;
    let lba_padding = y_range * 0.05;
    
    let min_time = min_time - time_padding;
    let max_time = max_time + time_padding;
    let min_lba = min_lba - lba_padding;
    let max_lba = max_lba + lba_padding;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("UFSCUSTOM LBA over Time by Command", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_time..max_time, min_lba..max_lba)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc("Time (s)")
        .y_desc("LBA")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 15))
        .draw()
        .map_err(|e| e.to_string())?;
    
    // Define colors
    let colors = [
        RGBColor(84, 112, 198),   // blue
        RGBColor(145, 204, 117),  // green
        RGBColor(250, 200, 88),   // yellow
        RGBColor(238, 102, 102),  // red
        RGBColor(115, 192, 222),  // light blue
        RGBColor(59, 162, 114),   // dark green
        RGBColor(252, 132, 82),   // orange
        RGBColor(154, 96, 180),   // purple
    ];
    
    // Add each command as a series
    let mut color_idx = 0;
    for (command, events) in &command_groups {
        let color = colors[color_idx % colors.len()];
        color_idx += 1;
        
        chart.draw_series(events.iter().map(|event| {
            Circle::new((event.start_time, event.lba as f64), 2, color.filled())
        }))
        .map_err(|e| e.to_string())?
        .label(command)
        .legend(move |(x, y)| Circle::new((x + 10, y), 3, color.filled()));
    }
    
    // Draw the legend
    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .legend_area_size(22)
        .draw()
        .map_err(|e| e.to_string())?;
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFSCUSTOM LBA over Time PNG chart saved to: {}", png_path);
    
    // DTOC vs Time 스캐터 플롯 생성
    let png_path = format!("{}_ufscustom_dtoc_time_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // Find min and max values for latency axis
    let min_dtoc = data.iter()
        .filter(|e| e.dtoc > 0.0)
        .map(|e| e.dtoc)
        .fold(f64::MAX, |a, b| a.min(b));
    
    let max_dtoc = data.iter()
        .map(|e| e.dtoc)
        .fold(f64::MIN, |a, b| a.max(b));
    
    // Add padding for latency axis
    let y_range = max_dtoc - min_dtoc;
    let dtoc_padding = y_range * 0.05;
    
    let min_dtoc = min_dtoc - dtoc_padding.max(0.0);
    let max_dtoc = max_dtoc + dtoc_padding;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&root)
        .caption("UFSCUSTOM Latency over Time by Command", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_time..max_time, min_dtoc..max_dtoc)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc("Time (s)")
        .y_desc("Latency (ms)")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 15))
        .draw()
        .map_err(|e| e.to_string())?;
    
    // Add each command as a series
    let mut color_idx = 0;
    for (command, events) in &command_groups {
        let color = colors[color_idx % colors.len()];
        color_idx += 1;
        
        let filtered_events: Vec<&UFSCUSTOM> = events.iter()
            .filter(|e| e.dtoc > 0.0)
            .cloned()
            .collect();
        
        if !filtered_events.is_empty() {
            chart.draw_series(filtered_events.iter().map(|event| {
                Circle::new((event.start_time, event.dtoc), 2, color.filled())
            }))
            .map_err(|e| e.to_string())?
            .label(command)
            .legend(move |(x, y)| Circle::new((x + 10, y), 3, color.filled()));
        }
    }
    
    // Draw the legend
    chart.configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .legend_area_size(22)
        .draw()
        .map_err(|e| e.to_string())?;
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFSCUSTOM Latency over Time PNG chart saved to: {}", png_path);
    
    Ok(())
}
