use arrow::array::*;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::{arrow_reader::ParquetRecordBatchReaderBuilder, ArrowWriter};
use parquet::file::properties::WriterProperties;
use parquet::basic::{Compression, ZstdLevel};
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[allow(unused_imports)]
use crate::models::{Block, UFS, UFSCUSTOM};

/// Parquet 파일 마이그레이션 도구
pub struct ParquetMigrator {
    backup_enabled: bool,
}

impl ParquetMigrator {
    pub fn new(_chunk_size: usize, backup_enabled: bool) -> Self {
        Self {
            backup_enabled,
        }
    }

    /// 파일 마이그레이션 실행
    pub fn migrate_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        println!("Starting migration for: {}", file_path);

        if !Path::new(file_path).exists() {
            return Err(format!("File not found: {}", file_path).into());
        }

        // 백업 생성
        if self.backup_enabled {
            self.create_backup(file_path)?;
        }

        // 파일 타입 감지
        let file_type = self.detect_file_type(file_path)?;
        println!("Detected file type: {:?}", file_type);

        // 마이그레이션 실행
        match file_type {
            FileType::Block => self.migrate_block_file(file_path)?,
            FileType::UFS => self.migrate_ufs_file(file_path)?,
            FileType::UFSCustom => self.migrate_ufscustom_file(file_path)?,
            FileType::Unknown => {
                return Err("Unknown file type - unable to migrate".into());
            }
        }

