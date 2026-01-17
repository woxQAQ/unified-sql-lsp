# PERF-001: Performance Profiling & Benchmarking Infrastructure

**Status:** Design Approved
**Date:** 2025-01-18
**Author:** Claude (with user collaboration)
**Feature ID:** PERF-001

## Overview

This document describes the design for establishing a comprehensive performance profiling and benchmarking infrastructure for the Unified SQL LSP project. The goal is to establish a performance baseline, identify actual bottlenecks through data (not assumptions), and enable informed optimization decisions for subsequent performance features (PERF-002, PERF-003, PERF-004).

## Motivation

The current system works well, but we need to:
- Establish a proactive performance baseline before issues arise
- Identify actual bottlenecks in the completion pipeline (parse → lower → semantic → complete)
- Understand how query complexity, dialect differences, and concurrency affect performance
- Make data-driven decisions about where optimization efforts will have the most impact

## Architecture

### Directory Structure

```
crates/lsp/
├── benches/
│   ├── mod.rs                    # Benchmark entry point
│   ├── completion_pipeline.rs    # End-to-end completion benchmarks
│   ├── parsing.rs                # Tree-sitter parsing benchmarks
│   ├── semantic.rs               # Semantic analysis benchmarks
│   ├── catalog.rs                # Catalog query benchmarks
│   ├── concurrency.rs            # Multi-document concurrent benchmarks
│   ├── memory.rs                 # Memory allocation profiling
│   └── fixtures/                 # Test SQL files of varying complexity
│       ├── simple/
│       │   ├── mysql_01_single_table.sql
│       │   ├── mysql_02_basic_where.sql
│       │   └── postgresql_01_single_table.sql
│       ├── medium/
│       │   ├── mysql_01_joins.sql
│       │   ├── mysql_02_aggregates.sql
│       │   └── postgresql_01_joins.sql
│       └── complex/
│           ├── mysql_01_cte.sql
│           ├── mysql_02_nested_subqueries.sql
│           └── postgresql_01_cte.sql
├── Cargo.toml                    # Add criterion benchmark harness
└── src/
    └── profiling/                # New instrumentation module
        ├── mod.rs
        ├── timer.rs              # Fine-grained timing utilities
        └── stats.rs              # Statistics collection

scripts/profiling/
├── run_all.sh                    # Master profiling script
├── flamegraph.sh                 # Flamegraph generation
└── generate_report.sh            # Report aggregation
```

### Key Design Decisions

1. **Criterion Integration**: Use Criterion's benchmark harness via `[[bench]]` sections in `Cargo.toml`. This provides statistical analysis, warmup iterations, and automatic comparison between runs.

2. **Instrumentation Module**: Create a lightweight `profiling` module in `src/` that can be reused by both benchmarks and production code (feature-gated). Uses `std::time::Instant` for low-overhead timing.

3. **Fixture Organization**: SQL query fixtures organized by complexity and dialect for systematic performance testing.

4. **Fine-Grained Scopes**: Define timing scopes at the operation level within each stage to pinpoint exact bottlenecks.

## Benchmark Scenarios

### 1. Query Complexity Levels

Each benchmark tests three complexity levels:

**Simple**: Single-table SELECT with 3-5 columns, basic WHERE clause
```sql
SELECT id, name, email FROM users WHERE active = true
```

**Medium**: 2-3 table JOINs with aliases, subqueries in WHERE, aggregates
```sql
SELECT u.id, u.name, COUNT(o.id) as order_count
FROM users u
JOIN orders o ON u.id = o.user_id
WHERE u.created_at > '2024-01-01'
GROUP BY u.id, u.name
HAVING COUNT(o.id) > 5
```

**Complex**: CTEs, nested subqueries, window functions, multiple JOINs
```sql
WITH user_stats AS (
  SELECT user_id, COUNT(*) as total_orders, SUM(amount) as total_spent
  FROM orders
  GROUP BY user_id
)
SELECT u.id, u.name, us.total_orders, us.total_spent,
       RANK() OVER (ORDER BY us.total_spent DESC) as spending_rank
FROM users u
JOIN user_stats us ON u.id = us.user_id
WHERE us.total_orders > (
  SELECT AVG(total_orders) FROM user_stats
)
```

### 2. Completion Hot Path Benchmarks

`completion_pipeline.rs` benchmarks the full flow with fine-grained instrumentation:

