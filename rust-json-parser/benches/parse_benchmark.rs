use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

const FIXTURES: &[&str] = &["sample", "twitter", "citm_catalog", "canada"];

fn criterion_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    for name in FIXTURES {
        let data = std::fs::read_to_string(format!("benches/data/{name}.json")).unwrap();
        group.throughput(Throughput::Bytes(data.len() as u64));
        group.bench_function(*name, |b| b.iter(|| rust_json_parser::parse(black_box(&data))));
    }
    group.finish();
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
