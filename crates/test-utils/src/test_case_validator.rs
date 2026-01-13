// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Test case validation logic
//!
//! This module provides validation logic for checking LSP completion results
//! against expected test case outcomes.

use crate::test_case_parser::{TestCase, ExpectedItem, TestOptions, ParseError};
use thiserror::Error;

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Too few completion items: expected at least {expected}, got {actual}")]
    TooFewItems { expected: usize, actual: usize },

    #[error("Missing required item: {0}")]
    MissingItem(String),

    #[error("Item not found: {label} [{kind}] {detail}")]
    ItemNotFound { label: String, kind: String, detail: String },

    #[error("Item not found: {0}")]
    ItemNotFoundSimple(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] ParseError),

    #[error("Invalid completion item: {0}")]
    InvalidItem(String),
}

/// Validate LSP completion results against a test case
///
/// This function checks that the actual completion items match the expected
/// items defined in the test case.
///
/// # Arguments
///
/// * `actual` - The actual completion items from the LSP server
/// * `expected_case` - The test case with expected results
///
/// # Returns
///
/// * `Ok(())` if validation passes
/// * `Err(ValidationError)` if validation fails
pub fn validate_completion(
    actual: &[lsp_types::CompletionItem],
    expected_case: &TestCase,
) -> Result<(), ValidationError> {
    let default_options = TestOptions::default();
    let options = expected_case.options.as_ref().unwrap_or(&default_options);

    // Check minimum item count
    if let Some(min) = options.min_items {
        if actual.len() < min {
            return Err(ValidationError::TooFewItems {
                expected: min,
                actual: actual.len(),
            });
        }
    }

    // Check required items
    if let Some(contains) = &options.contains {
        for item in contains {
            if !actual.iter().any(|i| i.label.contains(item)) {
                return Err(ValidationError::MissingItem(item.clone()));
            }
        }
    }

    // Exact match validation (if requested)
    if options.exact_match.unwrap_or(false) {
        validate_exact_match(actual, expected_case)?;
    } else {
        // Simple label matching
        validate_labels(actual, expected_case)?;
    }

    Ok(())
}

/// Validate that all expected items are present with exact details
fn validate_exact_match(
    actual: &[lsp_types::CompletionItem],
    expected_case: &TestCase,
) -> Result<(), ValidationError> {
    for exp in &expected_case.expected {
        match exp {
            ExpectedItem::Full { label, kind, detail } => {
                let found = actual.iter().any(|item| {
                    item.label == *label
                        && kind_matches(item, kind)
                        && item.detail.as_deref() == Some(detail.as_str())
                });
                if !found {
                    return Err(ValidationError::ItemNotFound {
                        label: label.clone(),
                        kind: kind.clone(),
                        detail: detail.clone(),
                    });
                }
            }
            ExpectedItem::Simple(name) => {
                if !actual.iter().any(|i| i.label == *name) {
                    return Err(ValidationError::ItemNotFoundSimple(name.clone()));
                }
            }
        }
    }
    Ok(())
}

/// Validate that all expected labels are present
fn validate_labels(
    actual: &[lsp_types::CompletionItem],
    expected_case: &TestCase,
) -> Result<(), ValidationError> {
    for exp in &expected_case.expected {
        let label = match exp {
            ExpectedItem::Full { label, .. } => label,
            ExpectedItem::Simple(name) => name,
        };

        if !actual.iter().any(|i| i.label == *label) {
            return Err(ValidationError::ItemNotFoundSimple(label.clone()));
        }
    }
    Ok(())
}

/// Check if completion item kind matches the expected kind string
fn kind_matches(item: &lsp_types::CompletionItem, kind: &str) -> bool {
    if let Some(item_kind) = item.kind {
        let kind_str = match item_kind {
            lsp_types::CompletionItemKind::CLASS => "Class",
            lsp_types::CompletionItemKind::COLOR => "Color",
            lsp_types::CompletionItemKind::CONSTANT => "Constant",
            lsp_types::CompletionItemKind::CONSTRUCTOR => "Constructor",
            lsp_types::CompletionItemKind::ENUM => "Enum",
            lsp_types::CompletionItemKind::ENUM_MEMBER => "EnumMember",
            lsp_types::CompletionItemKind::EVENT => "Event",
            lsp_types::CompletionItemKind::FIELD => "Field",
            lsp_types::CompletionItemKind::FILE => "File",
            lsp_types::CompletionItemKind::FOLDER => "Folder",
            lsp_types::CompletionItemKind::FUNCTION => "Function",
            lsp_types::CompletionItemKind::INTERFACE => "Interface",
            lsp_types::CompletionItemKind::KEYWORD => "Keyword",
            lsp_types::CompletionItemKind::METHOD => "Method",
            lsp_types::CompletionItemKind::MODULE => "Module",
            lsp_types::CompletionItemKind::OPERATOR => "Operator",
            lsp_types::CompletionItemKind::PROPERTY => "Property",
            lsp_types::CompletionItemKind::REFERENCE => "Reference",
            lsp_types::CompletionItemKind::SNIPPET => "Snippet",
            lsp_types::CompletionItemKind::STRUCT => "Struct",
            lsp_types::CompletionItemKind::TEXT => "Text",
            lsp_types::CompletionItemKind::TYPE_PARAMETER => "TypeParameter",
            lsp_types::CompletionItemKind::UNIT => "Unit",
            lsp_types::CompletionItemKind::VALUE => "Value",
            lsp_types::CompletionItemKind::VARIABLE => "Variable",
            _ => return false,
        };
        kind_str == kind
    } else {
        false
    }
}

