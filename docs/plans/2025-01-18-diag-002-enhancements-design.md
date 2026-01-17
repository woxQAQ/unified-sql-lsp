# DIAG-002 Enhancement Design

**Date**: 2025-01-18
**Status**: Approved
**Author**: Claude (Brainstorming Session)
**Feature**: DIAG-002 - Syntax Error Diagnostics

## Overview

This document describes enhancements to DIAG-002 (Syntax Error Diagnostics) to improve error detection, messaging, and testing. The current implementation correctly detects Tree-sitter ERROR nodes but needs cleanup, better error messages, and comprehensive testing.

## Current State Analysis

### What's Working ✅

- Recursive ERROR node detection with depth limit (100)
- False positive filtering (empty, single-character errors)
- LSP integration via `publish_diagnostics_for_document()`
- Graceful degradation for locked/missing trees
- 10+ unit tests covering types and conversions

### Identified Issues ❌

1. **Debug statements in production code** - 7 `!!! DIAG` prefix statements
2. **Limited error context** - Messages show error text but not expectations
3. **No suggestions** - No "quick fix" hints for common errors
4. **E2E tests ignored** - Tests marked as not implemented despite working
5. **Integration tests incomplete** - No tests with actual syntax errors

## Proposed Enhancements

### 1. Cleanup: Remove Debug Statements

**Problem**: Debug statements with `!!! DIAG` prefix clutter logs and impact performance.

**Locations**: Lines 267, 280, 324-327, 335, 372-374, 380, 386 in `diagnostic.rs`

**Solution**:
- Remove all `!!! DIAG` debug statements
- Keep meaningful `debug!` and `info!` logs using proper tracing
- Follow existing logging patterns in codebase

**Impact**: Cleaner logs, reduced overhead, more professional codebase

---

### 2. Enhanced Error Messages

**Problem**: Current messages like `"Syntax error near: 'username FROM'"` don't explain what's wrong or how to fix it.

**Solution**: Three-level enhancement system

#### Level 1: Error Context
```rust
format!("Syntax error near '{}'", error_text)
```

#### Level 2: Expected Tokens
Analyze parent node to suggest what was expected:
```rust
format!(
    "Syntax error near '{}'. Expected: comma, keyword, or operator",
    error_text
)
```

#### Level 3: Specific Suggestions
Pattern matching for common errors:

| Pattern | Current Message | Enhanced Message |
|---------|----------------|------------------|
| Missing comma | `Syntax error near: 'username FROM'` | `Syntax error: missing comma between column names. Suggestion: Add comma after 'id'` |
| Missing FROM | `Syntax error near: 'WHERE id = 1'` | `Syntax error: SELECT statement missing FROM clause. Expected: 'SELECT ... FROM table WHERE ...'` |
| Unbalanced parens | `Syntax error near: '('` | `Syntax error: unbalanced parentheses. Check opening/closing pairs` |

#### Implementation

```rust
fn enhance_error_message(&self, error_node: &Node, source: &str, error_text: &str) -> String {
    // Try pattern-based suggestions first
    if let Some(suggestion) = self.analyze_common_patterns(error_node, source, error_text) {
        return suggestion;
    }

    // Fall back to context-aware message
    if let Some(context) = self.get_parent_context(error_node) {
        format!("Syntax error near '{}'. {}", error_text, context)
    } else {
        format!("Syntax error near '{}'", error_text)
    }
}

fn analyze_common_patterns(&self, node: &Node, source: &str, error_text: &str) -> Option<String> {
    // Pattern 1: Missing comma
    if is_missing_comma_pattern(error_text) {
        return Some(format!(
            "Syntax error: missing comma between identifiers. Consider adding comma after '{}'",
            first_identifier(error_text)
        ));
    }

    // Pattern 2: Missing FROM clause
    if is_missing_from_pattern(node, source) {
        return Some(
            "Syntax error: SELECT statement missing FROM clause. Expected: 'SELECT ... FROM table ...'".to_string()
        );
    }

    // Pattern 3: Missing parentheses
    if is_unmatched_paren(error_text) {
        return Some("Syntax error: unbalanced parentheses. Check opening/closing pairs".to_string());
    }

    None
}
```

#### Helper Functions

```rust
fn is_missing_comma_pattern(text: &str) -> bool {
    // Matches: "id username", "col1 col2 FROM", etc.
    let trimmed = text.trim();
    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    parts.len() >= 2 &&
        parts.iter().all(|p| is_identifier(p)) &&
        !trimmed.contains(',')
}

fn is_missing_from_pattern(node: &Node, source: &str) -> bool {
    // Check if we're in SELECT without FROM
    let parent = node.parent()?;
    if parent.kind() == "select_statement" {
        !has_child_of_kind(&parent, "from_clause")
    } else {
        false
    }
}
```

