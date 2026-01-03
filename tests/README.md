# Tests Directory

This directory contains all tests for the unified-sql-lsp project.

## Directory Structure

```
tests/
├── fixtures/         # Test data (SQL queries, schemas)
├── integration/      # End-to-end integration tests
├── matrix/          # Dialect/version test matrix
└── unit/            # Per-crate unit tests
    ├── grammar/     # Parser tests
    ├── ir/          # IR representation tests
    ├── lowering/    # CST→IR conversion tests
    ├── semantic/    # Semantic analysis tests
    ├── catalog/     # Catalog metadata tests
    └── lsp/         # LSP server tests
```

## Running Tests

See the main [TESTING.md](../TESTING.md) guide for detailed instructions.

### Quick Start

```bash
# Run all tests
cargo test --workspace

# Run unit tests only
cargo test --workspace --lib

# Run integration tests
cargo test --workspace --test '*'
```

## Fixtures

The `fixtures/` directory contains reusable test data:

- **queries.sql**: Sample SQL queries for various scenarios
- **schemas.sql**: Database schema definitions and sample data

Use these in tests via the `test-utils` crate:

```rust
use unified_sql_lsp_test_utils::SqlFixtures;

let query = SqlFixtures::inner_join();
```

## Unit Tests

Unit tests are organized by crate and test specific functionality:

- **grammar/**: Parser utilities, dialect detection
- **ir/**: Query/expression construction, serialization
- **lowering/**: Dialect-specific CST→IR conversion
- **semantic/**: Scope analysis, symbol resolution
- **catalog/**: Metadata types, relationships
- **lsp/**: Document sync, completion flow

## Integration Tests

Integration tests verify cross-cutting functionality:

- **completion_flow_tests.rs**: End-to-end completion pipeline
- **multi_document_tests.rs**: Document isolation
- **catalog_integration_tests.rs**: Real database integration

## Test Matrix

The `matrix/` directory contains tests that run across multiple dialects and versions:

```rust
test_all_dialects!(test_basic_select, {
    // Test runs for MySQL, PostgreSQL, etc.
});
```

See [TESTING.md](../TESTING.md) for details on writing matrix tests.

## Adding Tests

1. Place unit tests in `tests/unit/<crate>/`
2. Place integration tests in `tests/integration/`
3. Add fixtures to `tests/fixtures/` if reusable
4. Update this README if adding new test categories

## Test Utilities

All tests can use the shared utilities from `crates/test-utils/`:

- `MockCatalog`: In-memory catalog for testing
- `MockCstBuilder`: Build CST trees
- `SqlFixtures`: Predefined SQL queries
- `SqlAssertions`: SQL-specific test helpers

See the [test-utils README](../crates/test-utils/README.md) for details.
