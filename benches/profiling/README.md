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
`fixtures::load_test_queries()` in `src/fixtures.rs`.

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