```rust
fn bench_completion_simple(c: &mut Criterion) {
    let rt = runtime::Builder::new_multi_thread()
        .enable_all()
        .build().unwrap();

    let mut group = c.benchmark_group("completion/simple");

    group.bench_function("end_to_end", |b| {
        b.to_async(&rt).iter(|| async {
            let result = completion_operation(
                "SELECT | FROM users",  // cursor at |
                CompletionContext::SelectProjection
            ).await;
            black_box(result);
        });
    });

    // Fine-grained stage timings
    group.bench_function("parse_only", |b| {
        b.iter(|| bench_parse_stage("SELECT id FROM users"));
    });

    group.bench_function("lowering_only", |b| {
        b.iter(|| bench_lowering_stage(/* ... */));
    });

    group.bench_function("semantic_scope_building", |b| {
        b.iter(|| bench_semantic_scope(/* ... */));
    });

    group.bench_function("semantic_alias_resolution", |b| {
        b.iter(|| bench_semantic_aliases(/* ... */));
    });

    group.bench_function("semantic_column_resolution", |b| {
        b.iter(|| bench_semantic_columns(/* ... */));
    });

    group.bench_function("catalog_query_cached", |b| {
        b.iter(|| bench_catalog_cached(/* ... */));
    });

    group.bench_function("catalog_query_uncached", |b| {
        b.iter(|| bench_catalog_uncached(/* ... */));
    });

    group.bench_function("render_lsp_items", |b| {
        b.iter(|| bench_rendering(/* ... */));
    });
}
```

### 3. Dialect Comparison

Each benchmark runs against both MySQL and PostgreSQL grammars:

```rust
fn bench_parsing_mysql_vs_postgres(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing/dialect_comparison");

    for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
        for complexity in ["simple", "medium", "complex"] {
            group.bench_with_input(
                BenchmarkId::new(dialect.as_str(), complexity),
                &(dialect, complexity),
                |b, (dialect, complexity)| {
                    b.iter(|| {
                        let query = load_fixture(*dialect, *complexity);
                        parse_query(*dialect, query)
                    });
                }
            );
        }
    }
}
```

### 4. Throughput & Concurrency

`concurrency.rs` tests multi-document scenarios:

```rust
fn bench_concurrent_completions(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrency");

    for doc_count in [1, 5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_docs", doc_count),
            doc_count,
            |b, &count| {
                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..count)
                        .map(|_| completion_operation(/* ... */))
                        .collect();
                    join_all(tasks).await;
                });
            }
        );
    }
}
```

## Fine-Grained Instrumentation

### Core Instrumentation Module

```rust
// crates/lsp/src/profiling/timer.rs
use std::time::Instant;

/// A scoped timer that measures execution time
/// Automatically records when dropped
pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
        }
    }
}

impl Drop for ScopedTimer {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        // Record to thread-local storage or global collector
        TimingCollector::record(self.name, duration);
    }
}

/// Macro for easy scoped timing
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        let _timer = $crate::profiling::ScopedTimer::new($name);
    };
}
```

### Usage in Completion Pipeline

```rust
// crates/lsp/src/completion/mod.rs
use crate::profiling::timed_scope;

impl CompletionEngine {
    pub async fn complete(&self, params: &CompletionParams) -> Vec<CompletionItem> {
        timed_scope!("completion/total");

        // Parse stage
        let (context, cst) = {
            timed_scope!("completion/parse");
            self.detect_context_and_parse(params).await
        };

        // Lowering stage
        let ir = {
            timed_scope!("completion/lowering");
            self.lower_cst_to_ir(&cst).await
        };

        // Semantic stage with sub-scopes
        let semantic_info = {
            timed_scope!("completion/semantic");
            self.build_semantic_info(&ir).await
            // Internally will have:
            // - semantic/scope_building
            // - semantic/alias_resolution
            // - semantic/column_resolution
        };

        // Catalog stage
        let candidates = {
            timed_scope!("completion/catalog");
            self.fetch_candidates(&semantic_info).await
            // Internally:
            // - catalog/connection_acquire
            // - catalog/query_execution
            // - catalog/cache_lookup
        };

        // Rendering stage
        let items = {
            timed_scope!("completion/render");
            self.render_lsp_items(candidates)
        };

        items
    }
}
```

### Feature-Gated Compilation

