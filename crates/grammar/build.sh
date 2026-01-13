#!/usr/bin/env bash
set -e

# Build script for tree-sitter grammars
# Builds grammar for multiple dialects

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
GRAMMAR_DIR="$SCRIPT_DIR/src/grammar"

DIALECTS=("base" "mysql-5.7" "mysql-8.0" "postgresql-12" "postgresql-14")

echo "Building tree-sitter grammars..."
echo "======================================"

cd "$GRAMMAR_DIR"

for DIALECT in "${DIALECTS[@]}"; do
  echo ""
  echo "Building dialect: $DIALECT"
  echo "-------------------------"

  export DIALECT=$DIALECT

  tree-sitter generate -o gen

  if [ -f "gen/parser.c" ]; then
    echo "✓ Successfully built $DIALECT grammar"

    # Rename to dialect-specific file to avoid overwriting
    mv gen/parser.c "gen/parser-${DIALECT}.c"

    echo "  → Saved as gen/parser-${DIALECT}.c"
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
ls -lh gen/parser-*.c 2>/dev/null || echo "  No parser files found"
echo ""
