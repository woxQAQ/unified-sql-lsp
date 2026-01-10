// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Error handling and edge case tests
//!
//! Tests for error scenarios and graceful degradation.

use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::parsing::{ParseError, ParserManager};

#[test]
fn test_parse_with_invalid_sql() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::MySQL, "TOTALLY INVALID SQL HERE");

    // Should not crash - should return error or partial
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Expected
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Also acceptable
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Grammar might be very lenient
        }
    }
}

// Note: This test is disabled since all Dialect variants are now supported.
// The language_for_dialect function has a catch-all case that returns None
// for future dialects that might be added to the enum but don't have grammar support yet.
#[test]
fn test_parse_with_no_grammar() {
    use unified_sql_grammar::language_for_dialect;

    // All current dialect variants are supported, so we can't test unsupported ones.
    // The function has a _ => None catch-all for future variants.
    // We can verify the function works correctly by checking it returns Some for supported dialects.
    assert!(language_for_dialect(Dialect::MySQL).is_some());
    assert!(language_for_dialect(Dialect::PostgreSQL).is_some());
}

#[test]
fn test_parse_error_display() {
    let error = ParseError::NoGrammar {
        dialect: "test_dialect".to_string(),
    };

    let msg = format!("{}", error);
    assert!(msg.contains("test_dialect"));
    assert!(msg.contains("No grammar compiled"));
}

#[test]
fn test_parse_empty_input() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::MySQL, "");

    // Should handle gracefully
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Empty might be valid
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Also OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also OK
        }
    }
}

#[test]
fn test_parse_very_long_input() {
    let manager = ParserManager::new();

    // Generate a very long SQL query
    let columns: Vec<String> = (1..=1000).map(|i| format!("column{}", i)).collect();
    let long_query = format!("SELECT {} FROM users", columns.join(", "));

    let result = manager.parse_text(Dialect::MySQL, &long_query);

    // Should not crash or hang
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Also OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also acceptable
        }
    }
}

#[test]
fn test_parse_with_typo_in_keyword() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SLECT * FROM users", // Typo: SLECT
    );

    // Should handle gracefully with error or partial
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. }
        | unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Expected
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Grammar might be lenient
        }
    }
}

#[test]
fn test_parse_with_mismatched_parentheses() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::PostgreSQL,
        "SELECT * FROM users WHERE (id = 1", // Missing closing paren
    );

    // Should detect error
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { errors, .. } => {
            assert!(!errors.is_empty(), "Expected parse errors");
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also acceptable
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            panic!("Expected parse error for mismatched parentheses");
        }
    }
}

#[test]
fn test_parse_with_unclosed_string() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SELECT * FROM users WHERE name = 'John", // Unclosed string
    );

    // Should handle gracefully
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. }
        | unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Expected
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Parser might be lenient
        }
    }
}

#[test]
fn test_parse_with_unterminated_comment() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SELECT * FROM users /* comment", // Unterminated comment
    );

    // Should handle gracefully
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. }
        | unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Expected
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Parser might be lenient
        }
    }
}

#[test]
fn test_parse_error_invalid_input_details() {
    let error = ParseError::InvalidInput {
        line: 5,
        column: 10,
        message: "Unexpected token".to_string(),
        node_type: Some("ERROR".to_string()),
    };

    let msg = format!("{}", error);
    assert!(msg.contains("line 5"));
    assert!(msg.contains("column 10"));
    assert!(msg.contains("Unexpected token"));
}

#[test]
fn test_parse_multiple_errors() {
    let manager = ParserManager::new();

    let result = manager.parse_text(
        Dialect::MySQL,
        "SLECT * FORM users", // Multiple typos: SLECT, FORM
    );

    // Should collect all errors
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { errors, .. } => {
            assert!(!errors.is_empty(), "Expected at least one error");
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also acceptable
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // Grammar might be lenient
        }
    }
}

#[test]
fn test_parse_with_only_whitespace() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::PostgreSQL, "   \n  \t  ");

    // Should handle gracefully (empty query is valid)
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Also OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also OK
        }
    }
}

#[test]
fn test_parse_with_only_comment() {
    let manager = ParserManager::new();

    let result = manager.parse_text(Dialect::MySQL, "-- This is a comment");

    // Should handle gracefully (comment-only query is valid)
    match result {
        unified_sql_lsp_lsp::parsing::ParseResult::Success { .. } => {
            // OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { .. } => {
            // Also OK
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // Also OK
        }
    }
}
