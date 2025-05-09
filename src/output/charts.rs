use crate::models::{Block, UFS};
use plotly::color::NamedColor;
use plotly::common::{Marker, Mode, Title};
use plotly::layout::{Axis, BarMode, Legend};
use plotly::{Layout, Pie, Plot, Scatter};
use std::collections::HashMap;
use std::path::Path;
use charming::Chart;
use charming::component::{Title as CharmingTitle, Legend as CharmingLegend, Grid, Axis as CharmingAxis};
use charming::element::{AxisType, ItemStyle, NameLocation, Orient, Tooltip, Trigger};
use charming::series::{Line, Bar, Pie as CharmingPie, EffectScatter};
use charming::renderer::{HtmlRenderer, ImageRenderer};

/// Creates Plotly charts from UFS data.
pub fn create_ufs_charts(data: &[UFS], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("UFS data is empty.".to_string());
    }

    // Sort data by time for time-based charts
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 1. LBA over Time chart
    let mut lba_plot = Plot::new();
    let lba_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.lba as f64)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("LBA over Time");

    lba_plot.add_trace(lba_scatter);
    lba_plot.set_layout(
        Layout::new()
            .title(Title::from("LBA over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("LBA"))),
    );

    let lba_chart_path = format!("{}_ufs_lba_time.html", output_prefix);
    lba_plot.write_html(Path::new(&lba_chart_path));
    println!("UFS LBA chart saved: {}", lba_chart_path);

    // 2. Queue Depth over Time chart
    let mut qd_plot = Plot::new();
    let qd_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.qd as f64)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Queue Depth over Time");

    qd_plot.add_trace(qd_scatter);
    qd_plot.set_layout(
        Layout::new()
            .title(Title::from("Queue Depth over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Queue Depth"))),
    );

    let qd_chart_path = format!("{}_ufs_qd_time.html", output_prefix);
    qd_plot.write_html(Path::new(&qd_chart_path));
    println!("UFS Queue Depth chart saved: {}", qd_chart_path);

    // 3. Dispatch to Complete Latency over Time chart
    let mut dtoc_plot = Plot::new();
    let dtoc_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.dtoc)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Dispatch to Complete Latency over Time");

    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Dispatch to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Dispatch to Complete Latency (ms)"))),
    );

    let dtoc_chart_path = format!("{}_ufs_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!("UFS Dispatch to Complete chart saved: {}", dtoc_chart_path);

    // 4. Complete to Dispatch Latency over Time chart
    let mut ctod_plot = Plot::new();
    let ctod_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.ctod)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Complete to Dispatch Latency over Time");

    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Dispatch Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Dispatch Latency (ms)"))),
    );

    let ctod_chart_path = format!("{}_ufs_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!("UFS Complete to Dispatch chart saved: {}", ctod_chart_path);

    // 5. Complete to Complete Latency over Time chart
    let mut ctoc_plot = Plot::new();
    let ctoc_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.ctoc)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Complete to Complete Latency over Time");

    ctoc_plot.add_trace(ctoc_scatter);
    ctoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Complete Latency (ms)"))),
    );

    let ctoc_chart_path = format!("{}_ufs_ctoc_time.html", output_prefix);
    ctoc_plot.write_html(Path::new(&ctoc_chart_path));
    println!("UFS Complete to Complete chart saved: {}", ctoc_chart_path);

    // 6. Continuity pie chart
    let continuous_count = data.iter().filter(|d| d.continuous).count() as f64;
    let non_continuous_count = (data.len() as f64) - continuous_count;

    let mut continuous_plot = Plot::new();

    // Pie chart creation and settings (labels, values setting modified)
    let values = vec![continuous_count, non_continuous_count];
    let labels = vec!["Continuous".to_string(), "Non-continuous".to_string()];
    let pie = Pie::new(values)
        .labels(labels)
        .name("Continuity Distribution");

    continuous_plot.add_trace(pie);
    continuous_plot.set_layout(
        Layout::new()
            .title(Title::from("UFS Continuity Distribution"))
            .legend(Legend::new().title(Title::from("Continuity"))),
    );

    let continuous_chart_path = format!("{}_ufs_continuous.html", output_prefix);
    continuous_plot.write_html(Path::new(&continuous_chart_path));
    println!("UFS Continuity pie chart saved: {}", continuous_chart_path);

    Ok(())
}

