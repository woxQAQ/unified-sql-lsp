# Enhanced Table Alias Support Design

**Date:** 2026-01-19
**Status:** Approved
**Priority:** P0 - Critical completion gaps
**Author:** Claude (AI Assistant)
**Issue Reference:** PLAYGROUND_HANDOFF.md

---

## Executive Summary

This design enhances table alias support in the main LSP server to fix critical completion gaps for JOIN statements, subqueries, and self-joins. The current implementation only handles simple FROM clause aliases, causing completion failures in common multi-table queries.

**Impact:**
- Fixes JOIN alias completion (e.g., `SELECT u., o. FROM users u JOIN orders o ON ...`)
- Enables subquery alias support (e.g., `FROM (SELECT ...) AS sub`)
- Allows self-joins with same table, different aliases (e.g., `FROM users u1 JOIN users u2`)
- Estimated implementation time: 5-8 days

---

## Problem Analysis

### What Works ✅

- Simple alias completion: `SELECT u. FROM users u` correctly shows `u.id`, `u.username`, etc.
- `AliasResolver` has sophisticated 4-strategy resolution:
  1. Exact match
  2. Starts with match
  3. First letter + numeric pattern (e.g., "e1" → "employees")
  4. Single table fallback
- `ScopeManager` properly tracks tables and aliases in scopes
- `TableSymbol` correctly handles display names with alias distinction

### What's Broken ❌

#### Issue 1: JOIN Alias Resolution Failure

**Symptom:** `SELECT u., o. FROM users u JOIN orders o ON u.id = o.user_id`
- Expected: Show `u.*` columns when typing `u.`
- Actual: Only `u.*` works, `o.*` shows nothing

**Root Cause:** `crates/context/src/scope_builder.rs:141`

```rust
if node.kind() == "table_reference" {
    // Only handles FROM clause tables
}
```

JOINs create `join_clause` nodes, not `table_reference` nodes:
```
from_clause
  table_reference: users u
  join_clause: JOIN orders o ON ...
```

The scope builder ignores `join_clause` nodes entirely.

#### Issue 2: Subquery Alias Not Supported

**Symptom:** `SELECT s. FROM (SELECT id, name FROM users) AS s`
- Expected: Show `s.id`, `s.name`
- Actual: No completion for `s.`

**Root Cause:** Subqueries in FROM clauses aren't parsed to extract column names from the subquery's projection.

#### Issue 3: Duplicate Table Restriction

**Symptom:** `SELECT u1., u2. FROM users u1 JOIN users u2 ON u1.manager_id = u2.id`
- Expected: Both `u1.*` and `u2.*` work
- Actual: Returns error "Duplicate table reference: users"

**Root Cause:** `scope_builder.rs:146-153` prevents multiple instances of the same table:

```rust
if table_counts[&display_name] == 1 {
    tables.push(table);
} else {
    return Err(ScopeBuildError::ScopeBuild(format!(
        "Duplicate table reference: {}", display_name
    )));
}
```

This check doesn't account for aliases distinguishing the same table.

---

## Proposed Solution

### Architecture Overview

We'll fix all three issues by enhancing the `ScopeBuilder` in `crates/context` to:

1. **Extract tables from `join_clause` nodes** (Phase 1)
2. **Parse subqueries and derive columns from projection** (Phase 2)
3. **Remove duplicate table restriction** (Phase 3)

**Key Design Principles:**
- **Reuse existing infrastructure:** `AliasResolver`, `ScopeManager`, `TableSymbol` already work correctly
- **Grammar-aware parsing:** Use tree-sitter node types, not regex/string matching
- **Incremental population:** Don't fetch columns during scope building - defer to catalog
- **Graceful degradation:** Log warnings on parse failures, don't fail entire completion

### Phase 1: JOIN Alias Support

**Implementation Location:** `crates/context/src/scope_builder.rs`

**Changes:**

1. **Add `join_clause` handling in `extract_tables_recursive()`:**

