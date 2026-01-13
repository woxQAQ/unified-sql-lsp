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
    #[error(
        "Set operation column count mismatch: LEFT has {left} columns, RIGHT has {right} columns"
    )]
    SetOperationColumnCountMismatch { left: usize, right: usize },

    /// Recursive CTE depth limit exceeded
    #[error("Recursive CTE depth limit ({limit}) exceeded")]
    RecursiveCteDepthLimit { limit: usize },

    /// Function argument count mismatch
    #[error("Function '{function}' expects {expected} argument(s) but found {found}")]
    FunctionArgumentCountMismatch {
        function: String,
        expected: usize,
        found: usize,
    },

    /// FILTER clause used on non-aggregate function
    #[error("FILTER clause can only be used with aggregate functions, not '{function}'")]
    FilterOnNonAggregateFunction { function: String },

    /// OVER clause used on non-window function
    #[error("OVER clause can only be used with window functions, not '{function}'")]
    OverClauseOnNonWindowFunction { function: String },

    /// Invalid window frame specification
    #[error("Invalid window frame: {reason}")]
    InvalidWindowFrame { reason: String },
}