/// Creates Plotly charts from Block I/O data.
pub fn create_block_charts(data: &[Block], output_prefix: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Block I/O data is empty.".to_string());
    }

    // Sort data by time for time-based charts
    let mut time_sorted_data = data.to_vec();
    time_sorted_data.sort_by(|a, b| {
        a.time
            .partial_cmp(&b.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // 1. Sector over Time chart
    let mut sector_plot = Plot::new();
    let sector_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.sector as f64)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Sector over Time");

    sector_plot.add_trace(sector_scatter);
    sector_plot.set_layout(
        Layout::new()
            .title(Title::from("Sector over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Sector"))),
    );

    let sector_chart_path = format!("{}_block_sector_time.html", output_prefix);
    sector_plot.write_html(Path::new(&sector_chart_path));
    println!("Block Sector chart saved: {}", sector_chart_path);

    // 2. Queue Depth over Time chart
    let mut qd_plot = Plot::new();
    let qd_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.qd as f64)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Queue Depth over Time");

    qd_plot.add_trace(qd_scatter);
    qd_plot.set_layout(
        Layout::new()
            .title(Title::from("Queue Depth over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Queue Depth"))),
    );

    let qd_chart_path = format!("{}_block_qd_time.html", output_prefix);
    qd_plot.write_html(Path::new(&qd_chart_path));
    println!("Block Queue Depth chart saved: {}", qd_chart_path);

    // 3. Dispatch to Complete Latency over Time chart
    let mut dtoc_plot = Plot::new();
    let dtoc_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.dtoc)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Dispatch to Complete Latency over Time");

    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Dispatch to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Dispatch to Complete Latency (ms)"))),
    );

    let dtoc_chart_path = format!("{}_block_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!(
        "Block Dispatch to Complete chart saved: {}",
        dtoc_chart_path
    );

    // 4. Complete to Dispatch Latency over Time chart
    let mut ctod_plot = Plot::new();
    let ctod_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.ctod)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Complete to Dispatch Latency over Time");

    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Dispatch Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Dispatch Latency (ms)"))),
    );

    let ctod_chart_path = format!("{}_block_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!(
        "Block Complete to Dispatch chart saved: {}",
        ctod_chart_path
    );

    // 5. Complete to Complete Latency over Time chart
    let mut ctoc_plot = Plot::new();
    let ctoc_scatter = Scatter::new(
        time_sorted_data
            .iter()
            .map(|d| d.time)
            .collect::<Vec<f64>>(),
        time_sorted_data
            .iter()
            .map(|d| d.ctoc)
            .collect::<Vec<f64>>(),
    )
    .mode(Mode::Markers)
    .name("Complete to Complete Latency over Time");

    ctoc_plot.add_trace(ctoc_scatter);
    ctoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Complete Latency (ms)"))),
    );

    let ctoc_chart_path = format!("{}_block_ctoc_time.html", output_prefix);
    ctoc_plot.write_html(Path::new(&ctoc_chart_path));
    println!(
        "Block Complete to Complete chart saved: {}",
        ctoc_chart_path
    );

    // 6. Continuity pie chart
    let continuous_count = data.iter().filter(|d| d.continuous).count() as f64;
    let non_continuous_count = (data.len() as f64) - continuous_count;

    let mut continuous_plot = Plot::new();

    // Pie chart creation and settings (labels, values setting modified)
    let values = vec![continuous_count, non_continuous_count];
    let labels = vec!["Continuous".to_string(), "Non-continuous".to_string()];
    let pie = Pie::new(values)
        .labels(labels)
        .name("Continuity Distribution");

    continuous_plot.add_trace(pie);
    continuous_plot.set_layout(
        Layout::new()
            .title(Title::from("Block I/O Continuity Distribution"))
            .legend(Legend::new().title(Title::from("Continuity"))),
    );

    let continuous_chart_path = format!("{}_block_continuous.html", output_prefix);
    continuous_plot.write_html(Path::new(&continuous_chart_path));
    println!(
        "Block Continuity pie chart saved: {}",
        continuous_chart_path
    );

    Ok(())
}