```rust
fn extract_tables_recursive(
    node: &Node,
    source: &str,
    tables: &mut Vec<TableSymbol>,
    table_counts: &mut HashMap<String, usize>,
) -> Result<(), ScopeBuildError> {
    // Existing table_reference handling
    if node.kind() == "table_reference" {
        // ... existing code ...
        return Ok(());
    }

    // NEW: Handle join_clause nodes
    if node.kind() == "join_clause" {
        match Self::parse_join_clause(node, source) {
            Ok(table) => {
                let display_name = table.display_name().to_string();
                *table_counts.entry(display_name.clone()).or_insert(0) += 1;
                tables.push(table);
            }
            Err(e) => {
                tracing::warn!("Failed to parse JOIN clause: {}", e);
                // Continue - don't fail entire scope build
            }
        }
        return Ok(()); // Don't recurse deeper into JOIN
    }

    // Recurse into children
    for child in node.children(&mut node.walk()) {
        Self::extract_tables_recursive(&child, source, tables, table_counts)?;
    }
    Ok(())
}
```

2. **Add `parse_join_clause()` method:**

```rust
/// Parse a join_clause node to extract table and alias
///
/// Supports formats:
/// - JOIN table_name [AS alias]
/// - JOIN (subquery) [AS alias]
///
/// join_clause structure from grammar.js:
/// seq(optional(join_type), "JOIN", table_name, optional(alias), "ON", ...)
fn parse_join_clause(node: &Node, source: &str) -> Result<TableSymbol, ScopeBuildError> {
    let mut table_name = None;
    let mut alias = None;

    for child in node.children(&mut node.walk()) {
        match child.kind() {
            "table_name" | "identifier" => {
                if table_name.is_none() {
                    table_name = Some(Self::extract_node_text(&child, source));
                }
            }
            "alias" => {
                if let Some(a) = Self::extract_alias(&child, source) {
                    alias = Some(a);
                }
            }
            _ => {} // Ignore join_type, ON clause, etc.
        }
    }

    let table_name = table_name.ok_or_else(|| {
        ScopeBuildError::ScopeBuild("Table name not found in join_clause".to_string())
    })?;

    let mut table = TableSymbol::new(&table_name);
    if let Some(a) = alias {
        table = table.with_alias(a);
    }
    Ok(table)
}
```

**Test Cases:**
```sql
-- Simple JOIN
SELECT u., o. FROM users u JOIN orders o ON u.id = o.user_id
-- Expected: u.id, u.username, ..., o.id, o.total_amount, ...

-- Multiple JOINs
SELECT u., o., oi. FROM users u
  JOIN orders o ON u.id = o.user_id
  JOIN order_items oi ON o.id = oi.order_id
-- Expected: All three alias-qualified columns

-- JOIN without alias
SELECT users., orders. FROM users JOIN orders ON users.id = orders.user_id
-- Expected: users.*, orders.*
```

**Timeline:** 1-2 days implementation + 1 day tests

---

### Phase 2: Subquery Alias Support

**Challenge:** Subqueries require deriving column information from the subquery's SELECT list, not from the catalog.

**Implementation Strategy:**

1. **Detect subqueries in FROM clause:**

```sql
FROM (SELECT id, name FROM users) AS u
--     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ subquery
```

Tree-sitter structure:
```
from_clause
  table_reference
    select_statement (subquery)
    alias (optional AS u)
```

2. **Add subquery parsing to `extract_tables_recursive()`:**

```rust
// In extract_tables_recursive, after table_reference check:
if node.kind() == "table_reference" {
    // Check if this is a subquery
    if let Some(subquery) = node.children(&mut node.walk())
        .find(|n| n.kind() == "select_statement")
    {
        // This is a subquery reference
        match Self::parse_subquery_reference(node, source) {
            Ok(table) => tables.push(table),
            Err(e) => {
                tracing::warn!("Failed to parse subquery: {}", e);
            }
        }
        return Ok(());
    }

    // Regular table reference (existing code)
    // ...
}
```

3. **Add `parse_subquery_reference()` method:**

