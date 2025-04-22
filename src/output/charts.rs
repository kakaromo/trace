use crate::models::{Block, UFS};
use plotly::color::NamedColor;
use plotly::common::{ColorScale, Font, Marker, Mode, Title};
use plotly::layout::{Axis, BarMode, Legend};
use plotly::{Layout, Pie, Plot, Scatter};
use std::collections::HashMap;
use std::path::Path;

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

    // 3. Device to Complete Latency over Time chart
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
    .name("Device to Complete Latency over Time");

    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Device to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Device to Complete Latency (ms)"))),
    );

    let dtoc_chart_path = format!("{}_ufs_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!("UFS Device to Complete chart saved: {}", dtoc_chart_path);

    // 4. Complete to Device Latency over Time chart
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
    .name("Complete to Device Latency over Time");

    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Device Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Device Latency (ms)"))),
    );

    let ctod_chart_path = format!("{}_ufs_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!("UFS Complete to Device chart saved: {}", ctod_chart_path);

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

    // 3. Device to Complete Latency over Time chart
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
    .name("Device to Complete Latency over Time");

    dtoc_plot.add_trace(dtoc_scatter);
    dtoc_plot.set_layout(
        Layout::new()
            .title(Title::from("Device to Complete Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Device to Complete Latency (ms)"))),
    );

    let dtoc_chart_path = format!("{}_block_dtoc_time.html", output_prefix);
    dtoc_plot.write_html(Path::new(&dtoc_chart_path));
    println!("Block Device to Complete chart saved: {}", dtoc_chart_path);

    // 4. Complete to Device Latency over Time chart
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
    .name("Complete to Device Latency over Time");

    ctod_plot.add_trace(ctod_scatter);
    ctod_plot.set_layout(
        Layout::new()
            .title(Title::from("Complete to Device Latency over Time"))
            .x_axis(Axis::new().title(Title::from("Time (s)")))
            .y_axis(Axis::new().title(Title::from("Complete to Device Latency (ms)"))),
    );

    let ctod_chart_path = format!("{}_block_ctod_time.html", output_prefix);
    ctod_plot.write_html(Path::new(&ctod_chart_path));
    println!("Block Complete to Device chart saved: {}", ctod_chart_path);

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

                let scatter = Scatter::new(x, y).mode(Mode::Markers).name(&opcode).marker(
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

                let scatter = Scatter::new(x, y)
                    .mode(Mode::Markers)
                    .name(&io_type)
                    .marker(Marker::new().color(match io_type.as_str() {
                        "READ" => NamedColor::Blue,
                        "WRITE" => NamedColor::Green,
                        _ => NamedColor::Gray,
                    }));

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
                    .name(&format!("Tag {} Start", tag))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Blue).size(10));

                let end_scatter = Scatter::new(vec![*end], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(&format!("Tag {} End", tag))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Red).size(10));

                let line_scatter = Scatter::new(vec![*start, *end], vec![y_pos, y_pos])
                    .mode(Mode::Lines)
                    .name(&format!("Tag {} ({})", tag, opcode))
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
                    .name(&format!("Sector {} Start", sector))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Blue).size(10));

                let end_scatter = Scatter::new(vec![*end], vec![y_pos])
                    .mode(Mode::Markers)
                    .name(&format!("Sector {} End", sector))
                    .show_legend(false)
                    .marker(Marker::new().color(NamedColor::Red).size(10));

                let line_scatter = Scatter::new(vec![*start, *end], vec![y_pos, y_pos])
                    .mode(Mode::Lines)
                    .name(&format!("Sector {} (+{}) - {}", sector, size, io_type))
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

    Ok(())
}