/// Create a Sankey diagram showing I/O flow paths
fn create_sankey_diagram(
    ufs_data: &[UFS],
    block_data: &[Block],
    output_prefix: &str,
) -> Result<(), String> {
    if ufs_data.is_empty() && block_data.is_empty() {
        return Err("Both UFS and Block I/O data are empty.".to_string());
    }

    let mut plot = Plot::new();

    // 데이터 준비
    let mut categories = Vec::new();
    let mut values = Vec::new();
    let mut colors = Vec::new();

    if !ufs_data.is_empty() {
        let ufs_requests = ufs_data.iter().filter(|d| d.action == "send_req").count() as f64;
        if ufs_requests > 0.0 {
            categories.push("UFS Requests".to_string());
            values.push(ufs_requests);
            colors.push(NamedColor::Blue);
        }

        // 작업 유형별 분류
        let mut opcode_counts: HashMap<String, usize> = HashMap::new();
        for event in ufs_data {
            if event.action == "send_req" {
                *opcode_counts.entry(event.opcode.clone()).or_insert(0) += 1;
            }
        }

        for (opcode, count) in opcode_counts.iter() {
            if *count > 0 {
                categories.push(format!("UFS Op: {}", opcode));
                values.push(*count as f64);
                colors.push(match opcode.as_str() {
                    "0x28" => NamedColor::LightBlue, // READ_10
                    "0x2a" => NamedColor::Green,     // WRITE_10
                    "0x35" => NamedColor::Red,       // SYNCHRONIZE_CACHE_10
                    _ => NamedColor::Gray,
                });
            }
        }
    }

    if !block_data.is_empty() {
        let block_requests = block_data.iter().filter(|d| d.action == "D").count() as f64;
        if block_requests > 0.0 {
            categories.push("Block I/O Requests".to_string());
            values.push(block_requests);
            colors.push(NamedColor::Orange);
        }

        // I/O 유형별 분류
        let mut io_type_counts: HashMap<String, usize> = HashMap::new();
        for event in block_data {
            if event.action == "D" {
                *io_type_counts.entry(event.io_type.clone()).or_insert(0) += 1;
            }
        }

        for (io_type, count) in io_type_counts.iter() {
            if *count > 0 {
                categories.push(format!("Block I/O: {}", io_type));
                values.push(*count as f64);
                colors.push(match io_type.as_str() {
                    "READ" => NamedColor::LightBlue,
                    "WRITE" => NamedColor::Green,
                    _ => NamedColor::Gray,
                });
            }
        }
    }

    // 막대 그래프 생성
    let bar = plotly::Bar::new(values, categories)
        .name("I/O Requests")
        .orientation(plotly::common::Orientation::Horizontal)
        .marker(Marker::new().color_array(colors));

    plot.add_trace(bar);
    plot.set_layout(
        Layout::new()
            .title(Title::from("I/O Request Flow"))
            .x_axis(Axis::new().title(Title::from("Request Count")))
            .y_axis(Axis::new().title(Title::from("Request Type")))
            .width(1000)
            .height(600),
    );

    let flow_path = format!("{}_io_flow_diagram.html", output_prefix);
    plot.write_html(Path::new(&flow_path));
    println!("I/O Flow diagram saved: {}", flow_path);

    Ok(())
}