```rust
/// Parse a subquery table reference
///
/// Format: FROM (SELECT ...) [AS alias]
///
/// Extracts column names from the subquery's projection
fn parse_subquery_reference(
    node: &Node,
    source: &str,
) -> Result<TableSymbol, ScopeBuildError> {
    // Find the select_statement child
    let select_stmt = node
        .children(&mut node.walk())
        .find(|n| n.kind() == "select_statement")
        .ok_or_else(|| ScopeBuildError::ScopeBuild("No SELECT in subquery".to_string()))?;

    // Extract alias if present
    let alias = node
        .children(&mut node.walk())
        .find(|n| n.kind() == "alias")
        .and_then(|a| Self::extract_alias(&a, source));

    // Extract column names from subquery's projection
    let column_names = Self::extract_projection_columns(&select_stmt, source)?;

    // Create TableSymbol with subquery columns (no catalog needed)
    let table_name = alias.clone().unwrap_or_else(|| "subquery".to_string());
    let mut table = TableSymbol::new(&table_name);
    if let Some(a) = alias {
        table = table.with_alias(a);
    }

    // Add columns without type info (subquery columns have unknown types)
    let columns: Vec<ColumnSymbol> = column_names
        .into_iter()
        .map(|name| ColumnSymbol::new(&name, "UNKNOWN".to_string(), &table_name))
        .collect();

    table = table.with_columns(columns);
    Ok(table)
}
```

4. **Add `extract_projection_columns()` helper:**

```rust
/// Extract column names from SELECT projection
///
/// Handles:
/// - SELECT col1, col2, ...
/// - SELECT table.col, ...
/// - SELECT expr AS alias
///
/// Limitations:
/// - * wildcards can't be expanded without catalog (returns empty)
/// - Complex expressions use alias or skip
fn extract_projection_columns(select_node: &Node, source: &str) -> Result<Vec<String>, ScopeBuildError> {
    let projection = select_node
        .children(&mut select_node.walk())
        .find(|n| n.kind() == "select_projection")
        .ok_or_else(|| ScopeBuildError::ScopeBuild("No projection in SELECT".to_string()))?;

    let mut columns = Vec::new();
    for child in projection.children(&mut projection.walk()) {
        match child.kind() {
            "column_reference" => {
                columns.push(Self::extract_node_text(&child, source));
            }
            "identifier" => {
                // This could be a column or an alias
                columns.push(Self::extract_node_text(&child, source));
            }
            "*" => {
                // Wildcard - can't expand without catalog
                // Return empty and let catalog handle it later
                tracing::debug!("Wildcard in subquery projection, skipping extraction");
            }
            _ => {
                // Complex expression - look for alias
                if let Some(alias_node) = child.children(&mut child.walk()).find(|n| n.kind() == "alias") {
                    if let Some(alias) = Self::extract_alias(&alias_node, source) {
                        columns.push(alias);
                    }
                }
            }
        }
    }

    Ok(columns)
}
```

**Test Cases:**
```sql
-- Simple subquery
SELECT s. FROM (SELECT id, username FROM users) AS s
-- Expected: s.id, s.username

-- Subquery with expression aliases
SELECT s. FROM (SELECT id, COUNT(*) AS cnt FROM users GROUP BY id) AS s
-- Expected: s.id, s.cnt

-- Subquery without alias
SELECT subquery. FROM (SELECT id, name FROM users)
-- Expected: subquery.id, subquery.name
```

**Limitations:**
- `*` wildcards in subqueries return empty (requires catalog integration)
- Type information unavailable (shows as "UNKNOWN")
- Complex expressions without aliases are skipped

**Timeline:** 2-3 days implementation + 1 day tests

---

### Phase 3: Remove Duplicate Table Restriction

**Implementation:**

In `extract_tables_recursive()` (line 146-153), remove the duplicate check:

```rust
// OLD CODE (remove entirely):
if table_counts[&display_name] == 1 {
    tables.push(table);
} else {
    return Err(ScopeBuildError::ScopeBuild(format!(
        "Duplicate table reference: {}", display_name
    )));
}

// NEW CODE:
tables.push(table); // Always push - alias distinguishes
```

**Rationale:**
- `TableSymbol::display_name()` already includes alias when present
- Self-joins are a valid and common SQL pattern
- The `table_counts` HashMap is no longer needed after this change

