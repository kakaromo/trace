# 성능 BM 평가 및 Trace 파싱 가이드

## 개요
성능 평가해서 Iteration 별 read, write성능 확인 및 각 성능에서 trace 파싱도 같이 되어야 합니다.
성능 결과는 csv형태로 되어야 합니다.
paring할때 Iteration을 추가하여 구분하여 나중에 1,2,3,4,5... 폴더 만들어서 parquet 파일 생성되도록 해야해요

## 지원 성능 벤치마크 도구
fio, tiotest, iozone 3가지 성능 BM 결과를 봐야합니다.

## Trace 파싱 방법

### 명령어 형식
```bash
./trace -p <로그_파일_경로> <출력_파일_접두사>
```

### 파라미터 설명
- `-p`: 성능 벤치마크 모드 활성화 (Performance benchmark mode)
- `<로그_파일_경로>`: 성능 BM 결과와 trace가 혼재된 로그 파일 경로
- `<출력_파일_접두사>`: 출력 파일의 접두사 (예: fio_result, tiotest_output 등)

### 자동 감지 기능
- **Trace 타입 자동 감지**: 로그를 읽으면서 ufs, block, ufscustom 타입을 자동으로 감지하여 처리
- **Iteration 번호 자동 추출**: 성능 BM 결과 로그에서 Iteration 번호를 자동으로 파싱하여 구분
- **기존 파싱 방식 사용**: 기존 trace 파싱 로직을 그대로 사용하여 처리

### Parquet 출력 구조
`<출력_파일_접두사>` 폴더 내에 Iteration별 하위 폴더가 자동 생성됩니다:
```
<출력_파일_접두사>/
  1/
    ufs_trace.parquet
    block_trace.parquet
    ufscustom_trace.parquet
  2/
    ufs_trace.parquet
    block_trace.parquet
  3/
    ufscustom_trace.parquet
  ...
  10/
    ufs_trace.parquet
```

예시: `./trace benchmark.log fio_result` 실행 시
```
fio_result/
  1/
    ufs_trace.parquet
    block_trace.parquet
  2/
    ufs_trace.parquet
  ...
```

### 실행 흐름
1. 성능 BM 실행 (fio/tiotest/iozone) - trace 수집 포함
2. trace 파일에는 성능 결과와 trace 데이터가 혼재
3. 파서가 로그를 읽으면서:
   - 성능 결과에서 Iteration 번호 추출
   - Trace 라인에서 타입(ufs/block/ufscustom) 자동 감지
   - 각 타입별로 기존 파싱 로직 적용
4. Iteration 번호별 폴더에 타입별 parquet 파일 생성
5. 다음 Iteration으로 반복

성능 결과 CSV와 trace parquet 파일이 모두 생성되어야 합니다.

fio 평가 결과 데이터입니다.
seq write, read 
--- FIO 1GB Sequential Write Test (Iteration 1) ---
seqwrite: (g=0): rw=write, bs=(R) 1024KiB-1024KiB, (W) 1024KiB-1024KiB, (T) 1024KiB-1024KiB, ioengine=libaio, iodepth=32
fio-3.33
Starting 1 process
seqwrite: Laying out IO file (1 file / 1024MiB)