/// Create a heatmap showing latency distribution by time of day and operation
fn create_latency_heatmaps(
    ufs_data: &[UFS],
    block_data: &[Block],
    output_prefix: &str,
) -> Result<(), String> {
    // UFS Latency Scatter Plot By Operation
    if !ufs_data.is_empty() {
        let mut opcode_latency: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

        for event in ufs_data {
            if event.dtoc > 0.0 {
                opcode_latency
                    .entry(event.opcode.clone())
                    .or_default()
                    .push((event.time, event.dtoc));
            }
        }

        if opcode_latency.is_empty() {
            println!("No valid latency data for UFS latency plot");
        } else {
            let mut plot = Plot::new();

            for (opcode, points) in &opcode_latency {
                let x: Vec<f64> = points.iter().map(|(time, _)| *time).collect();
                let y: Vec<f64> = points.iter().map(|(_, latency)| *latency).collect();

                let scatter = Scatter::new(x, y).mode(Mode::Markers).name(opcode).marker(
                    Marker::new().color(match opcode.as_str() {
                        "0x28" => NamedColor::Blue,  // READ_10
                        "0x2a" => NamedColor::Green, // WRITE_10
                        "0x35" => NamedColor::Red,   // SYNCHRONIZE_CACHE_10
                        _ => NamedColor::Gray,
                    }),
                );

                plot.add_trace(scatter);
            }

            plot.set_layout(
                Layout::new()
                    .title(Title::from("UFS Latency by Operation Code and Time"))
                    .x_axis(Axis::new().title(Title::from("Time (s)")))
                    .y_axis(Axis::new().title(Title::from("Latency (ms)")))
                    .show_legend(true),
            );

            let latency_path = format!("{}_ufs_latency_by_opcode.html", output_prefix);
            plot.write_html(Path::new(&latency_path));
            println!("UFS Latency by Operation Code plot saved: {}", latency_path);
        }
    }

    // Block I/O Latency Scatter Plot By I/O Type
    if !block_data.is_empty() {
        let mut io_type_latency: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

        for event in block_data {
            if event.dtoc > 0.0 {
                io_type_latency
                    .entry(event.io_type.clone())
                    .or_default()
                    .push((event.time, event.dtoc));
            }
        }

        if io_type_latency.is_empty() {
            println!("No valid latency data for Block I/O latency plot");
        } else {
            let mut plot = Plot::new();

            for (io_type, points) in &io_type_latency {
                let x: Vec<f64> = points.iter().map(|(time, _)| *time).collect();
                let y: Vec<f64> = points.iter().map(|(_, latency)| *latency).collect();

                let scatter = Scatter::new(x, y).mode(Mode::Markers).name(io_type).marker(
                    Marker::new().color(match io_type.as_str() {
                        "READ" => NamedColor::Blue,
                        "WRITE" => NamedColor::Green,
                        _ => NamedColor::Gray,
                    }),
                );

                plot.add_trace(scatter);
            }

            plot.set_layout(
                Layout::new()
                    .title(Title::from("Block I/O Latency by I/O Type and Time"))
                    .x_axis(Axis::new().title(Title::from("Time (s)")))
                    .y_axis(Axis::new().title(Title::from("Latency (ms)")))
                    .show_legend(true),
            );

            let latency_path = format!("{}_block_latency_by_iotype.html", output_prefix);
            plot.write_html(Path::new(&latency_path));
            println!("Block I/O Latency by I/O Type plot saved: {}", latency_path);
        }
    }

    Ok(())
}

