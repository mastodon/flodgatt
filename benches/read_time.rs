use criterion::black_box;
use criterion::criterion_group;
use criterion::criterion_main;
use criterion::Criterion;
use log;
use std::time;

fn simple_read_time(_: &str) {
    let start_time = time::Instant::now();

    if start_time.elapsed() > time::Duration::from_millis(20) {
        log::warn!("Polling took: {:?}", start_time.elapsed());
    }
}

fn criterion_benchmark2(c: &mut Criterion) {
    c.bench_function("read elapsed time", |b| {
        b.iter(|| simple_read_time(black_box("foo")))
    });
}

criterion_group!(benches, criterion_benchmark2);
criterion_main!(benches);
