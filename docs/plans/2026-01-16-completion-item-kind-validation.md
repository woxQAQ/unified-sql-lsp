# Completion Item Kind Validation Design

**Date:** 2026-01-16
**Status:** Draft
**Author:** Claude (Brainstorming Session)

## Problem Statement

Current E2E tests only validate whether completion items are present or absent, but do not validate the `CompletionItemKind` (e.g., Field, Function, Struct, Keyword). This can lead to bugs where the LSP server returns incorrect item types that go undetected by tests.

## Proposed Solution

Add explicit kind validation to completion expectations, allowing tests to specify both the name and expected kind of each completion item.

## YAML Schema Enhancement

### New Structure

```yaml
expect_completion:
  contains:
    - name: "id"
      kind: "Field"
    - name: "username"
      kind: "Field"
    - name: "COUNT"
      kind: "Function"
    - name: "users"
      kind: "Struct"
    - name: "*"
      kind: "Operator"
    - name: "SELECT"        # kind is optional - validates presence only
  min_count: 5
```

### Rules

- `contains` array requires objects with a `name` field (required)
- `kind` field is optional - if not specified, only validates presence
- Both `name` and `kind` must match for validation to pass

### Supported Kinds

The following LSP `CompletionItemKind` values are supported:

- `Function` - SQL functions (COUNT, SUM, AVG, etc.)
- `Field` - Column references
- `Struct` - Table names
- `Variable` - Variables
- `Constant` - Constants/literals
- `Keyword` - SQL keywords (SELECT, FROM, WHERE, etc.)
- `Operator` - Operators (*, +, -, etc.)
- `Method` - Methods
- `Property` - Properties
- `Class` - Classes (for dialect-specific object types)
- `Enum` - Enum types
- `EnumMember` - Enum members
- `Interface` - Interface types
- `Reference` - References

## Implementation Changes

### 1. Update YAML Parser (`src/yaml_parser.rs`)

Add new type for completion item expectations:

```rust
use tower_lsp::lsp_types::CompletionItemKind;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CompletionItemExpectation {
    /// Item label/name
    pub name: String,

    /// Expected completion item kind (optional)
    pub kind: Option<String>,
}

impl CompletionItemExpectation {
    /// Parse LSP kind string to CompletionItemKind
    pub fn parse_kind(&self) -> Option<CompletionItemKind> {
        self.kind.as_ref().and_then(|k| match k.as_str() {
            "Function" => Some(CompletionItemKind::FUNCTION),
            "Field" => Some(CompletionItemKind::FIELD),
            "Struct" => Some(CompletionItemKind::STRUCT),
            "Class" => Some(CompletionItemKind::CLASS),
            "Variable" => Some(CompletionItemKind::VARIABLE),
            "Constant" => Some(CompletionItemKind::CONSTANT),
            "Keyword" => Some(CompletionItemKind::KEYWORD),
            "Operator" => Some(CompletionItemKind::OPERATOR),
            "Method" => Some(CompletionItemKind::METHOD),
            "Property" => Some(CompletionItemKind::PROPERTY),
            "Enum" => Some(CompletionItemKind::ENUM),
            "EnumMember" => Some(CompletionItemKind::ENUM_MEMBER),
            "Interface" => Some(CompletionItemKind::INTERFACE),
            "Reference" => Some(CompletionItemKind::REFERENCE),
            _ => None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CompletionExpectation {
    /// Items that MUST be present (with optional kind validation)
    #[serde(default)]
    pub contains: Vec<CompletionItemExpectation>,

    /// Items that MUST NOT be present
    #[serde(default)]
    pub not_contains: Vec<String>,

    /// Expected total count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,

    /// Expected order (first N items)
    #[serde(default)]
    pub order: Vec<String>,

    /// Minimum count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<usize>,
}
```

### 2. Update Assertions (`src/assertions.rs`)

Add new function to validate completion items with kinds:

```rust
use std::collections::HashMap;
use crate::yaml_parser::CompletionItemExpectation;

/// Assert completion contains specific items with optional kind validation
pub fn assert_completion_contains_with_kinds(
    items: &[CompletionItem],
    expected: &[CompletionItemExpectation],
) -> Result<()> {
    let item_map: HashMap<&str, &CompletionItem> = items.iter()
        .map(|i| (i.label.as_str(), i))
        .collect();

    for expected_item in expected {
        let item = item_map.get(expected_item.name.as_str())
            .ok_or_else(|| anyhow::anyhow!(
                "Expected completion to contain '{}', but it was not found. Available items: {:?}",
                expected_item.name,
                item_map.keys().collect::<Vec<_>>()
            ))?;

        // Validate kind if specified
        if let Some(expected_kind_str) = &expected_item.kind {
            let expected_kind = expected_item.parse_kind()
                .ok_or_else(|| anyhow::anyhow!(
                    "Invalid kind string '{}' for item '{}'",
                    expected_kind_str,
                    expected_item.name
                ))?;

            if item.kind != Some(expected_kind) {
                bail!(
                    "Expected item '{}' to have kind {:?}, but got {:?}",
                    expected_item.name,
                    expected_kind,
                    item.kind
                );
            }
        }
    }

    Ok(())
}
```