/// Create box plots for latency statistics visualization
fn create_latency_box_plots(
    ufs_data: &[UFS],
    block_data: &[Block],
    output_prefix: &str,
) -> Result<(), String> {
    // UFS Latency Comparison Chart
    if !ufs_data.is_empty() {
        let mut opcode_dtoc: HashMap<String, Vec<f64>> = HashMap::new();

        for event in ufs_data {
            if event.dtoc > 0.0 {
                opcode_dtoc
                    .entry(event.opcode.clone())
                    .or_default()
                    .push(event.dtoc);
            }
        }

        if opcode_dtoc.is_empty() {
            println!("No valid latency data for UFS latency comparison");
        } else {
            let mut plot = Plot::new();

            let mut categories = Vec::new();
            let mut avg_values = Vec::new();
            let mut max_values = Vec::new();
            let mut min_values = Vec::new();

            for (opcode, latencies) in &opcode_dtoc {
                categories.push(opcode.clone());

                let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
                let max = latencies.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let min = latencies.iter().fold(f64::INFINITY, |a, &b| a.min(b));

                avg_values.push(avg);
                max_values.push(max);
                min_values.push(min);
            }

            let avg_bar = plotly::Bar::new(avg_values.clone(), categories.clone())
                .name("Average Latency")
                .marker(Marker::new().color(NamedColor::Blue));

            let max_bar = plotly::Bar::new(max_values, categories.clone())
                .name("Maximum Latency")
                .marker(Marker::new().color(NamedColor::Red));

            let min_bar = plotly::Bar::new(min_values, categories.clone())
                .name("Minimum Latency")
                .marker(Marker::new().color(NamedColor::Green));

            plot.add_trace(avg_bar);
            plot.add_trace(max_bar);
            plot.add_trace(min_bar);

            plot.set_layout(
                Layout::new()
                    .title(Title::from("UFS Latency Statistics by Operation Code"))
                    .y_axis(Axis::new().title(Title::from("Latency (ms)")))
                    .bar_mode(BarMode::Group)
                    .show_legend(true),
            );

            let latency_stats_path = format!("{}_ufs_latency_stats.html", output_prefix);
            plot.write_html(Path::new(&latency_stats_path));
            println!("UFS Latency Statistics plot saved: {}", latency_stats_path);
        }
    }

    // Block I/O Latency Comparison Chart
    if !block_data.is_empty() {
        let mut iotype_dtoc: HashMap<String, Vec<f64>> = HashMap::new();

        for event in block_data {
            if event.dtoc > 0.0 {
                iotype_dtoc
                    .entry(event.io_type.clone())
                    .or_default()
                    .push(event.dtoc);
            }
        }

        if iotype_dtoc.is_empty() {
            println!("No valid latency data for Block I/O latency comparison");
        } else {
            let mut plot = Plot::new();

            let mut categories = Vec::new();
            let mut avg_values = Vec::new();
            let mut max_values = Vec::new();
            let mut min_values = Vec::new();

            for (io_type, latencies) in &iotype_dtoc {
                categories.push(io_type.clone());

                let avg = latencies.iter().sum::<f64>() / latencies.len() as f64;
                let max = latencies.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                let min = latencies.iter().fold(f64::INFINITY, |a, &b| a.min(b));

                avg_values.push(avg);
                max_values.push(max);
                min_values.push(min);
            }

            let avg_bar = plotly::Bar::new(avg_values.clone(), categories.clone())
                .name("Average Latency")
                .marker(Marker::new().color(NamedColor::Blue));

            let max_bar = plotly::Bar::new(max_values, categories.clone())
                .name("Maximum Latency")
                .marker(Marker::new().color(NamedColor::Red));

            let min_bar = plotly::Bar::new(min_values, categories.clone())
                .name("Minimum Latency")
                .marker(Marker::new().color(NamedColor::Green));

            plot.add_trace(avg_bar);
            plot.add_trace(max_bar);
            plot.add_trace(min_bar);

            plot.set_layout(
                Layout::new()
                    .title(Title::from("Block I/O Latency Statistics by I/O Type"))
                    .y_axis(Axis::new().title(Title::from("Latency (ms)")))
                    .bar_mode(BarMode::Group)
                    .show_legend(true),
            );

            let latency_stats_path = format!("{}_block_latency_stats.html", output_prefix);
            plot.write_html(Path::new(&latency_stats_path));
            println!(
                "Block I/O Latency Statistics plot saved: {}",
                latency_stats_path
            );
        }
    }

    Ok(())
}

