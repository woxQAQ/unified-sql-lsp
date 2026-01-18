#!/bin/bash
# Generate flamegraph for completion pipeline

set -e

# Output directory for flamegraphs (also where perf.data will be written)
FLAMEGRAPH_DIR="target/flamegraphs"
mkdir -p "$FLAMEGRAPH_DIR"

echo "=== Generating Flamegraph ==="

# Check if flamegraph is installed
if ! command -v cargo-flamegraph &> /dev/null; then
    echo "Error: cargo-flamegraph not installed"
    echo "Install with: cargo install flamegraph"
    exit 1
fi

# Build with profiling instrumentation
cargo build --release --features profiling

# Generate flamegraph with timestamp
# We cd to FLAMEGRAPH_DIR so perf.data is written there instead of project root
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
OUTPUT_PATH="$FLAMEGRAPH_DIR/flamegraph-$TIMESTAMP.svg"

(
    cd "$FLAMEGRAPH_DIR" && \
    cargo flamegraph --bench completion_pipeline \
        --output "flamegraph-$TIMESTAMP.svg"
)

echo ""
echo "Flamegraph generated: $OUTPUT_PATH"
echo "Open with: xdg-open $OUTPUT_PATH"
echo ""
echo "Recent flamegraphs:"
ls -lt "$FLAMEGRAPH_DIR"/flamegraph-*.svg 2>/dev/null | head -5 || echo "  No previous flamegraphs found"
