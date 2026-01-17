# PERF-001: Performance Profiling & Benchmarking Infrastructure Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish a comprehensive performance profiling and benchmarking infrastructure to identify actual bottlenecks in the completion pipeline through data-driven analysis.

**Architecture:**
- Criterion-based benchmark suite in `crates/lsp/benches/` with 6 benchmark files covering parsing, semantic analysis, catalog queries, completion pipeline, concurrency, and memory
- Lightweight instrumentation module in `crates/lsp/src/profiling/` with feature-gated compilation for zero-overhead in production
- SQL query fixtures organized by complexity (simple/medium/complex) and dialect (MySQL/PostgreSQL)
- Automation scripts for flamegraphs, memory profiling, and report generation

**Tech Stack:**
- Criterion 0.5 for statistical benchmarking
- flamegraph 0.6 for CPU profiling
- dhat 0.3 for memory profiling
- std::time::Instant for low-overhead timing
- tokio for async runtime in benchmarks

---

## Prerequisites

Before starting, ensure you have:
- Rust 2024 edition with Criterion installed: `cargo install cargo-criterion`
- flamegraph CLI: `cargo install flamegraph`
- Read the design document: `docs/plans/2025-01-18-perf-001-profiling-design.md`
- Understand the completion pipeline: `crates/lsp/src/completion/mod.rs`

---

## Task 1: Add Benchmark Infrastructure to Cargo.toml

**Files:**
- Modify: `crates/lsp/Cargo.toml`

**Step 1: Add Criterion dependency**

Add to `[dev-dependencies]` section:

```toml
[dev-dependencies]
tokio-test = "0.4"
unified-sql-lsp-test-utils = { path = "../test-utils" }
criterion = "0.5"
```

**Step 2: Add profiling dependencies**

Add to `[dependencies]` section:

```toml
[dependencies]
# ... existing dependencies ...

# Profiling (optional, feature-gated)
flamegraph = { version = "0.6", optional = true }
dhat = { version = "0.3", optional = true }
```

**Step 3: Add benchmark harness sections**

Add at end of Cargo.toml:

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
```

**Step 4: Add feature flags**

Add to `[features]` section (create if doesn't exist):

```toml
[features]
default = []
profiling = []
flamegraph = ["dep:flamegraph", "profiling"]
dhat = ["dep:dhat"]
```

**Step 5: Verify compilation**

Run: `cargo check --benches`

Expected: No errors, Cargo.toml is valid

**Step 6: Commit**

```bash
git add crates/lsp/Cargo.toml
git commit -m "feat(perf-001): add Criterion and profiling dependencies

Add Criterion 0.5 for benchmarking, flamegraph and dhat for profiling.
Configure benchmark harness sections and feature flags for profiling.
"
```

---

## Task 2: Create Instrumentation Module Foundation

**Files:**
- Create: `crates/lsp/src/profiling/mod.rs`

**Step 1: Create module directory**

Run: `mkdir -p crates/lsp/src/profiling`

**Step 2: Write profiling module with feature gating**

Create `crates/lsp/src/profiling/mod.rs`:

```rust
//! Performance profiling instrumentation
//!
//! This module provides low-overhead timing utilities for performance profiling.
//! Instrumentation is feature-gated and compiles to zero cost when the `profiling`
//! feature is disabled (default).

// Only compile instrumentation when profiling feature is enabled
#[cfg(feature = "profiling")]
mod timer;
#[cfg(feature = "profiling")]
mod stats;

#[cfg(feature = "profiling")]
pub use timer::ScopedTimer;
#[cfg(feature = "profiling")]
pub use stats::{TimingCollector, TimingReport, TimingStats};

// When profiling is disabled, provide a no-op macro
#[cfg(not(feature = "profiling"))]
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        // Compiles to nothing - zero runtime overhead
    };
}
```

**Step 3: Add profiling module to lib.rs**

Add to `crates/lsp/src/lib.rs`:

```rust
// ... existing modules ...

#[cfg(feature = "profiling")]
pub mod profiling;
```

**Step 4: Verify compilation**

Run: `cargo check`

Expected: No errors

**Step 5: Commit**

```bash
git add crates/lsp/src/profiling/mod.rs crates/lsp/src/lib.rs
git commit -m "feat(perf-001): create profiling module foundation

Add profiling module with feature-gated compilation.
Provides zero-overhead when disabled (default).
"
```

---

## Task 3: Implement ScopedTimer

**Files:**
- Create: `crates/lsp/src/profiling/timer.rs`

**Step 1: Write ScopedTimer implementation**

Create `crates/lsp/src/profiling/timer.rs`:

```rust
//! Scoped timing utilities for performance profiling
//!
//! # Example
//!
//! ```ignore
//! use crate::profiling::ScopedTimer;
//!
//! {
//!     let _timer = ScopedTimer::new("my_operation");
//!     // ... do work ...
//! } // Timer automatically records elapsed time when dropped
//! ```

use std::time::Instant;

/// A scoped timer that measures execution time
///
/// Automatically records the elapsed time to the global TimingCollector
/// when dropped. Use the `timed_scope!` macro for more ergonomic usage.
///
/// # Overhead
///
/// When the `profiling` feature is enabled, overhead is approximately:
/// - Construction: ~50ns (one Instant::now() call)
/// - Destruction: ~200ns (recording to thread-local storage)
///
/// When disabled, compiles to nothing (zero overhead).
#[derive(Debug)]
pub struct ScopedTimer {
    name: &'static str,
    start: Instant,
}

impl ScopedTimer {
    /// Create a new scoped timer with the given name
    ///
    /// # Example
    ///
    /// ```ignore
    /// let _timer = ScopedTimer::new("parse_cst");
    /// // ... parsing code ...
    /// ```
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
        super::stats::TimingCollector::record(self.name, duration);
    }
}

