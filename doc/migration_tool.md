# Parquet 파일 마이그레이션 도구

## 개요

이 도구는 기존 Parquet 파일을 새로운 스키마로 마이그레이션하는 기능을 제공합니다. 스키마 변경사항으로 인해 기존 파일을 읽을 수 없는 경우 이 도구를 사용하여 파일을 업데이트할 수 있습니다.

## 사용법

### 기본 사용법

```bash
# 단일 파일 마이그레이션
./target/release/trace --migrate <file_path>

# 디렉토리 내 모든 Parquet 파일 마이그레이션
./target/release/trace --migrate <directory_path>

# 재귀적 마이그레이션 (서브디렉토리 포함)
./target/release/trace --migrate <directory_path> --recursive
```

### 마이그레이션 옵션

- `--chunk-size <size>`: 마이그레이션 처리 시 사용할 청크 크기 (기본값: 10000)
- `--no-backup`: 백업 파일을 생성하지 않음 (기본적으로 `.backup` 확장자로 백업 생성)
- `--recursive`: 서브디렉토리를 포함하여 재귀적으로 마이그레이션

### 예제

```bash
# 백업 생성 후 단일 파일 마이그레이션
./target/release/trace --migrate data/output_block.parquet

# 백업 없이 디렉토리 마이그레이션
./target/release/trace --migrate data/ --no-backup

# 큰 청크 크기로 재귀적 마이그레이션
./target/release/trace --migrate data/ --chunk-size 50000 --recursive
```

## 지원 파일 타입

### Block 파일
- 자동 감지: `time`, `process`, `action`, `sector` 필드 존재 시
- 타겟 스키마: 17개 필드 (time, process, cpu, flags, action, devmajor, devminor, io_type, extra, sector, size, comm, qd, dtoc, ctoc, ctod, continuous)

### UFS 파일  
- 자동 감지: `time`, `process`, `action`, `opcode`, `lba` 필드 존재 시
- 타겟 스키마: 15개 필드 (time, process, cpu, action, tag, opcode, lba, size, groupid, hwqid, qd, dtoc, ctoc, ctod, continuous)

### UFSCustom 파일
- 현재 미구현 (향후 지원 예정)

## 마이그레이션 프로세스

1. **파일 타입 감지**: 스키마 분석을 통해 파일 타입 자동 감지
2. **스키마 호환성 확인**: 현재 스키마와 타겟 스키마 비교
3. **백업 생성**: 기본적으로 원본 파일의 백업 생성 (`.backup` 확장자)
4. **배치 처리**: 메모리 효율적인 배치 단위 데이터 변환
5. **필드 매핑**: 
   - 기존 필드: 그대로 유지
   - 새로운 필드: 타입별 기본값으로 채움
     - Float64: 0.0
     - UInt32/UInt64: 0
     - String: ""
     - Boolean: false
6. **원본 파일 교체**: 마이그레이션 완료 후 임시 파일로 원본 교체

## 안전성 기능

- **백업 생성**: 기본적으로 원본 파일 백업 (--no-backup으로 비활성화 가능)
- **임시 파일 사용**: 마이그레이션 중 임시 파일 사용하여 원본 파일 보호
- **배치 처리**: 메모리 사용량 제한을 통한 안정적인 대용량 파일 처리
- **오류 처리**: 각 배치 처리 시 오류 발생 시 안전한 중단

## 성능 최적화

- **병렬 처리**: 스키마 변환 과정에서 병렬 처리 활용
- **메모리 효율성**: 배치 단위 처리로 메모리 사용량 최소화
- **압축 최적화**: SNAPPY 압축 사용으로 빠른 I/O 성능
- **진행 상태 표시**: 대용량 파일 처리 시 진행 상태 표시

## 제한사항

- UFSCustom 파일 타입은 현재 미지원
- 스키마가 이미 최신 버전인 경우 변경사항 없음
- 매우 큰 파일(수십 GB)의 경우 처리 시간이 오래 걸릴 수 있음

## 문제 해결

### 일반적인 문제

1. **파일 타입 감지 실패**
   - 파일이 손상되었거나 지원되지 않는 스키마일 수 있음
   - 파일 권한 확인 필요

2. **메모리 부족**
   - `--chunk-size`를 더 작은 값으로 설정

3. **디스크 공간 부족**
   - 마이그레이션 시 임시 파일 생성을 위한 충분한 디스크 공간 필요

### 복구 방법

- 마이그레이션 실패 시 `.backup` 파일을 사용하여 원본 복구 가능
- 임시 파일(`.tmp`)이 남아있다면 수동으로 삭제 필요

## 향후 계획

- UFSCustom 파일 타입 지원
- 스키마 버전 관리 기능
- 더 정교한 데이터 타입 변환 지원
- 마이그레이션 히스토리 로깅
