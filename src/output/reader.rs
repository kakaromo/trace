use crate::models::{Block, UFS, UFSCUSTOM};
use arrow::array::AsArray;
use arrow::datatypes::{Float64Type, Schema, UInt32Type, UInt64Type};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use std::fs::File;

/// UFS Parquet 파일에서 데이터를 읽어 UFS 구조체 벡터로 반환
pub fn read_ufs_from_parquet(filepath: &str) -> Result<Vec<UFS>, Box<dyn std::error::Error>> {
    // 파일 열기
    let file = File::open(filepath)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;

    // 스키마 복제
    let schema = builder.schema().clone();

    // 레코드 배치 읽기
    let reader = builder.build()?;

    // UFS 구조체로 변환
    let mut results = Vec::new();
    for batch_result in reader {
        let batch = batch_result?;
        let records = convert_batch_to_ufs(&batch, &schema)?;
        results.extend(records);
    }

    Ok(results)
}

/// Block Parquet 파일에서 데이터를 읽어 Block 구조체 벡터로 반환
pub fn read_block_from_parquet(filepath: &str) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    // 파일 열기
    let file = File::open(filepath)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;

    // 스키마 복제
    let schema = builder.schema().clone();

    // 레코드 배치 읽기
    let reader = builder.build()?;

    // Block 구조체로 변환
    let mut results = Vec::new();
    for batch_result in reader {
        let batch = batch_result?;
        let records = convert_batch_to_block(&batch, &schema)?;
        results.extend(records);
    }

    Ok(results)
}

/// UFSCUSTOM Parquet 파일에서 데이터를 읽어 UFSCUSTOM 구조체 벡터로 반환
pub fn read_ufscustom_from_parquet(filepath: &str) -> Result<Vec<UFSCUSTOM>, Box<dyn std::error::Error>> {
    // 파일 열기
    let file = File::open(filepath)?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;

    // 스키마 복제
    let schema = builder.schema().clone();

    // 레코드 배치 읽기
    let reader = builder.build()?;

    // UFSCUSTOM 구조체로 변환
    let mut results = Vec::new();
    for batch_result in reader {
        let batch = batch_result?;
        let records = convert_batch_to_ufscustom(&batch, &schema)?;
        results.extend(records);
    }

    Ok(results)
}

/// RecordBatch를 UFS 구조체 벡터로 변환
fn convert_batch_to_ufs(
    batch: &arrow::record_batch::RecordBatch,
    schema: &Schema,
) -> Result<Vec<UFS>, Box<dyn std::error::Error>> {
    let num_rows = batch.num_rows();
    let mut result = Vec::with_capacity(num_rows);

    // 각 컬럼에서 데이터 추출 (현대적인 Arrow API 사용)
    let time_array = batch
        .column(schema.index_of("time")?)
        .as_primitive::<Float64Type>();
    let process_array = batch.column(schema.index_of("process")?).as_string::<i32>();
    let cpu_array = batch
        .column(schema.index_of("cpu")?)
        .as_primitive::<UInt32Type>();
    let action_array = batch.column(schema.index_of("action")?).as_string::<i32>();
    let tag_array = batch
        .column(schema.index_of("tag")?)
        .as_primitive::<UInt32Type>();
    let opcode_array = batch.column(schema.index_of("opcode")?).as_string::<i32>();
    let lba_array = batch
        .column(schema.index_of("lba")?)
        .as_primitive::<UInt64Type>();
    let size_array = batch
        .column(schema.index_of("size")?)
        .as_primitive::<UInt32Type>();
    let groupid_array = batch
        .column(schema.index_of("groupid")?)
        .as_primitive::<UInt32Type>();
    let hwqid_array = batch
        .column(schema.index_of("hwqid")?)
        .as_primitive::<UInt32Type>();
    let qd_array = batch
        .column(schema.index_of("qd")?)
        .as_primitive::<UInt32Type>();
    let dtoc_array = batch
        .column(schema.index_of("dtoc")?)
        .as_primitive::<Float64Type>();
    let ctoc_array = batch
        .column(schema.index_of("ctoc")?)
        .as_primitive::<Float64Type>();
    let ctod_array = batch
        .column(schema.index_of("ctod")?)
        .as_primitive::<Float64Type>();
    let continuous_array = batch.column(schema.index_of("continuous")?).as_boolean();

    // 각 행을 UFS 구조체로 변환
    for i in 0..num_rows {
        let ufs = UFS {
            time: time_array.value(i),
            process: process_array.value(i).to_string(),
            cpu: cpu_array.value(i),
            action: action_array.value(i).to_string(),
            tag: tag_array.value(i),
            opcode: opcode_array.value(i).to_string(),
            lba: lba_array.value(i),
            size: size_array.value(i),
            groupid: groupid_array.value(i),
            hwqid: hwqid_array.value(i),
            qd: qd_array.value(i),
            dtoc: dtoc_array.value(i),
            ctoc: ctoc_array.value(i),
            ctod: ctod_array.value(i),
            continuous: continuous_array.value(i),
        };
        result.push(ufs);
    }

    Ok(result)
}

