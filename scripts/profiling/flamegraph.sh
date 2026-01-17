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