**Test Cases:**
```sql
-- Self-join with different aliases
SELECT u1.name, u2.name AS manager
FROM users u1
JOIN users u2 ON u1.manager_id = u2.id
-- Expected: u1.*, u2.*

-- Three-way self-join
SELECT e1.name, e2.name AS manager, e3.name AS grand_manager
FROM employees e1
JOIN employees e2 ON e1.manager_id = e2.id
JOIN employees e3 ON e2.manager_id = e3.id
-- Expected: e1.*, e2.*, e3.*
```

**Timeline:** 1 day

---

## Integration & Data Flow

### Completion Flow with Enhanced Aliases

```
User types: "SELECT u., o. FROM users u JOIN orders o ON u.id = o.user_id"
              ↓
CompletionEngine.complete(document, position)
              ↓
detect_completion_context(root_node, position)
    → returns CompletionContext::SelectColumns
              ↓
ScopeBuilder::build_from_select(select_node, source)
    ├─ parse_from_clause()
    │   ├─ extract table_reference: users u ✓
    │   └─ extract join_clause: orders o ✓ (NEW)
    │
    └─ ScopeManager contains:
        ├─ TableSymbol { name: "users", alias: "u" }
        └─ TableSymbol { name: "orders", alias: "o" }
              ↓
CatalogCompletionFetcher.fetch_columns(scope, context)
    ├─ For "u." → AliasResolver.resolve("u")
    │   → Strategy::ExactMatch → finds "users"
    │   → catalog.get_columns("users") → [id, username, ...]
    │
    └─ For "o." → AliasResolver.resolve("o")
        → Strategy::ExactMatch → finds "orders"
        → catalog.get_columns("orders") → [id, total_amount, ...]
              ↓
CompletionRenderer.render_completion(columns)
    → [CompletionItem { label: "u.id", ... },
        CompletionItem { label: "u.username", ... },
        CompletionItem { label: "o.id", ... },
        ...]
              ↓
Return to LSP client → Show in completion UI
```

### Error Handling Strategy

1. **Graceful degradation:** Parse failures log warnings but don't fail entire completion
   ```rust
   match Self::parse_join_clause(node, source) {
       Ok(table) => tables.push(table),
       Err(e) => {
           tracing::warn!("Failed to parse JOIN clause: {}", e);
           // Continue with other tables
       }
   }
   ```

2. **Partial results:** Return empty vec rather than error on complex subquery expressions
   ```rust
   fn extract_projection_columns(...) -> Result<Vec<String>> {
       let mut columns = Vec::new();
       // ... extraction logic ...
       Ok(columns) // Always succeed, even if empty
   }
   ```

3. **Validation:** Unit tests for each parsing function
   ```rust
   #[cfg(test)]
   mod tests {
       #[test]
       fn test_parse_join_clause_with_alias() { /* ... */ }

       #[test]
       fn test_parse_join_clause_without_alias() { /* ... */ }

       #[test]
       fn test_parse_subquery_with_columns() { /* ... */ }
   }
   ```

---

## Testing Strategy

### Unit Tests

