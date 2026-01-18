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
echo "2. Generating LSP flamegraph..."
if command -v cargo-flamegraph &> /dev/null; then
    cd benches/profiling
    cargo flamegraph --bench lsp_operations \
        --output "../../target/profiling-reports/$REPORT_DIR/flamegraph.svg" || true
    cd ../..
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
