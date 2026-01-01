# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Commands

### Building

```bash
# Build all crates
cargo build

# Build grammar for all dialects
cd crates/grammar && ./build.sh

# Build specific dialect manually
cd crates/grammar/src/grammar
DIALECT=mysql tree-sitter generate --no-bindings
```

### Testing

```bash
# Run all tests
cargo test

# Test specific dialect
cd crates/grammar/src/grammar
DIALECT=mysql tree-sitter test
```

### Prerequisites

- Rust 2024 edition
- Node.js (for tree-sitter CLI)
- tree-sitter-cli: `npm install -g tree-sitter-cli`

## Architecture Overview

This project implements a Language Server Protocol for SQL with multi-dialect support. The architecture follows a layered approach:

```
┌─────────────────────────────────────────────────────────────────┐
│                        LSP Server Layer                         │
│  - Completion, Hover, Diagnostics                              │
│  - Multi-connection & Multi-engine management                  │
├─────────────────────────────────────────────────────────────────┤
│                      Semantic / Context Layer                   │
│  - Scope & Namespace (table aliases, column resolution)         │
│  - Context Awareness (completion trigger points)                │
├─────────────────────────────────────────────────────────────────┤
│                  Dialect Adaptation Layer                       │
│  - MySQL (5.7, 8.0+) / PostgreSQL / TiDB / ...                  │
├─────────────────────────────────────────────────────────────────┤
│                       SQL IR / AST Layer                        │
│  - Unified Query / Expr / Statement types                      │
├─────────────────────────────────────────────────────────────────┤
│                    Tree-sitter Grammar Layer                    │
│  - Incremental CST parsing (crates/grammar)                     │
└─────────────────────────────────────────────────────────────────┘
```

## Key Design Principles

### IR Layer vs Semantic Layer

**IR (Intermediate Representation) Layer** (`crates/ir/`):
- Unified SQL AST that abstracts away dialect-specific syntax differences
- Example: MySQL's `LIMIT offset, count` and PostgreSQL's `LIMIT count OFFSET offset` both become `Query { limit, offset }`
- LSP logic works with this unified representation, not dialect-specific CST

**Semantic Layer** (`crates/semantic/`):
- Builds scope and symbol tables on top of IR
- Resolves column references (which table does `id` belong to?)
- Detects ambiguous references
- Determines completion context (e.g., are we in SELECT projection vs WHERE clause?)

### Grammar Layer Dialect Inheritance

The tree-sitter grammar uses a compile-time dialect merging strategy:
- `dialect/base.js` - Common SQL rules shared across all dialects
- `dialect/mysql.js`, `dialect/postgresql.js` - Dialect-specific extensions
- `grammar.js` - Main entry point that merges base + selected dialect at compile time via `DIALECT` environment variable

This ensures consistent node naming across dialects and zero runtime overhead.

### Module Dependencies

```
lsp/ ──────┐
           ├─── semantic/
semantic/ ────┤
           └── ir/
lowering/ ──────
  └── grammar/
```

## Crates Structure

- **`crates/grammar/`** - Tree-sitter grammar definitions with dialect support
- **`crates/ir/`** - Intermediate Representation types (unified SQL AST)
- **`crates/lowering/`** - CST to IR conversion (dialect-specific adapters)
- **`crates/semantic/`** - Scope analysis, symbol resolution, completion context
- **`crates/catalog/`** - Database schema abstraction (live connections, caching)
- **`crates/lsp/`** - Language Server Protocol backend

## Important Notes

### Grammar Development

When modifying grammar files:
1. Edit files in `crates/grammar/src/grammar/dialect/` or `grammar.js`
2. Rebuild with `./build.sh` or `DIALECT=<dialect> tree-sitter generate`
3. Test with `DIALECT=<dialect> tree-sitter test`

### Adding New Dialects

1. Create `crates/grammar/src/grammar/dialect/<dialect>.js`
2. Add to `DIALECTS` array in `crates/grammar/build.sh`
3. Implement lowering in `crates/lowering/src/dialect/<dialect>.rs`
4. Update FEATURE_LIST.yaml with new dialect features

### Error Handling

The lowering layer uses a graceful degradation strategy:
- `Success` - Full conversion to IR
- `Partial` - Partial conversion (some clauses couldn't be converted)
- `Failed` - Complete failure, fallback to syntax-based or keywords-only completion

### Multi-Version Support

Different versions of the same dialect (e.g., MySQL 5.7 vs 8.0) may support different features. The project handles this via:
1. Compile-time grammar variants (planned: `mysql-5.7.so`, `mysql-8.0.so`)
2. Runtime version detection using `semver` crate in the lowering layer
3. Feature support queries in the semantic layer

## References

- `DESIGN.md` - Detailed architecture documentation (in Chinese)
- `FEATURE_LIST.yaml` - Feature tracking and milestones
- DerekStride/tree-sitter-sql - Primary reference for grammar design
