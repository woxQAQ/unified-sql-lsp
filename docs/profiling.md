# Performance Profiling Guide

This guide explains how to run, interpret, and use the performance profiling infrastructure for Unified SQL LSP.

## Quick Start

### Run All Benchmarks

```bash
# Quick benchmark suite (parsing, semantic, completion)
make benchmark

# Or with cargo directly
cargo bench --benches completion,parsing,semantic
```

### Run Complete Profiling Suite

```bash
# Full profiling with report generation
make profile-all

# Or directly
./scripts/profiling/run_all.sh
```

This generates:
- Criterion HTML reports with statistical analysis
- Flamegraph (if `cargo-flamegraph` is installed)
- Memory profiles (if run with `--features dhat`)
- Summary markdown file

### View Results

```bash
# Open latest report
./scripts/profiling/generate_report.sh

# Or manually open HTML
xdg-open target/profiling-reports/*/criterion/index.html
```

## Understanding the Output

### Criterion Reports

Criterion generates detailed HTML reports in `target/criterion/`:

- **Index page**: Overview of all benchmarks
- **Per-benchmark pages**: Detailed statistics including:
  - Mean, median, standard deviation
  - p95, p99 percentiles
  - Confidence intervals
  - Comparison across runs (if baselines exist)

#### Interpreting Statistics

- **Mean**: Average execution time across all iterations
- **Median**: Middle value (less affected by outliers)
- **Std Dev**: Variability - lower is more consistent
- **p95**: 95th percentile - 95% of operations complete faster than this
- **p99**: 99th percentile - tail latency indicator

For LSP completion, **p95 is most important** - it represents the "typical worst case" users experience.

#### Comparing Baselines

```bash
# Save current performance as baseline
cargo bench -- --save-baseline before-optimization

# Make changes...

# Compare against baseline
cargo bench -- --baseline before-optimization
```

Criterion shows:
- Percentage change (green for improvement, red for regression)
- Statistical significance check
- Confidence intervals for the difference

### Flamegraphs

Flamegraphs show CPU time distribution as an interactive SVG.

#### How to Read

- **Y-axis**: Stack depth (call hierarchy)
- **X-axis**: Time (not sequential, sorted by hot path)
- **Width**: Time spent in that function
- **Colors**: Random (warm colors often indicate more time)

#### What to Look For

1. **Wide rectangles**: Functions consuming most CPU time
2. **Tall stacks**: Deep call hierarchies (potential for optimization)
3. **Surprising names**: Unexpected functions in hot path

#### Common Patterns

- **Parsing hotspots**: Tree-sitter operations, CST traversal
- **Semantic bottlenecks**: Scope building, resolution loops
- **Catalog delays**: Database queries, connection overhead
- **Allocation hotspots**: Memory allocation patterns

### Memory Profiling (DHAT)

DHAT shows heap allocation patterns:

- **Total bytes**: Overall heap usage
- **Live bytes**: Memory retained vs temporary
- **Allocation count**: Number of allocations (fewer is better)

## Benchmark Scenarios

### Query Complexity Levels

**Simple** (10-50 tokens):
- Single table SELECT
- Basic WHERE clause
- Expected parse: <100µs
- Expected completion: <10ms

**Medium** (50-150 tokens):
- 2-3 JOINs
- Subqueries
- Aggregates (GROUP BY, HAVING)
- Expected parse: <500µs
- Expected completion: <50ms

**Complex** (150+ tokens):
- CTEs
- Window functions
- Nested subqueries
- Expected parse: <1ms
- Expected completion: <100ms

### Completion Pipeline Stages

```
┌─────────────────────────────────────────┐
│  completion/total (target: <50ms p95)  │
├─────────────────────────────────────────┤
│  completion/parse (target: <1ms)        │
│  completion/lowering (target: <500µs)   │
│  completion/semantic (target: <5ms)     │
│    └─ semantic/scope_building           │
│    └─ semantic/alias_resolution         │
│    └─ semantic/column_resolution        │
│  completion/catalog (target: <1ms)      │
│    └─ catalog/cache_hit (cached)        │
│    └─ catalog/query_execution (uncached)│
│  completion/render (target: <1ms)       │
└─────────────────────────────────────────┘
```

## Writing New Benchmarks

### Template

```rust
// crates/lsp/benches/my_benchmark.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_my_operation(c: &mut Criterion) {
    c.bench_function("my_operation", |b| {
        b.iter(|| {
            let result = my_function(black_box(input));
            black_box(result)
        });
    });
}

criterion_group!(benches, bench_my_operation);
criterion_main!(benches);
```

### Adding to Suite

1. Create file in `crates/lsp/benches/`
2. Add to `benches/mod.rs`: `mod my_benchmark;`
3. Add to `Cargo.toml`:
   ```toml
   [[bench]]
   name = "my_benchmark"
   harness = false
   ```

### Best Practices

- **Use `black_box`**: Prevent compiler from optimizing away
- **Warmup**: Criterion handles this automatically
- **Sample size**: Default is fine, reduce for slow operations
- **Async**: Use `b.to_async(&rt).iter()` for async code
- **Throughput**: Add `.throughput(Throughput::Bytes(size))` for I/O

## Troubleshooting

### Benchmarks Fail to Compile

```bash
# Check with full output
cargo check --benches --bench my_benchmark --verbose

# Common issues:
# - Missing feature flags
# - Wrong function signature (iter vs iter_async)
# - Missing black_box usage
```

### Flamegraph Not Generated

```bash
# Install flamegraph
cargo install flamegraph

# Verify installation
cargo flamegraph --version

# Run manually for more output
cargo flamegraph --bench completion_pipeline
```

### Inconsistent Benchmark Results

- **Close other applications**: Reduce system noise
- **Increase sample size**: `.sample_size(100)` in Criterion config
- **Use warmup**: Criterion does this, but may need more
- **Check frequency scaling**: CPU turbo boost causes variance
  ```bash
  # Disable turbo boost for consistent results (Linux)
  sudo cpupower frequency-set -g performance
  ```

### Catalog Benchmarks Fail

```bash
# Set connection string
export MYSQL_TEST_CONNECTION_STRING="mysql://localhost:3306/test"
export POSTGRES_TEST_CONNECTION_STRING="postgresql://localhost:5432/test"

# Or skip catalog benchmarks
cargo bench --benches completion,parsing,semantic
```

## Performance Targets

Based on user perception thresholds:

| Metric | Target | Rationale |
|--------|--------|-----------|
| Completion p95 | <50ms | User doesn't notice lag |
| Parse | <1ms | Should be negligible |
| Semantic | <5ms | Complex but fast |
| Catalog (cached) | <1ms | Cache hits are fast |
| Catalog (uncached) | <50ms | Acceptable for schema queries |

## Next Steps

After profiling:

1. **Identify the bottleneck**: Use Criterion data and flamegraphs
2. **Propose optimization**: Create design document
3. **Implement**: Use PERF-002, PERF-003, or PERF-004 as appropriate
4. **Verify**: Re-run benchmarks to measure improvement
5. **Document**: Update this guide with lessons learned

## Resources

- [Criterion User Guide](https://bheisler.github.io/criterion.rs/book/index.html)
- [Flamegraph Guide](http://www.brendangregg.com/flamegraphs.html)
- [DHAT Documentation](https://www.valgrind.org/docs/manual/dh-manual.html)
- PERF-001 Design: `docs/plans/2025-01-18-perf-001-profiling-design.md`