```rust
// crates/lsp/Cargo.toml
[features]
default = []
profiling = []  # Enable instrumentation
flamegraph = ["dep:flamegraph", "profiling"]
dhat = ["dep:dhat"]

# crates/lsp/src/profiling/mod.rs
#[cfg(feature = "profiling")]
mod timer;
#[cfg(feature = "profiling")]
mod stats;

#[cfg(feature = "profiling")]
pub use timer::{ScopedTimer, timed_scope};

#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        // Compile to nothing - zero overhead
    };
}
```

### Statistics Collection

```rust
// crates/lsp/src/profiling/stats.rs
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::Duration;

thread_local! {
    static TIMINGS: RefCell<HashMap<&'static str, Vec<Duration>>> =
        RefCell::new(HashMap::new());
}

pub struct TimingReport {
    pub scopes: HashMap<&'static str, TimingStats>,
}

pub struct TimingStats {
    pub count: usize,
    pub total: Duration,
    pub min: Duration,
    pub max: Duration,
    pub avg: Duration,
    pub p95: Duration,
    pub p99: Duration,
}

impl TimingReport {
    pub fn print_summary(&self) {
        println!("\n=== Performance Profile ===");
        for (scope, stats) in &self.scopes {
            println!(
                "{:30} | calls: {:6} | avg: {:8.2?} | p95: {:8.2?} | p99: {:8.2?}",
                scope, stats.count, stats.avg, stats.p95, stats.p99
            );
        }
    }
}
```

## Profiling Tools Integration

### 1. Flamegraph Generation

```bash
#!/bin/bash
# scripts/profiling/flamegraph.sh

set -e

# Build with profiling instrumentation
cargo build --release --features profiling

# Run the binary with flamegraph profiler
# Generates flamegraph.svg in current directory
cargo flamegraph --bin unified-sql-lsp -- \
    --test-scenario completion-heavy \
    --duration 30s

echo "Flamegraph generated: flamegraph.svg"
```

`Cargo.toml` additions:
```toml
[dependencies]
flamegraph = { version = "0.6", optional = true }
```

### 2. Memory Profiling with DHAT

```rust
// crates/lsp/benches/memory.rs
#[cfg(feature = "dhat")]
fn bench_memory_completion_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/completion");

    group.bench_function("1000_completions", |b| {
        b.iter(|| {
            let _dhat_guard = dhat::Profiler::new();

            // Run realistic completion workload
            for _ in 0..1000 {
                let result = completion_operation(
                    COMPLEX_QUERY,
                    CompletionContext::SelectProjection
                ).await;
                black_box(result);
            }
        });
    });
}
```

`Cargo.toml` additions:
```toml
[dependencies]
dhat = { version = "0.3", optional = true }
```

### 3. Automated Profiling Scripts

**Master Script** (`scripts/profiling/run_all.sh`):
```bash
#!/bin/bash
# Run complete profiling suite

set -e

echo "=== Running Criterion Benchmarks ==="
cargo bench --benches completion,parsing,semantic,catalog,concurrency \
    --save-baseline main

echo ""
echo "=== Generating Flamegraph ==="
./scripts/profiling/flamegraph.sh

echo ""
echo "=== Memory Profiling ==="
cargo bench --benches memory --features dhat

echo ""
echo "=== Generating Report ==="
./scripts/profiling/generate_report.sh
```

**Report Generation** (`scripts/profiling/generate_report.sh`):
```bash
#!/bin/bash

REPORT_DIR="target/profiling-reports/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$REPORT_DIR"

# Copy Criterion HTML reports
cp -r target/criterion "$REPORT_DIR/"

# Copy flamegraph
cp flamegraph.svg "$REPORT_DIR/"

# Generate summary
cat > "$REPORT_DIR/SUMMARY.md" <<EOF
# Performance Profiling Report
Generated: $(date)

## Benchmark Results
- [Criterion Reports](criterion/index.html)
- [Flamegraph](flamegraph.svg)

## Key Findings
$(cargo bench -- --output-format short | tee -a "$REPORT_DIR/SUMMARY.md")
EOF

echo "Report generated: $REPORT_DIR"
echo "Open with: xdg-open $REPORT_DIR/criterion/index.html"
```

## Deliverables

### 1. Code Implementation

- **Benchmark Suite**: 6 benchmark files (~1000 LOC total)
  - `completion_pipeline.rs` - End-to-end completion with stage breakdown
  - `parsing.rs` - Tree-sitter parsing performance
  - `semantic.rs` - Semantic analysis performance
  - `catalog.rs` - Catalog query performance
  - `concurrency.rs` - Multi-document concurrent operations
  - `memory.rs` - Memory allocation profiling