/// Macro for creating scoped timers with ergonomic syntax
///
/// # Example
///
/// ```ignore
/// use crate::profiling::timed_scope;
///
/// fn my_function() {
///     timed_scope!("my_function");
///     // ... code ...
/// } // Timing automatically recorded here
/// ```
#[macro_export]
macro_rules! timed_scope {
    ($name:expr) => {
        let _timer = $crate::profiling::ScopedTimer::new($name);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoped_timer_timing() {
        let collector = super::stats::TimingCollector::new();

        {
            let _timer = ScopedTimer::new("test_operation");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let report = collector.report();
        assert!(report.scopes.contains_key("test_operation"));
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check --features profiling`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/src/profiling/timer.rs
git commit -m "feat(perf-001): implement ScopedTimer for fine-grained timing

Add low-overhead scoped timer that automatically records elapsed time
on drop. Includes timed_scope! macro for ergonomic usage.
"
```

---

## Task 4: Implement TimingCollector and Statistics

**Files:**
- Create: `crates/lsp/src/profiling/stats.rs`

**Step 1: Write statistics collection implementation**

Create `crates/lsp/src/profiling/stats.rs`:

```rust
//! Statistics collection for performance profiling
//!
//! Thread-local storage for timing data with percentile calculation

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

thread_local! {
    /// Thread-local storage for timing data
    static TIMINGS: RefCell<HashMap<&'static str, Vec<Duration>>> =
        RefCell::new(HashMap::new());
}

/// Collector for timing data
///
/// # Example
///
/// ```ignore
/// // Record a timing
/// TimingCollector::record("parse_cst", Duration::from_micros(500));
///
/// // Generate report
/// let report = TimingCollector::report();
/// report.print_summary();
/// ```
pub struct TimingCollector;

impl TimingCollector {
    /// Record a timing measurement
    ///
    /// This is called automatically by ScopedTimer when dropped.
    pub fn record(name: &'static str, duration: Duration) {
        TIMINGS.with(|timings| {
            timings.borrow_mut()
                .entry(name)
                .or_insert_with(Vec::new)
                .push(duration);
        });
    }

    /// Generate a report of all recorded timings
    ///
    /// Returns a TimingReport with calculated statistics (min, max, avg, p95, p99).
    pub fn report() -> TimingReport {
        let mut scopes = HashMap::new();

        TIMINGS.with(|timings| {
            let timings = timings.borrow();
            for (name, durations) in timings.iter() {
                if !durations.is_empty() {
                    scopes.insert(*name, TimingStats::from_durations(durations));
                }
            }
        });

        TimingReport { scopes }
    }

    /// Clear all recorded timings
    pub fn clear() {
        TIMINGS.with(|timings| {
            timings.borrow_mut().clear();
        });
    }
}

/// Statistics for a single scope
#[derive(Debug, Clone)]
pub struct TimingStats {
    /// Number of times this scope was executed
    pub count: usize,
    /// Total time spent in this scope
    pub total: Duration,
    /// Minimum execution time
    pub min: Duration,
    /// Maximum execution time
    pub max: Duration,
    /// Average execution time
    pub avg: Duration,
    /// 95th percentile execution time
    pub p95: Duration,
    /// 99th percentile execution time
    pub p99: Duration,
}

impl TimingStats {
    /// Calculate statistics from a slice of durations
    fn from_durations(durations: &[Duration]) -> Self {
        assert!(!durations.is_empty(), "Cannot calculate stats for empty slice");

        let mut sorted: Vec<_> = durations.iter().copied().collect();
        sorted.sort();

        let count = sorted.len();
        let total: Duration = sorted.iter().sum();
        let min = sorted[0];
        let max = sorted[count - 1];
        let avg = total / count as u32;

        let p95_idx = (count as f64 * 0.95) as usize;
        let p95 = sorted[p95_idx.min(count - 1)];

        let p99_idx = (count as f64 * 0.99) as usize;
        let p99 = sorted[p99_idx.min(count - 1)];

        Self {
            count,
            total,
            min,
            max,
            avg,
            p95,
            p99,
        }
    }
}

/// Report containing statistics for all scopes
#[derive(Debug, Clone)]
pub struct TimingReport {
    pub scopes: HashMap<&'static str, TimingStats>,
}

impl TimingReport {
    /// Print a formatted summary of all timings
    pub fn print_summary(&self) {
        println!("\n=== Performance Profile ===");

        let mut sorted_scopes: BTreeMap<_> = self.scopes.iter().collect();
        for (scope, stats) in sorted_scopes {
            println!(
                "{:30} | calls: {:6} | avg: {:8.2?} | p95: {:8.2?} | p99: {:8.2?}",
                scope, stats.count, stats.avg, stats.p95, stats.p99
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_stats_calculation() {
        let durations = vec![
            Duration::from_micros(100),
            Duration::from_micros(200),
            Duration::from_micros(300),
            Duration::from_micros(400),
            Duration::from_micros(500),
        ];

        let stats = TimingStats::from_durations(&durations);

        assert_eq!(stats.count, 5);
        assert_eq!(stats.min, Duration::from_micros(100));
        assert_eq!(stats.max, Duration::from_micros(500));
        assert_eq!(stats.avg, Duration::from_micros(300));
    }

    #[test]
    fn test_timing_collector() {
        TimingCollector::clear();

        TimingCollector::record("test1", Duration::from_micros(100));
        TimingCollector::record("test1", Duration::from_micros(200));
        TimingCollector::record("test2", Duration::from_micros(300));

        let report = TimingCollector::report();

        assert_eq!(report.scopes.len(), 2);
        assert!(report.scopes.contains_key("test1"));
        assert!(report.scopes.contains_key("test2"));
        assert_eq!(report.scopes["test1"].count, 2);
        assert_eq!(report.scopes["test2"].count, 1);
    }
}
```

**Step 2: Verify compilation**

Run: `cargo check --features profiling`

Expected: No errors

**Step 3: Run tests**

Run: `cargo test --package unified-sql-lsp-lsp --features profiling --lib profiling`

Expected: All tests pass

**Step 4: Commit**

```bash
git add crates/lsp/src/profiling/stats.rs
git commit -m "feat(perf-001): implement TimingCollector and statistics

Add thread-local timing data collection with percentile calculation.
Includes TimingStats for min/max/avg/p95/p99 per scope.
"
```

---

## Task 5: Create Benchmark Directory Structure

**Files:**
- Create: `crates/lsp/benches/`
- Create: `crates/lsp/benches/fixtures/`
- Create: `crates/lsp/benches/fixtures/simple/`
- Create: `crates/lsp/benches/fixtures/medium/`
- Create: `crates/lsp/benches/fixtures/complex/`

**Step 1: Create directories**

Run:
```bash
mkdir -p crates/lsp/benches/fixtures/simple
mkdir -p crates/lsp/benches/fixtures/medium
mkdir -p crates/lsp/benches/fixtures/complex
```

**Step 2: Create benchmark entry point**

Create `crates/lsp/benches/mod.rs`:

```rust
//! Criterion benchmark suite for unified-sql-lsp
//!
//! # Running Benchmarks
//!
//! Run all benchmarks:
//! ```bash
//! cargo bench
//! ```
//!
//! Run specific benchmark:
//! ```bash
//! cargo bench --bench completion_pipeline
//! ```
//!
//! Compare against baseline:
//! ```bash
//! cargo bench -- --save-baseline main
//! cargo bench -- --baseline main
//! ```

mod completion_pipeline;
mod parsing;
mod semantic;
mod catalog;
mod concurrency;
mod memory;
```

**Step 3: Verify structure**

Run: `ls -la crates/lsp/benches/`

Expected: Shows fixtures/ directories and mod.rs

**Step 4: Commit**

```bash
git add crates/lsp/benches/
git commit -m "feat(perf-001): create benchmark directory structure

Add benchmark suite directory with fixtures for simple/medium/complex
queries organized by dialect.
"
```

---

## Task 6: Create Simple Query Fixtures

**Files:**
- Create: `crates/lsp/benches/fixtures/simple/mysql_01_single_table.sql`
- Create: `crates/lsp/benches/fixtures/simple/mysql_02_basic_where.sql`
- Create: `crates/lsp/benches/fixtures/simple/mysql_03_order_by.sql`
- Create: `crates/lsp/benches/fixtures/simple/postgresql_01_single_table.sql`
- Create: `crates/lsp/benches/fixtures/simple/postgresql_02_basic_where.sql`
- Create: `crates/lsp/benches/fixtures/simple/postgresql_03_order_by.sql`

**Step 1: Create MySQL simple fixture 1**

Create `crates/lsp/benches/fixtures/simple/mysql_01_single_table.sql`:

```sql
-- Simple single-table SELECT
-- Tests basic parsing and completion
SELECT id, name, email
FROM users
WHERE active = TRUE;
```

**Step 2: Create MySQL simple fixture 2**

Create `crates/lsp/benches/fixtures/simple/mysql_02_basic_where.sql`:

```sql
-- Basic WHERE clause with comparison
SELECT *
FROM orders
WHERE status = 'pending'
  AND amount > 100;
```

**Step 3: Create MySQL simple fixture 3**

Create `crates/lsp/benches/fixtures/simple/mysql_03_order_by.sql`:

```sql
-- Simple ORDER BY
SELECT product_id, name, price
FROM products
ORDER BY price DESC
LIMIT 10;
```

**Step 4: Create PostgreSQL simple fixtures**

Create PostgreSQL equivalents (same queries, different dialect):

`crates/lsp/benches/fixtures/simple/postgresql_01_single_table.sql`:
```sql
-- Simple single-table SELECT (PostgreSQL)
SELECT id, name, email
FROM users
WHERE active = TRUE;
```

`crates/lsp/benches/fixtures/simple/postgresql_02_basic_where.sql`:
```sql
-- Basic WHERE clause (PostgreSQL)
SELECT *
FROM orders
WHERE status = 'pending'
  AND amount > 100;
```

`crates/lsp/benches/fixtures/simple/postgresql_03_order_by.sql`:
```sql
-- Simple ORDER BY with LIMIT (PostgreSQL)
SELECT product_id, name, price
FROM products
ORDER BY price DESC
LIMIT 10;
```

**Step 5: Commit**

```bash
git add crates/lsp/benches/fixtures/simple/
git commit -m "feat(perf-001): add simple query fixtures

Add 6 simple query fixtures (3 MySQL, 3 PostgreSQL) for baseline
performance testing. Single-table SELECTs with basic WHERE and ORDER BY.
"
```

---

## Task 7: Create Medium Query Fixtures

**Files:**
- Create: `crates/lsp/benches/fixtures/medium/mysql_01_joins.sql`
- Create: `crates/lsp/benches/fixtures/medium/mysql_02_aggregates.sql`
- Create: `crates/lsp/benches/fixtures/medium/mysql_03_subquery_where.sql`
- Create: `crates/lsp/benches/fixtures/medium/postgresql_01_joins.sql`
- Create: `crates/lsp/benches/fixtures/medium/postgresql_02_aggregates.sql`
- Create: `crates/lsp/benches/fixtures/medium/postgresql_03_subquery_where.sql`

**Step 1: Create MySQL medium fixtures**

`crates/lsp/benches/fixtures/medium/mysql_01_joins.sql`:
```sql
-- Two-table JOIN with aliases
SELECT u.id, u.name, o.order_id, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE o.status = 'completed';
```

`crates/lsp/benches/fixtures/medium/mysql_02_aggregates.sql`:
```sql
-- Aggregate functions with GROUP BY and HAVING
SELECT u.id, u.name, COUNT(o.id) as order_count, SUM(o.amount) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name
HAVING order_count > 5
ORDER BY total_spent DESC;
```

`crates/lsp/benches/fixtures/medium/mysql_03_subquery_where.sql`:
```sql
-- Subquery in WHERE clause
SELECT id, name, email
FROM users
WHERE id IN (
    SELECT user_id
    FROM orders
    WHERE amount > 1000
);
```

**Step 2: Create PostgreSQL medium fixtures**

`crates/lsp/benches/fixtures/medium/postgresql_01_joins.sql`:
```sql
-- Two-table JOIN (PostgreSQL)
SELECT u.id, u.name, o.order_id, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE o.status = 'completed';
```

`crates/lsp/benches/fixtures/medium/postgresql_02_aggregates.sql`:
```sql
-- Aggregates with GROUP BY (PostgreSQL)
SELECT u.id, u.name, COUNT(o.id) as order_count, SUM(o.amount) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name
HAVING COUNT(o.id) > 5
ORDER BY total_spent DESC;
```

`crates/lsp/benches/fixtures/medium/postgresql_03_subquery_where.sql`:
```sql
-- Subquery in WHERE (PostgreSQL)
SELECT id, name, email
FROM users
WHERE id IN (
    SELECT user_id
    FROM orders
    WHERE amount > 1000
);
```

**Step 3: Commit**

```bash
git add crates/lsp/benches/fixtures/medium/
git commit -m "feat(perf-001): add medium query fixtures

Add 6 medium query fixtures with JOINs, aggregates, GROUP BY,
HAVING, and WHERE subqueries for intermediate performance testing.
"
```

---

## Task 8: Create Complex Query Fixtures

**Files:**
- Create: `crates/lsp/benches/fixtures/complex/mysql_01_cte.sql`
- Create: `crates/lsp/benches/fixtures/complex/mysql_02_window_function.sql`
- Create: `crates/lsp/benches/fixtures/complex/mysql_03_nested_subqueries.sql`
- Create: `crates/lsp/benches/fixtures/complex/postgresql_01_cte.sql`
- Create: `crates/lsp/benches/fixtures/complex/postgresql_02_window_function.sql`
- Create: `crates/lsp/benches/fixtures/complex/postgresql_03_nested_subqueries.sql`

**Step 1: Create MySQL complex fixtures**

`crates/lsp/benches/fixtures/complex/mysql_01_cte.sql`:
```sql
-- Common Table Expression (MySQL 8.0+)
WITH user_stats AS (
    SELECT
        user_id,
        COUNT(*) as total_orders,
        SUM(amount) as total_spent
    FROM orders
    GROUP BY user_id
),
high_value_users AS (
    SELECT user_id
    FROM user_stats
    WHERE total_spent > 5000
)
SELECT u.id, u.name, us.total_orders, us.total_spent
FROM users u
JOIN user_stats us ON u.id = us.user_id
WHERE u.id IN (SELECT user_id FROM high_value_users)
ORDER BY us.total_spent DESC;
```

`crates/lsp/benches/fixtures/complex/mysql_02_window_function.sql`:
```sql
-- Window functions (MySQL 8.0+)
SELECT
    id,
    name,
    amount,
    ROW_NUMBER() OVER (ORDER BY amount DESC) as row_num,
    RANK() OVER (ORDER BY amount DESC) as rank_num,
    DENSE_RANK() OVER (ORDER BY amount DESC) as dense_rank,
    SUM(amount) OVER (ORDER BY amount DESC) as running_total
FROM orders
WHERE status = 'completed';
```

`crates/lsp/benches/fixtures/complex/mysql_03_nested_subqueries.sql`:
```sql
-- Nested correlated subqueries
SELECT
    u.id,
    u.name,
    (SELECT COUNT(*) FROM orders WHERE user_id = u.id) as order_count,
    (SELECT AVG(amount) FROM orders WHERE user_id = u.id) as avg_order,
    (SELECT MAX(amount) FROM orders WHERE user_id = u.id) as max_order
FROM users u
WHERE EXISTS (
    SELECT 1 FROM orders o
    WHERE o.user_id = u.id
    AND o.amount > (
        SELECT AVG(amount) * 2 FROM orders
    )
);
```

**Step 2: Create PostgreSQL complex fixtures**

`crates/lsp/benches/fixtures/complex/postgresql_01_cte.sql`:
```sql
-- Common Table Expression (PostgreSQL)
WITH user_stats AS (
    SELECT
        user_id,
        COUNT(*) as total_orders,
        SUM(amount) as total_spent
    FROM orders
    GROUP BY user_id
),
high_value_users AS (
    SELECT user_id
    FROM user_stats
    WHERE total_spent > 5000
)
SELECT u.id, u.name, us.total_orders, us.total_spent
FROM users u
JOIN user_stats us ON u.id = us.user_id
WHERE u.id IN (SELECT user_id FROM high_value_users)
ORDER BY us.total_spent DESC;
```

`crates/lsp/benches/fixtures/complex/postgresql_02_window_function.sql`:
```sql
-- Window functions (PostgreSQL)
SELECT
    id,
    name,
    amount,
    ROW_NUMBER() OVER (ORDER BY amount DESC) as row_num,
    RANK() OVER (ORDER BY amount DESC) as rank_num,
    DENSE_RANK() OVER (ORDER BY amount DESC) as dense_rank,
    SUM(amount) OVER (ORDER BY amount DESC) as running_total
FROM orders
WHERE status = 'completed';
```

`crates/lsp/benches/fixtures/complex/postgresql_03_nested_subqueries.sql`:
```sql
-- Nested correlated subqueries (PostgreSQL)
SELECT
    u.id,
    u.name,
    (SELECT COUNT(*) FROM orders WHERE user_id = u.id) as order_count,
    (SELECT AVG(amount) FROM orders WHERE user_id = u.id) as avg_order,
    (SELECT MAX(amount) FROM orders WHERE user_id = u.id) as max_order
FROM users u
WHERE EXISTS (
    SELECT 1 FROM orders o
    WHERE o.user_id = u.id
    AND o.amount > (
        SELECT AVG(amount) * 2 FROM orders
    )
);
```

**Step 3: Commit**

```bash
git add crates/lsp/benches/fixtures/complex/
git commit -m "feat(perf-001): add complex query fixtures

Add 6 complex query fixtures with CTEs, window functions, and
nested correlated subqueries for stress testing.
"
```

---

## Task 9: Implement Parsing Benchmarks

**Files:**
- Create: `crates/lsp/benches/parsing.rs`

**Step 1: Write parsing benchmark skeleton**

Create `crates/lsp/benches/parsing.rs`:

```rust
//! Tree-sitter parsing performance benchmarks
//!
//! Measures the performance of Tree-sitter parsing across:
//! - Query complexity levels (simple, medium, complex)
//! - Dialects (MySQL, PostgreSQL)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use unified_sql_grammar::Parser;
use unified_sql_lsp_context::Dialect;

/// Load a fixture SQL file
fn load_fixture(dialect: Dialect, complexity: &str, index: usize) -> String {
    let dialect_name = match dialect {
        Dialect::MySQL => "mysql",
        Dialect::PostgreSQL => "postgresql",
    };

    let path = format!(
        "benches/fixtures/{}/{}/{}_{}.sql",
        complexity, dialect_name, dialect_name, index
    );

    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to load fixture {}: {}", path, e))
}

/// Benchmark parsing a single query
fn bench_parse_query(c: &mut Criterion, dialect: Dialect, complexity: &str, index: usize) {
    let query = load_fixture(dialect, complexity, index);
    let parser = Parser::new(dialect);

    let mut group = c.benchmark_group(format!("parsing/{:?}/{}", dialect, complexity));

    group.throughput(Throughput::Bytes(query.len() as u64));

    group.bench_function(BenchmarkId::from_parameter(index), |b| {
        b.iter(|| {
            let tree = parser.parse(black_box(&query));
            assert!(tree.is_ok(), "Parsing failed");
            black_box(tree.unwrap());
        });
    });

    group.finish();
}

fn bench_parsing_simple(c: &mut Criterion) {
    for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
        for index in 1..=3 {
            bench_parse_query(c, dialect, "simple", index);
        }
    }
}

fn bench_parsing_medium(c: &mut Criterion) {
    for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
        for index in 1..=3 {
            bench_parse_query(c, dialect, "medium", index);
        }
    }
}

fn bench_parsing_complex(c: &mut Criterion) {
    for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
        for index in 1..=3 {
            bench_parse_query(c, dialect, "complex", index);
        }
    }
}

fn bench_parsing_dialect_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("parsing/dialect_comparison");

    for (complexity, index) in [("simple", 1), ("medium", 1), ("complex", 1)] {
        for dialect in [Dialect::MySQL, Dialect::PostgreSQL] {
            let query = load_fixture(dialect, complexity, index);
            let parser = Parser::new(dialect);

            group.bench_function(
                BenchmarkId::new(format!("{:?}", dialect), format!("{}_{}", complexity, index)),
                |b| {
                    b.iter(|| {
                        let tree = parser.parse(black_box(&query));
                        black_box(tree.unwrap());
                    });
                },
            );
        }
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(100);
    targets =
        bench_parsing_simple,
        bench_parsing_medium,
        bench_parsing_complex,
        bench_parsing_dialect_comparison
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench parsing`

Expected: No errors

**Step 3: Run benchmark to verify**

Run: `cargo bench --bench parsing`

Expected: Benchmarks run successfully and generate output

**Step 4: Commit**

```bash
git add crates/lsp/benches/parsing.rs
git commit -m "feat(perf-001): implement parsing benchmarks

Add Criterion benchmarks for Tree-sitter parsing performance.
Tests all query complexity levels and dialects with throughput measurement.
"
```

---

## Task 10: Implement Semantic Analysis Benchmarks

**Files:**
- Create: `crates/lsp/benches/semantic.rs`

**Step 1: Write semantic benchmark skeleton**

Create `crates/lsp/benches/semantic.rs`:

```rust
//! Semantic analysis performance benchmarks
//!
//! Measures the performance of semantic analysis components:
//! - Scope building
//! - Alias resolution
//! - Column resolution

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use unified_sql_grammar::Parser;
use unified_sql_lsp_context::Dialect;
use unified_sql_lsp_semantic::{AliasResolver, ScopeManager, SemanticAnalyzer};

fn load_fixture(complexity: &str, index: usize) -> String {
    let path = format!("benches/fixtures/{}/mysql_{}.sql", complexity, index);
    std::fs::read_to_string(&path).expect("Failed to load fixture")
}

fn bench_semantic_analysis(c: &mut Criterion, complexity: &str, index: usize) {
    let query = load_fixture(complexity, index);
    let parser = Parser::new(Dialect::MySQL);

    // First parse the query
    let tree = parser.parse(&query).expect("Parse failed");
    let cst = tree.root_node();

    let mut group = c.benchmark_group(format!("semantic/{}", complexity));

    // Benchmark scope building
    group.bench_function(
        BenchmarkId::new("scope_building", format!("{}_{}", complexity, index)),
        |b| {
            b.iter(|| {
                let mut scope_manager = ScopeManager::new();
                let result = scope_manager.build_scopes(&cst);
                black_box(result);
            });
        },
    );

    // Benchmark alias resolution
    group.bench_function(
        BenchmarkId::new("alias_resolution", format!("{}_{}", complexity, index)),
        |b| {
            b.iter(|| {
                let resolver = AliasResolver::new();
                let aliases = resolver.resolve_table_aliases(&cst);
                black_box(aliases);
            });
        },
    );

    // Benchmark full semantic analysis
    group.bench_function(
        BenchmarkId::new("full_analysis", format!("{}_{}", complexity, index)),
        |b| {
            b.iter(|| {
                let analyzer = SemanticAnalyzer::new(None); // No catalog for benchmarks
                let result = analyzer.analyze(&cst);
                black_box(result);
            });
        },
    );

    group.finish();
}

fn bench_semantic_simple(c: &mut Criterion) {
    for index in 1..=3 {
        bench_semantic_analysis(c, "simple", index);
    }
}

fn bench_semantic_medium(c: &mut Criterion) {
    for index in 1..=3 {
        bench_semantic_analysis(c, "medium", index);
    }
}

fn bench_semantic_complex(c: &mut Criterion) {
    for index in 1..=3 {
        bench_semantic_analysis(c, "complex", index);
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets = bench_semantic_simple, bench_semantic_medium, bench_semantic_complex
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench semantic`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/benches/semantic.rs
git commit -m "feat(perf-001): implement semantic analysis benchmarks

Add benchmarks for scope building, alias resolution, and full
semantic analysis across all query complexity levels.
"
```

---

## Task 11: Implement Catalog Query Benchmarks

**Files:**
- Create: `crates/lsp/benches/catalog.rs`

**Step 1: Write catalog benchmark skeleton**

Create `crates/lsp/benches/catalog.rs`:

```rust
//! Catalog query performance benchmarks
//!
//! Measures the performance of catalog operations:
//! - Connection acquisition
//! - Schema queries (list_tables, get_columns)
//! - Cache hit/miss performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use unified_sql_lsp_catalog::{Catalog, LiveMySqlCatalog};

/// Benchmark connection acquisition
fn bench_connection_acquire(c: &mut Criterion) {
    c.bench_function("catalog/connection_acquire", |b| {
        let connection_string = std::env::var("MYSQL_TEST_CONNECTION_STRING")
            .unwrap_or_else(|_| "mysql://localhost:3306/test".to_string());

        b.iter(|| {
            let catalog = LiveMySqlCatalog::new(&connection_string);
            black_box(catalog)
        });
    });
}

/// Benchmark list_tables query
fn bench_list_tables(c: &mut Criterion) {
    let connection_string = std::env::var("MYSQL_TEST_CONNECTION_STRING")
        .unwrap_or_else(|_| "mysql://localhost:3306/test".to_string());

    let catalog = LiveMySqlCatalog::new(&connection_string);

    c.bench_function("catalog/list_tables", |b| {
        b.iter(|| {
            let tables = catalog.list_tables(black_box("test"));
            black_box(tables)
        });
    });
}

/// Benchmark get_columns query
fn bench_get_columns(c: &mut Criterion) {
    let connection_string = std::env::var("MYSQL_TEST_CONNECTION_STRING")
        .unwrap_or_else(|_| "mysql://localhost:3306/test".to_string());

    let catalog = LiveMySqlCatalog::new(&connection_string);

    c.bench_function("catalog/get_columns", |b| {
        b.iter(|| {
            let columns = catalog.get_columns(black_box("test"), black_box("users"));
            black_box(columns)
        });
    });
}

/// Benchmark combined catalog operations (simulating completion)
fn bench_completion_catalog_flow(c: &mut Criterion) {
    let connection_string = std::env::var("MYSQL_TEST_CONNECTION_STRING")
        .unwrap_or_else(|_| "mysql://localhost:3306/test".to_string());

    let catalog = LiveMySqlCatalog::new(&connection_string);

    c.bench_function("catalog/completion_flow", |b| {
        b.iter(|| {
            // Typical completion flow: list tables, then get columns for one
            let tables = catalog.list_tables(black_box("test")).unwrap();
            if !tables.is_empty() {
                let columns = catalog.get_columns(black_box("test"), black_box(&tables[0]));
                black_box(columns);
            }
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets =
        bench_connection_acquire,
        bench_list_tables,
        bench_get_columns,
        bench_completion_catalog_flow
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench catalog`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/benches/catalog.rs
git commit -m "feat(perf-001): implement catalog query benchmarks

Add benchmarks for catalog operations including connection acquisition,
schema queries, and typical completion flow.
"
```

---

## Task 12: Implement Completion Pipeline Benchmarks

**Files:**
- Create: `crates/lsp/benches/completion_pipeline.rs`

**Step 1: Write completion pipeline benchmark skeleton**

Create `crates/lsp/benches/completion_pipeline.rs`:

```rust
//! End-to-end completion pipeline benchmarks
//!
//! Measures the performance of the full completion flow:
//! - Context detection
//! - Parsing
//! - Lowering
//! - Semantic analysis
//! - Catalog queries
//! - Rendering

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::runtime::Runtime;
use unified_sql_lsp_completion::CompletionEngine;
use unified_sql_lsp_context::{CompletionContext, Dialect, Position};
use unified_sql_lsp_lsp::{Document, DocumentStore};

/// Create a test document
fn create_document(content: &str) -> Document {
    Document::new(
        "test.sql".to_string(),
        content.to_string(),
        Dialect::MySQL,
    )
}

/// Benchmark completion at SELECT clause (simple query)
fn bench_completion_simple_select(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("completion/simple");

    group.bench_function("select_projection", |b| {
        let doc = create_document("SELECT | FROM users");
        let engine = CompletionEngine::new(None); // No catalog

        b.to_async(&rt).iter(|| async {
            let result = engine.complete(
                black_box(&doc),
                black_box(Position { line: 0, character: 9 }),
                black_box(CompletionContext::SelectProjection),
            ).await;
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark completion at FROM clause (table completion)
fn bench_completion_simple_from(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("completion/simple");

    group.bench_function("from_clause", |b| {
        let doc = create_document("SELECT id FROM |");
        let engine = CompletionEngine::new(None);

        b.to_async(&rt).iter(|| async {
            let result = engine.complete(
                black_box(&doc),
                black_box(Position { line: 0, character: 19 }),
                black_box(CompletionContext::FromClause),
            ).await;
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark completion with qualified column reference
fn bench_completion_qualified_column(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("completion/simple");

    group.bench_function("qualified_column", |b| {
        let doc = create_document("SELECT u.| FROM users u");
        let engine = CompletionEngine::new(None);

        b.to_async(&rt).iter(|| async {
            let result = engine.complete(
                black_box(&doc),
                black_box(Position { line: 0, character: 10 }),
                black_box(CompletionContext::SelectProjection),
            ).await;
            black_box(result);
        });
    });

    group.finish();
}

/// Benchmark completion by complexity
fn bench_completion_by_complexity(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let queries = [
        ("simple", "SELECT id, name FROM users WHERE active = TRUE"),
        ("medium", "SELECT u.id, u.name, o.total FROM users u JOIN orders o ON u.id = o.user_id"),
        ("complex", "WITH stats AS (SELECT user_id, COUNT(*) as cnt FROM orders GROUP BY user_id) SELECT * FROM stats"),
    ];

    for (complexity, query) in queries {
        let mut group = c.benchmark_group(format!("completion/{}", complexity));

        group.bench_function("end_to_end", |b| {
            let doc = create_document(query);
            let engine = CompletionEngine::new(None);

            b.to_async(&rt).iter(|| async {
                let result = engine.complete(
                    black_box(&doc),
                    black_box(Position { line: 0, character: 7 }),
                    black_box(CompletionContext::SelectProjection),
                ).await;
                black_box(result);
            });
        });

        group.finish();
    }
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(50);
    targets =
        bench_completion_simple_select,
        bench_completion_simple_from,
        bench_completion_qualified_column,
        bench_completion_by_complexity
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench completion_pipeline`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/benches/completion_pipeline.rs
git commit -m "feat(perf-001): implement completion pipeline benchmarks

Add end-to-end completion benchmarks measuring full pipeline
performance across different contexts and query complexities.
"
```

---

## Task 13: Implement Concurrency Benchmarks

**Files:**
- Create: `crates/lsp/benches/concurrency.rs`

**Step 1: Write concurrency benchmark skeleton**

Create `crates/lsp/benches/concurrency.rs`:

```rust
//! Concurrency and throughput benchmarks
//!
//! Measures performance under concurrent load:
//! - Multiple document completions simultaneously
//! - Shared catalog access
//! - Thread safety and contention

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokio::runtime::Runtime;
use unified_sql_lsp_completion::CompletionEngine;
use unified_sql_lsp_context::{CompletionContext, Dialect, Position};
use unified_sql_lsp_lsp::Document;

fn create_document(id: usize) -> Document {
    Document::new(
        format!("test_{}.sql", id),
        "SELECT id, name FROM users WHERE active = TRUE".to_string(),
        Dialect::MySQL,
    )
}

/// Benchmark concurrent completion operations
fn bench_concurrent_completions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    for doc_count in [1, 5, 10, 50] {
        let mut group = c.benchmark_group("concurrency");

        group.throughput(Throughput::Elements(doc_count as u64));

        group.bench_with_input(
            BenchmarkId::new("concurrent_completions", doc_count),
            &doc_count,
            |b, &count| {
                let engine = CompletionEngine::new(None);

                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..count)
                        .map(|i| {
                            let doc = create_document(i);
                            let engine = &engine;
                            async move {
                                engine.complete(
                                    &doc,
                                    Position { line: 0, character: 7 },
                                    CompletionContext::SelectProjection,
                                ).await
                            }
                        })
                        .collect();

                    let results = futures::future::join_all(tasks).await;
                    black_box(results);
                });
            },
        );

        group.finish();
    }
}

/// Benchmark document store concurrent access
fn bench_concurrent_document_access(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let store = unified_sql_lsp_lsp::DocumentStore::new();

    // Pre-populate store
    for i in 0..50 {
        let doc = create_document(i);
        store.insert(doc);
    }

    let mut group = c.benchmark_group("concurrency");

    for doc_count in [1, 5, 10, 50] {
        group.bench_with_input(
            BenchmarkId::new("document_store_access", doc_count),
            &doc_count,
            |b, &count| {
                b.to_async(&rt).iter(|| {
                    let store = &store;
                    async move {
                        let tasks: Vec<_> = (0..count)
                            .map(|i| {
                                async move {
                                    store.get(&format!("test_{}.sql", i))
                                }
                            })
                            .collect();

                        let results = futures::future::join_all(tasks).await;
                        black_box(results);
                    }
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(20);
    targets = bench_concurrent_completions, bench_concurrent_document_access
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench concurrency`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/benches/concurrency.rs
git commit -m "feat(perf-001): implement concurrency benchmarks

Add benchmarks for concurrent completion operations and document
store access to test thread safety and contention.
"
```

---

## Task 14: Implement Memory Profiling Benchmarks

**Files:**
- Create: `crates/lsp/benches/memory.rs`

**Step 1: Write memory benchmark skeleton**

Create `crates/lsp/benches/memory.rs`:

```rust
//! Memory allocation profiling benchmarks
//!
//! Uses DHAT (heap profiler) to identify memory hotspots:
//! - Allocation patterns during completion
//! - Memory retained vs temporary
//! - Potential leaks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use unified_sql_lsp_completion::CompletionEngine;
use unified_sql_lsp_context::{CompletionContext, Dialect, Position};
use unified_sql_lsp_lsp::Document;

fn create_document(content: &str) -> Document {
    Document::new(
        "test.sql".to_string(),
        content.to_string(),
        Dialect::MySQL,
    )
}

/// Profile memory for 1000 completion operations
#[cfg(feature = "dhat")]
fn bench_memory_1000_completions(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("memory/completion");

    group.bench_function("1000_completions", |b| {
        let queries = [
            "SELECT id, name FROM users WHERE active = TRUE",
            "SELECT u.id, o.total FROM users u JOIN orders o ON u.id = o.user_id",
            "WITH stats AS (SELECT COUNT(*) FROM orders) SELECT * FROM stats",
        ];

        b.iter(|| {
            let _dhat_guard = dhat::Profiler::new();

            for query in queries {
                let doc = create_document(query);
                let engine = CompletionEngine::new(None);

                let result = rt.block_on(async {
                    engine.complete(
                        &doc,
                        Position { line: 0, character: 7 },
                        CompletionContext::SelectProjection,
                    ).await
                });

                black_box(result);
            }
        });
    });

    group.finish();
}

/// Profile memory for document parsing
#[cfg(feature = "dhat")]
fn bench_memory_document_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory/parsing");

    group.bench_function("parse_100_documents", |b| {
        let query = "SELECT id, name, email FROM users WHERE active = TRUE";

        b.iter(|| {
            let _dhat_guard = dhat::Profiler::new();

            for _ in 0..100 {
                let doc = create_document(query);
                black_box(doc);
            }
        });
    });

    group.finish();
}

/// Fallback benchmarks when DHAT is not enabled
#[cfg(not(feature = "dhat"))]
fn bench_memory_1000_completions(c: &mut Criterion) {
    c.bench_function("memory/1000_completions", |b| {
        let rt = Runtime::new().unwrap();
        let doc = create_document("SELECT id FROM users");
        let engine = CompletionEngine::new(None);

        b.iter(|| {
            rt.block_on(async {
                let result = engine.complete(
                    &doc,
                    Position { line: 0, character: 7 },
                    CompletionContext::SelectProjection,
                ).await;
                black_box(result);
            });
        });
    });
}

#[cfg(not(feature = "dhat"))]
fn bench_memory_document_parsing(c: &mut Criterion) {
    c.bench_function("memory/parse_100_documents", |b| {
        let query = "SELECT id FROM users";
        b.iter(|| {
            for _ in 0..100 {
                let doc = create_document(query);
                black_box(doc);
            }
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_memory_1000_completions, bench_memory_document_parsing
);

criterion_main!(benches);
```

**Step 2: Verify compilation**

Run: `cargo check --benches --bench memory`

Expected: No errors

**Step 3: Commit**

```bash
git add crates/lsp/benches/memory.rs
git commit -m "feat(perf-001): implement memory profiling benchmarks

Add memory profiling benchmarks using DHAT heap profiler.
Tests completion and parsing memory allocation patterns.
"
```

---

## Task 15: Create Profiling Scripts

**Files:**
- Create: `scripts/profiling/run_all.sh`
- Create: `scripts/profiling/flamegraph.sh`
- Create: `scripts/profiling/generate_report.sh`

**Step 1: Create scripts directory**

Run: `mkdir -p scripts/profiling`

**Step 2: Create master profiling script**

Create `scripts/profiling/run_all.sh`:

```bash
#!/bin/bash
# Run complete profiling suite
#
# This script runs all benchmarks and generates a comprehensive report
# including Criterion reports, flamegraphs, and memory profiles.

set -e

echo "=== PERF-001 Profiling Suite ==="
echo "Starting at $(date)"
echo ""

# Ensure we're in project root
cd "$(git rev-parse --show-toplevel)"

# Create output directory
REPORT_DIR="target/profiling-reports/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$REPORT_DIR"

echo "1. Running Criterion benchmarks..."
cargo bench --benches completion,parsing,semantic,catalog,concurrency \
    --save-baseline main \
    --output-format bencher | tee "$REPORT_DIR/bench_output.txt"

echo ""
echo "2. Generating flamegraph..."
if command -v cargo-flamegraph &> /dev/null; then
    cargo flamegraph --bench completion_pipeline -- \
        --output "$REPORT_DIR/flamegraph.svg" || true
else
    echo "  (flamegraph not installed, skipping)"
fi

echo ""
echo "3. Running memory profiling..."
cargo bench --bench memory --features dhat \
    --output-format bencher | tee -a "$REPORT_DIR/bench_output.txt" || true

echo ""
echo "4. Copying Criterion HTML reports..."
cp -r target/criterion "$REPORT_DIR/"

echo ""
echo "5. Generating summary..."
cat > "$REPORT_DIR/SUMMARY.md" <<EOF
# Performance Profiling Report
Generated: $(date)

## Benchmarks Run

### Completion Pipeline
- End-to-end completion latency
- Context detection (SELECT, FROM, WHERE)
- Qualified column references

### Parsing
- Tree-sitter parsing by complexity
- Dialect comparison (MySQL vs PostgreSQL)

### Semantic Analysis
- Scope building
- Alias resolution
- Full semantic analysis

### Catalog Queries
- Connection acquisition
- Schema queries
- Cache performance

### Concurrency
- Concurrent completion operations
- Document store contention

### Memory
- Allocation patterns
- Heap usage

## Results

### Criterion Reports
Open [criterion/index.html](criterion/index.html) for detailed statistical analysis.

### Flamegraph
Open [flamegraph.svg](flamegraph.svg) for CPU flamegraph (if generated).

## Key Findings

\`\`\`
$(cat "$REPORT_DIR/bench_output.txt" | grep -E "bench:|time:" || echo "No benchmark output available")
\`\`\`

## Next Steps

1. Identify bottlenecks from Criterion reports
2. Analyze flamegraph for CPU hotspots
3. Review memory profiles for allocation issues
4. Prioritize optimizations based on data
EOF

echo ""
echo "=== Profiling Complete ==="
echo "Report saved to: $REPORT_DIR"
echo ""
echo "View results:"
echo "  - HTML: xdg-open $REPORT_DIR/criterion/index.html"
echo "  - Flamegraph: xdg-open $REPORT_DIR/flamegraph.svg (if generated)"
echo "  - Summary: cat $REPORT_DIR/SUMMARY.md"
```

**Step 3: Create flamegraph script**

Create `scripts/profiling/flamegraph.sh`:

```bash
#!/bin/bash
# Generate flamegraph for completion pipeline

set -e

echo "=== Generating Flamegraph ==="

# Check if flamegraph is installed
if ! command -v cargo-flamegraph &> /dev/null; then
    echo "Error: cargo-flamegraph not installed"
    echo "Install with: cargo install flamegraph"
    exit 1
fi

# Build with profiling instrumentation
cargo build --release --features profiling

# Generate flamegraph
cargo flamegraph --bench completion_pipeline -- \
    --output "flamegraph-$(date +%Y%m%d-%H%M%S).svg"

echo "Flamegraph generated successfully"
echo "Open with: xdg-open flamegraph-*.svg"
```

**Step 4: Create report generation script**

Create `scripts/profiling/generate_report.sh`:

```bash
#!/bin/bash
# Generate profiling report from latest benchmark run

set -e

LATEST_REPORT=$(ls -td target/profiling-reports/* 2>/dev/null | head -1)

if [ -z "$LATEST_REPORT" ]; then
    echo "No profiling reports found. Run ./scripts/profiling/run_all.sh first."
    exit 1
fi

echo "Latest profiling report: $LATEST_REPORT"
echo ""
echo "Summary:"
cat "$LATEST_REPORT/SUMMARY.md"
```

**Step 5: Make scripts executable**

Run:
```bash
chmod +x scripts/profiling/run_all.sh
chmod +x scripts/profiling/flamegraph.sh
chmod +x scripts/profiling/generate_report.sh
```

**Step 6: Commit**

```bash
git add scripts/profiling/
git commit -m "feat(perf-001): add profiling automation scripts

Add master script (run_all.sh) for complete profiling suite,
flamegraph generator, and report aggregation script.
"
```

---

## Task 16: Update Makefile with Benchmark Targets

**Files:**
- Modify: `Makefile`

**Step 1: Add benchmark targets to Makefile**

Add to Makefile after the test section:

```makefile
# Benchmarking
.PHONY: benchmark profile-all flamegraph

benchmark:
	@echo "Running quick benchmark suite..."
	@cargo bench --benches completion,parsing,semantic

profile-all:
	@echo "Running complete profiling suite..."
	@./scripts/profiling/run_all.sh

flamegraph:
	@echo "Generating flamegraph..."
	@./scripts/profiling/flamegraph.sh
```

**Step 2: Verify Makefile works**

Run: `make benchmark`

Expected: Benchmarks run successfully

**Step 3: Commit**

```bash
git add Makefile
git commit -m "feat(perf-001): add benchmark targets to Makefile

Add 'make benchmark', 'make profile-all', and 'make flamegraph'
targets for convenient access to profiling tools.
"
```

---

## Task 17: Create Profiling Documentation

**Files:**
- Create: `docs/profiling.md`

**Step 1: Write comprehensive profiling guide**

Create `docs/profiling.md`:

```markdown
# Performance Profiling Guide

This guide explains how to run, interpret, and use the performance profiling infrastructure for Unified SQL LSP.

## Quick Start

### Run All Benchmarks

\`\`\`bash
# Quick benchmark suite (parsing, semantic, completion)
make benchmark

# Or with cargo directly
cargo bench --benches completion,parsing,semantic
\`\`\`

### Run Complete Profiling Suite

\`\`\`bash
# Full profiling with report generation
make profile-all

# Or directly
./scripts/profiling/run_all.sh
\`\`\`

This generates:
- Criterion HTML reports with statistical analysis
- Flamegraph (if `cargo-flamegraph` is installed)
- Memory profiles (if run with `--features dhat`)
- Summary markdown file

### View Results

\`\`\`bash
# Open latest report
./scripts/profiling/generate_report.sh

# Or manually open HTML
xdg-open target/profiling-reports/*/criterion/index.html
\`\`\`

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

\`\`\`bash
# Save current performance as baseline
cargo bench -- --save-baseline before-optimization

# Make changes...

# Compare against baseline
cargo bench -- --baseline before-optimization
\`\`\`

Criterion shows:
- Percentage change (green for improvement, red for regression)
- Statistical significance check
- Confidence intervals for the difference

### Flamegraphs

Flamegraphs show CPU time distribution as an interactive SVG:

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

\`\`\`
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
\`\`\`

## Writing New Benchmarks

### Template

\`\`\`rust
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
\`\`\`

### Adding to Suite

1. Create file in `crates/lsp/benches/`
2. Add to `benches/mod.rs`: `mod my_benchmark;`
3. Add to `Cargo.toml`:
   \`\`\`toml
   [[bench]]
   name = "my_benchmark"
   harness = false
   \`\`\`

### Best Practices

- **Use `black_box`**: Prevent compiler from optimizing away
- **Warmup**: Criterion handles this automatically
- **Sample size**: Default is fine, reduce for slow operations
- **Async**: Use `b.to_async(&rt).iter()` for async code
- **Throughput**: Add `.throughput(Throughput::Bytes(size))` for I/O

## Troubleshooting

### Benchmarks Fail to Compile

\`\`\`bash
# Check with full output
cargo check --benches --bench my_benchmark --verbose

# Common issues:
# - Missing feature flags
# - Wrong function signature (iter vs iter_async)
# - Missing black_box usage
\`\`\`

### Flamegraph Not Generated

\`\`\`bash
# Install flamegraph
cargo install flamegraph

# Verify installation
cargo flamegraph --version

# Run manually for more output
cargo flamegraph --bench completion_pipeline
\`\`\`

### Inconsistent Benchmark Results

- **Close other applications**: Reduce system noise
- **Increase sample size**: `.sample_size(100)` in Criterion config
- **Use warmup**: Criterion does this, but may need more
- **Check frequency scaling**: CPU turbo boost causes variance
  \`\`\`bash
  # Disable turbo boost for consistent results (Linux)
  sudo cpupower frequency-set -g performance
  \`\`\`

### Catalog Benchmarks Fail

\`\`\`bash
# Set connection string
export MYSQL_TEST_CONNECTION_STRING="mysql://localhost:3306/test"
export POSTGRES_TEST_CONNECTION_STRING="postgresql://localhost:5432/test"

# Or skip catalog benchmarks
cargo bench --benches completion,parsing,semantic
\`\`\`

## CI Integration (Optional)

To add automated regression detection:

\`\`\`yaml
# .github/workflows/benchmarks.yml
name: Benchmarks

on:
  push:
    branches: [main]
  pull_request:

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run benchmarks
        run: cargo bench -- --save-baseline ci-baseline

      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: target/criterion/index.html
\`\`\`

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
\`\`\`

**Step 2: Verify documentation builds**

Run: `head -n 50 docs/profiling.md`

Expected: Documentation is readable and formatted correctly

**Step 3: Commit**

```bash
git add docs/profiling.md
git commit -m "docs(perf-001): add comprehensive profiling guide

Add user guide for running benchmarks, interpreting results,
and writing custom benchmarks. Includes troubleshooting
and performance targets.
"
```

---

## Task 18: Run Full Benchmark Suite and Verify

**Files:**
- No file creation (verification task)

**Step 1: Run all benchmarks**

Run: `cargo bench --benches`

Expected: All benchmarks run successfully and generate reports

**Step 2: Check for compilation errors**

Run: `cargo check --benches`

Expected: No errors

**Step 3: Verify profiling scripts work**

Run: `./scripts/profiling/run_all.sh`

Expected: Scripts execute successfully and generate report

**Step 4: Verify Makefile targets**

Run: `make benchmark`

Expected: Benchmarks run via Makefile

**Step 5: Check profiling feature works**

Run: `cargo build --features profiling`

Expected: Builds successfully with profiling enabled

**Step 6: Generate final verification report**

Create summary:
```bash
echo "=== PERF-001 Implementation Verification ===" > VERIFICATION.md
echo "" >> VERIFICATION.md
echo "## Completed Tasks" >> VERIFICATION.md
echo "- [x] Benchmark infrastructure (Criterion, dependencies)" >> VERIFICATION.md
echo "- [x] Instrumentation module (profiling/timer, profiling/stats)" >> VERIFICATION.md
echo "- [x] Query fixtures (18 SQL files: simple/medium/complex × MySQL/PostgreSQL)" >> VERIFICATION.md
echo "- [x] Parsing benchmarks" >> VERIFICATION.md
echo "- [x] Semantic analysis benchmarks" >> VERIFICATION.md
echo "- [x] Catalog query benchmarks" >> VERIFICATION.md
echo "- [x] Completion pipeline benchmarks" >> VERIFICATION.md
echo "- [x] Concurrency benchmarks" >> VERIFICATION.md
echo "- [x] Memory profiling benchmarks" >> VERIFICATION.md
echo "- [x] Profiling automation scripts" >> VERIFICATION.md
echo "- [x] Makefile integration" >> VERIFICATION.md
echo "- [x] Documentation (profiling guide)" >> VERIFICATION.md
echo "" >> VERIFICATION.md
echo "## Verification Results" >> VERIFICATION.md
echo "All benchmarks compile and run successfully." >> VERIFICATION.md
echo "Profiling infrastructure ready for use." >> VERIFICATION.md
```

**Step 7: Final commit**

```bash
git add VERIFICATION.md
git commit -m "feat(perf-001): complete implementation verification

All benchmark infrastructure implemented and verified.
Ready for performance baseline establishment and bottleneck identification.
"
```

---

## Success Criteria Verification

Before considering PERF-001 complete, verify:

✅ All 6 benchmark files exist and compile without errors
✅ `cargo bench` runs successfully and generates Criterion HTML reports
✅ Flamegraphs can be generated with `./scripts/profiling/flamegraph.sh`
✅ Fine-grained instrumentation shows breakdown of completion pipeline stages
✅ Benchmarks cover both MySQL and PostgreSQL dialects
✅ At least 3 complexity levels (simple/medium/complex) are tested
✅ Documentation explains how to run and interpret results
✅ Makefile includes `make benchmark` target for convenience
✅ Profile data can identify actual bottlenecks (not assumptions)
✅ Baseline metrics are established for future comparison

---

## Next Steps After Implementation

1. **Run baseline profiling**: `./scripts/profiling/run_all.sh`
2. **Analyze results**: Review Criterion reports and flamegraphs
3. **Identify bottlenecks**: Determine which stage is slowest
4. **Propose optimization**: Based on data, choose:
   - PERF-002: Async semantic analysis (if semantic is bottleneck)
   - PERF-003: Catalog batching (if catalog is bottleneck)
   - PERF-004: Targeted optimization of specific bottleneck

---

## Implementation Notes

- **DRY**: Reuse instrumentation module across all benchmarks
- **YAGNI**: Don't add features not needed for profiling
- **TDD**: Each benchmark file is independently testable
- **Frequent commits**: Each task creates a git commit
- **Feature flags**: Profiling is zero-overhead when disabled

---

**End of Implementation Plan**