        println!("Migration completed in {:.2}s", start_time.elapsed().as_secs_f64());
        Ok(())
    }

    /// 디렉토리 내 모든 Parquet 파일 마이그레이션
    pub fn migrate_directory(&self, dir_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        println!("Starting directory migration: {}", dir_path);

        let mut migrated_count = 0;
        let mut failed_count = 0;

        for entry in std::fs::read_dir(dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "parquet" {
                        let file_path = path.to_string_lossy();
                        println!("\nMigrating: {}", file_path);
                        
                        match self.migrate_file(&file_path) {
                            Ok(_) => {
                                migrated_count += 1;
                                println!("✓ Successfully migrated: {}", file_path);
                            },
                            Err(e) => {
                                failed_count += 1;
                                println!("✗ Failed to migrate {}: {}", file_path, e);
                            }
                        }
                    }
                }
            }
        }

        println!("\nDirectory migration completed in {:.2}s", start_time.elapsed().as_secs_f64());
        println!("Successfully migrated: {}", migrated_count);
        println!("Failed migrations: {}", failed_count);

        Ok(())
    }

    /// 백업 파일 생성
    fn create_backup(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let backup_path = format!("{}.backup", file_path);
        std::fs::copy(file_path, &backup_path)?;
        println!("Backup created: {}", backup_path);
        Ok(())
    }

    /// 파일 타입 감지
    fn detect_file_type(&self, file_path: &str) -> Result<FileType, Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
        let schema = reader.schema();

        // 스키마 기반 파일 타입 감지
        if self.is_block_schema(&schema) {
            Ok(FileType::Block)
        } else if self.is_ufs_schema(&schema) {
            Ok(FileType::UFS)
        } else if self.is_ufscustom_schema(&schema) {
            Ok(FileType::UFSCustom)
        } else {
            Ok(FileType::Unknown)
        }
    }

    /// Block 스키마 검증
    fn is_block_schema(&self, schema: &Schema) -> bool {
        let _expected_fields = vec![
            "time", "process", "cpu", "flags", "action", "devmajor", "devminor",
            "io_type", "extra", "sector", "size", "comm", "qd", "dtoc", "ctoc", "ctod", "continuous"
        ];

        // 기본 필드들이 존재하는지 확인
        let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        
        // 최소 필수 필드 확인
        let required_fields = ["time", "process", "action", "sector"];
        required_fields.iter().all(|&field| field_names.contains(&field))
    }

    /// UFS 스키마 검증
    fn is_ufs_schema(&self, schema: &Schema) -> bool {
        let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        let required_fields = ["time", "process", "action", "opcode", "lba"];
        required_fields.iter().all(|&field| field_names.contains(&field))
    }

    /// UFSCustom 스키마 검증
    fn is_ufscustom_schema(&self, schema: &Schema) -> bool {
        let field_names: Vec<&str> = schema.fields().iter().map(|f| f.name().as_str()).collect();
        let required_fields = ["time", "process", "action", "opcode", "lba"];
        required_fields.iter().all(|&field| field_names.contains(&field))
    }

    /// Block 파일 마이그레이션
    fn migrate_block_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader_builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let current_schema = reader_builder.schema().clone();
        let target_schema = self.get_block_target_schema();

        // 스키마 호환성 확인
        if self.schemas_compatible(&current_schema, &target_schema) {
            println!("Schema is already up to date");
            return Ok(());
        }

        println!("Migrating Block file schema...");
        
        // 임시 파일 생성
        let temp_path = format!("{}.tmp", file_path);
        let temp_file = File::create(&temp_path)?;
        
        // 파일 크기 추정 (평균적으로 Block 레코드 약 250바이트)
        let estimated_size = std::fs::metadata(file_path)?.len() as usize;
        let compression = self.select_compression(estimated_size);
        let compression_name = match compression {
            Compression::SNAPPY => "SNAPPY",
            Compression::ZSTD(_) => "ZSTD",
            _ => "Other",
        };
        println!("Using {} compression for migration", compression_name);
        
        let props = WriterProperties::builder()
            .set_compression(compression)
            .build();
        let mut writer = ArrowWriter::try_new(temp_file, target_schema.clone(), Some(props))?;

        // 배치 단위로 데이터 변환
        let reader = reader_builder.build()?;
        let mut total_records = 0;

        for batch in reader {
            match batch {
                Ok(batch) => {
                    let converted_batch = self.convert_block_batch(&batch, &current_schema, &target_schema)?;
                    writer.write(&converted_batch)?;
                    total_records += converted_batch.num_rows();
                    
                    if total_records % 100000 == 0 {
                        println!("Processed {} records", total_records);
                    }
                },
                Err(e) => {
                    println!("Error reading batch: {}", e);
                    break;
                }
            }
        }

        writer.close()?;
        
        // 원본 파일 교체
        std::fs::rename(&temp_path, file_path)?;
        println!("Successfully migrated {} records", total_records);
        
        Ok(())
    }

    /// UFS 파일 마이그레이션
    fn migrate_ufs_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(file_path)?;
        let reader_builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let current_schema = reader_builder.schema().clone();
        let target_schema = self.get_ufs_target_schema();

        if self.schemas_compatible(&current_schema, &target_schema) {
            println!("Schema is already up to date");
            return Ok(());
        }

        println!("Migrating UFS file schema...");
        
        let temp_path = format!("{}.tmp", file_path);
        let temp_file = File::create(&temp_path)?;
        
        // 파일 크기 추정
        let estimated_size = std::fs::metadata(file_path)?.len() as usize;
        let compression = self.select_compression(estimated_size);
        let compression_name = match compression {
            Compression::SNAPPY => "SNAPPY",
            Compression::ZSTD(_) => "ZSTD",
            _ => "Other",
        };
        println!("Using {} compression for UFS migration", compression_name);
        
        let props = WriterProperties::builder()
            .set_compression(compression)
            .build();
        let mut writer = ArrowWriter::try_new(temp_file, target_schema.clone(), Some(props))?;

        let reader = reader_builder.build()?;
        let mut total_records = 0;

        for batch in reader {
            match batch {
                Ok(batch) => {
                    let converted_batch = self.convert_ufs_batch(&batch, &current_schema, &target_schema)?;
                    writer.write(&converted_batch)?;
                    total_records += converted_batch.num_rows();
                    
                    if total_records % 100000 == 0 {
                        println!("Processed {} records", total_records);
                    }
                },
                Err(e) => {
                    println!("Error reading batch: {}", e);
                    break;
                }
            }
        }

        writer.close()?;
        std::fs::rename(&temp_path, file_path)?;
        println!("Successfully migrated {} records", total_records);
        
        Ok(())
    }

    /// UFSCustom 파일 마이그레이션
    fn migrate_ufscustom_file(&self, _file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        // UFSCustom은 UFS와 유사한 구조이므로 비슷한 로직 적용
        println!("UFSCustom migration not implemented yet");
        Ok(())
    }

    /// Block 타겟 스키마 생성
    fn get_block_target_schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("time", DataType::Float64, false),
            Field::new("process", DataType::Utf8, false),
            Field::new("cpu", DataType::UInt32, false),
            Field::new("flags", DataType::Utf8, false),
            Field::new("action", DataType::Utf8, false),
            Field::new("devmajor", DataType::UInt32, false),
            Field::new("devminor", DataType::UInt32, false),
            Field::new("io_type", DataType::Utf8, false),
            Field::new("extra", DataType::UInt32, false),
            Field::new("sector", DataType::UInt64, false),
            Field::new("size", DataType::UInt32, false),
            Field::new("comm", DataType::Utf8, false),
            Field::new("qd", DataType::UInt32, false),
            Field::new("dtoc", DataType::Float64, false),
            Field::new("ctoc", DataType::Float64, false),
            Field::new("ctod", DataType::Float64, false),
            Field::new("continuous", DataType::Boolean, false),
        ]))
    }

    /// UFS 타겟 스키마 생성
    fn get_ufs_target_schema(&self) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("time", DataType::Float64, false),
            Field::new("process", DataType::Utf8, false),
            Field::new("cpu", DataType::UInt32, false),
            Field::new("action", DataType::Utf8, false),
            Field::new("tag", DataType::UInt32, false),
            Field::new("opcode", DataType::Utf8, false),
            Field::new("lba", DataType::UInt64, false),
            Field::new("size", DataType::UInt32, false),
            Field::new("groupid", DataType::UInt32, false),
            Field::new("hwqid", DataType::UInt32, false),
            Field::new("qd", DataType::UInt32, false),
            Field::new("dtoc", DataType::Float64, false),
            Field::new("ctoc", DataType::Float64, false),
            Field::new("ctod", DataType::Float64, false),
            Field::new("continuous", DataType::Boolean, false),
        ]))
    }

    /// 스키마 호환성 확인
    fn schemas_compatible(&self, current: &Schema, target: &Schema) -> bool {
        if current.fields().len() != target.fields().len() {
            return false;
        }

        for (current_field, target_field) in current.fields().iter().zip(target.fields().iter()) {
            if current_field.name() != target_field.name() ||
               current_field.data_type() != target_field.data_type() {
                return false;
            }
        }

        true
    }

    /// Block 배치 변환
    fn convert_block_batch(
        &self,
        batch: &RecordBatch,
        current_schema: &Schema,
        target_schema: &Schema,
    ) -> Result<RecordBatch, Box<dyn std::error::Error>> {
        let mut columns: Vec<ArrayRef> = Vec::new();
        
        // 타겟 스키마의 각 필드에 대해 데이터 변환
        for target_field in target_schema.fields() {
            let field_name = target_field.name();
            
            if let Some(column_index) = current_schema.fields()
                .iter()
                .position(|f| f.name() == field_name) {
                // 기존 필드가 존재하는 경우 그대로 사용
                columns.push(batch.column(column_index).clone());
            } else {
                // 새로운 필드인 경우 기본값으로 채움
                let array = self.create_default_array(target_field.data_type(), batch.num_rows())?;
                columns.push(array);
            }
        }

        Ok(RecordBatch::try_new(Arc::new(target_schema.clone()), columns)?)
    }

    /// UFS 배치 변환
    fn convert_ufs_batch(
        &self,
        batch: &RecordBatch,
        current_schema: &Schema,
        target_schema: &Schema,
    ) -> Result<RecordBatch, Box<dyn std::error::Error>> {
        let mut columns: Vec<ArrayRef> = Vec::new();
        
        for target_field in target_schema.fields() {
            let field_name = target_field.name();
            
            if let Some(column_index) = current_schema.fields()
                .iter()
                .position(|f| f.name() == field_name) {
                columns.push(batch.column(column_index).clone());
            } else {
                let array = self.create_default_array(target_field.data_type(), batch.num_rows())?;
                columns.push(array);
            }
        }

        Ok(RecordBatch::try_new(Arc::new(target_schema.clone()), columns)?)
    }

    /// 기본값 배열 생성
    fn create_default_array(&self, data_type: &DataType, length: usize) -> Result<ArrayRef, Box<dyn std::error::Error>> {
        match data_type {
            DataType::Float64 => {
                let values = vec![0.0_f64; length];
                Ok(Arc::new(Float64Array::from(values)))
            },
            DataType::UInt32 => {
                let values = vec![0_u32; length];
                Ok(Arc::new(UInt32Array::from(values)))
            },
            DataType::UInt64 => {
                let values = vec![0_u64; length];
                Ok(Arc::new(UInt64Array::from(values)))
            },
            DataType::Utf8 => {
                let values = vec![""; length];
                Ok(Arc::new(StringArray::from(values)))
            },
            DataType::Boolean => {
                let values = vec![false; length];
                Ok(Arc::new(BooleanArray::from(values)))
            },
            _ => Err(format!("Unsupported data type: {:?}", data_type).into())
        }
    }

    /// 압축 알고리즘 선택 (데이터 크기에 따라 동적 결정)
    fn select_compression(&self, estimated_size: usize) -> Compression {
        match estimated_size {
            // 소형 데이터 (< 1MB): SNAPPY (빠른 속도)
            n if n < 1024 * 1024 => Compression::SNAPPY,
            // 중형 데이터 (1MB ~ 10MB): ZSTD 레벨 3 (균형)
            n if n < 10 * 1024 * 1024 => Compression::ZSTD(ZstdLevel::try_new(3).unwrap()),
            // 대형 데이터 (10MB ~ 100MB): ZSTD 레벨 6 (높은 압축률)
            n if n < 100 * 1024 * 1024 => Compression::ZSTD(ZstdLevel::try_new(6).unwrap()),
            // 초대형 데이터 (≥ 100MB): ZSTD 레벨 9 (최고 압축률)
            _ => Compression::ZSTD(ZstdLevel::try_new(9).unwrap()),
        }
    }
}
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
enum FileType {
    Block,
    UFS,
    UFSCustom,
    Unknown,
}

