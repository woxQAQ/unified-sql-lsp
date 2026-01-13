# Repository Guidelines

## Project Overview

Unified SQL LSP is a Language Server Protocol implementation supporting multiple SQL dialects including MySQL, PostgreSQL, TiDB, MariaDB, and CockroachDB. Key features include multi-dialect grammar support using Tree-sitter, schema-aware code completion, real-time syntax and semantic validation, and high-performance incremental parsing with multi-level caching.

## 1. Project Structure & Module Organization

The codebase follows a layered architecture with the following dependency flow:

- `crates/lsp/` - LSP server using tower-lsp framework. Handles completion, hover, diagnostics, and multi-connection management. Binary entry point at `src/bin/main.rs`
- `crates/semantic/` - Semantic analysis layer including scope management, symbol tables, and resolution of table/column references
- `crates/lowering/` - CST to IR conversion layer with dialect-specific implementations. Each dialect implements the `Lowering` trait with three outcomes: Success, Partial (with placeholders), or Failed
- `crates/ir/` - Unified intermediate representation types (Query, Expr, Stmt, etc.) that abstract away dialect differences
- `crates/grammar/` - Tree-sitter grammar definitions with compile-time dialect selection via `DIALECT` environment variable
- `crates/catalog/` - Database schema abstraction. `LiveCatalog` connects to real databases, `StaticCatalog` loads from YAML/JSON files
- `crates/function-registry/` - Function metadata and registry for completion
- `crates/test-utils/` - Testing utilities and fixtures

### Dialect Strategy

Dialects use compile-time merging approach:

- Base SQL grammar defined in `grammar.js`
- Dialect extensions in `dialect/{mysql,postgresql}.js`
- `DIALECT` environment variable selects which grammar to generate
- Compatible dialects share parsers (TiDB/MariaDB use MySQL parser, CockroachDB uses PostgreSQL parser)
- Dialect-specific parsers generated as separate object files: `parser-base.c`, `parser-mysql.c`, `parser-postgresql.c`

### Key Architecture Concepts

- IR is a unified syntax tree that is dialect-independent
- Semantic layer adds meaning through scope, symbols, and resolution
- Lowering layer uses three-tier error handling: Success, Partial, or Failed
- Partial success mode allows degraded completion when some parts fail to parse
- Coarse-grained cache invalidation (any edit invalidates entire document cache)

## 2. Common Development Commands

The Makefile provides simplified commands for all development tasks. Run `make help` to see all available commands.

### Building

- `make build` - Build entire workspace
- `make build-release` - Build in release mode
- `make build-grammar` - Build grammar crate
- `make build-lsp` - Build LSP server
- `make grammar` - Build grammar for all dialects
- `make grammar-mysql` - Build MySQL dialect grammar
- `make grammar-postgresql` - Build PostgreSQL dialect grammar

### Testing

- `make test` - Run all tests
- `make test-verbose` - Run tests with output
- `make test-grammar` - Run grammar tests
- `make test-grammar-mysql` - Test MySQL dialect
- `make test-grammar-postgresql` - Test PostgreSQL dialect
- `make test-lsp` - Run LSP server tests
- `make test-specific TEST=test_name` - Run specific test

### Running

- `make run` - Run the LSP server
- `make run-release` - Run the LSP server in release mode

### Code Quality

- `make check` - Run all checks (fmt + clippy)
- `make fmt` - Format code
- `make clippy` - Run clippy linter

### E2E Testing

- `make test-e2e` - Run all E2E tests
- `make test-e2e-completion` - Run completion E2E tests
- `make test-e2e-diagnostics` - Run diagnostics E2E tests
- `make test-e2e-hover` - Run hover E2E tests
- `make test-e2e-verbose` - Run E2E tests with output

### Cleanup

- `make clean` - Clean build artifacts

### Prerequisites

- Rust 2024 edition
- Node.js (for tree-sitter CLI)
- tree-sitter-cli: `npm install -g tree-sitter-cli`

## 3. Coding Style & Naming Conventions

- Rust 2024 edition with workspace-level dependency management
- `thiserror` for error types
- `anyhow` for error propagation
- `async-trait` for trait definitions
- `tokio` for async runtime
- `DashMap` for thread-safe maps
- `ArcSwap` for lock-free updates
- `ropey` for efficient text editing operations
- `tracing` framework with `tracing-subscriber` for logging
- `unified-sql-lsp-test-utils` for shared test fixtures

## 4. E2E Testing

The project includes end-to-end testing framework at `tests/e2e-rs/` that provides:

- Full LSP protocol testing through actual client-server communication
- Live database connections via Docker (MySQL/PostgreSQL)
- Declarative test definitions in YAML format
- Comprehensive assertion helpers for LSP responses

E2E Test Structure

- `tests/e2e-rs/src/lib.rs` - E2E test library with LSP client and test framework
- `tests/e2e-rs/src/client.rs` - LSP client implementation for protocol testing
- `tests/e2e-rs/src/runner.rs` - LSP server spawn and lifecycle management
- `tests/e2e-rs/src/yaml_parser.rs` - YAML test case parser
- `tests/e2e-rs/tests/completion.rs` - Completion E2E tests
- `tests/e2e-rs/tests/diagnostics.rs` - Diagnostics E2E tests
- `tests/e2e-rs/tests/hover.rs` - Hover E2E tests

Requirements

- Docker and Docker Compose for database containers
- MySQL and PostgreSQL Docker images
- Sufficient system resources for running databases

Test Workflow

1. Initialize test database using init_database()
2. Spawn LSP server process using LspRunner
3. Create LSP client connection
4. Load test cases from YAML files
5. Execute test scenarios and assert results
6. Clean up database connections

Writing E2E Tests

Tests are defined in YAML files with declarative syntax. Example:

name: Test case name
description: Detailed description
database: mysql
setup:
  - CREATE TABLE test (id INT)
input: SELECT * FROM t
position: line: 0, character: 14
expect:
  completions:
    - label: test
      kind: Table

## 5. Security Tips

- `LiveCatalog` connects to real databases with connection pooling (max 10 connections, 5s query timeout)
- Use parameterized queries only when interacting with databases
- `SchemaFilter` restricts which tables/schemas are accessible via glob patterns
- Three-tier caching (Tree, IR, Semantic) prevents repeated expensive operations
- Cache invalidation is currently coarse-grained (any edit invalidates entire document cache)
- Check `FEATURE_LIST.yaml` for dialect support status and version-specific features

### Important Implementation Notes

- IR is a unified syntax tree (dialect-independent)
- Semantic layer adds meaning through scope, symbols, and resolution
- Tree-sitter provides built-in incremental parsing
- Phase 1-3 uses coarse-grained cache invalidation (any edit triggers full document re-parse)
- SQL queries are typically short, so full re-parsing is acceptable
- Not all dialects are equally supported
- It's unnecessary to keep backward compatible
