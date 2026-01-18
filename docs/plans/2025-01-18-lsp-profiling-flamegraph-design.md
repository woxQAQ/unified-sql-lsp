# LSP Profiling Flamegraph Design

**Date:** 2025-01-18
**Status:** Approved
**Author:** Claude (with user collaboration)

## Problem Statement

The current flamegraph implementation (`cargo flamegraph --bench completion_pipeline`) profiles `cargo build` performance rather than actual LSP server operations. This makes it ineffective for identifying performance bottlenecks in real LSP operations like completion, hover, and diagnostics.

## Goals

1. Generate authentic CPU flamegraphs of LSP operation performance
2. Profile realistic workloads without tower-lsp server overhead
3. Provide actionable insights into semantic analysis and completion hotspots
4. Maintain clean separation between production code and profiling infrastructure

## Architecture

### Overall Approach

Create a separate profiling workspace that simulates realistic LSP operations on representative SQL queries. The benchmark executes core semantic analysis, completion logic, and parsing operations that directly impact LSP user experience.

**Key Principle:** Profile the actual work (parsing, semantic analysis, completion) without the noise of IO, tokio runtime, and LSP protocol serialization.

### Workspace Structure

```
benches/
├── profiling/
│   ├── Cargo.toml              # Separate workspace
│   ├── README.md               # Usage instructions
│   └── src/
│       ├── main.rs             # Benchmark entry points
│       ├── workload.rs         # Workload generation
│       ├── operations.rs       # LSP operation executors
│       └── fixtures/
│           ├── queries/        # SQL test queries
│           └── scenarios/      # Operation sequences
```

**Cargo.toml Structure:**
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
# ... other core crates

# Dev/profiling-only dependencies
criterion = "0.5"
flamegraph = "0.6"

