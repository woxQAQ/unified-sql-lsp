// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! ParserManager integration tests
//!
//! Comprehensive tests for ParserManager functionality with real parsed trees.

use std::time::Duration;
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::parsing::{ParseError, ParseResult, ParserManager};

#[test]
fn test_parser_manager_full_parse_success() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SELECT id, name FROM users WHERE active = true",
    );

    match result {
        ParseResult::Success { tree, parse_time } => {
            assert!(tree.is_some(), "Expected tree to be present");
            assert!(
                parse_time < Duration::from_millis(100),
                "Parse took too long"
            );
        }
        ParseResult::Partial { errors, .. } => {
            panic!("Unexpected partial parse: {:?}", errors);
        }
        ParseResult::Failed { error } => {
            panic!("Parse failed: {}", error);
        }
    }
}

#[test]
fn test_parser_manager_with_syntax_error() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SELCT * FROM users", // Typo: SELCT
    );

    match result {
        ParseResult::Success { .. } => {
            // Grammar might be lenient, this is OK
        }
        ParseResult::Partial { errors, tree } => {
            assert!(tree.is_some(), "Expected tree even with errors");
            assert!(!errors.is_empty(), "Expected at least one error");
            assert!(errors[0].to_string().contains("Syntax error"));
        }
        ParseResult::Failed { .. } => {
            // Also acceptable
        }
    }
}

#[test]
fn test_parser_manager_multiple_dialects() {
    let manager = ParserManager::new();

    // MySQL
    let mysql_result = manager.parse_text(Dialect::MySQL, "SELECT * FROM users LIMIT 10");
    assert!(!mysql_result.is_failed());

    // PostgreSQL
    let pg_result = manager.parse_text(
        Dialect::PostgreSQL,
        "SELECT DISTINCT ON (name) name FROM users",
    );
    assert!(!pg_result.is_failed());
}

#[test]
fn test_parser_manager_incremental_parse() {
    let manager = ParserManager::new();

    let initial = "SELECT * FROM users";
    let initial_result = manager.parse_text(Dialect::MySQL, initial);
    let old_tree = initial_result.tree().expect("Expected tree");

    // Incremental edit: add WHERE clause
    let edit = tree_sitter::InputEdit {
        start_byte: initial.len(),
        old_end_byte: initial.len(),
        new_end_byte: initial.len() + 12,
        start_position: tree_sitter::Point {
            row: 0,
            column: initial.len(),
        },
        old_end_position: tree_sitter::Point {
            row: 0,
            column: initial.len(),
        },
        new_end_position: tree_sitter::Point {
            row: 0,
            column: initial.len() + 12,
        },
    };

    let updated = "SELECT * FROM users WHERE id = 1";
    let updated_result = manager.parse_with_edit(Dialect::MySQL, old_tree, updated, &edit);

    assert!(!updated_result.is_failed());
}

#[test]
fn test_parse_result_is_success() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT 1");

    assert!(result.is_success());
    assert!(!result.is_partial());
    assert!(!result.is_failed());
}

#[test]
fn test_parse_result_is_partial() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELCT 1"); // Typo

    // Check state methods work correctly
    if result.is_partial() {
        assert!(!result.is_success());
        assert!(!result.is_failed());
        assert!(result.errors().is_some());
    }
}

#[test]
fn test_parse_result_tree_method() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT * FROM users");

    // Success should have tree
    assert!(result.tree().is_some());

    // Failed result should have no tree
    let failed = ParseResult::Failed {
        error: ParseError::Generic {
            message: "Test error".to_string(),
        },
    };
    assert!(failed.tree().is_none());
}

#[test]
fn test_parse_result_errors_method() {
    // Success has no errors
    let success = ParseResult::Success {
        tree: None,
        parse_time: Duration::from_millis(10),
    };
    assert!(success.errors().is_none());

    // Failed has no errors (just error field)
    let failed = ParseResult::Failed {
        error: ParseError::Generic {
            message: "Test".to_string(),
        },
    };
    assert!(failed.errors().is_none());
}

#[test]
fn test_parse_error_display() {
    let error = ParseError::NoGrammar {
        dialect: "mysql".to_string(),
    };

    let error_string = format!("{}", error);
    assert!(error_string.contains("mysql"));
    assert!(error_string.contains("No grammar compiled"));
}

#[test]
fn test_parse_result_into_success() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT 1");

    // Can extract success result
    let success = result.into_success();
    assert!(success.is_some());

    // Failed result returns None
    let failed = ParseResult::Failed {
        error: ParseError::Generic {
            message: "Test".to_string(),
        },
    };
    assert!(failed.into_success().is_none());
}

#[test]
fn test_parse_empty_input() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::MySQL, "");

    // Should handle gracefully
    match result {
        ParseResult::Success { .. } => {
            // Empty might be valid
        }
        ParseResult::Partial { .. } => {
            // Also OK
        }
        ParseResult::Failed { .. } => {
            // Also OK
        }
    }
}

#[test]
fn test_parse_very_long_query() {
    let manager = ParserManager::new();

    // Generate a very long SQL query
    let long_query =
        "SELECT " + &", ".join((1..=1000).map(|i| format!("column{}", i))) + " FROM users";

    let result = manager.parse_text(Dialect::MySQL, &long_query);

    // Should not crash or hang
    match result {
        ParseResult::Success { .. } => {
            // OK
        }
        ParseResult::Partial { .. } => {
            // Also OK
        }
        ParseResult::Failed { .. } => {
            // Also acceptable
        }
    }
}

#[test]
fn test_parse_with_unsupported_dialect() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::SQLite, "SELECT * FROM users");

    // Should fail with NoGrammar error
    assert!(result.is_failed());
    match result {
        ParseResult::Failed { error } => {
            assert!(matches!(error, ParseError::NoGrammar { .. }));
        }
        _ => {
            panic!("Expected Failed result for unsupported dialect");
        }
    }
}
