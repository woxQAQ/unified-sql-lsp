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
    --output "../../$OUTPUT_PATH"

echo ""
echo "Flamegraph generated: $OUTPUT_PATH"
echo "Open with: xdg-open $OUTPUT_PATH"
echo ""
echo "Recent flamegraphs:"
ls -lt "target/flamegraphs"/flamegraph-*.svg 2>/dev/null | head -5 || echo "  No previous flamegraphs found"