seqwrite: (groupid=0, jobs=1): err= 0: pid=24188: Sun Nov  9 20:12:52 2025
  write: IOPS=604, BW=604MiB/s (633MB/s)(1024MiB/1695msec); 0 zone resets
    slat (usec): min=1085, max=5858, avg=1653.31, stdev=549.68
    clat (nsec): min=1719, max=58882k, avg=50161066.97, stdev=5876522.28
     lat (usec): min=1366, max=63704, avg=51814.38, stdev=5919.34
    clat percentiles (usec):
     |  1.00th=[16319],  5.00th=[45876], 10.00th=[46400], 20.00th=[47973],
     | 30.00th=[49546], 40.00th=[50594], 50.00th=[51119], 60.00th=[51643],
     | 70.00th=[52167], 80.00th=[53216], 90.00th=[54789], 95.00th=[55313],
     | 99.00th=[56886], 99.50th=[57410], 99.90th=[58459], 99.95th=[58983],
     | 99.99th=[58983]
   bw (  KiB/s): min=528384, max=657408, per=96.89%, avg=599381.33, stdev=65482.64, samples=3
   iops        : min=  516, max=  642, avg=585.33, stdev=63.95, samples=3
  lat (usec)   : 2=0.10%
  lat (msec)   : 2=0.10%, 4=0.10%, 10=0.39%, 20=0.59%, 50=32.42%
  lat (msec)   : 100=66.31%
  cpu          : usr=1.30%, sys=52.60%, ctx=6317, majf=0, minf=11
  IO depths    : 1=0.1%, 2=0.2%, 4=0.4%, 8=0.8%, 16=1.6%, 32=97.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=99.9%, 8=0.0%, 16=0.0%, 32=0.1%, 64=0.0%, >=64=0.0%
     issued rwts: total=0,1024,0,0 short=0,0,0,0 dropped=0,0,0,0
     latency   : target=0, window=0, percentile=100.00%, depth=32

Run status group 0 (all jobs):
  WRITE: bw=604MiB/s (633MB/s), 604MiB/s-604MiB/s (633MB/s-633MB/s), io=1024MiB (1074MB), run=1695-1695msec
-----------------------------------
--- FIO 1GB Sequential Read Test (Iteration 1) ---
seqread: (g=0): rw=read, bs=(R) 1024KiB-1024KiB, (W) 1024KiB-1024KiB, (T) 1024KiB-1024KiB, ioengine=libaio, iodepth=32
fio-3.33
Starting 1 process

seqread: (groupid=0, jobs=1): err= 0: pid=24201: Sun Nov  9 20:12:52 2025
  read: IOPS=3190, BW=3190MiB/s (3345MB/s)(1024MiB/321msec)
    slat (usec): min=18, max=2162, avg=40.99, stdev=68.60
    clat (usec): min=2731, max=16496, avg=9867.48, stdev=1295.50
     lat (usec): min=2771, max=16530, avg=9908.47, stdev=1288.13
    clat percentiles (usec):
     |  1.00th=[ 8160],  5.00th=[ 8291], 10.00th=[ 8356], 20.00th=[ 8455],
     | 30.00th=[ 9765], 40.00th=[ 9896], 50.00th=[10159], 60.00th=[10159],
     | 70.00th=[10290], 80.00th=[10421], 90.00th=[10552], 95.00th=[11600],
     | 99.00th=[14877], 99.50th=[15664], 99.90th=[16319], 99.95th=[16450],
     | 99.99th=[16450]
  lat (msec)   : 4=0.49%, 10=41.99%, 20=57.52%
  cpu          : usr=0.00%, sys=12.81%, ctx=1018, majf=0, minf=537
  IO depths    : 1=0.1%, 2=0.2%, 4=0.4%, 8=0.8%, 16=1.6%, 32=97.0%, >=64=0.0%
     submit    : 0=0.0%, 4=100.0%, 8=0.0%, 16=0.0%, 32=0.0%, 64=0.0%, >=64=0.0%
     complete  : 0=0.0%, 4=99.9%, 8=0.0%, 16=0.0%, 32=0.1%, 64=0.0%, >=64=0.0%
     issued rwts: total=1024,0,0,0 short=0,0,0,0 dropped=0,0,0,0
     latency   : target=0, window=0, percentile=100.00%, depth=32

Run status group 0 (all jobs):
   READ: bw=3190MiB/s (3345MB/s), 3190MiB/s-3190MiB/s (3345MB/s-3345MB/s), io=1024MiB (1074MB), run=321-321msec

seq,rand write는   "WRITE: bw=604MiB/s (633MB/s), 604MiB/s-604MiB/s (633MB/s-633MB/s), io=1024MiB (1074MB), run=1695-1695msec" 에서 bw=604MiB/s 에서 604 이 결과입니다.
seq,rand read는    "READ: bw=3190MiB/s (3345MB/s), 3190MiB/s-3190MiB/s (3345MB/s-3345MB/s), io=1024MiB (1074MB), run=321-321msec" 에서 bw=3190MiB/s 에서 3190 이 결과입니다.