/// Get cursor position from input SQL
///
/// This function finds the cursor marker `|` in the SQL string and returns
/// the position. Returns None if no cursor marker is found.
pub fn get_cursor_position(input: &str) -> Option<usize> {
    input.find('|')
}

/// Remove cursor marker from input SQL
pub fn remove_cursor_marker(input: &str) -> String {
    input.replace('|', "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{CompletionItem, CompletionItemKind};

    fn make_completion_item(label: &str, kind: CompletionItemKind, detail: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_validate_min_items() {
        let items = vec![
            make_completion_item("id", CompletionItemKind::FIELD, "users.id"),
            make_completion_item("name", CompletionItemKind::FIELD, "users.name"),
        ];

        let mut test_case = TestCase {
            description: "Test".to_string(),
            dialect: crate::test_case_parser::Dialect::All,
            context: None,
            input: "SELECT | FROM users".to_string(),
            expected: vec![],
            options: None,
        };

        // Should pass
        test_case.options = Some(TestOptions {
            min_items: Some(2),
            ..Default::default()
        });
        assert!(validate_completion(&items, &test_case).is_ok());

        // Should fail
        test_case.options = Some(TestOptions {
            min_items: Some(3),
            ..Default::default()
        });
        assert!(validate_completion(&items, &test_case).is_err());
    }

    #[test]
    fn test_validate_contains() {
        let items = vec![
            make_completion_item("id", CompletionItemKind::FIELD, "users.id"),
            make_completion_item("name", CompletionItemKind::FIELD, "users.name"),
        ];

        let test_case = TestCase {
            description: "Test".to_string(),
            dialect: crate::test_case_parser::Dialect::All,
            context: None,
            input: "SELECT | FROM users".to_string(),
            expected: vec![],
            options: Some(TestOptions {
                contains: Some(vec!["id".to_string(), "name".to_string()]),
                ..Default::default()
            }),
        };

        assert!(validate_completion(&items, &test_case).is_ok());

        let test_case_missing = TestCase {
            description: "Test".to_string(),
            dialect: crate::test_case_parser::Dialect::All,
            context: None,
            input: "SELECT | FROM users".to_string(),
            expected: vec![],
            options: Some(TestOptions {
                contains: Some(vec!["email".to_string()]),
                ..Default::default()
            }),
        };

        assert!(validate_completion(&items, &test_case_missing).is_err());
    }

    #[test]
    fn test_validate_exact_match() {
        let items = vec![
            make_completion_item("id", CompletionItemKind::FIELD, "users.id"),
            make_completion_item("name", CompletionItemKind::FIELD, "users.name"),
        ];

        let test_case = TestCase {
            description: "Test".to_string(),
            dialect: crate::test_case_parser::Dialect::All,
            context: None,
            input: "SELECT | FROM users".to_string(),
            expected: vec![
                ExpectedItem::Full {
                    label: "id".to_string(),
                    kind: "Field".to_string(),
                    detail: "users.id".to_string(),
                },
                ExpectedItem::Full {
                    label: "name".to_string(),
                    kind: "Field".to_string(),
                    detail: "users.name".to_string(),
                },
            ],
            options: Some(TestOptions {
                exact_match: Some(true),
                ..Default::default()
            }),
        };

        assert!(validate_completion(&items, &test_case).is_ok());
    }

    #[test]
    fn test_get_cursor_position() {
        assert_eq!(get_cursor_position("SELECT | FROM users"), Some(7));
        assert_eq!(get_cursor_position("SELECT * FROM users"), None);
    }

    #[test]
    fn test_remove_cursor_marker() {
        assert_eq!(remove_cursor_marker("SELECT | FROM users"), "SELECT  FROM users");
        assert_eq!(remove_cursor_marker("SELECT * FROM users"), "SELECT * FROM users");
    }

    #[test]
    fn test_kind_matches() {
        let item = make_completion_item("id", CompletionItemKind::FIELD, "users.id");
        assert!(kind_matches(&item, "Field"));
        assert!(!kind_matches(&item, "Function"));
    }
}
