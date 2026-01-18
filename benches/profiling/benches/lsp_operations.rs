use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::path::Path;

mod workload;
mod operations;
mod fixtures;

fn benchmark_completion(c: &mut Criterion) {
    let mut group = c.benchmark_group("completion");

    let queries = fixtures::load_test_queries();
    for (name, query) in queries.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            query,
            |b, query| {
                b.iter(|| workload::run_completion_scenario(black_box(query)))
            },
        );
    }

    group.finish();
}

fn benchmark_hover(c: &mut Criterion) {
    let mut group = c.benchmark_group("hover");

    let queries = fixtures::load_test_queries();
    for (name, query) in queries.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            query,
            |b, query| {
                b.iter(|| workload::run_hover_scenario(black_box(query)))
            },
        );
    }

    group.finish();
}

fn benchmark_diagnostics(c: &mut Criterion) {
    let mut group = c.benchmark_group("diagnostics");

    let queries = fixtures::load_test_queries();
    for (name, query) in queries.iter() {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            query,
            |b, query| {
                b.iter(|| workload::run_diagnostics_scenario(black_box(query)))
            },
        );
    }

    group.finish();
}

fn benchmark_editing_session(c: &mut Criterion) {
    c.bench_function("editing_session", |b| {
        b.iter(|| workload::simulate_editing_session(black_box()))
    });
}

criterion_group!(
    benches,
    benchmark_completion,
    benchmark_hover,
    benchmark_diagnostics,
    benchmark_editing_session
);
criterion_main!(benches);
