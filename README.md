# Unified SQL LSP

A unified Language Server Protocol implementation for multiple SQL dialects.

## Overview

This project provides a comprehensive LSP server for SQL that supports multiple database engines with intelligent features like:

- **Multi-dialect support**: MySQL, PostgreSQL, and more
- **Schema-aware completion**: Context-aware suggestions based on your database schema
- **Real-time validation**: Syntax and semantic error detection
- **High performance**: Incremental parsing and caching

## Architecture

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

## Quick Start

### Building

```bash
# Build all crates
cargo build --workspace

# Or build specific crates
cargo build -p unified-sql-grammar
cargo build -p unified-sql-lsp-lsp
```

### Testing

```bash
# Run all tests
cargo test --workspace

# Run specific test suites
cargo test -p unified-sql-grammar
cargo test -p unified-sql-lsp-lsp
```

See [TESTING.md](TESTING.md) for comprehensive testing documentation.

## Project Structure

```
unified-sql-lsp/
├── crates/
│   ├── grammar/          # Tree-sitter grammar definitions
│   │   └── src/grammar/  # Grammar files (dialect/*.js)
│   ├── ir/               # Intermediate Representation
│   ├── lowering/         # CST → IR conversion
│   ├── semantic/         # Semantic analysis
│   └── catalog/          # Database schema abstraction
├── scripts/              # Build and utility scripts
├── FEATURE_LIST.yaml     # Feature tracking and milestones
├── DESIGN.md             # Detailed architecture documentation
├── TESTING.md            # Comprehensive testing guide
└── flake.nix             # Nix flake configuration
```

## Supported Dialects

| Dialect    | Status      | Version Support |
|------------|-------------|-----------------|
| MySQL      | ✅ Planned  | 5.7, 8.0+       |
| PostgreSQL | ✅ Planned  | 12+             |
| TiDB       | 🚧 Planned  | 5.0, 6.0, 7.0+  |

## Development

### Prerequisites

- Rust 2024 edition
- Node.js (for tree-sitter CLI)
- tree-sitter-cli: `npm install -g tree-sitter-cli`

### Building

```bash
# Build all crates
cargo build

# Build grammar for all dialects
cd crates/grammar && ./build.sh
```

### Testing

```bash
# Run all tests
cargo test

# Test specific dialect
cd crates/grammar
DIALECT=mysql tree-sitter test
```

## Current Status

See [FEATURE_LIST.yaml](./FEATURE_LIST.yaml) for detailed feature tracking.

### Completed

- ✅ IR core types
- ✅ Lowering trait definition
- ✅ Catalog trait definition
- ✅ LiveCatalog (MySQL) implementation
- ✅ Tree-sitter grammar foundation

### In Progress

- 🚧 MySQL dialect grammar implementation
- 🚧 PostgreSQL dialect grammar implementation
- 🚧 Semantic analyzer core logic

## Roadmap

### MVP - Basic MySQL Completion
- Foundation: Grammar, IR, Lowering, Semantic
- Catalog integration
- Basic completion (columns, tables)

### M1 - Multi-dialect Completion
- PostgreSQL support
- All completion contexts
- Function completion

### M2 - Diagnostics
- Syntax errors
- Undefined tables/columns
- Ambiguity detection

### M3 - Performance
- Three-tier caching
- Concurrent semantic analysis
- Schema filtering

## Contributing

Contributions are welcome! Please see [DESIGN.md](./DESIGN.md) for architecture guidelines.

## License

MIT OR Apache-2.0

## References

- [DESIGN.md](./DESIGN.md) - Detailed architecture documentation
- [FEATURE_LIST.yaml](./FEATURE_LIST.yaml) - Feature tracking and milestones
- [Tree-sitter documentation](https://tree-sitter.github.io/tree-sitter/)
