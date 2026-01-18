//! Memory allocation profiling benchmarks
//!
//! Uses basic operations to identify memory patterns.

use criterion::{Criterion, black_box, criterion_group, criterion_main};

fn bench_memory_1000_operations(c: &mut Criterion) {
    c.bench_function("memory/1000_operations", |b| {
        b.iter(|| {
            let mut data = Vec::new();
            for i in 0..1000 {
                data.push(black_box(i));
            }
            black_box(data);
        });
    });
}

fn bench_memory_string_allocation(c: &mut Criterion) {
    c.bench_function("memory/string_allocation", |b| {
        b.iter(|| {
            let query = "SELECT id, name, email FROM users WHERE active = TRUE";
            let _owned = query.to_string();
            black_box(query);
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_memory_1000_operations, bench_memory_string_allocation
);

criterion_main!(benches);
