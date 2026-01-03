// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion error types
//!
//! This module defines error types for the completion system.

use tower_lsp::lsp_types::Position;
use unified_sql_lsp_catalog::CatalogError;
use unified_sql_lsp_semantic::SemanticError;

/// Errors that can occur during completion
#[derive(Debug, thiserror::Error)]
pub enum CompletionError {
    /// Document has not been parsed yet
    #[error("Document not parsed")]
    NotParsed,

    /// Invalid position for completion
    #[error("Invalid position: {0:?}")]
    InvalidPosition(Position),

    /// Catalog-related error
    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    /// Scope building error
    #[error("Scope build error: {0}")]
    ScopeBuild(String),

    /// No FROM clause found in SELECT statement
    #[error("No FROM clause found")]
    NoFromClause,

    /// Semantic analysis error
    #[error("Semantic error: {0}")]
    Semantic(#[from] SemanticError),

    /// Context detection error
    #[error("Context detection error: {0}")]
    ContextDetection(String),

    /// Unknown or unsupported SQL construct
    #[error("Unknown SQL construct: {0}")]
    UnknownConstruct(String),
}

impl CompletionError {
    /// Check if this error should result in an empty completion list
    /// (vs. propagating the error to the client)
    pub fn should_return_empty(&self) -> bool {
        matches!(
            self,
            CompletionError::NotParsed
                | CompletionError::InvalidPosition(_)
                | CompletionError::NoFromClause
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_error_display() {
        let err = CompletionError::NotParsed;
        assert_eq!(err.to_string(), "Document not parsed");

        let pos = Position::new(0, 0);
        let err = CompletionError::InvalidPosition(pos);
        assert!(err.to_string().contains("Invalid position"));

        let err = CompletionError::NoFromClause;
        assert_eq!(err.to_string(), "No FROM clause found");
    }

    #[test]
    fn test_should_return_empty() {
        assert!(CompletionError::NotParsed.should_return_empty());
        assert!(CompletionError::InvalidPosition(Position::new(0, 0)).should_return_empty());
        assert!(CompletionError::NoFromClause.should_return_empty());

        // Catalog errors should propagate
        let catalog_err = CatalogError::TableNotFound("test".to_string(), "public".to_string());
        assert!(!CompletionError::Catalog(catalog_err).should_return_empty());
    }
}