**Location:** `crates/context/src/scope_builder.rs` (existing test module)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_sql(sql: &str) -> Node {
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_sql::language()).unwrap();
        let tree = parser.parse(sql, None).unwrap();
        tree.root_node()
    }

    #[test]
    fn test_extract_join_with_alias() {
        let sql = "SELECT u.id FROM users u JOIN orders o ON u.id = o.user_id";
        let root = parse_sql(sql);
        let select = root.children(&mut root.walk()).find(|n| n.kind() == "select_statement").unwrap();

        let result = ScopeBuilder::build_from_select(&select, sql);
        assert!(result.is_ok());

        let manager = result.unwrap();
        let scope = manager.get_scope(0).unwrap();
        assert_eq!(scope.tables.len(), 2);

        let users_table = scope.tables.iter().find(|t| t.table_name == "users").unwrap();
        assert_eq!(users_table.alias, Some("u".to_string()));

        let orders_table = scope.tables.iter().find(|t| t.table_name == "orders").unwrap();
        assert_eq!(orders_table.alias, Some("o".to_string()));
    }

    #[test]
    fn test_self_join_allows_same_table() {
        let sql = "SELECT u1.id FROM users u1 JOIN users u2 ON u1.id = u2.manager_id";
        let root = parse_sql(sql);
        let select = root.children(&mut root.walk()).find(|n| n.kind() == "select_statement").unwrap();

        let result = ScopeBuilder::build_from_select(&select, sql);
        assert!(result.is_ok());

        let manager = result.unwrap();
        let scope = manager.get_scope(0).unwrap();
        assert_eq!(scope.tables.len(), 2); // Both u1 and u2
    }

    #[test]
    fn test_subquery_extracts_columns() {
        let sql = "SELECT s.id FROM (SELECT id, name FROM users) AS s";
        let root = parse_sql(sql);
        let select = root.children(&mut root.walk()).find(|n| n.kind() == "select_statement").unwrap();

        let result = ScopeBuilder::build_from_select(&select, sql);
        assert!(result.is_ok());

        let manager = result.unwrap();
        let scope = manager.get_scope(0).unwrap();
        assert_eq!(scope.tables.len(), 1);

        let subquery = &scope.tables[0];
        assert_eq!(subquery.table_name, "s");
        assert_eq!(subquery.alias, Some("s".to_string()));
        assert_eq!(subquery.columns.len(), 2);
        assert_eq!(subquery.columns[0].name, "id");
        assert_eq!(subquery.columns[1].name, "name");
    }
}
```

### E2E Tests

**Location:** `tests/e2e-rs/tests/mysql-8.0/completion/`

#### New File: `join_aliases.yaml`

```yaml
name: "JOIN alias completion"
description: "Column completion for JOINed tables with aliases"

database:
  dialect: "mysql"
  connection_string: "mysql://test_user:test_password@127.0.0.1:3307/test_db"
  schemas:
    - "../../../fixtures/schema/mysql/01_create_tables.sql"
  data:
    - "../../../fixtures/data/mysql/02_insert_basic_data.sql"

tests:
  - name: "simple join with two aliases"
    description: "Should show columns from both joined tables"
    sql: "SELECT u.| FROM users u JOIN orders o ON u.id = o.user_id"
    expect_completion:
      contains:
        - "u.id"
        - "u.username"
        - "u.email"
        - "u.full_name"
      not_contains:
        - "o.id"
        - "orders"
      min_count: 10

  - name: "switch to second table alias"
    description: "Should show columns from orders table when using o. prefix"
    sql: "SELECT u.id, o.| FROM users u JOIN orders o ON u.id = o.user_id"
    expect_completion:
      contains:
        - "o.id"
        - "o.total_amount"
        - "o.user_id"
        - "o.status"
      not_contains:
        - "u.id"
        - "users"
      min_count: 5

  - name: "multiple joins"
    description: "Should handle three-way JOINs"
    sql: "SELECT u., o., oi.| FROM users u JOIN orders o ON u.id = o.user_id JOIN order_items oi ON o.id = oi.order_id"
    expect_completion:
      contains:
        - "oi.id"
        - "oi.quantity"
        - "oi.unit_price"
        - "oi.discount_percent"
      min_count: 5

  - name: "join without explicit alias"
    description: "Should use table name when alias not specified"
    sql: "SELECT users., orders.| FROM users JOIN orders ON users.id = orders.user_id"
    expect_completion:
      contains:
        - "orders.id"
        - "orders.total_amount"
      min_count: 5
```

#### New File: `self_join.yaml`

```yaml
name: "Self-join completion"
description: "Column completion for self-joins with same table"

database:
  dialect: "mysql"
  connection_string: "mysql://test_user:test_password@127.0.0.1:3307/test_db"
  schemas:
    - "../../../fixtures/schema/mysql/01_create_tables.sql"
  data:
    - "../../../fixtures/data/mysql/02_insert_basic_data.sql"

tests:
  - name: "self-join with different aliases"
    description: "Should distinguish same table with different aliases"
    sql: "SELECT u1., u2.| FROM users u1 JOIN users u2 ON u1.manager_id = u2.id"
    expect_completion:
      contains:
        - "u1.id"
        - "u1.username"
        - "u1.email"
        - "u2.id"
        - "u2.username"
        - "u2.email"
      min_count: 20

  - name: "three-way self-join"
    description: "Should handle multiple self-joins"
    sql: "SELECT e1., e2., e3.| FROM employees e1 JOIN employees e2 ON e1.manager_id = e2.id JOIN employees e3 ON e2.manager_id = e3.id"
    expect_completion:
      contains:
        - "e1.id"
        - "e2.id"
        - "e3.id"
      min_count: 3
