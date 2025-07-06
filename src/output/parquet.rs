use crate::models::{Block, UFS, UFSCUSTOM};
use arrow::array::{ArrayRef, BooleanArray, Float64Array, StringArray, UInt32Array, UInt64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::file::properties::{WriterProperties, EnabledStatistics};
use parquet::basic::{Compression, Encoding};
use rayon::prelude::*;
use std::fs::File;
use std::sync::Arc;
use std::time::Instant;

pub fn save_to_parquet(
    ufs_traces: &[UFS],
    block_traces: &[Block],
    ufscustom_traces: &[UFSCUSTOM],
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();
    
    // 순차적으로 파일 저장 (스레드 안전성 문제 해결)
    if !ufs_traces.is_empty() {
        save_ufs_to_parquet(ufs_traces, &format!("{}_ufs.parquet", output_path), chunk_size)?;
    }

    if !block_traces.is_empty() {
        save_block_to_parquet(block_traces, &format!("{}_block.parquet", output_path), chunk_size)?;
    }

    if !ufscustom_traces.is_empty() {
        save_ufscustom_to_parquet(ufscustom_traces, &format!("{}_ufscustom.parquet", output_path), chunk_size)?;
    }

    println!("All Parquet files saved in {:.2}s", start_time.elapsed().as_secs_f64());
    Ok(())
}

// 최적화된 WriterProperties 생성
fn create_writer_properties() -> WriterProperties {
    WriterProperties::builder()
        .set_compression(Compression::SNAPPY)  // 빠른 압축
        .set_encoding(Encoding::PLAIN)         // 빠른 인코딩
        .set_dictionary_enabled(false)         // 딕셔너리 비활성화로 속도 향상
        .set_statistics_enabled(EnabledStatistics::None)  // 통계 비활성화로 속도 향상
        .set_max_row_group_size(1_000_000)    // 큰 로우 그룹으로 I/O 최적화
        .build()
}

// 최적화된 UFS Parquet 저장
fn save_ufs_to_parquet(
    traces: &[UFS], 
    filepath: &str, 
    chunk_size: usize
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let start_time = Instant::now();
    let total_chunks = traces.len().div_ceil(chunk_size);
    println!("Saving {} UFS traces to {} using optimized method ({} chunks)", 
              traces.len(), filepath, total_chunks);

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
    let props = create_writer_properties();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces.par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!("Processing UFS chunk {}/{} ({} records)", 
                          chunk_idx + 1, total_chunks, chunk.len());
            }

            // 사전 할당된 벡터로 메모리 할당 최적화
            let len = chunk.len();
            let mut time_vec = Vec::with_capacity(len);
            let mut process_vec = Vec::with_capacity(len);
            let mut cpu_vec = Vec::with_capacity(len);
            let mut action_vec = Vec::with_capacity(len);
            let mut tag_vec = Vec::with_capacity(len);
            let mut opcode_vec = Vec::with_capacity(len);
            let mut lba_vec = Vec::with_capacity(len);
            let mut size_vec = Vec::with_capacity(len);
            let mut groupid_vec = Vec::with_capacity(len);
            let mut hwqid_vec = Vec::with_capacity(len);
            let mut qd_vec = Vec::with_capacity(len);
            let mut dtoc_vec = Vec::with_capacity(len);
            let mut ctoc_vec = Vec::with_capacity(len);
            let mut ctod_vec = Vec::with_capacity(len);
            let mut continuous_vec = Vec::with_capacity(len);

            // 단일 루프로 모든 데이터 추출
            for t in chunk {
                time_vec.push(t.time);
                process_vec.push(t.process.as_str());
                cpu_vec.push(t.cpu);
                action_vec.push(t.action.as_str());
                tag_vec.push(t.tag);
                opcode_vec.push(t.opcode.as_str());
                lba_vec.push(t.lba);
                size_vec.push(t.size);
                groupid_vec.push(t.groupid);
                hwqid_vec.push(t.hwqid);
                qd_vec.push(t.qd);
                dtoc_vec.push(t.dtoc);
                ctoc_vec.push(t.ctoc);
                ctod_vec.push(t.ctod);
                continuous_vec.push(t.continuous);
            }

            let columns: Vec<ArrayRef> = vec![
                Arc::new(Float64Array::from(time_vec)),
                Arc::new(StringArray::from(process_vec)),
                Arc::new(UInt32Array::from(cpu_vec)),
                Arc::new(StringArray::from(action_vec)),
                Arc::new(UInt32Array::from(tag_vec)),
                Arc::new(StringArray::from(opcode_vec)),
                Arc::new(UInt64Array::from(lba_vec)),
                Arc::new(UInt32Array::from(size_vec)),
                Arc::new(UInt32Array::from(groupid_vec)),
                Arc::new(UInt32Array::from(hwqid_vec)),
                Arc::new(UInt32Array::from(qd_vec)),
                Arc::new(Float64Array::from(dtoc_vec)),
                Arc::new(Float64Array::from(ctoc_vec)),
                Arc::new(Float64Array::from(ctod_vec)),
                Arc::new(BooleanArray::from(continuous_vec)),
            ];

            RecordBatch::try_new(schema.clone(), columns)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 순차적으로 배치 작성 (파일 I/O는 순차적이어야 함)
    for batch in batches {
        writer.write(&batch)?;
    }

    writer.close()?;
    println!("UFS Parquet file saved in {:.2}s: {}", 
             start_time.elapsed().as_secs_f64(), filepath);
    Ok(())
}

