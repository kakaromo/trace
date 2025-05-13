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
        match create_block_io_plotters(processed_blocks, output_prefix) {
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
    let mut color_idx = 0;
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
    let mut color_idx = 0;
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
    let mut color_idx = 0;
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
    let mut color_idx = 0;
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

/// Creates Block I/O charts using Plotters library
fn create_block_io_plotters(data: &[Block], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O data is empty.".to_string());
    }
    
    // I/O 타입별로 데이터 그룹화
    let mut io_type_groups: HashMap<String, Vec<&Block>> = HashMap::new();
    for block in data {
        io_type_groups.entry(block.io_type.clone()).or_default().push(block);
    }
    
    // PNG 파일 경로 생성
    let png_path = format!("{}_block_io_analysis_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
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
        .fold(0.0, |a, b| if a > b { a } else { b });
    
    // Add padding
    let x_range = max_time - min_time;
    let y_range = max_latency - min_latency;
    let time_padding = x_range * 0.05;
    let latency_padding = y_range * 0.05;
    
    let min_time = min_time - time_padding;
    let max_time = max_time + time_padding;
    let min_latency = min_latency.max(0.0) - latency_padding.max(0.0);  // Don't go below 0
    let max_latency = max_latency + latency_padding;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption("Block I/O Latency over Time by I/O Type", ("sans-serif", 30).into_font())
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
    
    // I/O 타입에 따른 색상 매핑
    let get_color_for_io_type = |io_type: &str| -> RGBColor {
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
    };
    
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
            
            // 스캐터 플롯 그리기 - 포인트 크기를 2로 작게 설정
            chart.draw_series(
                filtered_events.iter().map(|event| 
                    Circle::new((event.time, event.dtoc), 2, color.filled())
                )
            )
            .map_err(|e| e.to_string())?;
        }
    }
    
    // 레전드 영역을 오른쪽에 그리기
    legend_area.fill(&WHITE.mix(0.95)).map_err(|e| e.to_string())?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        // 정수 좌표를 명시적으로 i32로 변환
        let y_i32 = (50 + i * 30) as i32;
        
        // 원 대신 직선으로 표시
        legend_area
            .draw(&PathElement::new(
                vec![(20_i32, y_i32), (50_i32, y_i32)],
                color.stroke_width(2)
            ))
            .map_err(|e| e.to_string())?;
            
        legend_area
            .draw(&Text::new(
                name.clone(),
                (60_i32, y_i32),
                ("sans-serif", 15)
            ))
            .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("Block I/O analysis PNG chart saved to: {}", png_path);
    
    // LBA vs Latency 스캐터 플롯 추가
    let png_path = format!("{}_block_lba_latency_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1000, 800))
        .into_drawing_area();
    root.fill(&WHITE).map_err(|e| e.to_string())?;
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Find min and max values for LBA axis
    let min_sector = data.iter().map(|e| e.sector as f64).fold(f64::MAX, |a, b| a.min(b));
    let max_sector = data.iter().map(|e| e.sector as f64).fold(f64::MIN, |a, b| a.max(b));
    
    // Add padding for LBA axis
    let sector_range = max_sector - min_sector;
    let sector_padding = sector_range * 0.05;
    
    let min_sector = min_sector - sector_padding;
    let max_sector = max_sector + sector_padding;
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption("Block I/O Sector/LBA vs Latency by I/O Type", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(50)
        .y_label_area_size(60)
        .build_cartesian_2d(min_sector..max_sector, min_latency..max_latency)
        .map_err(|e| e.to_string())?;
    
    // Configure the chart
    chart.configure_mesh()
        .x_desc("Sector/LBA")
        .y_desc("Latency (ms)")
        .axis_desc_style(("sans-serif", 20))
        .label_style(("sans-serif", 15))
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
                    Circle::new((event.sector as f64, event.dtoc), 2, color.filled())
                )
            )
            .map_err(|e| e.to_string())?;
        }
    }
    
    // 레전드 영역을 오른쪽에 그리기
    legend_area.fill(&WHITE.mix(0.95)).map_err(|e| e.to_string())?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        // 정수 좌표를 명시적으로 i32로 변환
        let y_i32 = (50 + i * 30) as i32;
        
        // 원 대신 직선으로 표시
        legend_area
            .draw(&PathElement::new(
                vec![(20_i32, y_i32), (50_i32, y_i32)],
                color.stroke_width(2)
            ))
            .map_err(|e| e.to_string())?;
            
        legend_area
            .draw(&Text::new(
                name.clone(),
                (60_i32, y_i32),
                ("sans-serif", 15)
            ))
            .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("Block I/O LBA vs Latency PNG chart saved to: {}", png_path);
    
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
    
    // 윈도우 기반 집계 대신 모든 포인트를 표시
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
    
    // 색상 매핑 함수 - 명령어 타입에 따라 색상 지정
    let get_color_for_opcode = |opcode: &str| -> RGBColor {
        match opcode {
            "0x28" => RGBColor(65, 105, 225),   // READ - 파란색 계열
            "0x2a" => RGBColor(220, 20, 60),    // WRITE - 빨간색 계열
            "0x35" => RGBColor(255, 215, 0),    // SYNC - 노란색 계열
            "0x2c" | "0x42" => RGBColor(138, 43, 226), // UNMAP/DISCARD - 보라색 계열
            _ => RGBColor(50, 50, 50),          // 기타 - 검은색 계열
        }
    };
    
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
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
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
    
    // 명령어별 스캐터 플롯 추가
    let mut legends = Vec::new();
    
    for (opcode, points) in &chart_data {
        // Map opcode to a readable name
        let opcode_name = match opcode.as_str() {
            "0x28" => "READ_10",
            "0x2a" => "WRITE_10",
            "0x35" => "SYNCHRONIZE_CACHE_10",
            "0x2c" => "UNMAP",
            "0x42" => "DISCARD",
            _ => opcode.as_str(),
        };
        
        let color = get_color_for_opcode(opcode);
        legends.push((opcode_name.to_string(), color));
        
        // 스캐터 플롯 그리기 - 포인트 크기를 2로 작게 설정
        chart.draw_series(
            points.iter().map(|&(x, y)| Circle::new((x, y), 2, color.filled())),
        )
        .map_err(|e| e.to_string())?;
    }
    
    // Draw the legend in the separate area
    legend_area.fill(&WHITE.mix(0.95)).map_err(|e| e.to_string())?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        // 정수 좌표를 명시적으로 i32로 변환
        let y_i32 = (50 + i * 30) as i32;
        
        // 원형 대신 직선으로 표시
        legend_area
            .draw(&PathElement::new(
                vec![(20_i32, y_i32), (50_i32, y_i32)],
                color.stroke_width(2)
            ))
            .map_err(|e| e.to_string())?;
            
        legend_area
            .draw(&Text::new(
                name.clone(),
                (60_i32, y_i32),
                ("sans-serif", 15)
            ))
            .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFS latency trend PNG chart saved to: {}", png_path);
    
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
    
    // 명령어 타입에 따른 색상 매핑 함수 정의
    let get_color_for_opcode = |opcode: &str| -> RGBColor {
        match opcode {
            opcode if opcode.contains("0x28") => RGBColor(65, 105, 225),    // READ 명령: 파란색 계열
            opcode if opcode.contains("0x2a") => RGBColor(220, 20, 60),    // WRITE 명령: 빨간색 계열
            opcode if opcode.contains("0x35") => RGBColor(255, 215, 0),   // SYNC 명령: 노란색
            opcode if opcode.contains("0x42") => RGBColor(138, 43, 226),  // UNMAP/DISCARD 명령: 보라색
            _ => RGBColor(50, 50, 50),   // 기타: 검은색 계열
        }
    };
    
    // LBA vs Time 스캐터 플롯 생성
    // PNG 파일 경로 생성
    let png_path = format!("{}_ufscustom_lba_time_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1400, 800))
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
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
        .caption("UFSCUSTOM LBA over Time by Opcode", ("sans-serif", 30).into_font())
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
    
    // Add each command as a series
    let mut legends = Vec::new();
    
    for (command, events) in &command_groups {
        // 명령어 타입에 따라 색상 지정
        let color = get_color_for_opcode(command);
        
        legends.push((command.clone(), color));
        
        // 산점도 포인트 크기를 2로 작게 설정
        chart.draw_series(events.iter().map(|event| {
            Circle::new((event.start_time, event.lba as f64), 2, color.filled())
        }))
        .map_err(|e| e.to_string())?;
    }
    
    // 레전드 영역을 오른쪽에 그리기
    legend_area.fill(&WHITE.mix(0.95)).map_err(|e| e.to_string())?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        // 정수 좌표를 명시적으로 i32로 변환
        let y_i32 = (50 + i * 30) as i32;
        
        // 원 대신 직선으로 표시
        legend_area
            .draw(&PathElement::new(
                vec![(20_i32, y_i32), (50_i32, y_i32)],
                color.stroke_width(2)
            ))
            .map_err(|e| e.to_string())?;
            
        legend_area
            .draw(&Text::new(
                name.clone(),
                (60_i32, y_i32),
                ("sans-serif", 15)
            ))
            .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFSCUSTOM LBA over Time PNG chart saved to: {}", png_path);
    
    // DTOC vs Time 스캐터 플롯 생성
    let png_path = format!("{}_ufscustom_dtoc_time_plotters.png", output_prefix);
    
    // Create the drawing area
    let root = BitMapBackend::new(&png_path, (1400, 800))
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
    
    // 차트 영역과 레전드 영역을 분리
    let (chart_area, legend_area) = root.split_horizontally(800);
    
    // Create the chart
    let mut chart = ChartBuilder::on(&chart_area)
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
    let mut legends = Vec::new();
    
    for (command, events) in &command_groups {
        // 명령어 타입에 따라 색상 지정
        let color = get_color_for_opcode(command);
        
        let filtered_events: Vec<&UFSCUSTOM> = events.iter()
            .filter(|e| e.dtoc > 0.0)
            .cloned()
            .collect();
        
        if !filtered_events.is_empty() {
            legends.push((command.clone(), color));
            
            // 산점도 포인트 크기를 2로 작게 설정
            chart.draw_series(filtered_events.iter().map(|event| {
                Circle::new((event.start_time, event.dtoc), 2, color.filled())
            }))
            .map_err(|e| e.to_string())?;
        }
    }
    
    // 레전드 영역을 오른쪽에 그리기
    legend_area.fill(&WHITE.mix(0.95)).map_err(|e| e.to_string())?;
    
    for (i, (name, color)) in legends.iter().enumerate() {
        // 정수 좌표를 명시적으로 i32로 변환
        let y_i32 = (50 + i * 30) as i32;
        
        // 원 대신 직선으로 표시
        legend_area
            .draw(&PathElement::new(
                vec![(20_i32, y_i32), (50_i32, y_i32)],
                color.stroke_width(2)
            ))
            .map_err(|e| e.to_string())?;
            
        legend_area
            .draw(&Text::new(
                name.clone(),
                (60_i32, y_i32),
                ("sans-serif", 15)
            ))
            .map_err(|e| e.to_string())?;
    }
    
    root.present().map_err(|e| e.to_string())?;
    println!("UFSCUSTOM Latency over Time PNG chart saved to: {}", png_path);
    
    Ok(())
}
