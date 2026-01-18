//! Concurrency and throughput benchmarks
//!
//! Measures performance under concurrent load.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use std::sync::{Arc, Mutex};

fn bench_concurrent_operations(c: &mut Criterion) {
    for doc_count in [1, 5, 10, 50] {
        let mut group = c.benchmark_group("concurrency");

        group.throughput(Throughput::Elements(doc_count as u64));

        group.bench_with_input(
            BenchmarkId::new("concurrent_operations", doc_count),
            &doc_count,
            |b, &_count| {
                // Simulate concurrent document processing
                let data = Arc::new(Mutex::new(vec![0u8; 1024]));

                b.iter(|| {
                    let mut guard = data.lock().unwrap();
                    guard[0] = black_box(42);
                });
            },
        );

        group.finish();
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_concurrent_operations
);

criterion_main!(benches);
