use std::fs::File;
use std::sync::Arc;
use arrow::array::{ArrayRef, BooleanArray, Float64Array, StringArray, UInt32Array, UInt64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::WriterProperties;
use crate::models::{UFS, Block};

pub fn save_to_parquet(ufs_traces: &[UFS], block_traces: &[Block], output_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // UFS 데이터 저장
    save_ufs_to_parquet(ufs_traces, &format!("{}_ufs.parquet", output_path))?;
    
    // Block 데이터 저장
    save_block_to_parquet(block_traces, &format!("{}_block.parquet", output_path))?;
    
    Ok(())
}

fn save_ufs_to_parquet(traces: &[UFS], filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let time = Float64Array::from(traces.iter().map(|t| t.time).collect::<Vec<_>>());
    let process = StringArray::from(traces.iter().map(|t| t.process.as_str()).collect::<Vec<_>>());
    let cpu = UInt32Array::from(traces.iter().map(|t| t.cpu).collect::<Vec<_>>());
    let action = StringArray::from(traces.iter().map(|t| t.action.as_str()).collect::<Vec<_>>());
    let tag = UInt32Array::from(traces.iter().map(|t| t.tag).collect::<Vec<_>>());
    let opcode = StringArray::from(traces.iter().map(|t| t.opcode.as_str()).collect::<Vec<_>>());
    let lba = UInt64Array::from(traces.iter().map(|t| t.lba).collect::<Vec<_>>());
    let size = UInt32Array::from(traces.iter().map(|t| t.size).collect::<Vec<_>>());
    let groupid = UInt32Array::from(traces.iter().map(|t| t.groupid).collect::<Vec<_>>());
    let hwqid = UInt32Array::from(traces.iter().map(|t| t.hwqid).collect::<Vec<_>>());
    let qd = UInt32Array::from(traces.iter().map(|t| t.qd).collect::<Vec<_>>());
    let dtoc = Float64Array::from(traces.iter().map(|t| t.dtoc).collect::<Vec<_>>());
    let ctoc = Float64Array::from(traces.iter().map(|t| t.ctoc).collect::<Vec<_>>());
    let ctod = Float64Array::from(traces.iter().map(|t| t.ctod).collect::<Vec<_>>());
    let continuous = BooleanArray::from(traces.iter().map(|t| t.continuous).collect::<Vec<_>>());

    let schema = arrow::datatypes::Schema::new(vec![
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
    ]);

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

    let batch = RecordBatch::try_new(Arc::new(schema), columns)?;
    let file = File::create(filepath)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
    
    writer.write(&batch)?;
    writer.close()?;
    
    Ok(())
}

fn save_block_to_parquet(traces: &[Block], filepath: &str) -> Result<(), Box<dyn std::error::Error>> {
    let time = Float64Array::from(traces.iter().map(|t| t.time).collect::<Vec<_>>());
    let process = StringArray::from(traces.iter().map(|t| t.process.as_str()).collect::<Vec<_>>());
    let cpu = UInt32Array::from(traces.iter().map(|t| t.cpu).collect::<Vec<_>>());
    let flags = StringArray::from(traces.iter().map(|t| t.flags.as_str()).collect::<Vec<_>>());
    let action = StringArray::from(traces.iter().map(|t| t.action.as_str()).collect::<Vec<_>>());
    let devmajor = UInt32Array::from(traces.iter().map(|t| t.devmajor).collect::<Vec<_>>());
    let devminor = UInt32Array::from(traces.iter().map(|t| t.devminor).collect::<Vec<_>>());
    let io_type = StringArray::from(traces.iter().map(|t| t.io_type.as_str()).collect::<Vec<_>>());
    let extra = UInt32Array::from(traces.iter().map(|t| t.extra).collect::<Vec<_>>());
    let sector = UInt64Array::from(traces.iter().map(|t| t.sector).collect::<Vec<_>>());
    let size = UInt32Array::from(traces.iter().map(|t| t.size).collect::<Vec<_>>());
    let comm = StringArray::from(traces.iter().map(|t| t.comm.as_str()).collect::<Vec<_>>());
    let qd = UInt32Array::from(traces.iter().map(|t| t.qd).collect::<Vec<_>>());
    let dtoc = Float64Array::from(traces.iter().map(|t| t.dtoc).collect::<Vec<_>>());
    let ctoc = Float64Array::from(traces.iter().map(|t| t.ctoc).collect::<Vec<_>>());
    let ctod = Float64Array::from(traces.iter().map(|t| t.ctod).collect::<Vec<_>>());
    let continuous = BooleanArray::from(traces.iter().map(|t| t.continuous).collect::<Vec<_>>());

    let schema = arrow::datatypes::Schema::new(vec![
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
    ]);

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

    let batch = RecordBatch::try_new(Arc::new(schema), columns)?;
    let file = File::create(filepath)?;
    let props = WriterProperties::builder().build();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))?;
    
    writer.write(&batch)?;
    writer.close()?;
    
    Ok(())
}