```

#### New File: `subquery_aliases.yaml`

```yaml
name: "Subquery alias completion"
description: "Column completion for subqueries with aliases"

database:
  dialect: "mysql"
  connection_string: "mysql://test_user:test_password@127.0.0.1:3307/test_db"
  schemas:
    - "../../../fixtures/schema/mysql/01_create_tables.sql"
  data:
    - "../../../fixtures/data/mysql/02_insert_basic_data.sql"

tests:
  - name: "simple subquery with alias"
    description: "Should show columns from subquery projection"
    sql: "SELECT s.| FROM (SELECT id, username FROM users) AS s"
    expect_completion:
      contains:
        - "s.id"
        - "s.username"
      not_contains:
        - "s.email"
        - "users"
      min_count: 2

  - name: "subquery with expression aliases"
    description: "Should use expression aliases as column names"
    sql: "SELECT s.| FROM (SELECT id, COUNT(*) AS cnt FROM users GROUP BY id) AS s"
    expect_completion:
      contains:
        - "s.id"
        - "s.cnt"
      min_count: 2

  - name: "subquery without explicit alias"
    description: "Should use default subquery name"
    sql: "SELECT subquery.| FROM (SELECT id, name FROM users)"
    expect_completion:
      contains:
        - "subquery.id"
        - "subquery.name"
      min_count: 2
