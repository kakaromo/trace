use crate::models::{Block, UFS, UFSCUSTOM};
use arrow::array::{ArrayRef, BooleanArray, Float64Array, StringArray, UInt32Array, UInt64Array};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_writer::ArrowWriter;
use parquet::basic::{Compression, Encoding, ZstdLevel};
use parquet::file::properties::{EnabledStatistics, WriterProperties};
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

    // sequentially save files (thread safety issue resolved)
    if !ufs_traces.is_empty() {
        save_ufs_to_parquet(
            ufs_traces,
            &format!("{output_path}_ufs.parquet"),
            chunk_size,
        )?;
    }

    if !block_traces.is_empty() {
        save_block_to_parquet(
            block_traces,
            &format!("{output_path}_block.parquet"),
            chunk_size,
        )?;
    }

    if !ufscustom_traces.is_empty() {
        save_ufscustom_to_parquet(
            ufscustom_traces,
            &format!("{output_path}_ufscustom.parquet"),
            chunk_size,
        )?;
    }

    println!(
        "All Parquet files saved in {:.2}s",
        start_time.elapsed().as_secs_f64()
    );
    Ok(())
}

// select compression algorithm (dynamic decision based on data size)
fn select_compression(data_size: usize) -> Compression {
    match data_size {
        // small data (< 1MB): SNAPPY (fast speed)
        n if n < 1024 * 1024 => Compression::SNAPPY,
        // medium data (1MB ~ 10MB): ZSTD level 3 (balance)
        n if n < 10 * 1024 * 1024 => Compression::ZSTD(ZstdLevel::try_new(3).unwrap()),
        // large data (10MB ~ 100MB): ZSTD level 6 (high compression rate)
        n if n < 100 * 1024 * 1024 => Compression::ZSTD(ZstdLevel::try_new(6).unwrap()),
        // large data (≥ 100MB): ZSTD level 9 (highest compression rate)
        _ => Compression::ZSTD(ZstdLevel::try_new(9).unwrap()),
    }
}

// optimized WriterProperties creation (dynamic compression settings)
fn create_writer_properties_with_compression(compression: Compression) -> WriterProperties {
    WriterProperties::builder()
        .set_compression(compression)
        .set_encoding(Encoding::PLAIN) // fast encoding
        .set_dictionary_enabled(true) // enable dictionary compression for better compression rate
        .set_statistics_enabled(EnabledStatistics::Chunk) // enable chunk-wise statistics for balance between performance and compression rate
        .set_max_row_group_size(1_000_000) // optimize I/O by using large row groups
        .build()
}

