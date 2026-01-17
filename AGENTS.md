# Repository Guidelines for AI Collaboration

## Project Overview

Unified SQL LSP is a multi-dialect SQL Language Server Protocol implementation supporting MySQL, PostgreSQL, TiDB, MariaDB, and CockroachDB.

**Core Features:**
- Multi-dialect grammar support via Tree-sitter
- Schema-aware code completion
- Real-time syntax and semantic validation
- High-performance incremental parsing with multi-level caching

**Tech Stack:** Rust 2024, tower-lsp, Tree-sitter, Docker (E2E testing)

## Architecture

### Layered Dependency Flow

```
LSP Layer (~3,000 lines)
    ↓ delegates to
Semantic Layer (scope, symbols, resolution)
    ↓ depends on
Context Layer (CST utilities, completion detection)
    ↓ depends on
IR & Grammar Layers (unified IR, Tree-sitter parsers)
```

### Crate Responsibilities

- **`crates/lsp/`** - Thin LSP protocol adapter using tower-lsp. Handles protocol and type conversions only.
- **`crates/semantic/`** - Semantic analysis: `ScopeManager` (tracks tables/columns), `AliasResolver` (multi-strategy alias resolution), `ColumnResolver`
- **`crates/context/`** - CST utilities: `NodeExt`, `ScopeBuilder`, `CompletionContext` detection, SQL keyword providers
- **`crates/lowering/`** - CST to IR conversion with three-tier outcomes: Success, Partial, Failed
- **`crates/ir/`** - Dialect-independent intermediate representation
- **`crates/grammar/`** - Tree-sitter grammar definitions with compile-time dialect selection via `DIALECT` env var
- **`crates/catalog/`** - Database schema abstraction: `LiveCatalog` (real DBs), `StaticCatalog` (YAML/JSON)
- **`crates/function-registry/`** - Function metadata and hover information
- **`crates/test-utils/`** - Testing utilities and fixtures

### Dialect Strategy

- Base SQL grammar in `grammar.js`
- Dialect extensions in `dialect/{mysql,postgresql}.js`
- Compatible dialects share parsers: TiDB/MariaDB → MySQL, CockroachDB → PostgreSQL
- Separate parser object files: `parser-base.c`, `parser-mysql.c`, `parser-postgresql.c`

### Key Design Patterns

**Three-tier Lowering:**
- Success - Complete conversion
- Partial - Degraded mode when parsing partially fails
- Failed - Complete conversion failure

**Performance Strategy:**
- Coarse-grained cache invalidation (any edit invalidates entire document)
- SQL queries are short, so full re-parsing is acceptable
- Partial success mode allows degraded completion

## Development Workflow

### Essential Commands

```bash
make build              # Build workspace
make test               # Run all tests
make run                # Run LSP server
make check              # Run fmt + clippy
make test-e2e           # Run E2E tests
make test-e2e-parallel  # Parallel E2E tests (3-4x faster)
```

### Prerequisites

- Rust 2024 edition
- Node.js + tree-sitter CLI: `npm install -g tree-sitter-cli`

## Coding Standards

**Naming Conventions:**
- Variables/functions: `snake_case`
- Structs/enums: `PascalCase`

**Comment Philosophy:**
- Code explains WHAT (actions)
- Comments explain WHY and HOW

**Architectural Constraints:**
- LSP layer must remain thin (~3,000 lines) - delegate business logic to semantic/context layers
- Follow dependency flow: LSP → Semantic → Context → IR/Grammar
- Use IR layer for dialect-independent operations

## E2E Testing

**CRITICAL RULES:**
- **NEVER** run `cargo test` directly in `tests/e2e-rs`
- **NEVER** run `cargo nextest` without specific targets (runs all database engines)
- **ALWAYS** use Makefile commands for test execution
- **After code changes**, run only relevant test subset
- **When tests fail**, re-run only failed tests with `cargo nextest run --failed`

### Test Execution

**By Database Engine:**
```bash
make test-e2e-mysql-5.7       # MySQL 5.7 only
make test-e2e-mysql-8.0       # MySQL 8.0 only
make test-e2e-postgresql-12   # PostgreSQL 12 only
make test-e2e-postgresql-16   # PostgreSQL 16 only
make test-e2e-mysql           # All MySQL versions
make test-e2e-postgresql      # All PostgreSQL versions
make list-e2e                 # List all E2E tests
```

**Individual Test:**
```bash
cd tests/e2e-rs
cargo nextest run --package mysql-5-7-e2e-tests -- test_completion_basic_select
```

### Test Categories

- **Completion** - Tables, columns, functions, keywords, JOINs
- **Hover** - Types, signatures, aliases
- **Diagnostics** - Syntax, semantic, type mismatch errors

### Test Fixtures

- Schema fixtures: `fixtures/schema/`
- Data fixtures: `fixtures/data/`
- Ensure changes work across all supported database versions

## Common Tasks

### Adding New Completion Feature

1. Implement context detection in `crates/context/`
2. Add completion logic in `crates/semantic/` or `crates/lsp/`
3. Write E2E tests in `tests/e2e-rs/`
4. Run relevant database engine tests: `make test-e2e-mysql`

### Debugging Completion Issues

1. Check `CompletionContext` detection in context layer
2. Verify `ScopeManager` tracks correct tables/columns
3. Ensure `AliasResolver` strategy matches query pattern
4. Run E2E tests for specific database engine

### Adding New Dialect Support

1. Create dialect extension in `crates/grammar/dialect/{dialect}.js`
2. Update `DIALECT` environment variable handling
3. Add dialect-specific lowering in `crates/lowering/`
4. Create E2E test package in `tests/e2e-rs/`
5. Add Makefile targets for new dialect

## Domain-Specific Context

**Key Components:**
- `ScopeManager` - Tracks visible tables/columns at each position
- `AliasResolver` - Four strategies: ExactMatch, StartsWith, FirstLetterPlusNumeric, SingleTableFallback
- `HoverInfoProvider` - Function signatures, column types, table information
- `CompletionContext` - Detects SELECT/FROM/WHERE/etc. contexts

**Known Limitations:**
- Coarse-grained caching may affect performance for very large SQL files
- Partial success mode provides best-effort completion when parsing fails
