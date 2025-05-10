# UFS 및 Block I/O 로그 분석 도구 함수 설명서

이 문서는 UFS 및 Block I/O 로그 분석 도구의 주요 함수들에 대한 자세한 설명을 제공합니다.

## 목차
1. [메인 함수](#메인-함수)
2. [파서 모듈 함수](#파서-모듈-함수)
3. [프로세서 모듈 함수](#프로세서-모듈-함수)
4. [출력 모듈 함수](#출력-모듈-함수)
   - [통계 함수](#통계-함수)
   - [Parquet 저장 함수](#parquet-저장-함수)
   - [차트 생성 함수](#차트-생성-함수)
5. [유틸리티 함수](#유틸리티-함수)

## 메인 함수

### `main()`

**위치**: `src/main.rs`

**설명**: 프로그램의 진입점으로, 명령줄 인자를 처리하고 로그 파일 분석 과정 전체를 조율합니다.

**동작 순서**:
1. 명령줄 인자 처리 또는 사용자 입력 요청
2. 로그 파일 파싱 (`parse_log_file()` 호출)
3. UFS 및 Block I/O 데이터 후처리
4. 분석 결과 계산 및 출력
5. Parquet 파일 저장
6. 차트 생성

**인자**: 없음

**반환 값**: `io::Result<()>` - I/O 작업 성공 여부

**구현 세부사항**:
- 대화형 모드와 배치 모드를 모두 지원합니다
- 오류 발생 시 사용자에게 계속 진행 여부를 묻습니다
- 분석 단계별로 진행 상황과 소요 시간을 출력합니다

### `ask_continue()`

**위치**: `src/main.rs`

**설명**: 사용자에게 분석을 계속할지 묻는 유틸리티 함수입니다.

**동작**:
- 사용자에게 Y/N 입력을 요청
- Y 또는 yes인 경우 `true` 반환
- N 또는 no인 경우 `false` 반환
- 다른 입력의 경우 다시 요청

**인자**: 없음

**반환 값**: `io::Result<bool>` - 사용자가 계속할지 여부

**사용 사례**: 
- 명령줄 인자가 올바르지 않을 때
- 파일 파싱 오류 발생 시
- 하나의 분석 완료 후 다른 파일을 분석할지 확인할 때

## 파서 모듈 함수

### `parse_log_file()`

**위치**: `src/parsers/log.rs`

**설명**: 로그 파일을 읽고 파싱하여 UFS 및 Block I/O 이벤트를 추출합니다.

**동작**:
1. 로그 파일 열기 및 크기 확인
2. 파일 크기에 따라 적절한 처리 방식 선택:
   - 작은 파일(1GB 이하): `parse_log_file_in_memory()` 호출
   - 큰 파일(1GB 초과): `parse_log_file_streaming()` 호출
3. 파싱된 UFS 및 Block I/O 이벤트 반환

**인자**:
- `filepath: &str` - 분석할 로그 파일 경로

**반환 값**: `io::Result<(Vec<UFS>, Vec<Block>)>` - 파싱된 UFS 및 Block I/O 이벤트 벡터

**최적화 방식**:
- 파일 크기에 따른 자동 전략 선택
- 대용량 파일은 메모리 효율적인 스트리밍 방식 사용
- 소형 파일은 빠른 인메모리 처리 방식 사용

### `parse_log_file_in_memory()`

**위치**: `src/parsers/log.rs`

**설명**: 작은 로그 파일을 메모리에 모두 로드하여 파싱하는 함수입니다.

**동작**:
1. 로그 파일 전체를 메모리에 로드
2. 파일을 정의된 청크 크기(100,000줄)로 분할
3. 각 청크에 대해 `process_chunk_parallel()` 함수 호출하여 병렬 처리
4. 파싱된 결과를 벡터에 추가

**인자**:
- `filepath: &str` - 분석할 로그 파일 경로

**반환 값**: `io::Result<(Vec<UFS>, Vec<Block>)>` - 파싱된 UFS 및 Block I/O 이벤트 벡터

**성능 특성**:
- 빠른 처리 속도(메모리 내 처리)
- 멀티코어 CPU 활용을 위한 병렬 처리
- 1GB 이하 파일에 최적화됨
- 정기적인 진행 상황 출력

### `parse_log_file_streaming()`

**위치**: `src/parsers/log.rs`

**설명**: 대용량 로그 파일을 스트리밍 방식으로 파싱하는 함수입니다. 메모리 사용량을 최소화하여 대용량 파일을 효율적으로 처리합니다.

**동작**:
1. UFS 및 Block I/O 데이터를 저장할 임시 파일 생성
2. 로그 파일을 한 줄씩 읽어서 정의된 청크 크기(500,000줄)로 분할
3. 각 청크에 대해 `process_chunk()` 함수 호출하여 처리
4. 파싱된 결과를 임시 파일에 저장
5. 첫 번째 패스 완료 후 임시 파일에서 데이터 로드
6. 임시 파일 삭제 및 최종 결과 반환

**인자**:
- `filepath: &str` - 분석할 로그 파일 경로

**반환 값**: `io::Result<(Vec<UFS>, Vec<Block>)>` - 파싱된 UFS 및 Block I/O 이벤트 벡터

**메모리 관리**:
- 대용량 버퍼(8MB)를 사용하여 I/O 최적화
- 한 번에 최대 500,000줄만 메모리에 로드
- 임시 파일을 사용하여 중간 결과 저장
- 5초마다 진행 상황 출력

### `process_chunk()`

**위치**: `src/parsers/log.rs`

**설명**: 로그 파일의 한 청크(chunk)를 처리하여 UFS 및 Block I/O 이벤트를 추출합니다.

**동작**:
1. 각 줄에 대해 정규 표현식 매칭 수행
2. UFS 이벤트 매칭 시:
   - 캡처된 그룹에서 필드 추출
   - `UFS` 구조체 생성 및 초기 필드 설정
   - JSON 형식으로 임시 파일에 저장
3. Block I/O 이벤트 매칭 시:
   - 캡처된 그룹에서 필드 추출
   - `Block` 구조체 생성 및 초기 필드 설정
   - JSON 형식으로 임시 파일에 저장
4. 이벤트 카운터 업데이트

**인자**:
- `chunk: &[String]` - 처리할 로그 줄들
- `ufs_writer: &mut BufWriter<&File>` - UFS 이벤트를 저장할 버퍼 라이터
- `block_writer: &mut BufWriter<&File>` - Block I/O 이벤트를 저장할 버퍼 라이터

**반환 값**: `(usize, usize)` - 추출된 UFS 및 Block I/O 이벤트 수

**최적화 기법**:
- 컴파일된 정규 표현식 재사용(lazy_static)
- 명명된 캡처 그룹으로 가독성 향상
- 버퍼링된 I/O로 디스크 쓰기 최적화

### `process_chunk_parallel()`

**위치**: `src/parsers/log.rs`

**설명**: 로그 청크를 병렬로 처리하는 함수입니다. Rayon 라이브러리를 활용한 병렬 처리시 사용됩니다.

**동작**:
1. 청크 내 각 줄을 순회하며:
   - UFS 패턴 매칭 및 구조체 생성
   - Block I/O 패턴 매칭 및 구조체 생성
2. 추출된 이벤트를 각각의 벡터에 수집

**인자**:
- `chunk: Vec<String>` - 처리할 로그 줄 벡터

**반환 값**: `(Vec<UFS>, Vec<Block>)` - 파싱된 UFS 및 Block I/O 이벤트 벡터 쌍

**병렬화 특성**:
- 내부적으로 Rayon의 병렬 처리 기능을 활용
- 멀티코어 시스템에서 성능 향상
- 인메모리 처리에 적합한 구현

### `create_temp_file()`

**위치**: `src/parsers/log.rs`

**설명**: 임시 파일을 생성하는 유틸리티 함수입니다.

**동작**:
1. 랜덤 ID를 포함한 임시 파일 경로 생성 (`/tmp/[prefix]_[random_id].tmp`)
2. 읽기 및 쓰기 권한으로 파일 생성 및 열기

**인자**:
- `prefix: &str` - 임시 파일 이름의 접두사 (예: "ufs", "block")

**반환 값**: `io::Result<(File, String)>` - 생성된 파일 핸들과 파일 경로

**활용**:
- 대용량 로그 파일 처리 중 중간 결과 저장
- 메모리 사용을 최소화하기 위한 임시 저장소

## 프로세서 모듈 함수

### `ufs_bottom_half_latency_process()`

**위치**: `src/processors/ufs.rs`

**설명**: UFS 이벤트를 처리하여 지연 시간 및 연속성 정보를 계산합니다.

**동작**:
1. 시간 순으로 이벤트 정렬
2. 각 이벤트를 순회하며 다음을 계산:
   - Queue Depth(QD): 처리 중인 요청 수 트래킹 (send_req: +1, complete_rsp: -1)
   - Dispatch to Complete 지연 시간(dtoc): dispatch 요청에서 complete까지의 시간
   - Complete to Complete 지연 시간(ctoc): 이전 complete에서 현재 complete까지의 시간
   - Complete to Dispatch 지연 시간(ctod): 이전 complete에서 다음 dispatch 요청까지의 시간
   - Continuous(연속성): LBA와 크기 기반으로 이전 요청과의 연속성 판단
3. 계산된 정보로 UFS 이벤트 업데이트

**인자**:
- `ufs_list: Vec<UFS>` - 파싱된 UFS 이벤트 벡터

**반환 값**: `Vec<UFS>` - 지연 시간 및 연속성 정보가 추가된 UFS 이벤트 벡터

**주요 개념**:
- **Dispatch to Complete (dtoc)**: I/O 요청이 dispatch된 시점부터 complete될 때까지의 시간으로, 실제 I/O 처리 시간을 의미합니다.
- **Complete to Complete (ctoc)**: 연속된 complete 이벤트 사이의 시간으로, throughput을 이해하는 데 유용합니다.
- **Complete to Dispatch (ctod)**: complete 이벤트 후 다음 요청이 dispatch되기까지의 시간으로, 애플리케이션의 I/O 요청 생성 지연을 나타냅니다.
- **Continuous**: 현재 요청이 이전 요청의 논리적 연속인지를 판단하며, 순차적/임의 I/O 패턴을 식별하는 데 사용됩니다.

**최적화 기법**:
- 해시맵 사용으로 요청-완료 쌍 매칭 최적화
- 시간 정렬로 순차 처리 보장
- 메모리 효율적인 벡터 업데이트

### `block_bottom_half_latency_process()`

**위치**: `src/processors/block.rs`

**설명**: Block I/O 이벤트를 처리하여 지연 시간 및 연속성 정보를 계산합니다.

**동작**:
1. 시간 순으로 이벤트 정렬
2. 중복 이벤트 제거 및 필터링
3. 각 이벤트를 순회하며 다음을 계산:
   - Queue Depth(QD): 액션 타입에 따라 카운터 업데이트 (D(Dispatch): +1, C(Complete): -1)
   - Dispatch to Complete 지연 시간(dtoc): dispatch 요청에서 complete까지의 시간
   - Complete to Complete 지연 시간(ctoc): 이전 complete에서 현재 complete까지의 시간
   - Complete to Dispatch 지연 시간(ctod): 이전 complete에서 다음 dispatch 요청까지의 시간
   - Continuous(연속성): 섹터 번호와 크기 기반으로 연속성 판단
4. I/O 타입(READ, WRITE, DISCARD 등)별 특수 처리 적용

**인자**:
- `block_list: Vec<Block>` - 파싱된 Block I/O 이벤트 벡터

**반환 값**: `Vec<Block>` - 지연 시간 및 연속성 정보가 추가된 Block I/O 이벤트 벡터

**주요 개념**:
- **Dispatch(D) 이벤트**: 블록 I/O 요청이 디바이스 드라이버에 전송되는 시점
- **Complete(C) 이벤트**: 블록 I/O 요청이 처리 완료된 시점
- **Sector 기반 연속성**: 현재 요청의 시작 섹터가 이전 요청의 (시작 섹터 + 크기)와 연속적인지 확인

**특별 처리**:
- 중복 이벤트 제거
- 잘못된 시간 순서 수정
- 비정상적인 섹터 값 정규화 (최대 u64 값은 0으로 설정)
- 각 디바이스별 별도 추적

## 출력 모듈 함수

### 통계 함수

#### `print_ufs_statistics()`

**위치**: `src/output/statistics.rs`

**설명**: UFS 이벤트에 대한 통계 분석 결과를 콘솔에 출력합니다.

**동작**:
1. 총 요청 수, 최대 Queue Depth 등 기본 통계 계산 및 출력
2. 평균 지연 시간(dtoc, ctoc, ctod) 계산 및 출력
3. 연속 요청 비율 계산 및 출력
4. 오퍼레이션 코드(opcode)별 그룹화 및 통계 계산
5. `print_latency_stats_by_opcode()` 호출하여 지연 시간 통계 출력
6. `print_latency_ranges_by_opcode()` 호출하여 지연 시간 범위 분포 출력
7. 요청 크기 분포 계산 및 출력

**인자**:
- `traces: &[UFS]` - 분석할 UFS 이벤트 슬라이스

**반환 값**: 없음

**출력 형식**:
- 테이블 형태로 정렬된 통계 데이터
- 오퍼레이션 코드별 지연 시간 통계 테이블
- 지연 시간 범위별 분포 테이블
- 요청 크기 분포 요약

#### `print_block_statistics()`

**위치**: `src/output/statistics.rs`

**설명**: Block I/O 이벤트에 대한 통계 분석 결과를 콘솔에 출력합니다.

**동작**:
1. 총 요청 수, 최대 Queue Depth 등 기본 통계 계산 및 출력
2. 평균 지연 시간(dtoc, ctoc, ctod) 계산 및 출력
3. 연속 요청 비율 계산 및 출력
4. I/O 타입(READ, WRITE 등)별 그룹화 및 통계 계산
5. `print_latency_stats_by_iotype()` 호출하여 지연 시간 통계 출력
6. `print_latency_ranges_by_iotype()` 호출하여 지연 시간 범위 분포 출력
7. 디바이스별 통계 계산 및 출력
8. 요청 크기 분포 계산 및 출력

**인자**:
- `traces: &[Block]` - 분석할 Block I/O 이벤트 슬라이스

**반환 값**: 없음

**진단 지표**:
- 99%/99.9% percentile 지연 시간 - 성능 이상치 식별
- 읽기/쓰기 작업 비율 - 워크로드 특성 파악
- 연속성 비율 - 순차적/임의 I/O 패턴 식별
- 디바이스별 성능 비교 - 병목 장치 식별

#### `print_latency_stats_by_opcode()`

**위치**: `src/output/statistics.rs`

**설명**: UFS 오퍼레이션 코드별 지연 시간 통계를 계산하고 출력하는 내부 함수입니다.

**동작**:
1. 각 오퍼레이션 코드(opcode)별로 지연 시간 값들을 수집
2. 평균, 최소, 최대, 중앙값, 표준편차 계산
3. Percentile(90%, 95%, 99%, 99.9%, 99.99%) 계산
4. 결과를 표 형식으로 정렬하여 출력

**인자**:
- `opcode_groups: &HashMap<String, Vec<&UFS>>` - 오퍼레이션 코드별 UFS 이벤트 그룹
- `stat_name: &str` - 통계 이름(예: "Dispatch to Complete (dtoc)")
- `latency_fn: impl Fn(&&UFS) -> f64` - 이벤트에서 지연 시간 값을 추출하는 함수

**반환 값**: 없음

**통계 방법론**:
- 중앙값은 정렬 후 중간값 선택
- 표준편차는 분산의 제곱근
- Percentile은 정렬된 값에서 해당 위치 선택

#### `print_latency_stats_by_iotype()`

**위치**: `src/output/statistics.rs`

**설명**: Block I/O 타입별 지연 시간 통계를 계산하고 출력하는 내부 함수입니다.

**동작**:
1. 각 I/O 타입(READ, WRITE 등)별로 지연 시간 값들을 수집
2. 평균, 최소, 최대, 중앙값, 표준편차 계산
3. Percentile(90%, 95%, 99%, 99.9%, 99.99%) 계산
4. 결과를 표 형식으로 정렬하여 출력

**인자**:
- `iotype_groups: &HashMap<String, Vec<&Block>>` - I/O 타입별 Block 이벤트 그룹
- `stat_name: &str` - 통계 이름(예: "Dispatch to Complete (dtoc)")
- `latency_fn: impl Fn(&&Block) -> f64` - 이벤트에서 지연 시간 값을 추출하는 함수

**반환 값**: 없음

**I/O 타입 분류**:
- READ: 읽기 작업
- WRITE: 쓰기 작업
- DISCARD: 블록 삭제/초기화 작업
- FLUSH: 캐시 플러시 작업

#### `print_latency_ranges_by_opcode()`

**위치**: `src/output/statistics.rs`

**설명**: UFS 오퍼레이션 코드별 지연 시간 범위 분포를 계산하고 출력하는 내부 함수입니다.

**동작**:
1. 지연 시간 범위 정의 (0.1ms 이하, 0.1~0.5ms, 0.5~1ms, 1~5ms, 5~10ms, 10ms 이상)
2. 각 오퍼레이션 코드별로 각 범위에 속하는 이벤트 수 카운트
3. 백분율 계산 및 표 형식으로 출력

**인자**:
- `opcode_groups: &HashMap<String, Vec<&UFS>>` - 오퍼레이션 코드별 UFS 이벤트 그룹
- `stat_name: &str` - 통계 이름(예: "Dispatch to Complete (dtoc)")
- `latency_fn: impl Fn(&&UFS) -> f64` - 이벤트에서 지연 시간 값을 추출하는 함수

**반환 값**: 없음

**범위 정의 근거**:
- 0.1ms 이하: 매우 빠른 응답(캐시 히트)
- 0.1~0.5ms: 빠른 응답
- 0.5~1ms: 보통 응답
- 1~5ms: 느린 응답
- 5~10ms: 매우 느린 응답
- 10ms 이상: 비정상적으로 느린 응답(잠재적 문제)

#### `print_latency_ranges_by_iotype()`

**위치**: `src/output/statistics.rs`

**설명**: Block I/O 타입별 지연 시간 범위 분포를 계산하고 출력하는 내부 함수입니다.

**동작**:
1. 지연 시간 범위 정의 (0.1ms 이하, 0.1~0.5ms, 0.5~1ms, 1~5ms, 5~10ms, 10ms 이상)
2. 각 I/O 타입별로 각 범위에 속하는 이벤트 수 카운트
3. 백분율 계산 및 표 형식으로 출력

**인자**:
- `iotype_groups: &HashMap<String, Vec<&Block>>` - I/O 타입별 Block 이벤트 그룹
- `stat_name: &str` - 통계 이름(예: "Dispatch to Complete (dtoc)")
- `latency_fn: impl Fn(&&Block) -> f64` - 이벤트에서 지연 시간 값을 추출하는 함수

**반환 값**: 없음

**분석 활용**:
- 읽기/쓰기 작업의 지연 시간 분포 비교
- 지연 시간 범위 분포를 통한 성능 이상 식별
- 특정 범위에 분포가 집중되는지 확인

#### `count_sizes()`

**위치**: `src/output/statistics.rs`

**설명**: 요청 크기 분포를 계산하는 일반화된 내부 함수입니다.

**동작**:
1. 이벤트 슬라이스를 순회하며 각 크기별 요청 수 카운트
2. 크기별 빈도수를 해시맵으로 반환

**인자**:
- `traces: &[&T]` - 분석할 이벤트 슬라이스 (제네릭 타입 T)
- `size_fn: impl Fn(&&T) -> u32` - 이벤트에서 크기를 추출하는 함수

**반환 값**: `HashMap<u32, usize>` - 크기별 요청 수 맵

**사용 사례**:
- UFS 및 Block I/O 이벤트의 크기 분포 분석
- 일반적인 I/O 크기 패턴 식별
- 최적 I/O 크기 분석

### Parquet 저장 함수

#### `save_to_parquet()`

**위치**: `src/output/parquet.rs`

**설명**: UFS 및 Block I/O 이벤트를 Parquet 파일로 저장하는 함수입니다.

**동작**:
1. UFS 데이터를 "[prefix]_ufs.parquet" 파일로 저장 (`save_ufs_to_parquet()` 호출)
2. Block I/O 데이터를 "[prefix]_block.parquet" 파일로 저장 (`save_block_to_parquet()` 호출)

**인자**:
- `ufs_traces: &[UFS]` - 저장할 UFS 이벤트 슬라이스
- `block_traces: &[Block]` - 저장할 Block I/O 이벤트 슬라이스
- `output_prefix: &str` - 출력 파일 접두사

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 저장 성공 여부

**Parquet 포맷 장점**:
- Column 기반 저장으로 빠른 쿼리 가능
- 효율적인 압축 알고리즘 지원
- Schema 정보 내장
- 다양한 데이터 분석 도구와 호환성

#### `save_ufs_to_parquet()`

**위치**: `src/output/parquet.rs`

**설명**: UFS 이벤트를 Parquet 파일로 저장하는 내부 함수입니다.

**동작**:
1. UFS 구조체의 각 필드를 Arrow 배열로 변환:
   - time, cpu, tag, qd, dtoc, ctoc, ctod 등 숫자 필드
   - process, action, opcode 등 문자열 필드
   - continuous 불리언 필드
2. Parquet schema 정의 및 필드 매핑
3. Arrow RecordBatch 생성
4. Parquet 파일로 저장 (압축 및 인코딩 설정 적용)

**인자**:
- `traces: &[UFS]` - 저장할 UFS 이벤트 슬라이스
- `filepath: &str` - 저장할 파일 경로

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 저장 성공 여부

**구현 세부사항**:
- 컬럼별 적절한 Arrow 데이터 타입 사용
- SNAPPY 압축 방식 적용
- 내부적으로 Arrow 메모리 관리 최적화

#### `save_block_to_parquet()`

**위치**: `src/output/parquet.rs`

**설명**: Block I/O 이벤트를 Parquet 파일로 저장하는 내부 함수입니다.

**동작**:
1. Block 구조체의 각 필드를 Arrow 배열로 변환:
   - time, cpu, devmajor, devminor, sector, size, qd, dtoc, ctoc, ctod 등 숫자 필드
   - process, flags, action, io_type, comm 등 문자열 필드
   - continuous 불리언 필드
2. Parquet schema 정의 및 필드 매핑
3. Arrow RecordBatch 생성
4. Parquet 파일로 저장 (압축 및 인코딩 설정 적용)

**인자**:
- `traces: &[Block]` - 저장할 Block I/O 이벤트 슬라이스
- `filepath: &str` - 저장할 파일 경로

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 저장 성공 여부

**데이터 활용**:
- Python/R 등 데이터 분석 도구에서 후속 분석 가능
- Pandas DataFrame으로 쉽게 로드 가능
- 대용량 데이터의 효율적인 저장 및 분석

### 차트 생성 함수

#### `generate_charts()`

**위치**: `src/output/charts.rs`

**설명**: UFS 및 Block I/O 데이터를 시각화하는 차트를 생성합니다.

**동작**:
1. `generate_ufs_charts()` 호출하여 UFS 데이터 시각화
2. `generate_block_charts()` 호출하여 Block I/O 데이터 시각화
3. 생성된 차트 파일 경로 출력

**인자**:
- `ufs_traces: &[UFS]` - 시각화할 UFS 이벤트 슬라이스
- `block_traces: &[Block]` - 시각화할 Block I/O 이벤트 슬라이스
- `output_prefix: &str` - 출력 파일 접두사

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 차트 생성 성공 여부

**생성되는 파일**:
- HTML 형식 Plotly 차트 (대화형)
- PNG 형식 이미지 (정적)

#### `generate_ufs_charts()`

**위치**: `src/output/charts.rs`

**설명**: UFS 데이터를 시각화하는 다양한 차트를 생성하는 내부 함수입니다.

**동작**:
1. 지연 시간 분포 히스토그램 생성:
   - dtoc(Dispatch to Complete) 히스토그램
   - ctoc(Complete to Complete) 히스토그램
   - ctod(Complete to Dispatch) 히스토그램
2. 시간별 지연 시간 추이 차트 생성
3. Queue Depth(QD) 변화 차트 생성
4. 오퍼레이션 코드별 분포 파이 차트 생성
5. 생성된 차트를 HTML 및 PNG 파일로 저장

**인자**:
- `traces: &[UFS]` - 시각화할 UFS 이벤트 슬라이스
- `output_prefix: &str` - 출력 파일 접두사

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 차트 생성 성공 여부

**차트 유형 및 목적**:
- **Latency Histogram**: 지연 시간 분포 패턴 식별
- **Timeline Chart**: 시간에 따른 성능 변화 분석
- **Queue Depth Chart**: 부하 수준 및 병목 현상 식별
- **Operation Distribution**: 워크로드 특성 파악

#### `generate_block_charts()`

**위치**: `src/output/charts.rs`

**설명**: Block I/O 데이터를 시각화하는 다양한 차트를 생성하는 내부 함수입니다.

**동작**:
1. 지연 시간 분포 히스토그램 생성:
   - dtoc(Dispatch to Complete) 히스토그램
   - ctoc(Complete to Complete) 히스토그램
   - ctod(Complete to Dispatch) 히스토그램
2. 시간별 지연 시간 추이 차트 생성
3. Queue Depth(QD) 변화 차트 생성
4. I/O 타입별 분포 파이 차트 생성
5. 디바이스별 성능 비교 차트 생성
6. 생성된 차트를 HTML 및 PNG 파일로 저장

**인자**:
- `traces: &[Block]` - 시각화할 Block I/O 이벤트 슬라이스
- `output_prefix: &str` - 출력 파일 접두사

**반환 값**: `Result<(), Box<dyn std::error::Error>>` - 차트 생성 성공 여부

**시각화 최적화**:
- 대용량 데이터의 경우 downsampling 적용
- Log scale 옵션으로 넓은 범위의 값 표현
- 대화형 차트로 확대/축소 및 필터링 지원

## 유틸리티 함수

### `MILLISECONDS`

**위치**: `src/utils/constants.rs`

**설명**: 초를 밀리초로 변환하는 상수입니다.

**값**: 1000

**용도**: 초 단위의 시간을 밀리초로 변환할 때 사용됩니다.

**사용 예**:
- 지연 시간 계산 시 초 단위 차이를 밀리초로 변환
- 시간 범위 필터링 시 단위 변환

### `set_user_latency_ranges()`

**위치**: `src/utils/latency.rs`

**설명**: 사용자 정의 지연 시간 범위를 설정하는 함수입니다.

**동작**:
1. 사용자가 `-l` 옵션으로 제공한 지연 시간 경계값을 저장
2. 전역 상태로 저장하여 프로그램 전체에서 사용 가능하도록 설정

**인자**:
- `ranges: Vec<f64>` - 사용자 정의 지연 시간 경계값 목록

**반환 값**: 없음

**적용 예**:
- `-l 0.1,0.5,1,5,10,30,100,500,1000` 명령줄 옵션 사용 시 호출

### `get_user_latency_ranges()`

**위치**: `src/utils/latency.rs`

**설명**: 설정된 사용자 정의 지연 시간 범위를 가져오는 함수입니다.

**동작**:
1. 전역 저장소에서 사용자 정의 지연 시간 경계값 검색
2. 설정된 값이 있으면 복제하여 반환

**인자**: 없음

**반환 값**: `Option<Vec<f64>>` - 지연 시간 경계값 또는 None(설정되지 않은 경우)

**적용 예**:
- 지연 시간 범위별 분포 계산 시 호출
- 사용자 정의 범위가 없을 경우 기본 범위 사용

### `parse_latency_ranges()`

**위치**: `src/utils/latency.rs`

**설명**: 문자열로 제공된 지연 시간 범위를 파싱하는 함수입니다.

**동작**:
1. 쉼표로 구분된 문자열을 실수값 목록으로 파싱
2. 모든 값이 0 이상인지 검증
3. 값들이 오름차순으로 정렬되어 있는지 검증

**인자**:
- `value_str: &str` - 쉼표로 구분된 지연 시간 경계값 문자열

**반환 값**: `Result<Vec<f64>, String>` - 파싱 성공 시 경계값 목록, 실패 시 오류 메시지

**검증 조건**:
- 모든 값은 0 이상이어야 함
- 값들은 오름차순이어야 함
- 최소한 하나 이상의 유효한 값이 있어야 함