/// 마이그레이션 명령어 실행
pub fn run_migration(
    input_path: &str,
    chunk_size: Option<usize>,
    backup_enabled: bool,
    recursive: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let chunk_size = chunk_size.unwrap_or(10000);
    let migrator = ParquetMigrator::new(chunk_size, backup_enabled);

    let path = Path::new(input_path);
    
    if path.is_file() {
        // 단일 파일 마이그레이션
        migrator.migrate_file(input_path)?;
    } else if path.is_dir() {
        // 디렉토리 마이그레이션
        if recursive {
            migrate_directory_recursive(&migrator, input_path)?;
        } else {
            migrator.migrate_directory(input_path)?;
        }
    } else {
        return Err("Invalid path: not a file or directory".into());
    }

    Ok(())
}

/// 재귀적 디렉토리 마이그레이션
fn migrate_directory_recursive(
    migrator: &ParquetMigrator,
    dir_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            migrate_directory_recursive(migrator, &path.to_string_lossy())?;
        } else if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "parquet" {
                    let file_path = path.to_string_lossy();
                    match migrator.migrate_file(&file_path) {
                        Ok(_) => println!("✓ Successfully migrated: {}", file_path),
                        Err(e) => println!("✗ Failed to migrate {}: {}", file_path, e),
                    }
                }
            }
        }
    }
    Ok(())
}
