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

    /// Circular CTE dependency detected
    #[error("Circular CTE dependency detected: {0}")]
    CircularCteDependency(String),

    /// CTE column count mismatch
    #[error("CTE '{cte}' defines {defined} columns but query returns {returned} columns")]
    CteColumnCountMismatch {
        cte: String,
        defined: usize,
        returned: usize,
    },

    /// CTE name conflict with real table
    #[error("CTE name '{name}' conflicts with table in catalog")]
    CteNameConflict { name: String },

    /// Set operation column count mismatch
    #[error("Set operation column count mismatch: LEFT has {left} columns, RIGHT has {right} columns")]
    SetOperationColumnCountMismatch { left: usize, right: usize },

    /// Recursive CTE depth limit exceeded
    #[error("Recursive CTE depth limit ({limit}) exceeded")]
    RecursiveCteDepthLimit { limit: usize },
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

    #[test]
    fn test_error_display_circular_cte_dependency() {
        let err = SemanticError::CircularCteDependency("cte_a → cte_b → cte_a".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Circular"));
        assert!(msg.contains("CTE"));
        assert!(msg.contains("cte_a"));
    }

    #[test]
    fn test_error_display_cte_column_count_mismatch() {
        let err = SemanticError::CteColumnCountMismatch {
            cte: "my_cte".to_string(),
            defined: 2,
            returned: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("my_cte"));
        assert!(msg.contains("2"));
        assert!(msg.contains("3"));
    }

    #[test]
    fn test_error_display_cte_name_conflict() {
        let err = SemanticError::CteNameConflict {
            name: "users".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("users"));
        assert!(msg.contains("conflicts"));
    }

    #[test]
    fn test_error_display_set_operation_column_count_mismatch() {
        let err = SemanticError::SetOperationColumnCountMismatch {
            left: 2,
            right: 3,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("2"));
        assert!(msg.contains("3"));
        assert!(msg.contains("mismatch"));
    }

    #[test]
    fn test_error_display_recursive_cte_depth_limit() {
        let err = SemanticError::RecursiveCteDepthLimit { limit: 100 };
        let msg = format!("{}", err);
        assert!(msg.contains("100"));
        assert!(msg.contains("exceeded"));
    }
}