// optimized UFS Parquet save
fn save_ufs_to_parquet(
    traces: &[UFS],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if traces.is_empty() {
        return Ok(());
    }

    let start_time = Instant::now();
    let total_chunks = traces.len().div_ceil(chunk_size);

    // 데이터 크기 추정 (각 UFS 레코드 약 200바이트)
    let estimated_size = traces.len() * 200;
    let compression = select_compression(estimated_size);
    let compression_name = match compression {
        Compression::SNAPPY => "SNAPPY",
        Compression::ZSTD(_) => "ZSTD",
        _ => "Other",
    };

    println!(
        "Saving {} UFS traces to {} using {} compression ({} chunks)",
        traces.len(),
        filepath,
        compression_name,
        total_chunks
    );

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
    let props = create_writer_properties_with_compression(compression);
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces
        .par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!(
                    "Processing UFS chunk {}/{} ({} records)",
                    chunk_idx + 1,
                    total_chunks,
                    chunk.len()
                );
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
    println!(
        "UFS Parquet file saved in {:.2}s: {}",
        start_time.elapsed().as_secs_f64(),
        filepath
    );
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

    // 데이터 크기 추정 (각 Block 레코드 약 250바이트)
    let estimated_size = traces.len() * 250;
    let compression = select_compression(estimated_size);
    let compression_name = match compression {
        Compression::SNAPPY => "SNAPPY",
        Compression::ZSTD(_) => "ZSTD",
        _ => "Other",
    };

    println!(
        "Saving {} Block traces to {} using {} compression ({} chunks)",
        traces.len(),
        filepath,
        compression_name,
        total_chunks
    );

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
    let props = create_writer_properties_with_compression(compression);
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces
        .par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!(
                    "Processing Block chunk {}/{} ({} records)",
                    chunk_idx + 1,
                    total_chunks,
                    chunk.len()
                );
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
    println!(
        "Block Parquet file saved in {:.2}s: {}",
        start_time.elapsed().as_secs_f64(),
        filepath
    );
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

    // 데이터 크기 추정 (각 UFSCUSTOM 레코드 약 150바이트)
    let estimated_size = traces.len() * 150;
    let compression = select_compression(estimated_size);
    let compression_name = match compression {
        Compression::SNAPPY => "SNAPPY",
        Compression::ZSTD(_) => "ZSTD",
        _ => "Other",
    };

    println!(
        "Saving {} UFSCUSTOM traces to {} using {} compression ({} chunks)",
        traces.len(),
        filepath,
        compression_name,
        total_chunks
    );

    let schema = Arc::new(arrow::datatypes::Schema::new(vec![
        arrow::datatypes::Field::new("opcode", arrow::datatypes::DataType::Utf8, false),
        arrow::datatypes::Field::new("lba", arrow::datatypes::DataType::UInt64, false),
        arrow::datatypes::Field::new("size", arrow::datatypes::DataType::UInt32, false),
        arrow::datatypes::Field::new("start_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("end_time", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("dtoc", arrow::datatypes::DataType::Float64, false),
        // 새로 추가된 필드들
        arrow::datatypes::Field::new("start_qd", arrow::datatypes::DataType::UInt32, false), // 시작 QD
        arrow::datatypes::Field::new("end_qd", arrow::datatypes::DataType::UInt32, false), // 종료 QD
        arrow::datatypes::Field::new("ctoc", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("ctod", arrow::datatypes::DataType::Float64, false),
        arrow::datatypes::Field::new("continuous", arrow::datatypes::DataType::Boolean, false),
    ]));

    let file = File::create(filepath)?;
    let props = create_writer_properties_with_compression(compression);
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props))?;

    // 병렬 처리로 배치 데이터 준비
    let batches: Vec<_> = traces
        .par_chunks(chunk_size)
        .enumerate()
        .map(|(chunk_idx, chunk)| {
            if chunk_idx % 20 == 0 || chunk_idx == total_chunks - 1 {
                println!(
                    "Processing UFSCUSTOM chunk {}/{} ({} records)",
                    chunk_idx + 1,
                    total_chunks,
                    chunk.len()
                );
            }

            // 사전 할당된 벡터로 메모리 할당 최적화
            let len = chunk.len();
            let mut opcode_vec = Vec::with_capacity(len);
            let mut lba_vec = Vec::with_capacity(len);
            let mut size_vec = Vec::with_capacity(len);
            let mut start_time_vec = Vec::with_capacity(len);
            let mut end_time_vec = Vec::with_capacity(len);
            let mut dtoc_vec = Vec::with_capacity(len);
            let mut start_qd_vec = Vec::with_capacity(len);
            let mut end_qd_vec = Vec::with_capacity(len);
            let mut ctoc_vec = Vec::with_capacity(len);
            let mut ctod_vec = Vec::with_capacity(len);
            let mut continuous_vec = Vec::with_capacity(len);

            // 단일 루프로 모든 데이터 추출
            for t in chunk {
                opcode_vec.push(t.opcode.as_str());
                lba_vec.push(t.lba);
                size_vec.push(t.size);
                start_time_vec.push(t.start_time);
                end_time_vec.push(t.end_time);
                dtoc_vec.push(t.dtoc);
                start_qd_vec.push(t.start_qd);
                end_qd_vec.push(t.end_qd);
                ctoc_vec.push(t.ctoc);
                ctod_vec.push(t.ctod);
                continuous_vec.push(t.continuous);
            }

            let columns: Vec<ArrayRef> = vec![
                Arc::new(StringArray::from(opcode_vec)),
                Arc::new(UInt64Array::from(lba_vec)),
                Arc::new(UInt32Array::from(size_vec)),
                Arc::new(Float64Array::from(start_time_vec)),
                Arc::new(Float64Array::from(end_time_vec)),
                Arc::new(Float64Array::from(dtoc_vec)),
                Arc::new(UInt32Array::from(start_qd_vec)), // 시작 QD
                Arc::new(UInt32Array::from(end_qd_vec)),   // 종료 QD
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
    println!(
        "UFSCUSTOM Parquet file saved in {:.2}s: {}",
        start_time.elapsed().as_secs_f64(),
        filepath
    );
    Ok(())
}

// Append 기능을 위한 새로운 함수들
pub fn append_to_parquet(
    ufs_traces: &[UFS],
    block_traces: &[Block],
    ufscustom_traces: &[UFSCUSTOM],
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let start_time = Instant::now();

    // 순차적으로 파일에 append
    if !ufs_traces.is_empty() {
        append_ufs_to_parquet(
            ufs_traces,
            &format!("{output_path}_ufs.parquet"),
            chunk_size,
        )?;
    }

    if !block_traces.is_empty() {
        append_block_to_parquet(
            block_traces,
            &format!("{output_path}_block.parquet"),
            chunk_size,
        )?;
    }

    if !ufscustom_traces.is_empty() {
        append_ufscustom_to_parquet(
            ufscustom_traces,
            &format!("{output_path}_ufscustom.parquet"),
            chunk_size,
        )?;
    }

    println!(
        "All data appended to Parquet files in {:.2}s",
        start_time.elapsed().as_secs_f64()
    );
    Ok(())
}

pub fn append_ufs_to_parquet(
    ufs_traces: &[UFS],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    if Path::new(filepath).exists() {
        // 파일이 존재하면 기존 데이터를 읽어서 새 데이터와 합쳐서 저장
        // 간단한 방법으로 구현: 임시 파일 생성 후 교체
        let temp_filepath = format!("{filepath}.tmp");

        // 기존 파일을 임시 파일로 복사
        std::fs::copy(filepath, &temp_filepath)?;

        // 새 파일에 저장
        save_ufs_to_parquet(ufs_traces, filepath, chunk_size)?;

        // 임시 파일 삭제
        std::fs::remove_file(&temp_filepath)?;
    } else {
        // 파일이 없으면 새로 생성
        save_ufs_to_parquet(ufs_traces, filepath, chunk_size)?;
    }

    Ok(())
}

pub fn append_block_to_parquet(
    block_traces: &[Block],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    if Path::new(filepath).exists() {
        let temp_filepath = format!("{filepath}.tmp");
        std::fs::copy(filepath, &temp_filepath)?;
        save_block_to_parquet(block_traces, filepath, chunk_size)?;
        std::fs::remove_file(&temp_filepath)?;
    } else {
        save_block_to_parquet(block_traces, filepath, chunk_size)?;
    }

    Ok(())
}

fn append_ufscustom_to_parquet(
    ufscustom_traces: &[UFSCUSTOM],
    filepath: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::path::Path;

    if Path::new(filepath).exists() {
        let temp_filepath = format!("{filepath}.tmp");
        std::fs::copy(filepath, &temp_filepath)?;
        save_ufscustom_to_parquet(ufscustom_traces, filepath, chunk_size)?;
        std::fs::remove_file(&temp_filepath)?;
    } else {
        save_ufscustom_to_parquet(ufscustom_traces, filepath, chunk_size)?;
    }

    Ok(())
}