```

---

## Implementation Timeline

| Phase | Tasks | Duration | Dependencies |
|-------|-------|----------|--------------|
| **Phase 1** | JOIN alias support | 2-3 days | None |
| - | Implement `parse_join_clause()` | 1 day | - |
| - | Add to `extract_tables_recursive()` | 0.5 day | - |
| - | Unit tests | 0.5 day | - |
| - | E2E tests (`join_aliases.yaml`) | 1 day | - |
| **Phase 2** | Subquery alias support | 3-4 days | Phase 1 |
| - | Implement `parse_subquery_reference()` | 1 day | - |
| - | Implement `extract_projection_columns()` | 1 day | - |
| - | Unit tests | 0.5 day | - |
| - | E2E tests (`subquery_aliases.yaml`) | 1 day | - |
| - | Handle edge cases (wildcards, expressions) | 0.5 day | - |
| **Phase 3** | Remove duplicate restriction | 1 day | Phase 1 |
| - | Remove duplicate check | 0.5 day | - |
| - | E2E tests (`self_join.yaml`) | 0.5 day | - |
| **Total** | **All phases** | **6-8 days** | - |

---

## Rollout Plan

### Step 1: Phase 1 Implementation (Days 1-3)
1. Implement `parse_join_clause()` method
2. Integrate into `extract_tables_recursive()`
3. Write unit tests
4. Create E2E test file `join_aliases.yaml`
5. Run tests: `make test-e2e-mysql`
6. Fix any failures

### Step 2: Phase 2 Implementation (Days 4-7)
1. Implement `parse_subquery_reference()` method
2. Implement `extract_projection_columns()` helper
3. Write unit tests
4. Create E2E test file `subquery_aliases.yaml`
5. Run tests: `make test-e2e-mysql`
6. Fix any failures

### Step 3: Phase 3 Implementation (Day 8)
1. Remove duplicate table restriction
2. Clean up unused `table_counts` HashMap if no longer needed
3. Create E2E test file `self_join.yaml`
4. Run full test suite: `make test`
5. Fix any failures

### Step 4: Validation (Day 9)
1. Manual testing with real queries
2. Performance testing (no regressions)
3. Code review
4. Documentation updates

---

## Success Criteria

### Functional Requirements

✅ **Phase 1: JOIN Aliases**
- [ ] `SELECT u., o. FROM users u JOIN orders o ON ...` shows both `u.*` and `o.*`
- [ ] Multiple JOINs: `SELECT u., o., oi. FROM users u JOIN orders o ... JOIN order_items oi ...` works
- [ ] JOIN without alias: `SELECT users., orders. FROM users JOIN orders ON ...` works
- [ ] E2E tests pass: `make test-e2e-mysql`

✅ **Phase 2: Subquery Aliases**
- [ ] `SELECT s. FROM (SELECT id, name FROM users) AS s` shows `s.id`, `s.name`
- [ ] Subquery with expression aliases: `SELECT s. FROM (SELECT COUNT(*) AS cnt ...) AS s` shows `s.cnt`
- [ ] E2E tests pass: `make test-e2e-mysql`

✅ **Phase 3: Self-Joins**
- [ ] `SELECT u1., u2. FROM users u1 JOIN users u2 ON ...` shows both aliases
- [ ] Three-way self-join works
- [ ] E2E tests pass: `make test-e2e-mysql`

### Non-Functional Requirements

- [ ] No performance regressions (completion latency < 100ms)
- [ ] Error handling: parse failures don't crash completion
- [ ] Code coverage: >80% for new code
- [ ] All existing tests still pass
- [ ] Documentation updated (inline comments, this design doc)

---

## Risks & Mitigations

| Risk | Impact | Probability | Mitigation |
|------|--------|-------------|------------|
| **Tree-sitter node structure differs by dialect** | JOIN parsing works for MySQL but not PostgreSQL | Medium | Test on both MySQL and PostgreSQL grammars; add dialect-specific handling if needed |
| **Complex subquery expressions** | Can't extract columns without catalog | Low | Log warning, return empty; catalog can provide columns during completion |
| **Performance regression** | Scope building becomes slower | Low | Parse failures degrade gracefully; lazy evaluation already in place |
| **Wildcard in subquery** | Can't expand `SELECT *` without catalog | High | Document limitation; rely on catalog to expand during completion |

---

## Future Enhancements

Out of scope for this design, but worth considering:

1. **Common Table Expressions (CTEs):**
   ```sql
   WITH user_counts AS (SELECT user_id, COUNT(*) AS cnt FROM orders GROUP BY user_id)
   SELECT u., uc. FROM users u JOIN user_counts uc ON u.id = uc.user_id
   ```
   Currently not handled; would require CTE scope tracking.

2. **LATERAL JOINs:**
   ```sql
   SELECT u., o. FROM users u JOIN LATERAL (SELECT * FROM orders WHERE user_id = u.id) o ON true
   ```
   Would require context-sensitive subquery parsing.

3. **Subquery type inference:**
   Derive column types from subquery expressions (e.g., `COUNT(*)` → `INT`)

4. **Alias suggestion in completion:**
   When typing `JOIN user`, suggest `JOIN users AS u` based on table name patterns

---

## References

- **Playground Handoff:** `/home/woxQAQ/unified-sql-lsp/PLAYGROUND_HANDOFF.md`
- **Scope Builder:** `crates/context/src/scope_builder.rs`
- **Alias Resolver:** `crates/semantic/src/alias_resolution.rs`
- **Tree-sitter Grammar:** `crates/grammar/src/grammar/grammar.js`
- **E2E Test Structure:** `tests/e2e-rs/`

---

## Appendix: Grammar Node Structures

### `join_clause` Node

From `crates/grammar/src/grammar/grammar.js:281-286`:

```javascript
join_clause: $ => seq(
  optional($.join_type),      // INNER, LEFT, RIGHT, FULL, CROSS
  /[Jj][Oo][Ii][Nn]/,
  $.table_name,               // The table being joined
  optional(seq(/[Aa][Ss]/, $.alias)),  // Optional AS alias
  /[Oo][Nn]/,                 // ON keyword
  $.expression                // Join condition
)
```

**Example CST:**
```
join_clause [0, 63]
  join_type: "INNER" [0, 5]
  "JOIN" [6, 10]
  table_name: "orders" [11, 17]
  alias [18, 21]
    identifier: "o" [18, 19]
  "ON" [20, 22]
  expression: "u.id = o.user_id" [23, 40]
```

### Subquery in FROM

```
from_clause [10, 50]
  table_reference [10, 50]
    "(" [10, 11]
    select_statement [11, 40]
      select_projection: "id, username" [11, 36]
      ...
    ")" [40, 41]
    "AS" [42, 44]
    alias: "s" [45, 46]
```

---

**End of Design Document**