그 다음에 trace가 나옵니다.
    kworker/1:1H-175     [001] ..... 22218.735851: ufshcd_command: send_req: 1d84000.ufshc: tag: 7, DB: 0x0, size: 4096, IS: 0, LBA: 6221511, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
             cat-7511    [005] d.h1. 22218.736011: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 7, DB: 0x0, size: 4096, IS: 0, LBA: 6221511, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
    kworker/1:1H-175     [001] ..... 22218.736086: ufshcd_command: send_req: 1d84000.ufshc: tag: 0, DB: 0x0, size: 4096, IS: 0, LBA: 9669823, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
          <idle>-0       [005] d.h2. 22218.736234: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 0, DB: 0x0, size: 4096, IS: 0, LBA: 9669823, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
    kworker/1:1H-175     [001] ..... 22218.736311: ufshcd_command: send_req: 1d84000.ufshc: tag: 1, DB: 0x0, size: 4096, IS: 0, LBA: 6221509, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
          <idle>-0       [005] d.h2. 22218.736383: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 1, DB: 0x0, size: 4096, IS: 0, LBA: 6221509, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
    kworker/1:1H-175     [001] ..... 22218.736419: ufshcd_command: send_req: 1d84000.ufshc: tag: 2, DB: 0x0, size: 4096, IS: 0, LBA: 9669821, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
          <idle>-0       [005] d.h2. 22218.736488: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 2, DB: 0x0, size: 4096, IS: 0, LBA: 9669821, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1
    kworker/1:1H-175     [001] ..... 22218.736580: ufshcd_command: send_req: 1d84000.ufshc: tag: 3, DB: 0x0, size: 4096, IS: 0, LBA: 6168200, opcode: 0x28 (READ_10), group_id: 0x0, hwq_id: 1

### FIO Trace 파싱 예제
```bash
# -p 옵션으로 성능 벤치마크 모드 활성화
./trace -p fio_benchmark.log fio_result
```

파서가 자동으로:
- Iteration 1, 2, 3... 번호를 성능 결과에서 추출
- UFS, Block, UFSCustom trace 타입을 자동 감지
- 각 iteration별 폴더에 타입별 parquet 파일 생성

출력 예시:
- `fio_result/1/ufs_trace.parquet`
- `fio_result/1/block_trace.parquet`
- `fio_result/2/ufs_trace.parquet`
- `fio_result/3/ufscustom_trace.parquet`

성능 -> trace 성능 -> trace.... 반복이에요.

tiotest 평가 결과 데이터입니다.
seq write, read 
| Write        1024 MBs |    0.5 s | 1938.124 MB/s |   0.4 %  |  85.6 % |
| Read         1024 MBs |    0.2 s | 6455.599 MB/s |   2.8 %  |  96.3 % |
rand write, read
| Random Write  195 MBs |    0.4 s | 463.654 MB/s |   1.7 %  |  95.4 % |
| Random Read   195 MBs |    0.0 s | 6270.265 MB/s |   0.0 %  | 387.8 % |

그 다음에 trace가 나옵니다.
   kworker/u17:2-7053    [000] ..... 22386.148085: ufshcd_command: send_req: 1d84000.ufshc: tag: 49, DB: 0x0, size: 12288, IS: 0, LBA: 15628, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
          <idle>-0       [004] d.h2. 22386.148413: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 49, DB: 0x0, size: 12288, IS: 0, LBA: 15628, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
     jbd2/sda7-8-1005    [000] ..... 22386.916330: ufshcd_command: send_req: 1d84000.ufshc: tag: 50, DB: 0x0, size: 8192, IS: 0, LBA: 16274, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
          <idle>-0       [004] d.h2. 22386.916645: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 50, DB: 0x0, size: 8192, IS: 0, LBA: 16274, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
    kworker/0:0H-5183    [000] ..... 22386.916802: ufshcd_command: send_req: 1d84000.ufshc: tag: 51, DB: 0x0, size: -1, IS: 0, LBA: 0, opcode: 0x35 (SYNC), group_id: 0x0, hwq_id: 0
            adbd-5026    [004] d.h1. 22386.916928: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 51, DB: 0x0, size: -1, IS: 0, LBA: 0, opcode: 0x35 (SYNC), group_id: 0x0, hwq_id: 0
    kworker/4:3H-7530    [004] ..... 22386.917136: ufshcd_command: send_req: 1d84000.ufshc: tag: 52, DB: 0x0, size: 4096, IS: 0, LBA: 16276, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
          <idle>-0       [004] d.h2. 22386.917255: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 52, DB: 0x0, size: 4096, IS: 0, LBA: 16276, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0

