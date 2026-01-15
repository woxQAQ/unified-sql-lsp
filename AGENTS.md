# Repository Guidelines

## Project Overview

Unified SQL LSP is a Language Server Protocol implementation supporting multiple SQL dialects including MySQL, PostgreSQL, TiDB, MariaDB, and CockroachDB. Key features include multi-dialect grammar support using Tree-sitter, schema-aware code completion, real-time syntax and semantic validation, and high-performance incremental parsing with multi-level caching.

## 1. Project Structure & Module Organization

The codebase follows a layered architecture with clear separation of concerns. The dependency flow is:

```
┌─────────────────────────────────────────────┐
│  LSP Layer (crates/lsp/)                    │
│  - Protocol handlers only                   │
│  - LSP type conversions                     │
│  - Thin adapter layer (~3,000 lines)        │
└──────────────┬──────────────────────────────┘
               │ depends on
┌──────────────▼──────────────────────────────┐
│  Semantic Layer (crates/semantic/)          │
│  - AliasResolver for table alias resolution  │
│  - ScopeManager for tracking tables/columns │
│  - ColumnResolver for column references     │
└──────────────┬──────────────────────────────┘
               │ depends on
┌──────────────▼──────────────────────────────┐
│  Context Layer (crates/context/)            │
│  - CST utilities (NodeExt, ScopeBuilder)    │
│  - CompletionContext detection              │
│  - SQL keyword providers                    │
└──────────────┬──────────────────────────────┘
               │ depends on
┌──────────────▼──────────────────────────────┐
│  IR & Grammar Layers                        │
│  - crates/ir/ - Unified IR types            │
│  - crates/grammar/ - Tree-sitter parsers    │
└─────────────────────────────────────────────┘
```

**Crates Overview:**

- `crates/lsp/` - LSP server using tower-lsp framework. Handles LSP protocol, type conversions, and delegates business logic to semantic/context layers
- `crates/semantic/` - Semantic analysis including scope management, symbol resolution, and alias resolution
- `crates/context/` - CST utilities, context detection, and scope building for completion
- `crates/lowering/` - CST to IR conversion with dialect-specific implementations (Success, Partial, or Failed outcomes)
- `crates/ir/` - Unified intermediate representation types that abstract away dialect differences
- `crates/grammar/` - Tree-sitter grammar definitions with compile-time dialect selection
- `crates/catalog/` - Database schema abstraction. `LiveCatalog` for real databases, `StaticCatalog` for YAML/JSON files
- `crates/function-registry/` - Function metadata and hover information provider
- `crates/test-utils/` - Testing utilities and fixtures

### Dialect Strategy

Dialects use compile-time merging approach:

- Base SQL grammar defined in `grammar.js`
- Dialect extensions in `dialect/{mysql,postgresql}.js`
- `DIALECT` environment variable selects which grammar to generate
- Compatible dialects share parsers (TiDB/MariaDB use MySQL parser, CockroachDB uses PostgreSQL parser)
- Dialect-specific parsers generated as separate object files: `parser-base.c`, `parser-mysql.c`, `parser-postgresql.c`

### Key Architecture Concepts

**Layered Architecture:**
- LSP layer is a thin protocol adapter (~3,000 lines, down from ~4,000)
- Business logic moved to semantic and context layers for reusability
- Clear dependency flow: LSP → Semantic → Context → IR/Grammar

**Core Abstractions:**
- **IR (Intermediate Representation)**: Dialect-independent syntax tree
- **Semantic Layer**: Adds meaning through scope, symbols, and resolution
- **Context Layer**: Provides CST utilities and completion context detection
- **Lowering**: Three-tier error handling (Success, Partial, or Failed)

**Key Components:**
- `ScopeManager`: Tracks tables and columns visible at each position in SQL queries
- `AliasResolver`: Multi-strategy table alias resolution (ExactMatch, StartsWith, FirstLetterPlusNumeric, SingleTableFallback)
- `HoverInfoProvider`: Provides hover information for functions, columns, and tables
- `CompletionContext`: Detects where completion is requested (SELECT, FROM, WHERE, etc.)

**Performance:**
- Partial success mode allows degraded completion when parsing fails
- Coarse-grained cache invalidation (any edit invalidates entire document cache)
- SQL queries are typically short, so full re-parsing is acceptable

## 2. Common Development Commands

The Makefile provides simplified commands for all development tasks. Run `make help` to see all available commands.

### Key Commands

- `make build` - Build entire workspace
- `make test` - Run all tests
- `make run` - Run the LSP server
- `make check` - Run all checks (fmt + clippy)
- `make test-e2e` - Run all E2E tests

### Prerequisites

- Rust 2024 edition
- Node.js (for tree-sitter CLI)
- tree-sitter-cli: `npm install -g tree-sitter-cli`

## 3. Coding Style & Naming Conventions

- clean comment: codes should explain actions, and comments should explain why and how.
- Naming conventions: use snake_case for variables and functions, PascalCase for structs and enums.

## 4. E2E Testing

The project includes end-to-end testing framework at `tests/e2e-rs/` that provides:

- Full LSP protocol testing through actual client-server communication
- Live database connections via Docker (MySQL/PostgreSQL)
- Declarative test definitions in YAML format
- Comprehensive assertion helpers for LSP responses

### Test Workflow

the e2e test support one-click running the tests. the makefile command `make test-e2e` supports a quick entry and NOT need to run `cargo test` manually.

## 6. Important tips

- **NOT ALLOWED** to run cargo test directly
- **NOT ALLOWED** to build the project, run `cargo check`
- Keep the e2e tests up-to-date and maintainable, ensuring they are passing before you stop work.
