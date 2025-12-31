#!/bin/bash
set -e

# Build script for tree-sitter grammars
# Builds grammar for multiple dialects

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAMMAR_DIR="$SCRIPT_DIR/src/grammar"

DIALECTS=("base" "mysql" "postgresql")

echo "Building tree-sitter grammars..."
echo "======================================"

cd "$GRAMMAR_DIR"

for DIALECT in "${DIALECTS[@]}"; do
  echo ""
  echo "Building dialect: $DIALECT"
  echo "-------------------------"

  export DIALECT=$DIALECT

  if [ -f "src/parser.c" ]; then
    rm src/parser.c
  fi

  if [ -f "src/parser.h" ]; then
    rm src/parser.h
  fi

  tree-sitter generate --no-bindings

  if [ -f "src/parser.c" ]; then
    echo "✓ Successfully built $DIALECT grammar"

    # Copy the parser to a dialect-specific file
    cp src/parser.c "src/parser-${DIALECT}.c"

    echo "  → Saved as src/parser-${DIALECT}.c"
  else
    echo "✗ Failed to build $DIALECT grammar"
    exit 1
  fi
done

echo ""
echo "======================================"
echo "All grammars built successfully!"
echo ""
echo "Generated files:"
ls -lh src/parser-*.c 2>/dev/null || echo "  No parser files found"
echo ""