**Testing Strategy**:
- Unit tests for each pattern detection function
- Integration tests with known syntax errors
- Verify messages are actionable and clear

---

### 3. Enable E2E Tests

**Problem**: E2E diagnostic tests marked as `#[ignore]` despite DIAG-002 being implemented.

**Analysis**: `basic_diagnostics.yaml` contains:
- ✅ 4 syntax error tests (DIAG-002 - implemented)
- ❌ 9 semantic error tests (DIAG-003/004/005 - not implemented)
- ✅ 3 valid SQL tests (DIAG-002 - implemented)

**Solution**: Split into two test files

#### File 1: syntax_errors.yaml (Enable Now)

```yaml
name: "Syntax error detection tests"
description: "SQL syntax error detection (DIAG-002)"

database:
  dialect: "mysql"
  connection_string: "mysql://test_user:test_password@127.0.0.1:3307/test_db"
  schemas: ["../../../fixtures/schema/mysql/01_create_tables.sql"]
  data: ["../../../fixtures/data/mysql/02_insert_basic_data.sql"]

tests:
  - name: "missing FROM clause"
    description: "Should detect missing FROM clause"
    sql: "SELECT * WHERE id = 1|"
    expect_diagnostics:
      error_count: 1
      error_codes: ["SYNTAX-001"]

  - name: "unterminated string"
    description: "Should detect unterminated string literal"
    sql: "SELECT * FROM users WHERE username = '|"
    expect_diagnostics:
      error_count: 1
      error_codes: ["SYNTAX-001"]

  - name: "missing comma between columns"
    description: "Should detect missing comma"
    sql: "SELECT id username FROM users|"
    expect_diagnostics:
      error_count: 1
      error_codes: ["SYNTAX-001"]

  - name: "unbalanced parentheses"
    description: "Should detect unbalanced parentheses"
    sql: "SELECT * FROM users WHERE (id = 1|"
    expect_diagnostics:
      error_count: 1
      error_codes: ["SYNTAX-001"]

  - name: "invalid keyword placement"
    description: "Should detect misplaced keyword"
    sql: "SELECT FROM * users|"
    expect_diagnostics:
      error_count: 1
      error_codes: ["SYNTAX-001"]

  - name: "valid query no errors"
    description: "Valid query should have no diagnostics"
    sql: "SELECT id, username FROM users WHERE is_active = TRUE|"
    expect_diagnostics:
      error_count: 0
```

#### File 2: semantic_errors.yaml (Keep Ignored)

Move semantic error tests (unknown table, unknown column, ambiguous column, etc.) here. Keep `#[ignore]` attribute until DIAG-003/004/005 are implemented.

#### Update Test File

```rust
// tests/e2e-rs/tests/diagnostics.rs

//! E2E diagnostics tests
//!
//! Tests SQL error detection through actual LSP protocol with live database.

use unified_sql_lsp_e2e::{init_database, run_suite};

// Syntax error tests (DIAG-002) - ENABLED
#[tokio::test]
async fn test_syntax_errors() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/mysql-5.7/diagnostics/syntax_errors.yaml").await
}

// Semantic error tests (DIAG-003 through DIAG-005) - NOT YET IMPLEMENTED
#[tokio::test]
#[ignore = "DIAG-003, DIAG-004, DIAG-005 not implemented yet"]
async fn test_semantic_errors() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/mysql-5.7/diagnostics/semantic_errors.yaml").await
}
```

#### Update E2E Framework

Add error code validation:
```rust
if let Some(expected_codes) = test_case.expect_diagnostics.error_codes {
    for diagnostic in &actual_diagnostics {
        if let Some(code) = &diagnostic.code {
            let code_str = match code {
                NumberOrString::String(s) => s.clone(),
                NumberOrString::Number(n) => n.to_string(),
            };
            assert!(
                expected_codes.contains(&code_str),
                "Expected error code {:?}, got {}",
                expected_codes, code_str
            );
        }
    }
}
```

**Expected Outcomes**:
- ✅ 6 syntax error tests pass
- ✅ 1 valid SQL test passes
- ✅ E2E validation of DIAG-002
- ✅ Semantic tests remain ignored

---

### 4. Add Integration Tests

**Problem**: Current integration tests only use valid SQL, so syntax error detection is never actually tested.

**Current Coverage**:
- Infrastructure tests (tree locking, type conversions)
- Valid SQL tests (no errors expected)
- **Missing**: Tests with actual syntax errors

