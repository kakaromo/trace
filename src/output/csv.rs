use crate::models::{Block, UFS, UFSCUSTOM};
use csv::Writer;
/// Excel의 최대 행 수 (헤더 제외)
const EXCEL_MAX_ROWS: usize = 1_048_575;
use std::error::Error;
use std::fmt::Write as FmtWrite;
use std::fs::File;

/// 재사용 가능한 String 버퍼를 사용하여 to_string() 할당을 제거하는 매크로
macro_rules! write_buf {
    ($buf:expr, $val:expr) => {{
        $buf.clear();
        write!($buf, "{}", $val).unwrap();
        $buf.as_str()
    }};
}

/// CSV export function for UFS traces
pub fn save_ufs_to_csv(traces: &[UFS], output_prefix: &str) -> Result<(), Box<dyn Error>> {
    let mut start = 0;
    let total = traces.len();
    // 재사용 버퍼들 (매 레코드마다 할당 대신 clear+write로 재활용)
    let mut buf1 = String::with_capacity(32);
    let mut buf2 = String::with_capacity(32);
    let mut buf3 = String::with_capacity(32);
    let mut buf4 = String::with_capacity(32);
    let mut buf5 = String::with_capacity(32);
    let mut buf6 = String::with_capacity(32);
    let mut buf7 = String::with_capacity(32);
    let mut buf8 = String::with_capacity(32);
    let mut buf9 = String::with_capacity(32);
    let mut buf10 = String::with_capacity(32);
    let mut buf11 = String::with_capacity(32);
    let mut buf12 = String::with_capacity(32);
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
            writer.write_record([
                write_buf!(buf1, trace.time),
                &trace.process,
                write_buf!(buf2, trace.cpu),
                &trace.action,
                write_buf!(buf3, trace.tag),
                &trace.opcode,
                write_buf!(buf4, trace.lba),
                write_buf!(buf5, trace.size),
                write_buf!(buf6, trace.groupid),
                write_buf!(buf7, trace.hwqid),
                write_buf!(buf8, trace.qd),
                write_buf!(buf9, trace.dtoc),
                write_buf!(buf10, trace.ctoc),
                write_buf!(buf11, trace.ctod),
                write_buf!(buf12, trace.continuous),
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
    let mut buf1 = String::with_capacity(32);
    let mut buf2 = String::with_capacity(32);
    let mut buf3 = String::with_capacity(32);
    let mut buf4 = String::with_capacity(32);
    let mut buf5 = String::with_capacity(32);
    let mut buf6 = String::with_capacity(32);
    let mut buf7 = String::with_capacity(32);
    let mut buf8 = String::with_capacity(32);
    let mut buf9 = String::with_capacity(32);
    let mut buf10 = String::with_capacity(32);
    let mut buf11 = String::with_capacity(32);
    let mut buf12 = String::with_capacity(32);
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
            writer.write_record([
                write_buf!(buf1, trace.time),
                &trace.process,
                write_buf!(buf2, trace.cpu),
                &trace.flags,
                &trace.action,
                write_buf!(buf3, trace.devmajor),
                write_buf!(buf4, trace.devminor),
                &trace.io_type,
                write_buf!(buf5, trace.extra),
                write_buf!(buf6, trace.sector),
                write_buf!(buf7, trace.size),
                &trace.comm,
                write_buf!(buf8, trace.qd),
                write_buf!(buf9, trace.dtoc),
                write_buf!(buf10, trace.ctoc),
                write_buf!(buf11, trace.ctod),
                write_buf!(buf12, trace.continuous),
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
    let mut buf1 = String::with_capacity(32);
    let mut buf2 = String::with_capacity(32);
    let mut buf3 = String::with_capacity(32);
    let mut buf4 = String::with_capacity(32);
    let mut buf5 = String::with_capacity(32);
    let mut buf6 = String::with_capacity(32);
    let mut buf7 = String::with_capacity(32);
    let mut buf8 = String::with_capacity(32);
    let mut buf9 = String::with_capacity(32);
    let mut buf10 = String::with_capacity(32);
    while start < total {
        let end = usize::min(start + EXCEL_MAX_ROWS, total);
        let chunk = &traces[start..end];
        if chunk.is_empty() {
            break;
        }
        let start_time = chunk.first().map(|t| t.start_time).unwrap_or(0.0);
        let end_time = chunk.last().map(|t| t.start_time).unwrap_or(0.0);
        let filename = format!("{output_prefix}_ufscustom_{start_time}_{end_time}.csv");
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
            writer.write_record([
                write_buf!(buf1, trace.start_time),
                write_buf!(buf2, trace.end_time),
                &trace.opcode,
                write_buf!(buf3, trace.lba),
                write_buf!(buf4, trace.size),
                write_buf!(buf5, trace.start_qd),
                write_buf!(buf6, trace.end_qd),
                write_buf!(buf7, trace.dtoc),
                write_buf!(buf8, trace.ctoc),
                write_buf!(buf9, trace.ctod),
                write_buf!(buf10, trace.continuous),
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