- **Instrumentation Module**: `src/profiling/` (~300 LOC)
  - `mod.rs` - Public API and feature gating
  - `timer.rs` - ScopedTimer and macros
  - `stats.rs` - Statistics collection and reporting

- **Test Fixtures**: 15-20 SQL query files
  - 5 simple queries × 2 dialects (MySQL, PostgreSQL)
  - 5 medium queries × 2 dialects
  - 5 complex queries × 2 dialects

### 2. Build Configuration

Update `crates/lsp/Cargo.toml`:
```toml
[[bench]]
name = "completion_pipeline"
harness = false

[[bench]]
name = "parsing"
harness = false

[[bench]]
name = "semantic"
harness = false

[[bench]]
name = "catalog"
harness = false

[[bench]]
name = "concurrency"
harness = false

[[bench]]
name = "memory"
harness = false

[dev-dependencies]
criterion = "0.5"

[dependencies]
flamegraph = { version = "0.6", optional = true }
dhat = { version = "0.3", optional = true }

[features]
default = []
profiling = []
flamegraph = ["dep:flamegraph", "profiling"]
dhat = ["dep:dhat"]
```

### 3. Profiling Scripts

- `scripts/profiling/run_all.sh` - Master profiling script
- `scripts/profiling/flamegraph.sh` - Flamegraph generation
- `scripts/profiling/generate_report.sh` - Report aggregation
- All scripts with execute permissions

### 4. Documentation

- **User Guide**: `docs/profiling.md`
  - Quick start guide
  - Understanding Criterion output
  - Interpreting flamegraphs
  - Benchmark scenario descriptions
  - Writing new benchmarks
  - Troubleshooting guide

- **Inline Documentation**: Comprehensive comments in `profiling/` module
- **Fixture Documentation**: Comments explaining what each query tests

### 5. Makefile Integration

```makefile
# Quick benchmark command
benchmark:
	cargo bench --benches completion_pipeline,parsing,semantic

# Full profiling suite
profile-all:
	./scripts/profiling/run_all.sh

# Flamegraph only
flamegraph:
	./scripts/profiling/flamegraph.sh
```

## Developer Workflow

```bash
# Quick benchmark run (development)
cargo bench --bench completion_pipeline

# Full profiling suite (generates report)
./scripts/profiling/run_all.sh

# Compare against previous run
cargo bench --bench completion_pipeline -- --save-baseline after-optimization
cargo bench --bench completion_pipeline -- --baseline main

# Generate flamegraph manually
cargo flamegraph --bench completion_pipeline

# Memory profiling
cargo bench --bench memory --features dhat

# View HTML report
xdg-open target/criterion/report/index.html
```

## Success Criteria

PERF-001 is complete when:

✅ All 6 benchmark files exist and compile without errors
✅ `cargo bench` runs successfully and generates Criterion HTML reports
✅ Flamegraphs can be generated with `./scripts/profiling/flamegraph.sh`
✅ Fine-grained instrumentation shows breakdown of completion pipeline stages
✅ Benchmarks cover both MySQL and PostgreSQL dialects
✅ At least 3 complexity levels (simple/medium/complex) are tested
✅ Documentation explains how to run and interpret results
✅ Makefile includes `make benchmark` target for convenience
✅ Profile data identifies actual bottlenecks (not assumptions)
✅ Baseline metrics are established for future comparison

## Expected Performance Targets

Based on similar LSP implementations, we expect:

- **Parse**: <1ms for medium queries
- **Lowering**: <0.5ms
- **Semantic**: 1-5ms (depends on catalog)
- **Catalog**: 5-50ms uncached, <1ms cached
- **Render**: <1ms
- **Total Completion**: <50ms p95 (user perception threshold)

These targets will be validated or adjusted based on actual benchmark data.

## Next Steps After PERF-001

Once PERF-001 establishes baselines and identifies bottlenecks:

1. **PERF-002**: If semantic analysis is the bottleneck → Implement async/concurrent semantic analysis
2. **PERF-003**: If catalog queries are the bottleneck → Implement batch queries and preloading
3. **PERF-004**: Targeted optimization of the identified bottleneck with before/after comparison

The key principle: **Optimize based on data, not assumptions.**

## Dependencies

- LSP-002 (Document sync and incremental parsing) ✅ done
- SEMANTIC-002 (Semantic analyzer) ✅ done
- TEST-002 (Integration tests) ✅ done

All dependencies are complete, so PERF-001 can proceed immediately.