/// Create timeline chart to visualize request timeline
fn create_request_timeline(
    ufs_data: &[UFS],
    block_data: &[Block],
    output_prefix: &str,
) -> Result<(), String> {
    // UFS Request Timeline
    if !ufs_data.is_empty() {
        let mut request_map: HashMap<u32, (f64, String)> = HashMap::new();
        let mut timeline_data: Vec<(u32, f64, f64, String)> = Vec::new();

        for event in ufs_data {
            if event.action == "send_req" {
                request_map.insert(event.tag, (event.time, event.opcode.clone()));
            } else if event.action == "complete_rsp" {
                if let Some((start_time, opcode)) = request_map.remove(&event.tag) {
                    timeline_data.push((event.tag, start_time, event.time, opcode));
                }
            }
        }

        let max_requests = 50;
        if timeline_data.len() > max_requests {
            timeline_data
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            timeline_data.truncate(max_requests);
        }

        if !timeline_data.is_empty() {
            let mut plot = Plot::new();

            for (idx, (tag, start, end, opcode)) in timeline_data.iter().enumerate() {
                let y_pos = (timeline_data.len() - idx) as f64;

                let start_scatter = Scatter::new(vec![*start], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(format!("Tag {} Start", tag))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Blue).size(10));

                let end_scatter = Scatter::new(vec![*end], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(format!("Tag {} End", tag))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Red).size(10));

                let line_scatter = Scatter::new(vec![*start, *end], vec![y_pos, y_pos])
                    .mode(Mode::Lines)
                    .name(format!("Tag {} ({})", tag, opcode))
                    .line(plotly::common::Line::new().color(match opcode.as_str() {
                        "0x28" => NamedColor::Blue,
                        "0x2a" => NamedColor::Green,
                        "0x35" => NamedColor::Red,
                        _ => NamedColor::Gray,
                    }));

                plot.add_trace(start_scatter);
                plot.add_trace(end_scatter);
                plot.add_trace(line_scatter);
            }

            let y_tickvals: Vec<f64> = (1..=timeline_data.len()).map(|i| i as f64).collect();
            let y_ticktext: Vec<String> = timeline_data
                .iter()
                .map(|(tag, _, _, _)| format!("Tag {}", tag))
                .collect();

            plot.set_layout(
                Layout::new()
                    .title(Title::from("UFS Request Timeline"))
                    .x_axis(Axis::new().title(Title::from("Time (s)")))
                    .y_axis(
                        Axis::new()
                            .title(Title::from("Request"))
                            .tick_values(y_tickvals)
                            .tick_text(y_ticktext),
                    )
                    .width(1000)
                    .height(if timeline_data.len() > 10 {
                        timeline_data.len() * 30
                    } else {
                        600
                    })
                    .show_legend(true),
            );

            let timeline_path = format!("{}_ufs_request_timeline.html", output_prefix);
            plot.write_html(Path::new(&timeline_path));
            println!("UFS Request Timeline saved: {}", timeline_path);
        } else {
            println!("No complete UFS request-response pairs found for timeline");
        }
    }

    if !block_data.is_empty() {
        let mut request_map: HashMap<(u64, u32), (f64, String)> = HashMap::new();
        let mut timeline_data: Vec<((u64, u32), f64, f64, String)> = Vec::new();

        for event in block_data {
            let key = (event.sector, event.size);
            if event.action == "D" {
                request_map.insert(key, (event.time, event.io_type.clone()));
            } else if event.action == "C" {
                if let Some((start_time, io_type)) = request_map.remove(&key) {
                    timeline_data.push((key, start_time, event.time, io_type));
                }
            }
        }

        let max_requests = 50;
        if timeline_data.len() > max_requests {
            timeline_data
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            timeline_data.truncate(max_requests);
        }

        if !timeline_data.is_empty() {
            let mut plot = Plot::new();

            for (idx, ((sector, size), start, end, io_type)) in timeline_data.iter().enumerate() {
                let y_pos = (timeline_data.len() - idx) as f64;

                let start_scatter = Scatter::new(vec![*start], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(format!("Sector {} Start", sector))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Blue).size(10));

                let end_scatter = Scatter::new(vec![*end], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(format!("Sector {} End", sector))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Red).size(10));

                let line_scatter = Scatter::new(vec![*start, *end], vec![y_pos, y_pos])
                    .mode(Mode::Lines)
                    .name(format!("Sector {} (+{}) - {}", sector, size, io_type))
                    .line(plotly::common::Line::new().color(match io_type.as_str() {
                        "READ" => NamedColor::Blue,
                        "WRITE" => NamedColor::Green,
                        _ => NamedColor::Gray,
                    }));

                plot.add_trace(start_scatter);
                plot.add_trace(end_scatter);
                plot.add_trace(line_scatter);
            }

            let y_tickvals: Vec<f64> = (1..=timeline_data.len()).map(|i| i as f64).collect();
            let y_ticktext: Vec<String> = timeline_data
                .iter()
                .map(|((sector, size), _, _, _)| format!("Sec {} (+{})", sector, size))
                .collect();

            plot.set_layout(
                Layout::new()
                    .title(Title::from("Block I/O Request Timeline"))
                    .x_axis(Axis::new().title(Title::from("Time (s)")))
                    .y_axis(
                        Axis::new()
                            .title(Title::from("Request"))
                            .tick_values(y_tickvals)
                            .tick_text(y_ticktext),
                    )
                    .width(1000)
                    .height(if timeline_data.len() > 10 {
                        timeline_data.len() * 30
                    } else {
                        600
                    })
                    .show_legend(true),
            );

            let timeline_path = format!("{}_block_request_timeline.html", output_prefix);
            plot.write_html(Path::new(&timeline_path));
            println!("Block I/O Request Timeline saved: {}", timeline_path);
        } else {
            println!("No complete Block I/O request-response pairs found for timeline");
        }
    }

    Ok(())
}

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
    println!("UFS operation distribution HTML chart saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_ufs_operation_distribution_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("UFS operation distribution SVG chart saved to: {}", svg_output_path);
    
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
    std::fs::write(&html_output_path, chart.to_string()).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter HTML plot saved to: {}", html_output_path);
    
    // Save as SVG
    let svg_output_path = format!("{}_lba_latency_scatter_charming.svg", output_prefix);
    let mut renderer = ImageRenderer::new(1000, 800);
    renderer.save(&chart, &svg_output_path).map_err(|e| e.to_string())?;
    println!("LBA vs Latency scatter SVG plot saved to: {}", svg_output_path);
    
    Ok(())
}

