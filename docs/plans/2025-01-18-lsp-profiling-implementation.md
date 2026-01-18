# LSP Profiling Flamegraph Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a separate profiling workspace that generates authentic CPU flamegraphs of real LSP operations (completion, hover, diagnostics) without tower-lsp server overhead.

**Architecture:** Create isolated `benches/profiling/` workspace with Criterion benchmarks that simulate realistic LSP workloads. Reuse existing semantic/context infrastructure directly, bypassing LSP protocol layer. Generate flamegraphs using `cargo-flamegraph` on benchmark execution.

**Tech Stack:** Rust 2024, Criterion (benchmarking), cargo-flamegraph (profiling), tree-sitter (parsing), unified-sql-lsp crates (semantic, context, lowering).

---

## Task 1: Create Workspace Directory Structure

**Files:**
- Create: `benches/profiling/`
- Create: `benches/profiling/src/`
- Create: `benches/profiling/src/fixtures/queries/`
- Create: `benches/profiling/src/fixtures/scenarios/`

**Step 1: Create directory structure**

Run:
```bash
cd /home/woxQAQ/unified-sql-lsp/.worktrees/lsp-profiling
mkdir -p benches/profiling/src/fixtures/queries
mkdir -p benches/profiling/src/fixtures/scenarios
```

Expected: Directories created, no errors

**Step 2: Verify structure**

Run:
```bash
tree benches/profiling -L 3
```

Expected output:
```
benches/profiling
└── src
    └── fixtures
        ├── queries
        └── scenarios
```

**Step 3: Commit**

```bash
git add benches/profiling
git commit -m "feat(perf-001): create profiling workspace directory structure"
```

---

## Task 2: Create Profiling Cargo.toml

**Files:**
- Create: `benches/profiling/Cargo.toml`

**Step 1: Create Cargo.toml**

Create file `benches/profiling/Cargo.toml`:
```toml
[package]
name = "lsp-profiling"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
# Reuse core crates from parent workspace
unified-sql-lsp-lowering = { path = "../../crates/lowering" }
unified-sql-lsp-semantic = { path = "../../crates/semantic" }
unified-sql-lsp-context = { path = "../../crates/context" }
unified-sql-lsp-ir = { path = "../../crates/ir" }
unified-sql-lsp-grammar = { path = "../../crates/grammar" }
unified-sql-lsp-catalog = { path = "../../crates/catalog" }
unified-sql-lsp-function-registry = { path = "../../crates/function-registry" }

# LSP types for convenience (reusing data structures)
lsp-types = "0.95"

# Dev/profiling dependencies
criterion = "0.5"
serde_json = "1.0"

[dev-dependencies]
flamegraph = "0.6"

[features]
default = ["profiling"]
profiling = []

[[bench]]
name = "lsp_operations"
harness = false
```

**Step 2: Verify Cargo.toml syntax**

Run:
```bash
cd benches/profiling
cargo check --no-build 2>&1 | head -20
```