### TIOtest Trace 파싱 예제
```bash
# -p 옵션으로 성능 벤치마크 모드 활성화
./trace -p tiotest_benchmark.log tiotest_result
```

파서가 자동으로:
- 로그에서 "Write 1024 MBs", "Read 1024 MBs" 등의 패턴에서 iteration 정보 추출
- 각 테스트 단계(seq write, seq read, rand write, rand read)별 trace 구분
- UFS, Block, UFSCustom 타입 자동 감지 및 처리

출력 예시:
- `tiotest_result/1/ufs_trace.parquet`
- `tiotest_result/1/block_trace.parquet`
- `tiotest_result/2/ufs_trace.parquet`

성능 -> trace 성능 -> trace.... 반복이에요.

iozone 평가 결과 데이터 입니다.
seq write, read 
                                                                    random    random      bkwd     record     stride                                        
              kB  reclen    write    rewrite      read    reread      read     write      read    rewrite       read    fwrite  frewrite     fread   freread
         1048576    1024   2226634         0   9045852         0                                                                                  

write는 2226634 , read는 9045852 입니다.

rand write, read

rand read : Parent sees throughput for 8 random readers 	=  257470.02 kB/sec 에서 257470.02
rand write : Parent sees throughput for 8 random writers 	=  331848.00 kB/sec 에서 331848.00

그 다음에 trace가 나옵니다.
   kworker/u17:0-7337    [005] ..... 22713.388868: ufshcd_command: send_req: 1d84000.ufshc: tag: 36, DB: 0x0, size: 12288, IS: 0, LBA: 6400955, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
   kworker/u17:0-7337    [005] ..... 22713.388886: ufshcd_command: send_req: 1d84000.ufshc: tag: 37, DB: 0x0, size: 327680, IS: 0, LBA: 25295124, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
          <idle>-0       [004] d.h2. 22713.389049: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 36, DB: 0x0, size: 12288, IS: 0, LBA: 6400955, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
    kworker/5:1H-7525    [005] ..... 22713.389071: ufshcd_command: send_req: 1d84000.ufshc: tag: 38, DB: 0x0, size: -1, IS: 0, LBA: 0, opcode: 0x35 (SYNC), group_id: 0x0, hwq_id: 5
    kworker/5:1H-7525    [005] ..... 22713.389077: ufshcd_command: send_req: 1d84000.ufshc: tag: 39, DB: 0x0, size: 12288, IS: 0, LBA: 6406406, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 5
    kworker/5:1H-7525    [005] ..... 22713.389083: ufshcd_command: send_req: 1d84000.ufshc: tag: 40, DB: 0x0, size: 61440, IS: 0, LBA: 6406409, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 5
    kworker/5:1H-7525    [005] ..... 22713.389088: ufshcd_command: send_req: 1d84000.ufshc: tag: 41, DB: 0x0, size: 8192, IS: 0, LBA: 6375026, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 5
    kworker/5:1H-7525    [005] ..... 22713.389091: ufshcd_command: send_req: 1d84000.ufshc: tag: 42, DB: 0x0, size: 4096, IS: 0, LBA: 6396513, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 5
          <idle>-0       [004] d.h2. 22713.389146: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 37, DB: 0x0, size: 327680, IS: 0, LBA: 25295124, opcode: 0x2a (WRITE_10), group_id: 0x0, hwq_id: 0
          <idle>-0       [003] d.h2. 22713.389460: ufshcd_command: complete_rsp: 1d84000.ufshc: tag: 38, DB: 0x0, size: -1, IS: 0, LBA: 0, opcode: 0x35 (SYNC), group_id: 0x0, hwq_id: 5