/// RecordBatch를 Block 구조체 벡터로 변환
fn convert_batch_to_block(
    batch: &arrow::record_batch::RecordBatch,
    schema: &Schema,
) -> Result<Vec<Block>, Box<dyn std::error::Error>> {
    let num_rows = batch.num_rows();
    let mut result = Vec::with_capacity(num_rows);

    // 각 컬럼에서 데이터 추출 (현대적인 Arrow API 사용)
    let time_array = batch
        .column(schema.index_of("time")?)
        .as_primitive::<Float64Type>();
    let process_array = batch.column(schema.index_of("process")?).as_string::<i32>();
    let cpu_array = batch
        .column(schema.index_of("cpu")?)
        .as_primitive::<UInt32Type>();
    let flags_array = batch.column(schema.index_of("flags")?).as_string::<i32>();
    let action_array = batch.column(schema.index_of("action")?).as_string::<i32>();
    let devmajor_array = batch
        .column(schema.index_of("devmajor")?)
        .as_primitive::<UInt32Type>();
    let devminor_array = batch
        .column(schema.index_of("devminor")?)
        .as_primitive::<UInt32Type>();
    let io_type_array = batch.column(schema.index_of("io_type")?).as_string::<i32>();
    let extra_array = batch
        .column(schema.index_of("extra")?)
        .as_primitive::<UInt32Type>();
    let sector_array = batch
        .column(schema.index_of("sector")?)
        .as_primitive::<UInt64Type>();
    let size_array = batch
        .column(schema.index_of("size")?)
        .as_primitive::<UInt32Type>();
    let comm_array = batch.column(schema.index_of("comm")?).as_string::<i32>();
    let qd_array = batch
        .column(schema.index_of("qd")?)
        .as_primitive::<UInt32Type>();
    let dtoc_array = batch
        .column(schema.index_of("dtoc")?)
        .as_primitive::<Float64Type>();
    let ctoc_array = batch
        .column(schema.index_of("ctoc")?)
        .as_primitive::<Float64Type>();
    let ctod_array = batch
        .column(schema.index_of("ctod")?)
        .as_primitive::<Float64Type>();
    let continuous_array = batch.column(schema.index_of("continuous")?).as_boolean();

    // 각 행을 Block 구조체로 변환
    for i in 0..num_rows {
        let block = Block {
            time: time_array.value(i),
            process: process_array.value(i).to_string(),
            cpu: cpu_array.value(i),
            flags: flags_array.value(i).to_string(),
            action: action_array.value(i).to_string(),
            devmajor: devmajor_array.value(i),
            devminor: devminor_array.value(i),
            io_type: io_type_array.value(i).to_string(),
            extra: extra_array.value(i),
            sector: sector_array.value(i),
            size: size_array.value(i),
            comm: comm_array.value(i).to_string(),
            qd: qd_array.value(i),
            dtoc: dtoc_array.value(i),
            ctoc: ctoc_array.value(i),
            ctod: ctod_array.value(i),
            continuous: continuous_array.value(i),
        };
        result.push(block);
    }

    Ok(result)
}

/// RecordBatch를 UFSCUSTOM 구조체 벡터로 변환
fn convert_batch_to_ufscustom(
    batch: &arrow::record_batch::RecordBatch,
    schema: &Schema,
) -> Result<Vec<UFSCUSTOM>, Box<dyn std::error::Error>> {
    let num_rows = batch.num_rows();
    let mut result = Vec::with_capacity(num_rows);

    // 각 컬럼에서 데이터 추출
    let opcode_array = batch.column(schema.index_of("opcode")?).as_string::<i32>();
    let lba_array = batch
        .column(schema.index_of("lba")?)
        .as_primitive::<UInt64Type>();
    let size_array = batch
        .column(schema.index_of("size")?)
        .as_primitive::<UInt32Type>();
    let start_time_array = batch
        .column(schema.index_of("start_time")?)
        .as_primitive::<Float64Type>();
    let end_time_array = batch
        .column(schema.index_of("end_time")?)
        .as_primitive::<Float64Type>();
    let dtoc_array = batch
        .column(schema.index_of("dtoc")?)
        .as_primitive::<Float64Type>();

    // 각 행을 UFSCUSTOM 구조체로 변환
    for i in 0..num_rows {
        let ufscustom = UFSCUSTOM {
            opcode: opcode_array.value(i).to_string(),
            lba: lba_array.value(i),
            size: size_array.value(i),
            start_time: start_time_array.value(i),
            end_time: end_time_array.value(i),
            dtoc: dtoc_array.value(i),
        };
        result.push(ufscustom);
    }

    Ok(result)
}
