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

## 2. Build, Test and Development Commands

### Building

- `cargo build --workspace` - Build entire workspace
- `cargo build -p unified-sql-grammar` - Build grammar crate
- `cargo build -p unified-sql-lsp-lsp` - Build LSP server
- `cargo build -p unified-sql-lsp-ir` - Build IR crate
- `cargo build -p unified-sql-lsp-lowering` - Build lowering crate
- `cargo build -p unified-sql-lsp-semantic` - Build semantic crate
- `cargo build -p unified-sql-lsp-catalog` - Build catalog crate
- `cd crates/grammar && ./build.sh` - Build grammar for all dialects (also runs automatically via cargo build)
- `cd crates/grammar/src/grammar && DIALECT=mysql tree-sitter generate` - Build specific dialect grammar

### Running

- `cargo run -p unified-sql-lsp-lsp` - Run the LSP server

### Playground

Web-based testing environment with real database connections (MySQL, PostgreSQL, backend, frontend).

Quick start

- `cargo build --release` - Build LSP server binary
- `cd playground && ./start.sh` - Start all services
- Access at http://localhost:3000
- `cd playground && ./stop.sh` - Stop all services

Local development (without Docker)

- Start databases: `cd playground && docker-compose up -d mysql postgres`
- Start backend: `cd playground/backend && npm install && MYSQL_PORT=3307 PG_PORT=5433 npm start`
- Start frontend: `cd playground/frontend && npm install && npm run dev`

Testing features

- Table completion: `SELECT * FROM` (Ctrl+Space)
- Column completion: `SELECT customer_id,` (Ctrl+Space)
- JOIN completion: `SELECT * FROM orders o JOIN customers c ON o.` (Ctrl+Space)
- Function completion: `SELECT C` (Ctrl+Space)

### Testing

- `cargo test --workspace` - Run all tests
- `cargo test -p unified-sql-grammar` - Run grammar crate tests
- `cargo test -p unified-sql-lsp-lsp` - Run LSP server tests
- `cargo test -p unified-sql-lsp-lsp test_completion_flow` - Run specific test
- `cargo test -- --nocapture` - Run tests with output
- `cd crates/grammar && npm test` - Run tree-sitter corpus tests
- `cd crates/grammar && npm run test:mysql` - Test MySQL dialect
- `cd crates/grammar && npm run test:postgresql` - Test PostgreSQL dialect

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

## 4. Testing Guide

### Test Organization

- Unit tests in each crate's `src/` directory
- Integration tests in `crates/*/tests/` directories
- Tree-sitter corpus tests in `crates/grammar/test/corpus/`

### Running Tests

- `cargo test --workspace` - Run all workspace tests
- `cargo test -p unified-sql-lsp-lsp` - Run specific crate tests
- `cargo test -p unified-sql-lsp-lsp test_completion_flow` - Run specific test
- `cargo test -- --nocapture` - Run tests with output

### Test Utilities

The `test-utils` crate provides shared fixtures and helpers for testing LSP functionality, catalog integration, and lowering behavior.

## 5. Security Tips

- `LiveCatalog` connects to real databases with connection pooling (max 10 connections, 5s query timeout)
- Use parameterized queries only when interacting with databases
- `SchemaFilter` restricts which tables/schemas are accessible via glob patterns
- Three-tier caching (Tree, IR, Semantic) prevents repeated expensive operations
- Cache invalidation is currently coarse-grained (any edit invalidates entire document cache)
- Check `FEATURE_LIST.yaml` for dialect support status and version-specific features

### Performance Targets

- 10k line parsing: < 100ms
- Completion latency: < 50ms (p95)
- Memory usage: < 50MB
- Cache hit rate: > 80%

### Important Implementation Notes

- IR is a unified syntax tree (dialect-independent)
- Semantic layer adds meaning through scope, symbols, and resolution
- Tree-sitter provides built-in incremental parsing
- Phase 1-3 uses coarse-grained cache invalidation (any edit triggers full document re-parse)
- SQL queries are typically short, so full re-parsing is acceptable
- Not all dialects are equally supported
- It's unnecessary to keep backward compatible
