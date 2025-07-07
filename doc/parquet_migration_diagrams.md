# Parquet 마이그레이션 다이어그램

이 문서는 Parquet 마이그레이션 도구의 핵심 프로세스와 데이터 흐름을 시각적으로 표현합니다.

## 마이그레이션 처리 흐름

```mermaid
flowchart TD
    A[시작: --migrate 명령어] --> B{파일 유형 확인}
    B -->|단일 파일| C[파일 마이그레이션]
    B -->|디렉토리| D{재귀 옵션?}
    D -->|Yes: --recursive| E[재귀적 마이그레이션]
    D -->|No| F[디렉토리 마이그레이션]
    
    C --> G{백업 활성화?}
    G -->|Yes| H[백업 파일 생성]
    G -->|No: --no-backup| I[백업 생략]
    
    H --> J[파일 타입 감지]
    I --> J
    
    J --> K{파일 타입?}
    K -->|Block| L[Block 마이그레이션]
    K -->|UFS| M[UFS 마이그레이션]
    K -->|UFSCustom| N[미지원 오류]
    K -->|Unknown| O[알 수 없는 타입 오류]
    
    L --> P[스키마 호환성 확인]
    M --> P
    
    P -->|호환됨| Q[변환 불필요]
    P -->|호환 안됨| R[임시 파일 생성]
    
    R --> S[배치 단위 데이터 변환]
    S --> T[변환된 데이터 임시 파일에 쓰기]
    T --> U[원본 파일 교체]
    
    Q --> V[마이그레이션 완료]
    U --> V
```

## 파일 타입 감지 프로세스

```mermaid
flowchart LR
    A[Parquet 파일] --> B[스키마 읽기]
    B --> C{필드 확인}
    
    C --> D{time, process, action, sector 존재?}
    D -->|Yes| E[Block 타입]
    D -->|No| F{time, process, action, opcode, lba 존재?}
    
    F -->|Yes| G[UFS 타입]
    F -->|No| H{UFSCustom 패턴 확인}
    
    H -->|Match| I[UFSCustom 타입]
    H -->|No Match| J[알 수 없는 타입]
```

## 스키마 변환 프로세스

```mermaid
flowchart TD
    A[원본 Parquet 파일] --> B[Arrow RecordBatch로 읽기]
    B --> C[타겟 스키마 정의]
    
    C --> D[필드별 처리]
    D --> E{필드가 원본에 존재?}
    
    E -->|Yes| F[기존 값 유지]
    E -->|No| G[데이터 타입에 맞는 기본값 생성]
    
    F --> H[새 RecordBatch 구성]
    G --> H
    
    H --> I[새 Parquet 파일에 쓰기]
    I --> J[원본 파일 교체]
```

## 컴포넌트 다이어그램

```mermaid
classDiagram
    class ParquetMigrator {
        -backup_enabled: bool
        +new(chunk_size: usize, backup_enabled: bool)
        +migrate_file(file_path: &str)
        +migrate_directory(dir_path: &str)
        -detect_file_type(file_path: &str)
        -create_backup(file_path: &str)
        -migrate_block_file(file_path: &str)
        -migrate_ufs_file(file_path: &str)
        -migrate_ufscustom_file(file_path: &str)
        -get_block_target_schema()
        -get_ufs_target_schema()
        -convert_block_batch()
        -convert_ufs_batch()
        -create_default_array()
        -select_compression()
    }
    
    class FileType {
        <<enumeration>>
        Block
        UFS
        UFSCustom
        Unknown
    }
    
    class run_migration {
        <<function>>
        +run(input_path: &str, chunk_size: Option<usize>, backup_enabled: bool, recursive: bool)
    }
    
    class migrate_directory_recursive {
        <<function>>
        +run(migrator: &ParquetMigrator, dir_path: &str)
    }
    
    ParquetMigrator --> FileType
    run_migration --> ParquetMigrator
    migrate_directory_recursive --> ParquetMigrator
```

## 데이터 변환 예시 (Block 파일)

아래는 Block 파일 스키마 변환의 예시를 보여줍니다:

### 원본 스키마

```
time: Float64
process: Utf8
action: Utf8
sector: UInt64
size: UInt32
```

### 타겟 스키마

```
time: Float64      <- 기존 필드
process: Utf8      <- 기존 필드
cpu: UInt32        <- 새 필드 (기본값 0)
flags: Utf8        <- 새 필드 (기본값 "")
action: Utf8       <- 기존 필드
devmajor: UInt32   <- 새 필드 (기본값 0)
devminor: UInt32   <- 새 필드 (기본값 0)
io_type: Utf8      <- 새 필드 (기본값 "")
extra: UInt32      <- 새 필드 (기본값 0)
sector: UInt64     <- 기존 필드
size: UInt32       <- 기존 필드
comm: Utf8         <- 새 필드 (기본값 "")
qd: UInt32         <- 새 필드 (기본값 0)
dtoc: Float64      <- 새 필드 (기본값 0.0)
ctoc: Float64      <- 새 필드 (기본값 0.0)
ctod: Float64      <- 새 필드 (기본값 0.0)
continuous: Boolean <- 새 필드 (기본값 false)
```

## 파일 크기별 압축 전략

파일 크기에 따른 동적 압축 전략:

| 파일 크기 | 압축 알고리즘 | 목적 |
|-----------|--------------|------|
| < 1MB | SNAPPY | 빠른 속도 우선 |
| 1MB ~ 10MB | ZSTD (레벨 3) | 속도와 압축률 균형 |
| 10MB ~ 100MB | ZSTD (레벨 6) | 높은 압축률 |
| ≥ 100MB | ZSTD (레벨 9) | 최고 압축률 |