[features]
default = ["profiling"]
profiling = []  # Feature gate for profiling-specific code
```

### Components

#### 1. Workload Generator (`workload.rs`)

**Structures:**
- `WorkloadScenario` - Defines what operations to run
- `load_sql_fixtures()` - Loads test queries from fixtures
- `simulate_editing_session()` - Orchestrates document lifecycle

**Responsibilities:**
- Load representative SQL queries from fixtures
- Manage document state (open → edit → close lifecycle)
- Coordinate operation execution at realistic positions

#### 2. Operation Executors (`operations.rs`)

**Functions:**
- `execute_completion(doc, position)` - Calls completion logic directly
- `execute_hover(doc, position)` - Invokes hover provider
- `execute_diagnostics(doc)` - Runs full document analysis
- `execute_document_change(doc, changes)` - Simulates edits

**Implementation:** Reuse existing semantic/context layer APIs directly, bypassing LSP protocol layer.

#### 3. Position Locator

**Functions:**
- `find_interesting_positions(cst)` - Identifies completion/hover spots
- Returns positions in SELECT, FROM, WHERE, JOIN clauses

**Approach:** CST traversal to dynamically find positions (not hardcoded).

### Data Flow

```
1. Load SQL fixtures from fixtures/benchmarks/queries/*.sql
2. For each query:
   a. Create DocumentState (as if LSP did_open)
   b. Parse with tree-sitter → CST
   c. Run semantic analysis → ScopeManager
   d. Execute operations at detected positions:
      - completion at SELECT clause
      - hover at column references
      - diagnostics on full document
   e. Apply simulated edits (1-3 character changes)
   f. Re-run operations on edited document
```

### Benchmark Scenarios

**Individual Operations** (focused profiling):
- `completion` - Code completion at various positions
- `hover` - Type information and documentation
- `diagnostics` - Full document analysis
- `document_sync` - Document changes and incremental re-analysis

**Realistic Workload** (overall performance):
- `editing_session` - Simulated real-world editing pattern with mixed operations

### Main Entry Point

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_lsp_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsp_operations");

    // Individual operation benchmarks
    group.bench_function("completion", |b| {
        b.iter(|| workload::run_completion_scenario(black_box()))
    });

    group.bench_function("hover", |b| {
        b.iter(|| workload::run_hover_scenario(black_box()))
    });

    group.bench_function("diagnostics", |b| {
        b.iter(|| workload::run_diagnostics_scenario(black_box()))
    });

    // Realistic editing session (mixed workload)
    group.bench_function("editing_session", |b| {
        b.iter(|| workload::simulate_editing_session(black_box()))
    });

    group.finish();
}

criterion_group!(benches, benchmark_lsp_operations);
criterion_main!(benches);
```

### Flamegraph Generation

```bash
# From repo root
cd benches/profiling
cargo flamegraph --bench lsp_operations -- --bench editing_session

# Or run all operations
cargo flamegraph --bench lsp_operations

# Output: benches/profiling/target/flamegraph-*.svg
```

## Error Handling

### Strategy

1. **Benchmark Failures:**
   - Fixture loading fails → panic (fast failure, clear error)
   - Parsing fails → skip query, log warning, continue
   - Operation timeout → fail gracefully with metrics so far

2. **Graceful Degradation:**
   - Partial parsing success → still profile what works
   - Missing optional fixtures → use simplified defaults
   - Position detection fails → fallback to heuristic positions

3. **Logging:**
   - Use `env_logger` with `RUST_LOG=info`
   - Structured logs: "Loaded 15 fixtures", "Skipped query-X: parse error"

## Migration Plan

### Update Existing Scripts

**1. `scripts/profiling/flamegraph.sh`:**
```bash
# OLD: cargo flamegraph --bench completion_pipeline
# NEW:
cd benches/profiling
cargo flamegraph --bench lsp_operations \
    --output "../target/flamegraphs/flamegraph-$TIMESTAMP.svg"
```

**2. `scripts/profiling/run_all.sh`:**
```bash
# Keep Criterion benchmarks (regression tracking)
cargo bench --benches completion,parsing,semantic,catalog,concurrency

# Replace flamegraph section
echo "2. Generating LSP flamegraph..."
cd benches/profiling
cargo flamegraph --bench lsp_operations \
    --output "../../target/profiling-reports/$REPORT_DIR/flamegraph.svg"
```

### Workspace Configuration

- Update root `Cargo.toml` to exclude `benches/`
- Add `benches/profiling/Cargo.toml` as separate workspace
- Update Makefile to add `make flamegraph-lsp` target

### Backward Compatibility

- Old `completion_pipeline` benchmark remains for CI regression tests
- New profiling is **additional**, not replacing existing benchmarks
- Criterion for regression tracking, flamegraph for deep analysis

## Testing

### Strategy

1. **Unit Tests** (in `benches/profiling/src/`):
   - Test fixture loading: verify queries parse correctly
   - Test position detection: ensure interesting positions found
   - Test workload runner: validate operation execution

2. **Validation Tests:**
   - Run flamegraph benchmark → ensure no panics
   - Verify flamegraph SVG is generated
   - Check that all operation types complete

3. **Smoke Test:**
   ```bash
   cd benches/profiling
   cargo test
   cargo flamegraph --bench lsp_operations -- --test
   ```

## Documentation

### Files to Create

1. **`benches/profiling/README.md`:**
   - Purpose and quick start
   - Benchmark scenario descriptions
   - Fixture organization
   - Interpreting flamegraph results

2. **Update `scripts/profiling/README.md`:**
   - Add section: Flamegraph vs Criterion differences
   - When to use each profiling tool

### Makefile Integration

```makefile
flamegraph-lsp:
	cd benches/profiling && cargo flamegraph --bench lsp_operations
```

## Success Criteria

- [ ] Flamegraph shows semantic analysis hotspots (not cargo build)
- [ ] Can identify bottlenecks in completion/hover/diagnostics
- [ ] Workflow: `make flamegraph-lsp` → inspect SVG → optimize
- [ ] Benchmark runs without panics on all fixtures
- [ ] Generated flamegraph SVG is viewable and interpretable

## Future Enhancements

1. **Historical Tracking:** Store flamegraphs over time to detect regressions
2. **Comparison Mode:** Generate diff flamegraphs between commits
3. **Custom Scenarios:** Allow users to provide custom SQL queries for profiling
4. **CI Integration:** Automated flamegraph generation on performance benchmarks

## Appendix: Interpreting Flamegraphs

**Key Areas to Examine:**

1. **Tree-sitter Parsing** (grammar layer)
   - High time here → consider grammar optimization or caching

2. **Scope Building** (semantic layer)
   - High time here → optimize scope management logic

3. **Completion Logic** (context layer)
   - High time here → improve candidate filtering/computation

4. **Catalog Queries** (database layer)
   - High time here → improve caching or connection pooling

**Red Flags:**
- Time spent in serialization/deserialization (should be minimal)
- Exponential growth with query complexity
- Hotspots in utility functions that should be cheap