Keep existing simple assertions for `not_contains`, `count`, `order`, etc. unchanged.

### 3. Update Test Runner (`src/lib.rs`)

Modify `run_test()` to use new assertion:

```rust
// In run_test() function
if let Some(completion_expect) = &test.expect_completion {
    let completion_items = conn.completion(uri.clone(), position).await?
        .unwrap_or_default();

    // Validate contains with kinds (NEW)
    if !completion_expect.contains.is_empty() {
        assertions::assert_completion_contains_with_kinds(
            &completion_items,
            &completion_expect.contains
        )?;
    }

    // Validate not_contains (unchanged)
    if !completion_expect.not_contains.is_empty() {
        assertions::assert_completion_not_contains(&completion_items, &completion_expect.not_contains)?;
    }

    // Validate count (unchanged)
    if let Some(count) = completion_expect.count {
        assertions::assert_completion_count(&completion_items, count)?;
    }

    // Validate min_count (unchanged)
    if let Some(min_count) = completion_expect.min_count {
        assertions::assert_completion_min_count(&completion_items, min_count)?;
    }

    // Validate order (unchanged)
    if !completion_expect.order.is_empty() {
        assertions::assert_completion_order(&completion_items, &completion_expect.order)?;
    }
}
```

## Migration Strategy

### Phase 1: Update Type Definitions
1. Add `CompletionItemExpectation` struct to `src/yaml_parser.rs`
2. Add `parse_kind()` helper method
3. Update `CompletionExpectation.contains` from `Vec<String>` to `Vec<CompletionItemExpectation>`

### Phase 2: Update Assertion Logic
1. Add `assert_completion_contains_with_kinds()` to `src/assertions.rs`
2. Update `run_test()` in `src/lib.rs` to use new assertion function
3. Remove or deprecate old `assert_completion_contains()` function

### Phase 3: Update Test YAMLs
Convert all existing test YAMLs to new format. Example:

**Before:**
```yaml
tests:
  - name: "unqualified column completion"
    sql: "SELECT | FROM users"
    expect_completion:
      contains:
        - "id"
        - "username"
        - "email"
        - "full_name"
      min_count: 5
```

**After:**
```yaml
tests:
  - name: "unqualified column completion"
    sql: "SELECT | FROM users"
    expect_completion:
      contains:
        - name: "id"
          kind: "Field"
        - name: "username"
          kind: "Field"
        - name: "email"
          kind: "Field"
        - name: "full_name"
          kind: "Field"
      min_count: 5
```

**Additional example with different kinds:**
```yaml
tests:
  - name: "function completion"
    sql: "SELECT | FROM users"
    expect_completion:
      contains:
        - name: "COUNT"
          kind: "Function"
        - name: "SUM"
          kind: "Function"
        - name: "AVG"
          kind: "Function"
        - name: "id"
          kind: "Field"
```

### Phase 4: Determine Expected Kinds

Run tests initially without kind validation to observe actual kinds returned by LSP server, then add appropriate `kind` fields to test expectations:

1. Temporarily make `kind` optional in all tests
2. Run `make test-e2e` and log actual kinds
3. Update test YAMLs with correct kinds
4. Re-run tests to verify

### Phase 5: Run Tests & Verify
1. Run `make test-e2e` to ensure all tests pass
2. Verify kind validation works correctly by introducing bugs and confirming tests catch them
3. Add more kind-specific tests for edge cases

## Example Test Cases

### Table Completion (Struct Kind)
```yaml
- name: "table completion in FROM clause"
  sql: "SELECT * FROM |"
  expect_completion:
    contains:
      - name: "users"
        kind: "Struct"
      - name: "orders"
        kind: "Struct"
```

### Function Completion (Function Kind)
```yaml
- name: "aggregate function completion"
  sql: "SELECT | FROM users"
  expect_completion:
    contains:
      - name: "COUNT"
        kind: "Function"
      - name: "SUM"
        kind: "Function"
      - name: "AVG"
        kind: "Function"
```

### Keyword Completion (Keyword Kind)
```yaml
- name: "keyword completion"
  sql: "|"
  expect_completion:
    contains:
      - name: "SELECT"
        kind: "Keyword"
      - name: "FROM"
        kind: "Keyword"
      - name: "WHERE"
        kind: "Keyword"
```

## Benefits

1. **More Comprehensive Validation:** Validates both presence AND type of completion items
2. **Earlier Bug Detection:** Catches bugs where LSP returns wrong item kind
3. **Better Documentation:** Tests serve as documentation of expected item types
4. **Type Safety:** Ensures LSP server behavior matches specification
5. **Easier Debugging:** Clear error messages when kind mismatch occurs

## Error Messages

Example error when kind validation fails:

```
Expected item 'COUNT' to have kind Function, but got Some(Field)

Expected: Function
Actual: Field

This indicates the LSP server is misclassifying COUNT as a Field instead of a Function.
```

## Open Questions

None - approach is straightforward.

## Next Steps

After approval, proceed with implementation following the migration strategy phases.
