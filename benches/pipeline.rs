#[path = "pipeline/domain.rs"]
mod domain;
#[path = "pipeline/io.rs"]
mod io;
#[path = "pipeline/support.rs"]
mod support;

use criterion::{criterion_group, criterion_main, Criterion};

fn pipeline_benchmarks(c: &mut Criterion) {
    domain::benchmark(c);
    io::benchmark(c);
}

criterion_group!(benches, pipeline_benchmarks);
criterion_main!(benches);
