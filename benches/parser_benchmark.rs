use criterion::{black_box, criterion_group, criterion_main, Criterion};
use trace::parsers::log_common::{
    categorize_line_fast, parse_block_io_event, parse_ufs_event, parse_ufscustom_event,
    process_line_optimized,
};
use trace::utils::filter::{filter_data, FilterOptions};

// 샘플 UFS 라인
const UFS_LINE: &str = "    kworker/1:1H-175     [001] ..... 22218.735851: ufshcd_command: send_req: tag: 0, DB: 0x00000001 (UPIU Software), size: 4096, LBA: 12345678, opcode: 0x2a, group_id: 0x00, hwq_id: 0";

// 샘플 Block 라인
const BLOCK_LINE: &str = "  test-123   [000] ..... 12345.678901: block_rq_issue: 8,0 R 4096 () 1000 + 8 [test]";

// 샘플 UFSCUSTOM 라인
const UFSCUSTOM_LINE: &str = "0x28,1000,8,123.456,123.789";

fn bench_categorize_line(c: &mut Criterion) {
    let mut group = c.benchmark_group("line_categorization");

    group.bench_function("ufs_line", |b| {
        b.iter(|| categorize_line_fast(black_box(UFS_LINE)))
    });

    group.bench_function("block_line", |b| {
        b.iter(|| categorize_line_fast(black_box(BLOCK_LINE)))
    });

    group.bench_function("ufscustom_line", |b| {
        b.iter(|| categorize_line_fast(black_box(UFSCUSTOM_LINE)))
    });

    group.bench_function("empty_line", |b| {
        b.iter(|| categorize_line_fast(black_box("")))
    });

    group.finish();
}

fn bench_parse_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_parsing");

    group.bench_function("parse_ufs", |b| {
        b.iter(|| parse_ufs_event(black_box(UFS_LINE)))
    });

    group.bench_function("parse_block", |b| {
        b.iter(|| parse_block_io_event(black_box(BLOCK_LINE)))
    });

    group.bench_function("parse_ufscustom", |b| {
        b.iter(|| parse_ufscustom_event(black_box(UFSCUSTOM_LINE)))
    });

    group.finish();
}

fn bench_process_line_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_line");

    group.bench_function("ufs", |b| {
        b.iter(|| process_line_optimized(black_box(UFS_LINE)))
    });

    group.bench_function("block", |b| {
        b.iter(|| process_line_optimized(black_box(BLOCK_LINE)))
    });

    group.bench_function("ufscustom", |b| {
        b.iter(|| process_line_optimized(black_box(UFSCUSTOM_LINE)))
    });

    group.bench_function("no_match", |b| {
        b.iter(|| process_line_optimized(black_box("some random text")))
    });

    group.finish();
}

fn bench_filter(c: &mut Criterion) {
    // 테스트용 UFS 데이터 생성
    let ufs_data: Vec<trace::UFS> = (0..10000)
        .map(|i| trace::UFS {
            time: i as f64 * 0.001,
            process: "test".to_string().into_boxed_str(),
            cpu: (i % 8) as u32,
            action: if i % 2 == 0 {
                "send_req".to_string().into_boxed_str()
            } else {
                "complete_rsp".to_string().into_boxed_str()
            },
            tag: (i % 32) as u32,
            opcode: "0x2a".to_string().into_boxed_str(),
            lba: i as u64 * 8,
            size: 8,
            groupid: 0,
            hwqid: 0,
            qd: (i % 32) as u32,
            dtoc: (i % 100) as f64 * 0.01,
            ctoc: 0.0,
            ctod: 0.0,
            continuous: false,
            aligned: true,
        })
        .collect();

    let mut group = c.benchmark_group("filter");

    group.bench_function("no_filter", |b| {
        let filter = FilterOptions::default();
        b.iter(|| filter_data(black_box(ufs_data.clone()), &filter))
    });

    group.bench_function("time_filter", |b| {
        let filter = FilterOptions {
            start_time: 1.0,
            end_time: 5.0,
            ..Default::default()
        };
        b.iter(|| filter_data(black_box(ufs_data.clone()), &filter))
    });

    group.bench_function("cpu_filter", |b| {
        let mut filter = FilterOptions {
            cpu_list: vec![0, 1, 2, 3],
            ..Default::default()
        };
        filter.build_cpu_set();
        b.iter(|| filter_data(black_box(ufs_data.clone()), &filter))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_categorize_line,
    bench_parse_events,
    bench_process_line_optimized,
    bench_filter,
);
criterion_main!(benches);