Expected: No syntax errors (may have dependency resolution warnings, that's OK)

**Step 3: Commit**

```bash
git add benches/profiling/Cargo.toml
git commit -m "feat(perf-001): add profiling workspace Cargo.toml"
```

---

## Task 3: Create Benchmark Entry Point

**Files:**
- Create: `benches/profiling/benches/lsp_operations.rs`

**Step 1: Create main benchmark file**

Create file `benches/profiling/benches/lsp_operations.rs`:
```rust
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
```

**Step 2: Verify Rust syntax**

Run:
```bash
cd benches/profiling
cargo check --benches 2>&1 | grep -E "error|warning" | head -10
```

Expected: Errors about missing modules (workload, operations, fixtures) - this is expected for now

**Step 3: Commit**

```bash
git add benches/profiling/benches/lsp_operations.rs
git commit -m "feat(perf-001): add benchmark entry point"
```

---

## Task 4: Create Fixtures Module

**Files:**
- Create: `benches/profiling/benches/fixtures.rs`
- Create: `benches/profiling/src/fixtures/queries/simple_select.sql`
- Create: `benches/profiling/src/fixtures/queries/complex_join.sql`
- Create: `benches/profiling/src/fixtures/queries/nested_subquery.sql`

**Step 1: Create fixtures module**

Create file `benches/profiling/benches/fixtures.rs`:
```rust
use std::collections::HashMap;
use std::path::Path;

pub struct TestQuery {
    pub name: String,
    pub sql: String,
    pub dialect: String,
}

pub fn load_test_queries() -> HashMap<String, TestQuery> {
    let mut queries = HashMap::new();

    queries.insert(
        "simple_select".to_string(),
        TestQuery {
            name: "simple_select".to_string(),
            sql: "SELECT id, name FROM users WHERE active = true".to_string(),
            dialect: "mysql".to_string(),
        }
    );

    queries.insert(
        "complex_join".to_string(),
        TestQuery {
            name: "complex_join".to_string(),
            sql: r#"
                SELECT u.id, u.name, o.order_id, p.product_name
                FROM users u
                INNER JOIN orders o ON u.id = o.user_id
                LEFT JOIN order_items oi ON o.order_id = oi.order_id
                LEFT JOIN products p ON oi.product_id = p.id
                WHERE o.created_at > '2024-01-01'
            "#.trim().to_string(),
            dialect: "mysql".to_string(),
        }
    );

    queries.insert(
        "nested_subquery".to_string(),
        TestQuery {
            name: "nested_subquery".to_string(),
            sql: r#"
                SELECT u.name, u.email
                FROM users u
                WHERE u.id IN (
                    SELECT o.user_id
                    FROM orders o
                    WHERE o.total > (
                        SELECT AVG(o2.total)
                        FROM orders o2
                        WHERE o2.user_id = o.user_id
                    )
                )
            "#.trim().to_string(),
            dialect: "postgresql".to_string(),
        }
    );

    queries
}

pub fn load_queries_from_files() -> Vec<TestQuery> {
    // For future: load from src/fixtures/queries/*.sql
    vec![]
}
```

**Step 2: Create SQL fixture files**

Create `benches/profiling/src/fixtures/queries/simple_select.sql`:
```sql
SELECT id, name FROM users WHERE active = true
```

Create `benches/profiling/src/fixtures/queries/complex_join.sql`:
```sql
SELECT u.id, u.name, o.order_id, p.product_name
FROM users u
INNER JOIN orders o ON u.id = o.user_id
LEFT JOIN order_items oi ON o.order_id = oi.order_id
LEFT JOIN products p ON oi.product_id = p.id
WHERE o.created_at > '2024-01-01'
```

Create `benches/profiling/src/fixtures/queries/nested_subquery.sql`:
```sql
SELECT u.name, u.email
FROM users u
WHERE u.id IN (
    SELECT o.user_id
    FROM orders o
    WHERE o.total > (
        SELECT AVG(o2.total)
        FROM orders o2
        WHERE o2.user_id = o.user_id
    )
)
```

**Step 3: Verify module compiles**

Run:
```bash
cd benches/profiling
cargo check --benches 2>&1 | grep fixtures.rs
```

Expected: No compilation errors in fixtures.rs

**Step 4: Commit**

```bash
git add benches/profiling/benches/fixtures.rs
git add benches/profiling/src/fixtures/
git commit -m "feat(perf-001): add fixtures module and SQL test queries"
```

---

## Task 5: Create Operations Module

**Files:**
- Create: `benches/profiling/benches/operations.rs`

**Step 1: Create operations module**

Create file `benches/profiling/benches/operations.rs`:
```rust
use unified_sql_lsp_grammar::{Parser, Dialect};
use unified_sql_lsp_context::{DocumentState, CompletionContext};
use unified_sql_lsp_semantic::{ScopeManager, SemanticAnalyzer};
use lsp_types::{Position, Range};
use std::sync::Arc;

pub struct OperationResult {
    pub duration_ns: u128,
    pub output_size: usize,
}

/// Execute completion at the given position
pub fn execute_completion(
    doc: &DocumentState,
    position: Position,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Detect completion context
    let context = CompletionContext::detect(doc, position)
        .map_err(|e| format!("Context detection failed: {}", e))?;

    // Get completion items
    let _items = match context {
        unified_sql_lsp_context::CompletionKind::SelectColumns => {
            // This would call the actual completion logic
            vec![]
        }
        _ => vec![],
    };

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: 0,
    })
}

/// Execute hover at the given position
pub fn execute_hover(
    doc: &DocumentState,
    position: Position,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Parse document
    let parser = Parser::new(Dialect::MySQL);
    let cst = parser.parse(doc.content())
        .map_err(|e| format!("Parse failed: {}", e))?;

    // Build semantic analysis
    let mut scope_manager = ScopeManager::new();
    let _analyzer = SemanticAnalyzer::new(&mut scope_manager);
    // analyzer.analyze(&cst); // Would run full analysis

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: 0,
    })
}

/// Execute full diagnostics on document
pub fn execute_diagnostics(
    doc: &DocumentState,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Parse
    let parser = Parser::new(Dialect::MySQL);
    let cst = parser.parse(doc.content())
        .map_err(|e| format!("Parse failed: {}", e))?;

    // Full semantic analysis
    let mut scope_manager = ScopeManager::new();
    let _analyzer = SemanticAnalyzer::new(&mut scope_manager);
    // analyzer.analyze(&cst);

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: cst.root_node().child_count(),
    })
}

/// Apply simulated document changes
pub fn apply_document_change(
    doc: &mut DocumentState,
    changes: Vec<(Range, String)>,
) {
    for (range, new_text) in changes {
        doc.apply_change(range, new_text);
    }
}
```

**Step 2: Check for compilation errors**

Run:
```bash
cd benches/profiling
cargo check --benches 2>&1 | grep -A5 "operations.rs"
```

Expected: May have API mismatch errors (we'll fix in next task by checking actual APIs)

**Step 3: Commit (even if incomplete)**

```bash
git add benches/profiling/benches/operations.rs
git commit -m "feat(perf-001): add operations module with LSP executors"
```

---

## Task 6: Inspect Actual APIs and Fix Operations

**Files:**
- Reference: `crates/context/src/lib.rs`
- Reference: `crates/semantic/src/lib.rs`
- Reference: `crates/lsp/src/document.rs`
- Modify: `benches/profiling/benches/operations.rs`

**Step 1: Read actual API documentation**

Run:
```bash
cd /home/woxQAQ/unified-sql-lsp/.worktrees/lsp-profiling
rg "pub struct DocumentState" crates/
rg "pub fn CompletionContext::" crates/context/
rg "pub fn ScopeManager::" crates/semantic/
```

Expected: Find actual API signatures and usage patterns

**Step 2: Update operations.rs with correct APIs**

Edit `benches/profiling/benches/operations.rs` based on actual APIs found.
Key fixes typically needed:
- Correct `DocumentState` constructor
- Correct `CompletionContext::detect()` signature
- Correct `SemanticAnalyzer` usage

**Step 3: Verify compilation**

Run:
```bash
cd benches/profiling
cargo check --benches 2>&1 | grep -E "error|warning" | head -20
```

Expected: No errors in operations.rs (may still have missing workload module)

**Step 4: Commit API fixes**

```bash
git add benches/profiling/benches/operations.rs
git commit -m "fix(perf-001): update operations to use correct crate APIs"
```

---

## Task 7: Create Workload Module

**Files:**
- Create: `benches/profiling/benches/workload.rs`

**Step 1: Create workload module**

Create file `benches/profiling/benches/workload.rs`:
```rust
use crate::fixtures::{TestQuery, load_test_queries};
use crate::operations::{execute_completion, execute_hover, execute_diagnostics};
use unified_sql_lsp_context::DocumentState;
use unified_sql_lsp_grammar::Parser;
use lsp_types::Position;

pub struct WorkloadResult {
    pub operations_executed: usize,
    pub total_duration_ns: u128,
}

/// Run completion scenario on a single query
pub fn run_completion_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);
    let position = find_completion_position(&doc);

    let result = execute_completion(&doc, position).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Run hover scenario on a single query
pub fn run_hover_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);
    let position = find_hover_position(&doc);

    let result = execute_hover(&doc, position).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Run diagnostics scenario on a single query
pub fn run_diagnostics_scenario(query: &TestQuery) -> WorkloadResult {
    let doc = create_document(query);

    let result = execute_diagnostics(&doc).unwrap();

    WorkloadResult {
        operations_executed: 1,
        total_duration_ns: result.duration_ns,
    }
}

/// Simulate realistic editing session with mixed operations
pub fn simulate_editing_session() -> WorkloadResult {
    let queries = load_test_queries();
    let mut total_operations = 0;
    let mut total_duration = 0;

    for (_, query) in queries.iter().take(3) {
        // Simulate document open
        let mut doc = create_document(query);

        // Initial diagnostics
        let diag_result = execute_diagnostics(&doc).unwrap();
        total_duration += diag_result.duration_ns;
        total_operations += 1;

        // Simulate some edits
        simulate_edits(&mut doc);

        // Re-run diagnostics after edit
        let diag_result = execute_diagnostics(&doc).unwrap();
        total_duration += diag_result.duration_ns;
        total_operations += 1;

        // Try completion at various positions
        for _ in 0..3 {
            let pos = find_completion_position(&doc);
            let comp_result = execute_completion(&doc, pos).unwrap();
            total_duration += comp_result.duration_ns;
            total_operations += 1;
        }
    }

    WorkloadResult {
        operations_executed: total_operations,
        total_duration_ns: total_duration,
    }
}

fn create_document(query: &TestQuery) -> DocumentState {
    // Create a document state with the query content
    DocumentState::new(
        "test.sql".into(),
        query.sql.clone(),
        0, // version
    )
}

fn find_completion_position(doc: &DocumentState) -> Position {
    // Find SELECT clause position (simplified: line 0, char 10)
    Position { line: 0, character: 10 }
}

fn find_hover_position(doc: &DocumentState) -> Position {
    // Find a column reference position (simplified)
    Position { line: 0, character: 10 }
}

fn simulate_edits(doc: &mut DocumentState) {
    // Apply 1-3 character changes to simulate typing
    use lsp_types::Range;

    let content = doc.content();
    if let Some(pos) = content.find('FROM') {
        let range = Range {
            start: lsp_types::Position { line: 0, character: pos as u32 },
            end: lsp_types::Position { line: 0, character: pos as u32 + 4 },
        };
        // Small edit
        // doc.apply_change(range, "from".into()); // Uncomment if API exists
    }
}
```

**Step 2: Verify compilation**

Run:
```bash
cd benches/profiling
cargo check --benches 2>&1 | grep -E "error" | head -10
```

Expected: May have API mismatches in DocumentState usage (fix next)

**Step 3: Commit**

```bash
git add benches/profiling/benches/workload.rs
git commit -m "feat(perf-001): add workload generation module"
```

---

## Task 8: Fix API Mismatches in Workload

**Files:**
- Reference: `crates/lsp/src/document.rs` or `crates/context/src/document.rs`
- Modify: `benches/profiling/benches/workload.rs`

**Step 1: Find DocumentState constructor**

Run:
```bash
rg "impl DocumentState" crates/ -A 10 | head -30
```

**Step 2: Update workload.rs with correct APIs**

Fix DocumentState creation and any other API mismatches.

**Step 3: Build successfully**

Run:
```bash
cd benches/profiling
cargo build --benches 2>&1 | tail -5
```

Expected: "Finished" with no errors

**Step 4: Commit fixes**

```bash
git add benches/profiling/benches/workload.rs
git commit -m "fix(perf-001): fix DocumentState API usage in workload"
```

---

## Task 9: Update Root Cargo.toml

**Files:**
- Modify: `Cargo.toml` (root)

**Step 1: Add workspace exclusion**

Edit root `Cargo.toml`, add to `[workspace]` section:
```toml
[workspace]
exclude = ["benches"]
```

**Step 2: Verify both workspaces work**

Run:
```bash
# Root workspace
cargo check 2>&1 | tail -3

# Profiling workspace
cd benches/profiling
cargo check 2>&1 | tail -3
```

Expected: Both build successfully

**Step 3: Commit**

```bash
cd /home/woxQAQ/unified-sql-lsp/.worktrees/lsp-profiling
git add Cargo.toml
git commit -m "feat(perf-001): exclude benches/ from root workspace"
```

---

## Task 10: Update Profiling Scripts

**Files:**
- Modify: `scripts/profiling/flamegraph.sh`
- Modify: `scripts/profiling/run_all.sh`

**Step 1: Update flamegraph.sh**

Edit `scripts/profiling/flamegraph.sh`:
```bash
#!/bin/bash
# Generate flamegraph for LSP operations

set -e

FLAMEGRAPH_DIR="target/flamegraphs"
mkdir -p "$FLAMEGRAPH_DIR"

echo "=== Generating LSP Flamegraph ==="

if ! command -v cargo-flamegraph &> /dev/null; then
    echo "Error: cargo-flamegraph not installed"
    echo "Install with: cargo install flamegraph"
    exit 1
fi

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
OUTPUT_PATH="$FLAMEGRAPH_DIR/flamegraph-$TIMESTAMP.svg"

echo "Running LSP operations benchmark..."
cd benches/profiling
cargo flamegraph --bench lsp_operations \
    --output "../$OUTPUT_PATH"

echo ""
echo "Flamegraph generated: $OUTPUT_PATH"
echo "Open with: xdg-open $OUTPUT_PATH"
echo ""
echo "Recent flamegraphs:"
ls -lt "target/flamegraphs"/flamegraph-*.svg 2>/dev/null | head -5 || echo "  No previous flamegraphs found"
```

**Step 2: Update run_all.sh**

Edit `scripts/profiling/run_all.sh`, replace the flamegraph section (around line 26-32):
```bash
echo ""
echo "2. Generating LSP flamegraph..."
if command -v cargo-flamegraph &> /dev/null; then
    cd benches/profiling
    cargo flamegraph --bench lsp_operations \
        --output "../../target/profiling-reports/$REPORT_DIR/flamegraph.svg" || true
    cd ../..
else
    echo "  (flamegraph not installed, skipping)"
fi
```

**Step 3: Test the scripts**

Run:
```bash
bash scripts/profiling/flamegraph.sh 2>&1 | tail -10
```

Expected: Flamegraph generated successfully

**Step 4: Commit**

```bash
git add scripts/profiling/
git commit -m "feat(perf-001): update profiling scripts for LSP flamegraph"
```

---

## Task 11: Create Documentation

**Files:**
- Create: `benches/profiling/README.md`
- Modify: `scripts/profiling/README.md` (if exists, else create)

**Step 1: Create profiling README**

Create `benches/profiling/README.md`:
```markdown
# LSP Performance Profiling

## Purpose

Generate CPU flamegraphs of real LSP operations (completion, hover, diagnostics)
to identify performance bottlenecks in semantic analysis and parsing.

## Quick Start

```bash
# Generate flamegraph for all operations
cd benches/profiling
cargo flamegraph --bench lsp_operations

# Generate flamegraph for specific operation
cargo flamegraph --bench lsp_operations -- --bench completion

# Open the result
xdg-open target/flamegraph/flamegraph-*.svg
```

## Benchmark Scenarios

### Individual Operations

- `completion` - Code completion at various positions (SELECT, FROM, WHERE)
- `hover` - Type information and documentation lookup
- `diagnostics` - Full document analysis with error detection
- `editing_session` - Realistic mixed workload (open → edit → analyze)

### Fixtures

Test queries are located in `src/fixtures/queries/`:
- `simple_select.sql` - Basic SELECT query
- `complex_join.sql` - Multi-table JOIN with filters
- `nested_subquery.sql` - Subqueries with aggregations

To add more queries, place `.sql` files in the fixtures directory and update
`fixtures::load_test_queries()` in `benches/fixtures.rs`.

## Interpreting Results

### Key Areas to Examine

1. **Tree-sitter Parsing** (grammar layer)
   - High time here → consider grammar optimization or caching improvements

2. **Scope Building** (semantic layer)
   - High time here → optimize scope management and symbol resolution

3. **Completion Logic** (context layer)
   - High time here → improve candidate filtering and computation

4. **Catalog Queries** (database layer)
   - High time here → improve caching or connection pooling

### Red Flags

- Time spent in serialization/deserialization (should be minimal)
- Exponential growth with query complexity
- Hotspots in utility functions that should be cheap

## Workflow

1. **Generate baseline:** `make flamegraph-lsp`
2. **Identify bottleneck:** Inspect SVG for hotspots
3. **Optimize:** Address the identified issue
4. **Verify:** Re-run flamegraph to confirm improvement
5. **Track:** Save SVG with date for historical comparison

## Integration with CI

The profiling workspace is excluded from the main workspace to avoid
affecting normal build times. Profiling is run manually or in dedicated
performance pipelines.
```

**Step 2: Create or update profiling scripts README**

Create/edit `scripts/profiling/README.md`:
```markdown
# Profiling Scripts

This directory contains scripts for comprehensive performance profiling
of the unified-sql-lsp server.

## Available Scripts

### `run_all.sh`
Run complete profiling suite including Criterion benchmarks, flamegraphs,
and memory profiling.

```bash
./scripts/profiling/run_all.sh
```

Output: `target/profiling-reports/YYYYMMDD-HHMMSS/`

### `flamegraph.sh`
Quick generation of LSP flamegraph only.

```bash
./scripts/profiling/flamegraph.sh
```

Output: `target/flamegraphs/flamegraph-YYYYMMDD-HHMMSS.svg`

## Tools Used

### Criterion (Regression Tracking)
- **Purpose:** Statistical benchmarking over time
- **Output:** HTML reports with confidence intervals
- **Use Case:** Detect performance regressions in CI
- **Location:** `target/criterion/index.html`

### Flamegraph (Deep Analysis)
- **Purpose:** CPU profiling with stack trace visualization
- **Output:** Interactive SVG flamegraph
- **Use Case:** Identify specific bottlenecks in hot code paths
- **Location:** `target/flamegraphs/flamegraph-*.svg`

## When to Use Each

| Situation | Tool |
|-----------|------|
| CI regression testing | Criterion |
| Investigating slowdown | Flamegraph |
| Before/after comparison | Both |
| Memory leak analysis | Valgrind/DHat (future) |
| Optimization iteration | Flamegraph |

## LSP Operations Profiled

The flamegraph profiles these core LSP operations:

1. **Completion:** Code completion at various query positions
2. **Hover:** Type information and documentation lookup
3. **Diagnostics:** Full document syntax and semantic analysis
4. **Document Sync:** Incremental re-analysis on document changes

These operations are executed on realistic SQL queries without the
overhead of the LSP protocol server (tower-lsp), giving a clear view
of actual semantic analysis performance.
```

**Step 3: Commit documentation**

```bash
git add benches/profiling/README.md
git add scripts/profiling/README.md
git commit -m "docs(perf-001): add comprehensive profiling documentation"
```

---

## Task 12: Add Makefile Target

**Files:**
- Modify: `Makefile`

**Step 1: Add flamegraph target**

Add to `Makefile`:
```makefile
# LSP Profiling
.PHONY: flamegraph-lsp
flamegraph-lsp:
	cd benches/profiling && cargo flamegraph --bench lsp_operations
```

**Step 2: Test make target**

Run:
```bash
make flamegraph-lsp 2>&1 | tail -5
```

Expected: Flamegraph generated successfully

**Step 3: Update Makefile help target**

Add `flamegraph-lsp` to the help output if you have a help target.

**Step 4: Commit**

```bash
git add Makefile
git commit -m "feat(perf-001): add flamegraph-lsp make target"
```

---

## Task 13: End-to-End Testing

**Files:**
- Test: All created files

**Step 1: Run complete profiling suite**

Run:
```bash
# From worktree root
./scripts/profiling/run_all.sh 2>&1 | tail -20
```

Expected: Complete report generated with all sections

**Step 2: Verify flamegraph generation**

Run:
```bash
make flamegraph-lsp
ls -lh target/flamegraphs/flamegraph-*.svg | tail -1
```

Expected: SVG file exists and is non-empty

**Step 3: Check SVG is valid**

Run:
```bash
file target/flamegraphs/flamegraph-*.svg | tail -1
```

Expected: "SVG" or "XML" document type

**Step 4: Test individual operations**

Run:
```bash
cd benches/profiling
cargo bench --bench lsp_operations -- --bench completion 2>&1 | grep "completion"
```

Expected: Benchmark runs successfully

**Step 5: Verify Criterion reports**

Run:
```bash
cd benches/profiling
cargo bench --bench lsp_operations 2>&1 | tail -5
```

Expected: Criterion HTML report generated

**Step 6: Commit any fixes**

```bash
# Add any fixes discovered during testing
git add -A
git commit -m "test(perf-001): fix issues found during end-to-end testing"
```

---

## Task 14: Verify in Main Branch (Optional)

**Files:**
- Test: Git merge verification

**Step 1: Check branch status**

Run:
```bash
git status
git log --oneline -5
```

Expected: Clean working tree, all commits present

**Step 2: Prepare for merge**

Run:
```bash
git log main..HEAD --oneline
```

Expected: List of all feature commits

**Step 3: Create summary of changes**

Create summary in memory:
- Created `benches/profiling/` workspace
- Added LSP operations benchmark
- Updated profiling scripts
- Added comprehensive documentation

**Step 4: Final commit**

```bash
git commit --allow-empty -m "chore(perf-001): implementation complete

Implemented LSP profiling flamegraph infrastructure:
- Separate profiling workspace at benches/profiling/
- Criterion benchmarks for completion, hover, diagnostics
- Realistic editing session workload simulation
- Updated profiling scripts to use new benchmark
- Comprehensive documentation in README files

Ready for testing and integration."
```

---

## Success Criteria

Verify all criteria are met:

- [ ] `benches/profiling/` workspace builds successfully
- [ ] `make flamegraph-lsp` generates valid SVG flamegraph
- [ ] Flamegraph shows semantic analysis hotspots (not cargo build)
- [ ] All benchmark scenarios run without panics
- [ ] Documentation is comprehensive and accurate
- [ ] Profiling scripts updated and tested
- [ ] Root workspace excludes `benches/` directory
- [ ] Individual operation benchmarks complete successfully

---

## Next Steps After Implementation

1. **Generate baseline:** Run `make flamegraph-lsp` and save baseline SVG
2. **Analyze bottlenecks:** Identify top hotspots in semantic analysis
3. **Optimize:** Address the worst bottlenecks
4. **Compare:** Re-run flamegraph to verify improvements
5. **Historical tracking:** Store flamegraphs with dates for trend analysis

## Troubleshooting

**Issue:** Cargo.toml workspace conflicts
- **Fix:** Ensure root Cargo.toml has `exclude = ["benches"]`

**Issue:** Benchmark compilation errors
- **Fix:** Check API signatures in actual crates, update operations/workload modules

**Issue:** Flamegraph shows "cargo build"
- **Fix:** Ensure running `cd benches/profiling` before `cargo flamegraph`

**Issue:** No SVG generated
- **Fix:** Install flamegraph: `cargo install flamegraph`

**Issue:** Tests fail in new workspace
- **Fix:** This is expected - profiling workspace doesn't need unit tests
