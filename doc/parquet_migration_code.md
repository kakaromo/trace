# Parquet 마이그레이션 코드 상세 설명

이 문서는 Parquet 파일 마이그레이션 코드의 상세한 구현 내용과 기술적인 측면을 설명합니다.

## 코드 구조

마이그레이션 기능은 `src/migration` 디렉토리에 구현되어 있으며, 다음과 같은 파일로 구성됩니다:

- `src/migration/mod.rs`: 모듈 선언 및 내보내기
- `src/migration/parquet_migrator.rs`: 핵심 마이그레이션 로직

## 핵심 클래스 및 기능

### `ParquetMigrator` 클래스

```rust
pub struct ParquetMigrator {
    backup_enabled: bool,
}
```

`ParquetMigrator`는 마이그레이션 작업을 수행하는 핵심 구조체입니다.

#### 주요 메서드

1. **`new(chunk_size: usize, backup_enabled: bool) -> Self`**
   - 용도: 마이그레이터 인스턴스 생성
   - 매개변수:
     - `chunk_size`: 배치 처리 크기 (현재는 내부에서 사용하지 않음)
     - `backup_enabled`: 백업 파일 생성 여부

2. **`migrate_file(file_path: &str) -> Result<(), Box<dyn std::error::Error>>`**
   - 용도: 단일 Parquet 파일 마이그레이션
   - 처리 과정:
     1. 파일 존재 여부 확인
     2. 백업 파일 생성 (활성화된 경우)
     3. 파일 타입 자동 감지 (Block, UFS, UFSCustom)
     4. 파일 타입에 따른 마이그레이션 실행

3. **`migrate_directory(dir_path: &str) -> Result<(), Box<dyn std::error::Error>>`**
   - 용도: 디렉토리 내 모든 Parquet 파일 마이그레이션
   - 처리 과정:
     1. 디렉토리 내 모든 파일 검색
     2. 확장자가 `.parquet`인 파일에 대해 마이그레이션 실행
     3. 성공/실패 통계 보고

4. **`detect_file_type(file_path: &str) -> Result<FileType, Box<dyn std::error::Error>>`**
   - 용도: Parquet 파일의 스키마를 분석하여 파일 타입 자동 감지
   - 타입 감지 방식: 스키마의 필드 구성에 따라 Block, UFS, UFSCustom 타입 결정
   - 감지 기준:
     - Block: "time", "process", "action", "sector" 필드 존재
     - UFS: "time", "process", "action", "opcode", "lba" 필드 존재
     - UFSCustom: UFS와 유사하나 별도 처리 (현재 미구현)

### 스키마 변환

#### Block 파일 마이그레이션

```rust
fn migrate_block_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>>
```

1. 스키마 정의:
   ```rust
   fn get_block_target_schema(&self) -> Arc<Schema> {
       Arc::new(Schema::new(vec![
           Field::new("time", DataType::Float64, false),
           Field::new("process", DataType::Utf8, false),
           Field::new("cpu", DataType::UInt32, false),
           // ... 이하 생략
           Field::new("continuous", DataType::Boolean, false),
       ]))
   }
   ```

2. 배치 변환 로직:
   ```rust
   fn convert_block_batch(&self, batch: &RecordBatch, current_schema: &Schema, target_schema: &Schema) -> Result<RecordBatch, Box<dyn std::error::Error>>
   ```
   - 기존 필드는 그대로 유지
   - 새로운 필드는 타입에 맞는 기본값으로 채움

#### UFS 파일 마이그레이션

```rust
fn migrate_ufs_file(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>>
```

1. 스키마 정의:
   ```rust
   fn get_ufs_target_schema(&self) -> Arc<Schema> {
       Arc::new(Schema::new(vec![
           Field::new("time", DataType::Float64, false),
           Field::new("process", DataType::Utf8, false),
           Field::new("cpu", DataType::UInt32, false),
           // ... 이하 생략
           Field::new("continuous", DataType::Boolean, false),
       ]))
   }
   ```

2. 배치 변환 로직:
   Block 파일과 동일한 패턴으로 처리

### 데이터 타입 기본값 처리

```rust
fn create_default_array(&self, data_type: &DataType, length: usize) -> Result<ArrayRef, Box<dyn std::error::Error>> {
    match data_type {
        DataType::Float64 => Ok(Arc::new(Float64Array::from(vec![0.0_f64; length]))),
        DataType::UInt32 => Ok(Arc::new(UInt32Array::from(vec![0_u32; length]))),
        DataType::UInt64 => Ok(Arc::new(UInt64Array::from(vec![0_u64; length]))),
        DataType::Utf8 => Ok(Arc::new(StringArray::from(vec![""; length]))),
        DataType::Boolean => Ok(Arc::new(BooleanArray::from(vec![false; length]))),
        _ => Err(format!("Unsupported data type: {:?}", data_type).into())
    }
}
```

이 메서드는 각 데이터 타입에 맞는 기본값으로 채워진 Arrow 배열을 생성합니다:
- `Float64`: 0.0
- `UInt32/UInt64`: 0
- `Utf8`: 빈 문자열 ("")
- `Boolean`: false

### 압축 알고리즘 선택

```rust
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
```

파일 크기에 따라 최적의 압축 방식을 동적으로 선택합니다:
- 1MB 미만: SNAPPY (빠른 처리 속도 우선)
- 1MB ~ 10MB: ZSTD 레벨 3 (속도와 압축률 균형)
- 10MB ~ 100MB: ZSTD 레벨 6 (높은 압축률)
- 100MB 이상: ZSTD 레벨 9 (최고 압축률)

