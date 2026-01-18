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
