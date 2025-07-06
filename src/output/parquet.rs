use crate::models::{Block, UFS, UFSCUSTOM};
use arrow::array::{ArrayRef, BooleanArray, Float64Array, StringArray, UInt32Array, UInt64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use std::fs::File;
use std::sync::Arc;

pub fn save_to_parquet(
    ufs_traces: &[UFS],
    block_traces: &[Block],
    ufscustom_traces: &[UFSCUSTOM],
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // UFS 데이터 저장 (청크 단위로)
    if !ufs_traces.is_empty() {
        save_ufs_to_parquet_chunked(ufs_traces, &format!("{}_ufs.parquet", output_path), chunk_size)?;
    }

    // Block 데이터 저장 (청크 단위로)
    if !block_traces.is_empty() {
        save_block_to_parquet_chunked(block_traces, &format!("{}_block.parquet", output_path), chunk_size)?;
    }

    // UFSCUSTOM 데이터 저장 (청크 단위로)
    if !ufscustom_traces.is_empty() {
        save_ufscustom_to_parquet_chunked(
            ufscustom_traces,
            &format!("{}_ufscustom.parquet", output_path),
            chunk_size
        )?;
    }

    Ok(())
}

fn save_ufs_to_parquet_chunked(
    traces: &[UFS], 
    filepath: &str, 
    chunk_size: usize
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let total_chunks = traces.len().div_ceil(chunk_size); // 올림 계산
    eprintln!("Saving {} UFS traces to {} using chunk size {} ({} chunks)", 
              traces.len(), filepath, chunk_size, total_chunks);

    let schema = Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("process", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("cpu", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("action", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("tag", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("opcode", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("lba", arrow::datatypes::DataType::UInt64, false),
        arrow::datatypes::Field::new("size", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("groupid", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("hwqid", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("qd", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("dtoc", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("ctoc", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("ctod", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("continuous", arrow::datatypes::DataType::Boolean, false),
    ]));

    let file = File::create(filepath)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 청크 단위로 데이터 처리
    for (chunk_idx, chunk) in traces.chunks(chunk_size).enumerate() {
        if chunk_idx % 10 == 0 || chunk_idx == total_chunks - 1 {
            eprintln!("Processing UFS chunk {}/{} ({} records)", 
                      chunk_idx + 1, total_chunks, chunk.len());
        }

        let time = Float64Array::from(chunk.iter().map(|t| t.time).collect::<Vec<_>>());
        let process = StringArray::from(
            chunk
                .iter()
                .map(|t| t.process.as_str())
                .collect::<Vec<_>>(),
        );
        let cpu = UInt32Array::from(chunk.iter().map(|t| t.cpu).collect::<Vec<_>>());
        let action = StringArray::from(chunk.iter().map(|t| t.action.as_str()).collect::<Vec<_>>());
        let tag = UInt32Array::from(chunk.iter().map(|t| t.tag).collect::<Vec<_>>());
        let opcode = StringArray::from(chunk.iter().map(|t| t.opcode.as_str()).collect::<Vec<_>>());
        let lba = UInt64Array::from(chunk.iter().map(|t| t.lba).collect::<Vec<_>>());
        let size = UInt32Array::from(chunk.iter().map(|t| t.size).collect::<Vec<_>>());
        let groupid = UInt32Array::from(chunk.iter().map(|t| t.groupid).collect::<Vec<_>>());
        let hwqid = UInt32Array::from(chunk.iter().map(|t| t.hwqid).collect::<Vec<_>>());
        let qd = UInt32Array::from(chunk.iter().map(|t| t.qd).collect::<Vec<_>>());
        let dtoc = Float64Array::from(chunk.iter().map(|t| t.dtoc).collect::<Vec<_>>());
        let ctoc = Float64Array::from(chunk.iter().map(|t| t.ctoc).collect::<Vec<_>>());
        let ctod = Float64Array::from(chunk.iter().map(|t| t.ctod).collect::<Vec<_>>());
        let continuous = BooleanArray::from(chunk.iter().map(|t| t.continuous).collect::<Vec<_>>());

        let columns: Vec<ArrayRef> = vec![
            Arc::new(time),
            Arc::new(process),
            Arc::new(cpu),
            Arc::new(action),
            Arc::new(tag),
            Arc::new(opcode),
            Arc::new(lba),
            Arc::new(size),
            Arc::new(groupid),
            Arc::new(hwqid),
            Arc::new(qd),
            Arc::new(dtoc),
            Arc::new(ctoc),
            Arc::new(ctod),
            Arc::new(continuous),
        ];

        let batch = RecordBatch::try_new(schema.clone(), columns)?;
        writer.write(&batch)?;
    }

    writer.close()?;
    eprintln!("UFS Parquet file saved successfully: {}", filepath);
    Ok(())
}

fn save_block_to_parquet_chunked(
    traces: &[Block],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let total_chunks = traces.len().div_ceil(chunk_size); // 올림 계산
    eprintln!("Saving {} Block traces to {} using chunk size {} ({} chunks)", 
              traces.len(), filepath, chunk_size, total_chunks);

    let schema = Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("process", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("cpu", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("flags", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("action", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("devmajor", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("devminor", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("io_type", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("extra", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("sector", arrow::datatypes::DataType::UInt64, false),
        arrow::datatypes::Field::new("size", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("comm", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("qd", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("dtoc", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("ctoc", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("ctod", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("continuous", arrow::datatypes::DataType::Boolean, false),
    ]));

    let file = File::create(filepath)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 청크 단위로 데이터 처리
    for (chunk_idx, chunk) in traces.chunks(chunk_size).enumerate() {
        if chunk_idx % 10 == 0 || chunk_idx == total_chunks - 1 {
            eprintln!("Processing Block chunk {}/{} ({} records)", 
                      chunk_idx + 1, total_chunks, chunk.len());
        }

        let time = Float64Array::from(chunk.iter().map(|t| t.time).collect::<Vec<_>>());
        let process = StringArray::from(
            chunk
                .iter()
                .map(|t| t.process.as_str())
                .collect::<Vec<_>>(),
        );
        let cpu = UInt32Array::from(chunk.iter().map(|t| t.cpu).collect::<Vec<_>>());
        let flags = StringArray::from(chunk.iter().map(|t| t.flags.as_str()).collect::<Vec<_>>());
        let action = StringArray::from(chunk.iter().map(|t| t.action.as_str()).collect::<Vec<_>>());
        let devmajor = UInt32Array::from(chunk.iter().map(|t| t.devmajor).collect::<Vec<_>>());
        let devminor = UInt32Array::from(chunk.iter().map(|t| t.devminor).collect::<Vec<_>>());
        let io_type = StringArray::from(
            chunk
                .iter()
                .map(|t| t.io_type.as_str())
                .collect::<Vec<_>>(),
        );
        let extra = UInt32Array::from(chunk.iter().map(|t| t.extra).collect::<Vec<_>>());
        let sector = UInt64Array::from(chunk.iter().map(|t| t.sector).collect::<Vec<_>>());
        let size = UInt32Array::from(chunk.iter().map(|t| t.size).collect::<Vec<_>>());
        let comm = StringArray::from(chunk.iter().map(|t| t.comm.as_str()).collect::<Vec<_>>());
        let qd = UInt32Array::from(chunk.iter().map(|t| t.qd).collect::<Vec<_>>());
        let dtoc = Float64Array::from(chunk.iter().map(|t| t.dtoc).collect::<Vec<_>>());
        let ctoc = Float64Array::from(chunk.iter().map(|t| t.ctoc).collect::<Vec<_>>());
        let ctod = Float64Array::from(chunk.iter().map(|t| t.ctod).collect::<Vec<_>>());
        let continuous = BooleanArray::from(chunk.iter().map(|t| t.continuous).collect::<Vec<_>>());

        let columns: Vec<ArrayRef> = vec![
            Arc::new(time),
            Arc::new(process),
            Arc::new(cpu),
            Arc::new(flags),
            Arc::new(action),
            Arc::new(devmajor),
            Arc::new(devminor),
            Arc::new(io_type),
            Arc::new(extra),
            Arc::new(sector),
            Arc::new(size),
            Arc::new(comm),
            Arc::new(qd),
            Arc::new(dtoc),
            Arc::new(ctoc),
            Arc::new(ctod),
            Arc::new(continuous),
        ];

        let batch = RecordBatch::try_new(schema.clone(), columns)?;
        writer.write(&batch)?;
    }

    writer.close()?;
    eprintln!("Block Parquet file saved successfully: {}", filepath);
    Ok(())
}

fn save_ufscustom_to_parquet_chunked(
    traces: &[UFSCUSTOM],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let total_chunks = traces.len().div_ceil(chunk_size); // 올림 계산
    eprintln!("Saving {} UFSCUSTOM traces to {} using chunk size {} ({} chunks)", 
              traces.len(), filepath, chunk_size, total_chunks);

    let schema = Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("opcode", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("lba", arrow::datatypes::DataType::UInt64, false),
        arrow::datatypes::Field::new("size", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("start_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("end_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("dtoc", arrow::datatypes::DataType::Float64, false),
    ]));

    let file = File::create(filepath)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 청크 단위로 데이터 처리
    for (chunk_idx, chunk) in traces.chunks(chunk_size).enumerate() {
        if chunk_idx % 10 == 0 || chunk_idx == total_chunks - 1 {
            eprintln!("Processing UFSCUSTOM chunk {}/{} ({} records)", 
                      chunk_idx + 1, total_chunks, chunk.len());
        }

        let opcode = StringArray::from(chunk.iter().map(|t| t.opcode.as_str()).collect::<Vec<_>>());
        let lba = UInt64Array::from(chunk.iter().map(|t| t.lba).collect::<Vec<_>>());
        let size = UInt32Array::from(chunk.iter().map(|t| t.size).collect::<Vec<_>>());
        let start_time = Float64Array::from(chunk.iter().map(|t| t.start_time).collect::<Vec<_>>());
        let end_time = Float64Array::from(chunk.iter().map(|t| t.end_time).collect::<Vec<_>>());
        let dtoc = Float64Array::from(chunk.iter().map(|t| t.dtoc).collect::<Vec<_>>());

        let columns: Vec<ArrayRef> = vec![
            Arc::new(opcode),
            Arc::new(lba),
            Arc::new(size),
            Arc::new(start_time),
            Arc::new(end_time),
            Arc::new(dtoc),
        ];

        let batch = RecordBatch::try_new(schema.clone(), columns)?;
        writer.write(&batch)?;
    }

    writer.close()?;
    eprintln!("UFSCUSTOM Parquet file saved successfully: {}", filepath);
    Ok(())
}


