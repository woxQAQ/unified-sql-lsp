// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Error types for semantic analysis
//!
//! This module defines error types used throughout the semantic analysis layer.

use thiserror::Error;

/// Result type alias for semantic operations
pub type SemanticResult<T> = Result<T, SemanticError>;

/// Errors that can occur during semantic analysis
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SemanticError {
    /// Table not found in the current scope or any parent scopes
    #[error("Table not found in scope: {0}")]
    TableNotFound(String),

    /// Column not found in any visible table
    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    /// Column reference is ambiguous (found in multiple tables)
    #[error("Ambiguous column reference: {0} (found in {1:?})")]
    AmbiguousColumn(String, Vec<String>),

    /// Duplicate table alias in the same scope
    #[error("Duplicate table alias: {0}")]
    DuplicateAlias(String),

    /// Invalid scope reference (e.g., non-existent parent)
    #[error("Invalid scope reference: {0}")]
    InvalidScope(String),

    /// HAVING clause contains non-aggregate column without GROUP BY
    #[error("HAVING clause cannot contain non-aggregate column '{0}' without GROUP BY")]
    NonAggregateColumnInHaving(String),

    /// Wildcard table not found
    #[error("Wildcard table '{0}' not found in FROM clause")]
    WildcardTableNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_table_not_found() {
        let err = SemanticError::TableNotFound("users".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("users"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_error_display_column_not_found() {
        let err = SemanticError::ColumnNotFound("email".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("email"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_error_display_ambiguous_column() {
        let err = SemanticError::AmbiguousColumn(
            "id".to_string(),
            vec!["users".to_string(), "orders".to_string()],
        );
        let msg = format!("{}", err);
        assert!(msg.contains("id"));
        assert!(msg.contains("Ambiguous"));
        assert!(msg.contains("users"));
        assert!(msg.contains("orders"));
    }

    #[test]
    fn test_error_display_duplicate_alias() {
        let err = SemanticError::DuplicateAlias("u".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("u"));
        assert!(msg.contains("Duplicate"));
    }

    #[test]
    fn test_error_display_non_aggregate_column_in_having() {
        let err = SemanticError::NonAggregateColumnInHaving("user_id".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("user_id"));
        assert!(msg.contains("HAVING"));
        assert!(msg.contains("GROUP BY"));
    }

    #[test]
    fn test_error_display_wildcard_table_not_found() {
        let err = SemanticError::WildcardTableNotFound("users".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("users"));
        assert!(msg.contains("not found"));
        assert!(msg.contains("FROM"));
    }
}