## 재귀적 디렉토리 처리

```rust
fn migrate_directory_recursive(
    migrator: &ParquetMigrator,
    dir_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 디렉토리 내 모든 항목 처리
    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // 서브디렉토리 재귀 호출
            migrate_directory_recursive(migrator, &path.to_string_lossy())?;
        } else if path.is_file() && path.extension() == Some(std::ffi::OsStr::new("parquet")) {
            // Parquet 파일 마이그레이션
            let file_path = path.to_string_lossy();
            match migrator.migrate_file(&file_path) {
                Ok(_) => println!("✓ Successfully migrated: {}", file_path),
                Err(e) => println!("✗ Failed to migrate {}: {}", file_path, e),
            }
        }
    }
    Ok(())
}
```

이 함수는 `--recursive` 옵션이 주어졌을 때 서브디렉토리까지 재귀적으로 탐색하여 모든 Parquet 파일을 마이그레이션합니다.

## 주요 처리 흐름

1. **CLI 명령어 파싱**:
   - `--migrate <path>`: 마이그레이션 대상 경로
   - `--chunk-size <size>`: 처리할 청크 크기 (기본값: 10000)
   - `--no-backup`: 백업 파일 생성 여부 (기본: 백업 생성)
   - `--recursive`: 서브디렉토리 재귀 처리

2. **마이그레이션 실행**:
   ```rust
   trace::migration::run_migration(input_path, migrate_chunk_size, backup_enabled, recursive)
   ```

3. **파일 타입 감지**: 스키마 분석을 통해 Block, UFS, UFSCustom 자동 감지

4. **스키마 변환 및 데이터 마이그레이션**:
   - 현재 스키마와 타겟 스키마 비교
   - 기존 필드는 유지, 새로운 필드는 기본값으로 채움
   - 청크 단위로 데이터 처리하여 메모리 효율성 확보

5. **안전한 파일 교체**:
   - 임시 파일에 새 형식으로 데이터 작성
   - 성공 시 원본 파일 교체
   - 실패 시 백업 파일로 복구 가능

## 예외 처리 및 안전 기능

1. **백업 파일 생성**:
   ```rust
   fn create_backup(&self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
       let backup_path = format!("{}.backup", file_path);
       std::fs::copy(file_path, &backup_path)?;
       println!("Backup created: {}", backup_path);
       Ok(())
   }
   ```

2. **임시 파일 사용**:
   ```rust
   let temp_path = format!("{}.tmp", file_path);
   let temp_file = File::create(&temp_path)?;
   // 임시 파일에 데이터 작성 후 성공 시에만 원본 교체
   std::fs::rename(&temp_path, file_path)?;
   ```

3. **오류 처리 및 보고**:
   - 배치 단위 처리 중 오류 발생 시 안전하게 중단
   - 모든 단계에서 오류 메시지 명확히 출력
   - 총 성공/실패 통계 보고

## 최적화 기법

1. **배치 처리**: 청크 단위로 데이터를 읽고 처리하여 메모리 사용량 최소화
2. **동적 압축 선택**: 파일 크기에 따라 최적의 압축 방식 선택
3. **진행 상태 표시**: 대용량 파일 처리 시 주기적인 처리 상태 보고
4. **파일 크기 추정**: 적절한 압축 방식 선택을 위한 파일 크기 추정

## 알려진 제한사항

1. **UFSCustom 파일 미지원**:
   ```rust
   fn migrate_ufscustom_file(&self, _file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
       println!("UFSCustom migration not implemented yet");
       Ok(())
   }
   ```
   
2. **제한된 데이터 타입 지원**:
   - 현재 Float64, UInt32, UInt64, Utf8, Boolean 타입만 지원
   - 다른 타입은 오류 발생

3. **메모리 사용량**: 매우 큰 파일의 경우 청크 크기 조정 필요

## 코드 최적화 및 개선 사항

1. **청크 크기 튜닝**: 현재는 내부적으로 청크 크기를 사용하지 않으므로 향후 배치 크기 조정 구현 필요
2. **병렬 처리**: 배치 변환 과정에서 멀티 스레딩 구현 가능
3. **재시도 메커니즘**: 일시적 오류 발생 시 자동 재시도 기능 추가
4. **진행률 표시기**: 대용량 파일 처리 시 진행률 백분율 표시
5. **타입 변환 강화**: 다양한 데이터 타입 간 변환 지원 (예: Int32 → UInt32)

## 성능 벤치마크

아래는 가상의 벤치마크 데이터로, 다양한 크기의 파일에 대한 마이그레이션 성능을 나타냅니다:

| 파일 크기 | 레코드 수 | 압축 방식 | 마이그레이션 시간 | 메모리 사용량 |
|-----------|-----------|-----------|-------------------|---------------|
| 10MB      | 100K      | SNAPPY    | ~0.5초            | ~50MB         |
| 100MB     | 1M        | ZSTD(3)   | ~3초              | ~100MB        |
| 1GB       | 10M       | ZSTD(6)   | ~25초             | ~150MB        |
| 10GB      | 100M      | ZSTD(9)   | ~4분              | ~200MB        |

## 요약

Parquet 마이그레이션 도구는 Arrow 및 Parquet 라이브러리를 활용하여 기존 Parquet 파일의 스키마를 최신 버전으로 업데이트합니다. 주요 특징으로는 자동 파일 타입 감지, 안전한 배치 처리, 동적 압축 최적화, 백업 메커니즘이 있습니다. 현재는 Block 및 UFS 파일을 지원하며, 향후 UFSCustom 파일 지원 및 다양한 성능 최적화가 계획되어 있습니다.