// 최적화된 Block Parquet 저장  
fn save_block_to_parquet(
    traces: &[Block],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let start_time = Instant::now();
    let total_chunks = traces.len().div_ceil(chunk_size);
    println!("Saving {} Block traces to {} using optimized method ({} chunks)", 
              traces.len(), filepath, total_chunks);

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
    let props = create_writer_properties();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces.par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!("Processing Block chunk {}/{} ({} records)", 
                          chunk_idx + 1, total_chunks, chunk.len());
            }

            // 사전 할당된 벡터로 메모리 할당 최적화
            let len = chunk.len();
            let mut time_vec = Vec::with_capacity(len);
            let mut process_vec = Vec::with_capacity(len);
            let mut cpu_vec = Vec::with_capacity(len);
            let mut flags_vec = Vec::with_capacity(len);
            let mut action_vec = Vec::with_capacity(len);
            let mut devmajor_vec = Vec::with_capacity(len);
            let mut devminor_vec = Vec::with_capacity(len);
            let mut io_type_vec = Vec::with_capacity(len);
            let mut extra_vec = Vec::with_capacity(len);
            let mut sector_vec = Vec::with_capacity(len);
            let mut size_vec = Vec::with_capacity(len);
            let mut comm_vec = Vec::with_capacity(len);
            let mut qd_vec = Vec::with_capacity(len);
            let mut dtoc_vec = Vec::with_capacity(len);
            let mut ctoc_vec = Vec::with_capacity(len);
            let mut ctod_vec = Vec::with_capacity(len);
            let mut continuous_vec = Vec::with_capacity(len);

            // 단일 루프로 모든 데이터 추출
            for t in chunk {
                time_vec.push(t.time);
                process_vec.push(t.process.as_str());
                cpu_vec.push(t.cpu);
                flags_vec.push(t.flags.as_str());
                action_vec.push(t.action.as_str());
                devmajor_vec.push(t.devmajor);
                devminor_vec.push(t.devminor);
                io_type_vec.push(t.io_type.as_str());
                extra_vec.push(t.extra);
                sector_vec.push(t.sector);
                size_vec.push(t.size);
                comm_vec.push(t.comm.as_str());
                qd_vec.push(t.qd);
                dtoc_vec.push(t.dtoc);
                ctoc_vec.push(t.ctoc);
                ctod_vec.push(t.ctod);
                continuous_vec.push(t.continuous);
            }

            let columns: Vec<ArrayRef> = vec![
                Arc::new(Float64Array::from(time_vec)),
                Arc::new(StringArray::from(process_vec)),
                Arc::new(UInt32Array::from(cpu_vec)),
                Arc::new(StringArray::from(flags_vec)),
                Arc::new(StringArray::from(action_vec)),
                Arc::new(UInt32Array::from(devmajor_vec)),
                Arc::new(UInt32Array::from(devminor_vec)),
                Arc::new(StringArray::from(io_type_vec)),
                Arc::new(UInt32Array::from(extra_vec)),
                Arc::new(UInt64Array::from(sector_vec)),
                Arc::new(UInt32Array::from(size_vec)),
                Arc::new(StringArray::from(comm_vec)),
                Arc::new(UInt32Array::from(qd_vec)),
                Arc::new(Float64Array::from(dtoc_vec)),
                Arc::new(Float64Array::from(ctoc_vec)),
                Arc::new(Float64Array::from(ctod_vec)),
                Arc::new(BooleanArray::from(continuous_vec)),
            ];

            RecordBatch::try_new(schema.clone(), columns)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 순차적으로 배치 작성
    for batch in batches {
        writer.write(&batch)?;
    }

    writer.close()?;
    println!("Block Parquet file saved in {:.2}s: {}", 
             start_time.elapsed().as_secs_f64(), filepath);
    Ok(())
}

