use crate::models::{Block, UFS, UFSCUSTOM};
use csv::Writer;
/// Excel의 최대 행 수 (헤더 제외)
const EXCEL_MAX_ROWS: usize = 1_048_575;
use std::error::Error;
use std::fs::File;

/// CSV export function for UFS traces
pub fn save_ufs_to_csv(traces: &[UFS], output_prefix: &str) -> Result<(), Box<dyn Error>> {
    let mut start = 0;
    let total = traces.len();
    while start < total {
        let end = usize::min(start + EXCEL_MAX_ROWS, total);
        let chunk = &traces[start..end];
        if chunk.is_empty() {
            break;
        }
        let start_time = chunk.first().map(|t| t.time).unwrap_or(0.0);
        let end_time = chunk.last().map(|t| t.time).unwrap_or(0.0);
        let filename = format!("{output_prefix}_ufs_{start_time}_{end_time}.csv");
        let file = File::create(&filename)?;
        let mut writer = Writer::from_writer(file);
        writer.write_record([
            "time",
            "process",
            "cpu",
            "action",
            "tag",
            "opcode",
            "lba",
            "size",
            "groupid",
            "hwqid",
            "qd",
            "dtoc",
            "ctoc",
            "ctod",
            "continuous",
        ])?;
        for trace in chunk {
            writer.write_record(&[
                trace.time.to_string(),
                trace.process.clone(),
                trace.cpu.to_string(),
                trace.action.clone(),
                trace.tag.to_string(),
                trace.opcode.clone(),
                trace.lba.to_string(),
                trace.size.to_string(),
                trace.groupid.to_string(),
                trace.hwqid.to_string(),
                trace.qd.to_string(),
                trace.dtoc.to_string(),
                trace.ctoc.to_string(),
                trace.ctod.to_string(),
                trace.continuous.to_string(),
            ])?;
        }
        writer.flush()?;
        start = end;
    }
    Ok(())
}

/// CSV export function for Block traces
pub fn save_block_to_csv(traces: &[Block], output_prefix: &str) -> Result<(), Box<dyn Error>> {
    let mut start = 0;
    let total = traces.len();
    while start < total {
        let end = usize::min(start + EXCEL_MAX_ROWS, total);
        let chunk = &traces[start..end];
        if chunk.is_empty() {
            break;
        }
        let start_time = chunk.first().map(|t| t.time).unwrap_or(0.0);
        let end_time = chunk.last().map(|t| t.time).unwrap_or(0.0);
        let filename = format!("{output_prefix}_block_{start_time}_{end_time}.csv");
        let file = File::create(&filename)?;
        let mut writer = Writer::from_writer(file);
        writer.write_record([
            "time",
            "process",
            "cpu",
            "flags",
            "action",
            "devmajor",
            "devminor",
            "io_type",
            "extra",
            "sector",
            "size",
            "comm",
            "qd",
            "dtoc",
            "ctoc",
            "ctod",
            "continuous",
        ])?;
        for trace in chunk {
            writer.write_record(&[
                trace.time.to_string(),
                trace.process.clone(),
                trace.cpu.to_string(),
                trace.flags.clone(),
                trace.action.clone(),
                trace.devmajor.to_string(),
                trace.devminor.to_string(),
                trace.io_type.clone(),
                trace.extra.to_string(),
                trace.sector.to_string(),
                trace.size.to_string(),
                trace.comm.clone(),
                trace.qd.to_string(),
                trace.dtoc.to_string(),
                trace.ctoc.to_string(),
                trace.ctod.to_string(),
                trace.continuous.to_string(),
            ])?;
        }
        writer.flush()?;
        start = end;
    }
    Ok(())
}

/// CSV export function for UFSCUSTOM traces
pub fn save_ufscustom_to_csv(
    traces: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), Box<dyn Error>> {
    let mut start = 0;
    let total = traces.len();
    while start < total {
        let end = usize::min(start + EXCEL_MAX_ROWS, total);
        let chunk = &traces[start..end];
        if chunk.is_empty() {
            break;
        }
        let start_time = chunk.first().map(|t| t.start_time).unwrap_or(0.0);
        let end_time = chunk.last().map(|t| t.start_time).unwrap_or(0.0);
        let filename = format!(
            "{output_prefix}_ufscustom_{start_time}_{end_time}.csv"
        );
        let file = File::create(&filename)?;
        let mut writer = Writer::from_writer(file);
        writer.write_record([
            "start_time",
            "end_time",
            "opcode",
            "lba",
            "size",
            "start_qd",
            "end_qd",
            "dtoc",
            "ctoc",
            "ctod",
            "continuous",
        ])?;
        for trace in chunk {
            writer.write_record(&[
                trace.start_time.to_string(),
                trace.end_time.to_string(),
                trace.opcode.clone(),
                trace.lba.to_string(),
                trace.size.to_string(),
                trace.start_qd.to_string(),
                trace.end_qd.to_string(),
                trace.dtoc.to_string(),
                trace.ctoc.to_string(),
                trace.ctod.to_string(),
                trace.continuous.to_string(),
            ])?;
        }
        writer.flush()?;
        start = end;
    }
    Ok(())
}

/// Save all trace types to CSV files
pub fn save_to_csv(
    ufs_traces: &[UFS],
    block_traces: &[Block],
    ufscustom_traces: &[UFSCUSTOM],
    output_prefix: &str,
) -> Result<(), Box<dyn Error>> {
    // Save UFS traces if not empty
    if !ufs_traces.is_empty() {
        save_ufs_to_csv(ufs_traces, output_prefix)?;
    }

    // Save Block traces if not empty
    if !block_traces.is_empty() {
        save_block_to_csv(block_traces, output_prefix)?;
    }

    // Save UFSCUSTOM traces if not empty
    if !ufscustom_traces.is_empty() {
        save_ufscustom_to_csv(ufscustom_traces, output_prefix)?;
    }

    Ok(())
}