### IOzone Trace 파싱 예제
```bash
# -p 옵션으로 성능 벤치마크 모드 활성화
./trace -p iozone_benchmark.log iozone_result
```

파서가 자동으로:
- "Parent sees throughput for 8 random readers" 등의 패턴에서 iteration 추출
- Sequential/Random Write/Read 구분
- UFS, Block, UFSCustom 타입 자동 감지

출력 예시:
- `iozone_result/1/ufs_trace.parquet`
- `iozone_result/1/block_trace.parquet`
- `iozone_result/1/ufscustom_trace.parquet`
- `iozone_result/2/ufs_trace.parquet`

성능 -> trace 성능 -> trace.... 반복이에요.

## 통합 실행 스크립트 예제

```bash
#!/bin/bash

BENCHMARK_LOG="benchmark_all.log"

# Trace 수집 시작
echo "trace-cmd record -e ufs* -e block* -e ufscustom* &" > /dev/null

# 모든 성능 BM 결과와 trace를 하나의 로그에 수집
{
    echo "=== FIO Benchmark Start ==="
    for i in {1..5}; do
        echo "--- FIO Sequential Write Test (Iteration $i) ---"
        fio --name=seqwrite --bs=1M --iodepth=32 --rw=write --size=1G
        cat /sys/kernel/debug/tracing/trace
        echo > /sys/kernel/debug/tracing/trace
        
        echo "--- FIO Sequential Read Test (Iteration $i) ---"
        fio --name=seqread --bs=1M --iodepth=32 --rw=read --size=1G
        cat /sys/kernel/debug/tracing/trace
        echo > /sys/kernel/debug/tracing/trace
    done

    echo "=== TIOtest Benchmark Start ==="
    tiotest -f 1024 -t 1 -d /data
    cat /sys/kernel/debug/tracing/trace
    echo > /sys/kernel/debug/tracing/trace

    echo "=== IOzone Benchmark Start ==="
    iozone -a -s 1024M
    cat /sys/kernel/debug/tracing/trace
    
} > $BENCHMARK_LOG

# 전체 로그 파일을 한 번에 파싱 (-p 옵션 필수)
echo "Parsing all benchmark traces..."
./trace -p $BENCHMARK_LOG benchmark_output

echo "All benchmarks and trace parsing completed"
echo "Results are in benchmark_output/1/, benchmark_output/2/, ..."
```

## 주의사항

1. **명령어 형식**: `./trace -p <로그_파일_경로> <출력_파일_접두사>` 형식으로 사용합니다 (`-p` 옵션 필수).
2. **자동 감지**: Iteration 번호와 trace 타입(ufs/block/ufscustom)은 로그 파일에서 자동으로 감지됩니다.
3. **로그 형식**: 성능 BM 결과와 trace 데이터가 혼재된 로그 파일을 입력으로 사용합니다.
4. **출력 폴더 구조**: `<출력_파일_접두사>/1/`, `<출력_파일_접두사>/2/`, ..., `<출력_파일_접두사>/10/` 등 iteration 번호별 하위 폴더가 자동 생성됩니다.
5. **CSV 결과**: 성능 BM 결과는 별도 CSV 파일로 저장되며, trace 파싱 결과는 parquet 형식으로 저장됩니다.
6. **다중 타입 지원**: 하나의 로그에 ufs, block, ufscustom trace가 모두 있어도 각각 별도로 파싱되어 저장됩니다.
7. **기존 파싱 로직**: 기존에 사용하던 trace 파싱 방식을 그대로 사용하여 호환성을 유지합니다.

## Trace 타입별 파싱 규칙

로그 라인에서 다음 패턴으로 타입을 자동 감지합니다:
- **UFS**: `ufshcd_command` 패턴 감지
- **Block**: `block_rq_*` 패턴 감지  
- **UFSCustom**: 커스텀 trace 이벤트 패턴 감지

각 타입별로 기존 파서(`parsers/log.rs`, `parsers/log_high_perf.rs` 등)를 활용하여 처리합니다.