# Testing Guide

This guide covers how to run, write, and extend tests in the unified-sql-lsp project.

## Table of Contents

- [Running Tests](#running-tests)
- [Test Organization](#test-organization)
- [Writing Tests](#writing-tests)
- [Test Utilities](#test-utilities)
- [CI/CD](#cicd)
- [Coverage](#coverage)

## Running Tests

### Run All Tests

```bash
# Run all tests
cargo test --workspace

# Run tests with output
cargo test --workspace -- --nocapture

# Run tests in parallel
cargo test --workspace --test-threads=4
```

### Run Specific Crate Tests

```bash
# Test only the IR crate
cargo test -p unified-sql-lsp-ir

# Test only the catalog crate
cargo test -p unified-sql-lsp-catalog

# Test only the lowering crate
cargo test -p unified-sql-lsp-lowering

# Test only the LSP crate
cargo test -p unified-sql-lsp-lsp

# Test only the grammar crate
cargo test -p unified-sql-grammar
```

### Run Grammar Tests

```bash
# Test grammar API
cargo test -p unified-sql-grammar

# Test specific dialect using tree-sitter CLI
cd crates/grammar/src/grammar
DIALECT=mysql tree-sitter test
DIALECT=postgresql tree-sitter test
DIALECT=base tree-sitter test
```

### Run LSP Integration Tests

```bash
# Run all LSP integration tests
cargo test -p unified-sql-lsp-lsp --test '*'

# Run specific test suites
cargo test -p unified-sql-lsp-lsp --test completion_tests
cargo test -p unified-sql-lsp-lsp --test parser_manager_tests
cargo test -p unified-sql-lsp-lsp --test document_sync_tests
cargo test -p unified-sql-lsp-lsp --test e2e_completion_tests
cargo test -p unified-sql-lsp-lsp --test dialect_matrix_tests
cargo test -p unified-sql-lsp-lsp --test error_handling_tests
cargo test -p unified-sql-lsp-lsp --test integration_tests

# Run integration tests only (TEST-002)
cargo test -p unified-sql-lsp-lsp --test integration_tests
```

#### Integration Tests (TEST-002)

The `integration_tests.rs` file provides comprehensive integration tests covering:

- **Completion Flow Tests** (10 tests): Full pipeline from parsing to rendering
  - SELECT projection with/without qualifiers
  - FROM clause table completion
  - WHERE clause column completion
  - JOIN condition completion
  - Table aliases
  - Error handling (parse errors, empty documents)
  - Dialect-specific syntax (MySQL LIMIT, PostgreSQL DISTINCT ON)

- **Multi-Document Concurrent Tests** (6 tests): Thread-safety and concurrent operations
  - Concurrent document opening/closing
  - Rapid open/close cycles (50 iterations)
  - Parsing performance under load (20 concurrent parses)
  - Completion latency measurements (p95 < 200ms)
  - Error isolation in concurrent scenarios
  - Concurrent read operations

- **Catalog Integration Tests** (8 tests): Catalog integration and error handling
  - Standard schema integration
  - Custom table definitions
  - Multiple schemas with schema qualifiers
  - Function completion from catalog
  - Error handling (non-existent tables)
  - Metadata in completions (comments, row counts, column types)
  - Cross-dialect compatibility (MySQL vs PostgreSQL)

## Test Organization

The project follows this test structure:

```
tests/
├── unit/              # Unit tests per crate
│   ├── ir/            # IR type tests
│   ├── lowering/      # Lowering conversion tests
│   ├── semantic/      # Semantic analysis tests
│   └── grammar/       # Parser utility tests

crates/grammar/tests/
├── api_tests.rs       # Grammar API integration tests
                      # Tests language_for_dialect(), parsing with real trees
                      # Tests dialect-specific syntax (MySQL LIMIT, PostgreSQL DISTINCT ON)

crates/lsp/tests/
├── completion_tests.rs        # Completion engine integration tests
├── parser_manager_tests.rs    # ParserManager functionality tests
├── document_sync_tests.rs     # DocumentSync orchestration tests
├── e2e_completion_tests.rs    # End-to-end pipeline tests (parse → complete)
├── dialect_matrix_tests.rs    # Multi-dialect macro-based tests
├── error_handling_tests.rs    # Error scenarios and edge cases
└── integration_tests.rs       # Comprehensive integration tests (TEST-002)
                              # 24 tests covering completion flow,
                              # multi-document concurrency, and catalog integration
```

## Coverage Goals

The project aims for comprehensive test coverage:

- **Grammar crate**: >80% coverage
  - Grammar API integration tests
  - Dialect-specific syntax tests
  - Error detection tests

- **LSP crate**: >80% coverage
  - ParserManager tests
  - DocumentSync tests
  - Completion engine tests
  - End-to-end integration tests
  - Multi-dialect matrix tests
  - Error handling tests

To generate coverage reports:
```bash
cargo tarpaulin --workspace --lib --out Html
```

## Test Types

#### Unit Tests
Unit tests focus on single crates and test specific functionality in isolation:

- **IR tests**: Verify query and expression construction, serialization
- **Lowering tests**: Test CST→IR conversion for each dialect
- **Semantic tests**: Scope analysis, symbol resolution
- **Catalog tests**: Metadata types, serialization
- **LSP tests**: Document sync, completion flow

#### Integration Tests
Integration tests verify end-to-end functionality across multiple crates:

- Completion flow: parse → lower → analyze → complete
- Multi-document handling
- Catalog integration with real databases

### Grammar Tests

Tree-sitter corpus tests verify the grammar correctly parses SQL:

- Located in `crates/grammar/test/corpus/`
- One file per dialect (base.txt, mysql.txt, postgresql.txt)
- Format: SQL query followed by expected AST

## Writing Tests

### Basic Unit Test

```rust
#[test]
fn test_query_creation() {
    use unified_sql_lsp_ir::{Query, Dialect};

    let query = Query::new(Dialect::MySQL);
    assert_eq!(query.dialect, Dialect::MySQL);
}
```

### Using Mock Catalog

The `test-utils` crate provides mock implementations for testing:

```rust
use unified_sql_lsp_test_utils::{MockCatalog, MockCatalogBuilder};

#[tokio::test]
async fn test_with_catalog() {
    let catalog = MockCatalogBuilder::new()
        .with_standard_schema()
        .build();

    let tables = catalog.list_tables().await.unwrap();
    assert!(!tables.is_empty());
}
```

### Using Mock CST Nodes

For testing the lowering layer without tree-sitter:

```rust
use unified_sql_lsp_test_utils::{SqlCstHelpers, MockCstBuilder};

#[test]
fn test_lowering_select() {
    let cst = SqlCstHelpers::simple_select(
        vec!["id", "name"],
        "users"
    );

    assert_eq!(cst.kind(), "select_statement");
    assert_eq!(cst.children("from").len(), 1);
}
```

### Testing with Fixtures

Use predefined SQL queries for testing:

```rust
use unified_sql_lsp_test_utils::SqlFixtures;

#[test]
fn test_parse_query() {
    let query = SqlFixtures::inner_join();
    assert!(query.contains("JOIN"));
}
```

### Testing Function Completion

Function completion is tested at multiple levels:

#### Unit Tests

`crates/lsp/src/completion/render.rs`:
```rust
#[test]
fn test_render_functions_all() {
    use unified_sql_lsp_catalog::{FunctionMetadata, FunctionType};

    let functions = vec![
        FunctionMetadata::new("count", DataType::BigInt)
            .with_type(FunctionType::Aggregate)
            .with_description("Count rows"),
        FunctionMetadata::new("abs", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Absolute value"),
    ];

    let items = CompletionRenderer::render_functions(&functions, None);

    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.label == "count"));
    assert!(items.iter().any(|i| i.label == "abs"));
}
```

#### Integration Tests

`crates/lsp/tests/integration_tests.rs`:
```rust
#[tokio::test]
async fn test_catalog_integration_function_completion() {
    use unified_sql_lsp_catalog::{FunctionMetadata, FunctionType};

    let catalog = MockCatalogBuilder::new()
        .with_function(
            FunctionMetadata::new("count", DataType::BigInt)
                .with_type(FunctionType::Aggregate)
                .with_description("Count rows in a table")
                .with_example("SELECT COUNT(*) FROM users"),
        )
        .with_function(
            FunctionMetadata::new("upper", DataType::Varchar(None))
                .with_type(FunctionType::Scalar)
                .with_description("Convert string to uppercase")
                .with_parameters(vec![/* ... */]),
        )
        .with_table(/* ... */)
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM users";
    let document = create_test_document(sql, "mysql").await;
    let result = engine.complete(&document, Position::new(0, 20)).await;

    assert!(result.is_ok());
}
```

#### Mock Function Setup

```rust
use unified_sql_lsp_catalog::{FunctionMetadata, FunctionType, FunctionParameter};

let catalog = MockCatalogBuilder::new()
    .with_function(
        FunctionMetadata::new("concat", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Concatenate strings")
            .with_parameters(vec![
                FunctionParameter {
                    name: "str1".to_string(),
                    data_type: DataType::Text,
                    has_default: false,
                    is_variadic: false,
                },
                FunctionParameter {
                    name: "str2".to_string(),
                    data_type: DataType::Text,
                    has_default: false,
                    is_variadic: false,
                },
            ])
            .with_example("SELECT CONCAT(first, ' ', last) FROM users"),
    )
    .build();
```

## Test Utilities

### Mock Catalog

The `MockCatalog` provides an in-memory catalog:

```rust
use unified_sql_lsp_test_utils::MockCatalogBuilder;

let catalog = MockCatalogBuilder::new()
    .with_table(/* custom table */)
    .with_function(/* custom function */)
    .build();
```

### Mock CST Builder

Build CST trees for lowering tests:

```rust
use unified_sql_lsp_test_utils::MockCstBuilder;

let cst = MockCstBuilder::new("select_statement")
    .with_field("projection", select_list)
    .with_field("from", from_clause)
    .build();
```

### Custom Assertions

Use SQL-specific assertions:

```rust
use unified_sql_lsp_test_utils::SqlAssertions;

SqlAssertions::assert_column_ref(&expr, "user_id");
SqlAssertions::assert_literal_int(&expr, 42);
```

## CI/CD

The project uses GitHub Actions for continuous testing:

- **Unit Tests**: Run on all push/PR to main/develop
- **Grammar Tests**: Test all dialects with tree-sitter
- **Integration Tests**: With MySQL 8.0 and PostgreSQL 14 containers
- **Coverage**: Generate coverage reports with tarpaulin
- **Benchmarks**: Run on main branch only

See `.github/workflows/test.yml` for the complete workflow.

## Coverage

### Generate Coverage Locally

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --workspace --lib --out Html

# View coverage
open tarpaulin-report.html
```

### Coverage Goals

- Target: >80% line coverage for all crates
- Critical paths (lowering, semantic): >90% coverage
- Grammar/IR: >70% coverage (external deps)

## Debugging Failed Tests

### Run Single Test

```bash
# Run a specific test
cargo test test_query_creation

# Run tests in a file
cargo test --test query_tests

# Run tests with output
cargo test test_query_creation -- --nocapture
```

### Debug with println!

```rust
#[test]
fn test_with_debug() {
    println!("Debug info: {:?}", some_value);
    assert!(true);
}
```

Run with `--nocapture` to see output:
```bash
cargo test test_with_debug -- --nocapture
```

### Test Logs

Tests are logged to `target/test-results/` after running with:
```bash
cargo test --workspace -Z unstable-options --format json
```

## Best Practices

1. **Test names should be descriptive**: `test_column_ref_with_table_qualified`
2. **Use mock implementations**: Avoid external dependencies (databases, files)
3. **Test edge cases**: Empty inputs, null values, error conditions
4. **Keep tests fast**: Unit tests should complete in <5 seconds
5. **Use fixtures**: Reuse common test data via `SqlFixtures`
6. **Organize by functionality**: Group related tests together

## Adding New Tests

When adding new functionality:

1. Add unit tests in `tests/unit/<crate>/`
2. Add integration tests in `tests/integration/` if cross-crate
3. Update fixtures in `tests/fixtures/` if new SQL syntax
4. Update this document if new test patterns emerge
5. Ensure CI passes before merging

## Resources

- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
- [tree-sitter Testing](https://tree-sitter.github.io/tree-sitter/creating-parsers#command-test)
- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
