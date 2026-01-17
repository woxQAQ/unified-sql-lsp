# AI Collaboration Guide

## 1. Project Overview

**Purpose:** Multi-dialect SQL Language Server Protocol (LSP) implementation supporting MySQL, PostgreSQL, TiDB, MariaDB, and CockroachDB.

**Core Capabilities:**
- Multi-dialect grammar support via Tree-sitter
- Schema-aware code completion (tables, columns, functions, keywords)
- Real-time syntax and semantic validation
- High-performance incremental parsing with caching

**Tech Stack:** Rust 2024 edition, tower-lsp framework, Tree-sitter parsers, Docker (E2E testing)

## 2. Architecture

### Dependency Flow (Top → Bottom)

```
LSP Layer (~3,000 lines)
  └─ Protocol handlers, type conversions
  └─ Delegates business logic to ↓
Semantic Layer
  ├─ ScopeManager (tracks visible tables/columns)
  ├─ AliasResolver (4 resolution strategies)
  └─ ColumnResolver (resolves column references)
  └─ Depends on ↓
Context Layer
  ├─ NodeExt, ScopeBuilder (CST utilities)
  ├─ CompletionContext (detects SELECT/FROM/WHERE/etc.)
  └─ SQL keyword providers
  └─ Depends on ↓
IR & Grammar Layers
  ├─ IR: Unified intermediate representation
  └─ Grammar: Tree-sitter parsers (dialect-specific)
```

### Crate Map

| Crate | Purpose |
|-------|---------|
| `lsp/` | LSP protocol adapter (tower-lsp), thin layer only |
| `semantic/` | Scope management, symbol resolution, alias resolution |
| `context/` | CST utilities, completion context detection |
| `lowering/` | CST → IR conversion (Success/Partial/Failed) |
| `ir/` | Dialect-independent intermediate representation |
| `grammar/` | Tree-sitter grammars with `DIALECT` env var selection |
| `catalog/` | `LiveCatalog` (real DBs), `StaticCatalog` (YAML/JSON) |
| `function-registry/` | Function metadata, hover information |
| `test-utils/` | Shared test fixtures and utilities |

### Dialect Strategy

**Grammar Files:**
- `grammar.js` - Base SQL grammar
- `dialect/{mysql,postgresql}.js` - Dialect extensions

**Parser Sharing:**
- TiDB/MariaDB → use MySQL parser
- CockroachDB → uses PostgreSQL parser

**Generated Files:**
- `parser-base.c`, `parser-mysql.c`, `parser-postgresql.c`

### Key Architectural Patterns

**Lowering Outcomes (Three-tier):**
1. **Success** - Complete CST → IR conversion
2. **Partial** - Degraded mode (partial parsing failure)
3. **Failed** - Complete conversion failure

**Performance Approach:**
- Coarse-grained cache: any edit invalidates entire document cache
- Full re-parsing is acceptable (SQL queries are typically short)
- Partial success mode enables best-effort completion on errors

## 3. Development Workflow

### Essential Commands

```bash
make build              # Build entire workspace
make test               # Run all tests
make run                # Run LSP server
make check              # Run fmt + clippy checks
make help               # Show all available commands
```

### Prerequisites

- **Rust:** 2024 edition
- **Node.js:** Required for tree-sitter CLI
- **tree-sitter CLI:** `npm install -g tree-sitter-cli`

## 4. Coding Standards & Constraints

### Naming Conventions
- **Variables/Functions:** `snake_case`
- **Structs/Enums:** `PascalCase`

### Comment Philosophy
- **Code:** Explains WHAT (actions)
- **Comments:** Explain WHY and HOW (rationale, trade-offs)

### Architectural Constraints
- **LSP layer thickness:** Must remain ~3,000 lines (thin adapter)
  - Delegate business logic to semantic/context layers
  - Only handle LSP protocol and type conversions
- **Dependency flow:** Follow LSP → Semantic → Context → IR/Grammar
- **Dialect independence:** Use IR layer for dialect-agnostic operations

### Design Patterns
- **Three-tier lowering:** Success, Partial, Failed outcomes
- **Multi-strategy resolution:** AliasResolver with 4 strategies (ExactMatch, StartsWith, FirstLetterPlusNumeric, SingleTableFallback)

## 5. E2E Testing

### CRITICAL RULES (MUST FOLLOW)

**FORBIDDEN:**
- ❌ NEVER run `cargo test` directly in `tests/e2e-rs/`
- ❌ NEVER run `cargo nextest` without specific targets (will try all DB engines)

**REQUIRED:**
- ✅ ALWAYS use Makefile commands for test execution
- ✅ After code changes: Run only relevant test subset
- ✅ When tests fail: Re-run only failed tests with `cargo nextest run --failed`

### Test Execution Commands

**Run by Database Engine:**
```bash
make test-e2e-mysql-5.7       # MySQL 5.7 only
make test-e2e-mysql-8.0       # MySQL 8.0 only
make test-e2e-postgresql-12   # PostgreSQL 12 only
make test-e2e-postgresql-16   # PostgreSQL 16 only
make test-e2e-mysql           # All MySQL versions
make test-e2e-postgresql      # All PostgreSQL versions
make list-e2e                 # List all E2E tests
```

**Full Test Suite:**
```bash
make test-e2e           # Run all E2E tests
make test-e2e-parallel  # Parallel execution (3-4x faster)
```

**Individual Test:**
```bash
cd tests/e2e-rs
cargo nextest run --package mysql-5-7-e2e-tests -- test_completion_basic_select
```

**Re-run Failed Tests:**
```bash
cd tests/e2e-rs
cargo nextest run --failed
```

### Test Categories

| Category | Coverage |
|----------|----------|
| **Completion** | Tables, columns, functions, keywords, JOINs |
| **Hover** | Types, signatures, aliases |
| **Diagnostics** | Syntax, semantic, type mismatch errors |

### Test Fixtures

- **Schema:** `fixtures/schema/` - Database schema definitions
- **Data:** `fixtures/data/` - Test data
- **Requirement:** Changes must work across all supported database versions

## 6. Common Tasks

### Adding New Completion Feature

1. **Implement context detection** in `crates/context/`
2. **Add completion logic** in `crates/semantic/` or `crates/lsp/`
3. **Write E2E tests** in `tests/e2e-rs/`
4. **Run relevant tests:** `make test-e2e-mysql` (or specific engine)

### Debugging Completion Issues

1. Check `CompletionContext` detection (context layer)
2. Verify `ScopeManager` tracks correct tables/columns
3. Ensure `AliasResolver` strategy matches query pattern
4. Run E2E tests for specific database engine

### Adding New Dialect Support

1. Create dialect extension: `crates/grammar/dialect/{dialect}.js`
2. Update `DIALECT` environment variable handling
3. Add dialect-specific lowering in `crates/lowering/`
4. Create E2E test package: `tests/e2e-rs/`
5. Add Makefile targets for new dialect

## 7. Domain-Specific Context

### Key Components

| Component | Purpose |
|-----------|---------|
| `ScopeManager` | Tracks visible tables/columns at each query position |
| `AliasResolver` | 4 strategies: ExactMatch, StartsWith, FirstLetterPlusNumeric, SingleTableFallback |
| `HoverInfoProvider` | Function signatures, column types, table information |
| `CompletionContext` | Detects SELECT/FROM/WHERE/etc. contexts |

### Known Limitations

- **Coarse-grained caching:** May affect performance for very large SQL files
- **Partial success mode:** Provides best-effort completion when parsing fails (degraded UX)