/// Generate charts and save statistics data.
pub fn generate_charts(
    processed_ufs: &[UFS],
    processed_blocks: &[Block],
    output_prefix: &str,
) -> Result<(), String> {
    match create_ufs_charts(processed_ufs, output_prefix) {
        Ok(_) => {
            println!("UFS charts have been generated.");
        }
        Err(e) => {
            eprintln!("Error generating UFS charts: {}", e);
        }
    }

    match create_block_charts(processed_blocks, output_prefix) {
        Ok(_) => {
            println!("Block I/O charts have been generated.");
        }
        Err(e) => {
            eprintln!("Error generating Block I/O charts: {}", e);
        }
    }

    println!("\nGenerating advanced diagrams...");

    match create_sankey_diagram(processed_ufs, processed_blocks, output_prefix) {
        Ok(_) => println!("I/O Flow diagram has been generated."),
        Err(e) => eprintln!("Error generating I/O Flow diagram: {}", e),
    }

    match create_latency_heatmaps(processed_ufs, processed_blocks, output_prefix) {
        Ok(_) => println!("Latency analysis charts have been generated."),
        Err(e) => eprintln!("Error generating latency analysis charts: {}", e),
    }

    match create_latency_box_plots(processed_ufs, processed_blocks, output_prefix) {
        Ok(_) => println!("Latency statistics charts have been generated."),
        Err(e) => eprintln!("Error generating latency statistics charts: {}", e),
    }

    match create_request_timeline(processed_ufs, processed_blocks, output_prefix) {
        Ok(_) => println!("Request timeline charts have been generated."),
        Err(e) => eprintln!("Error generating request timeline charts: {}", e),
    }

    println!("\nGenerating Charming-based interactive charts...");
    match generate_charming_charts(processed_ufs, processed_blocks, output_prefix) {
        Ok(_) => println!("Charming interactive charts have been generated."),
        Err(e) => eprintln!("Error generating Charming charts: {}", e),
    }

    Ok(())
}