// 최적화된 UFSCUSTOM Parquet 저장
fn save_ufscustom_to_parquet(
    traces: &[UFSCUSTOM],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let start_time = Instant::now();
    let total_chunks = traces.len().div_ceil(chunk_size);
    println!("Saving {} UFSCUSTOM traces to {} using optimized method ({} chunks)", 
              traces.len(), filepath, total_chunks);

    let schema = Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("opcode", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("lba", arrow::datatypes::DataType::UInt64, false),
        arrow::datatypes::Field::new("size", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("start_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("end_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("dtoc", arrow::datatypes::DataType::Float64, false),
    ]));

    let file = File::create(filepath)?;
    let props = create_writer_properties();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces.par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!("Processing UFSCUSTOM chunk {}/{} ({} records)", 
                          chunk_idx + 1, total_chunks, chunk.len());
            }

            // 사전 할당된 벡터로 메모리 할당 최적화
            let len = chunk.len();
            let mut opcode_vec = Vec::with_capacity(len);
            let mut lba_vec = Vec::with_capacity(len);
            let mut size_vec = Vec::with_capacity(len);
            let mut start_time_vec = Vec::with_capacity(len);
            let mut end_time_vec = Vec::with_capacity(len);
            let mut dtoc_vec = Vec::with_capacity(len);

            // 단일 루프로 모든 데이터 추출
            for t in chunk {
                opcode_vec.push(t.opcode.as_str());
                lba_vec.push(t.lba);
                size_vec.push(t.size);
                start_time_vec.push(t.start_time);
                end_time_vec.push(t.end_time);
                dtoc_vec.push(t.dtoc);
            }

            let columns: Vec<ArrayRef> = vec![
                Arc::new(StringArray::from(opcode_vec)),
                Arc::new(UInt64Array::from(lba_vec)),
                Arc::new(UInt32Array::from(size_vec)),
                Arc::new(Float64Array::from(start_time_vec)),
                Arc::new(Float64Array::from(end_time_vec)),
                Arc::new(Float64Array::from(dtoc_vec)),
            ];

            RecordBatch::try_new(schema.clone(), columns)
        })
        .collect::<Result<Vec<_>, _>>()?;

    // 순차적으로 배치 작성
    for batch in batches {
        writer.write(&batch)?;
    }

    writer.close()?;
    println!("UFSCUSTOM Parquet file saved in {:.2}s: {}", 
             start_time.elapsed().as_secs_f64(), filepath);
    Ok(())
}


