# Parquet 마이그레이션 기술 요약

## 기술 개요

Trace 애플리케이션의 Parquet 마이그레이션 기능은 이전 버전의 스키마로 저장된 Parquet 파일을 최신 스키마 버전으로 변환하는 기능을 제공합니다. 이 기능은 스키마 변경으로 인해 이전 버전의 파일을 읽을 수 없는 문제를 해결합니다.

## 기술 스택

- **언어**: Rust
- **주요 라이브러리**:
  - Arrow: 메모리 내 데이터 표현 및 처리
  - Parquet: 파일 읽기/쓰기 및 압축

## 핵심 아키텍처

### 구성 요소

```
+------------------+     +------------------+     +-------------------+
| Parquet File     | --> | ParquetMigrator  | --> | Migrated          |
| (Old Schema)     |     | - Detect Type    |     | Parquet File      |
|                  |     | - Convert Schema |     | (New Schema)      |
+------------------+     +------------------+     +-------------------+
          |                       |
          v                       v
+------------------+     +------------------+
| Backup File      |     | Error Handling   |
| (.backup)        |     | - Restore Backup |
|                  |     | - Clean Tmp Files|
+------------------+     +------------------+
```

### 주요 처리 과정

1. **타입 감지**: 스키마 분석을 통한 파일 유형 자동 감지
2. **스키마 매핑**: 기존 필드 유지 및 새 필드 기본값으로 초기화
3. **배치 처리**: 청크 단위 데이터 변환으로 메모리 효율성 확보
4. **안전한 파일 교체**: 임시 파일 사용 및 백업을 통한 안전한 교체

## 기술적 특징

### 1. 자동 타입 감지 알고리즘

```rust
fn detect_file_type(&self, file_path: &str) -> Result<FileType, Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)?.build()?;
    let schema = reader.schema();

    // 스키마에서 필수 필드 존재 여부로 타입 판별
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
```

### 2. 스마트 압축 선택 알고리즘

파일 크기에 따라 최적의 압축 방식을 동적으로 선택:

- 소형 파일: SNAPPY (빠른 속도 우선)
- 중형/대형 파일: ZSTD (다양한 레벨로 압축률과 속도 균형)

### 3. 스키마 호환성 검사

```rust
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
```

## 성능 최적화

1. **메모리 효율성**:
   - 배치 단위 처리로 대용량 파일에서도 제한된 메모리 사용
   - 스트리밍 방식의 데이터 읽기/쓰기

2. **처리 속도**:
   - 파일 크기에 따른 최적의 압축 방식 선택
   - 향후: 병렬 처리를 통한 성능 개선 예정

3. **디스크 효율성**:
   - 임시 파일 사용으로 안전한 원자적 교체
   - 공간 효율적인 압축 알고리즘

## 안정성 및 오류 처리

1. **백업 메커니즘**:
   - 기본적으로 원본 파일의 `.backup` 생성
   - `--no-backup` 옵션으로 백업 비활성화 가능

2. **트랜잭션 방식 처리**:
   - 임시 파일에 쓰기 완료 후에만 원본 교체
   - 실패 시 임시 파일 제거

## CLI 인터페이스

```
./target/release/trace --migrate <path> [--chunk-size <size>] [--no-backup] [--recursive]
```

### 옵션:
- `--chunk-size`: 청크 크기 설정 (기본: 10000)
- `--no-backup`: 백업 파일 생성 안 함
- `--recursive`: 하위 디렉토리까지 재귀적으로 마이그레이션

## 지원되는 파일 타입

1. **Block 파일**:
   - 필수 필드: time, process, action, sector
   - 타겟 스키마: 17개 필드

2. **UFS 파일**:
   - 필수 필드: time, process, action, opcode, lba
   - 타겟 스키마: 15개 필드

3. **UFSCustom 파일**: 
   - 현재 미지원 (향후 개발 예정)

## 미래 개선 방향

1. **성능 최적화**:
   - 병렬 처리를 통한 변환 속도 개선
   - 메모리 사용량 최적화를 위한 스트리밍 처리 강화

2. **기능 확장**:
   - UFSCustom 파일 타입 지원
   - 스키마 버전 관리 시스템 도입
   - 마이그레이션 히스토리 로깅
   - 더 정교한 데이터 타입 변환

3. **사용자 경험**:
   - 진행 상태 표시 개선
   - 오류 보고 및 해결 지침 강화

## 기술적 제한 사항

1. 매우 큰 파일(수십 GB) 처리 시 성능 제한 가능
2. 특정 데이터 타입만 지원 (Float64, UInt32, UInt64, Utf8, Boolean)
3. 복잡한 스키마 변환의 경우 제한적 지원