**Solution**: Add 8 new integration tests

```rust
/// Test syntax error: missing FROM clause
#[tokio::test]
async fn test_syntax_error_missing_from() {
    let lang = match unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL) {
        Some(l) => l,
        None => return,
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(lang).is_err() {
        return;
    }

    let sql = "SELECT * WHERE id = 1";
    let tree = parser.parse(sql, None).expect("Parse should succeed");

    let collector = DiagnosticCollector::new();
    let uri = create_test_uri("/test_missing_from.sql");
    let diagnostics = collector.collect_diagnostics(&tree, sql, &uri);

    // Should detect at least one syntax error
    assert!(!diagnostics.is_empty(), "Should detect syntax error for missing FROM");

    // Verify it's a SYNTAX error
    let syntax_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.code == Some(DiagnosticCode::SyntaxError))
        .collect();
    assert!(!syntax_errors.is_empty(), "Should have SYNTAX-001 error code");

    // Verify error severity
    assert_eq!(syntax_errors[0].severity, DiagnosticSeverity::ERROR);
}

/// Test syntax error: missing comma between columns
#[tokio::test]
async fn test_syntax_error_missing_comma() {
    // Similar structure, test "SELECT id username FROM users"
    // ...
}

/// Test syntax error: unbalanced parentheses
#[tokio::test]
async fn test_syntax_error_unbalanced_parens() {
    // Test "SELECT * FROM users WHERE (id = 1"
    // ...
}

/// Test syntax error: unterminated string literal
#[tokio::test]
async fn test_syntax_error_unterminated_string() {
    // Test "SELECT * FROM users WHERE username = 'john"
    // ...
}

/// Test valid SQL produces no diagnostics
#[tokio::test]
async fn test_valid_sql_no_diagnostics() {
    // Test "SELECT id, username FROM users WHERE is_active = TRUE"
    // ...
}

/// Test multiple syntax errors in one query
#[tokio::test]
async fn test_multiple_syntax_errors() {
    // Test "SELECT id username FROM WHERE (id = 1"
    // Should detect 2-3 errors
    // ...
}

/// Test error range is correct
#[tokio::test]
async fn test_syntax_error_range() {
    // Verify error ranges are valid and within document bounds
    // ...
}

/// Test PostgreSQL syntax errors
#[tokio::test]
async fn test_postgresql_syntax_error() {
    // Test that PostgreSQL dialect also detects syntax errors
    // ...
}
```

**Test Organization**:
```
crates/lsp/tests/diagnostic_tests.rs
├── Infrastructure tests (existing - 6 tests)
└── Syntax error tests (new - 8 tests)
    ├── test_syntax_error_missing_from
    ├── test_syntax_error_missing_comma
    ├── test_syntax_error_unbalanced_parens
    ├── test_syntax_error_unterminated_string
    ├── test_valid_sql_no_diagnostics
    ├── test_multiple_syntax_errors
    ├── test_syntax_error_range
    └── test_postgresql_syntax_error
```

---

## Implementation Plan

### Phase 1: Cleanup
1. Remove debug statements from `diagnostic.rs`
2. Run existing tests to ensure no breakage

### Phase 2: Enhanced Messages
1. Implement `enhance_error_message()` method
2. Implement pattern detection helpers
3. Add unit tests for pattern detection
4. Update integration tests with new message assertions

### Phase 3: E2E Tests
1. Create `syntax_errors.yaml` test file
2. Create `semantic_errors.yaml` test file (for future)
3. Update `tests/e2e-rs/tests/diagnostics.rs`
4. Update E2E framework for error code validation
5. Run `make test-e2e` and verify syntax tests pass

### Phase 4: Integration Tests
1. Add 8 new integration tests to `diagnostic_tests.rs`
2. Run `cargo test --package unified-sql-lsp-lsp --test diagnostic_tests`
3. Verify all tests pass

### Total Changes
- ~50 lines removed (debug statements)
- ~200 lines added (error message enhancement, tests)
- 2 new test files (syntax_errors.yaml, semantic_errors.yaml)
- 8 new integration tests
- 6 enabled E2E tests

## Success Criteria

✅ All debug statements removed
✅ Error messages include context and suggestions
✅ E2E syntax error tests pass (6/6)
✅ Integration tests include actual syntax errors (8 new tests)
✅ All existing tests still pass
✅ Code compiles without warnings

## Future Work

Once DIAG-002 enhancements are complete, proceed to:
- DIAG-003: Undefined table diagnostics
- DIAG-004: Undefined column diagnostics
- DIAG-005: Ambiguous column diagnostics

These will enable the semantic_errors.yaml E2E tests.
