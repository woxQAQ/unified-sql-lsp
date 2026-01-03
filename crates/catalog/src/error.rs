// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Error types for Catalog operations
//!
//! This module defines the error types used throughout the catalog layer.

use serde::Serialize;
use thiserror::Error;

/// Result type alias for Catalog operations
pub type CatalogResult<T> = Result<T, CatalogError>;

/// Errors that can occur during Catalog operations
#[derive(Debug, Error, Clone, Serialize)]
pub enum CatalogError {
    /// Failed to connect to the database
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    /// Query execution timed out
    #[error("Query timed out after {0}s")]
    QueryTimeout(u64),

    /// Requested table was not found
    #[error("Table '{0}' not found in schema '{1}'")]
    TableNotFound(String, String),

    /// Invalid schema name provided
    #[error("Invalid schema name: {0}")]
    InvalidSchema(String),

    /// Failed to serialize or deserialize schema data
    #[error("Failed to serialize schema data: {0}")]
    SerializationError(String),

    /// Invalid catalog configuration
    #[error("Invalid catalog configuration: {0}")]
    ConfigurationError(String),

    /// Permission denied for requested operation
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// The specified feature is not supported by this catalog implementation
    #[error("Feature not supported: {0}")]
    NotSupported(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_connection_failed() {
        let err = CatalogError::ConnectionFailed("connection refused".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("connection refused"));
        assert!(msg.contains("Failed to connect"));
    }

    #[test]
    fn test_error_display_query_timeout() {
        let err = CatalogError::QueryTimeout(5);
        let msg = format!("{}", err);
        assert!(msg.contains("5"));
        assert!(msg.contains("timed out"));
    }

    #[test]
    fn test_error_display_table_not_found() {
        let err = CatalogError::TableNotFound("users".to_string(), "public".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("users"));
        assert!(msg.contains("public"));
        assert!(msg.contains("not found"));
    }

    #[test]
    fn test_error_display_invalid_schema() {
        let err = CatalogError::InvalidSchema("123_invalid".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("123_invalid"));
        assert!(msg.contains("Invalid schema"));
    }

    #[test]
    fn test_error_display_configuration_error() {
        let err = CatalogError::ConfigurationError("missing host".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("missing host"));
        assert!(msg.contains("configuration"));
    }

    #[test]
    fn test_error_display_permission_denied() {
        let err = CatalogError::PermissionDenied("access denied".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("access denied"));
        assert!(msg.contains("Permission denied"));
    }

    #[test]
    fn test_error_display_not_supported() {
        let err = CatalogError::NotSupported("batch queries".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("batch queries"));
        assert!(msg.contains("not supported"));
    }

    #[test]
    fn test_error_serialization() {
        let err = CatalogError::TableNotFound("test".to_string(), "schema".to_string());
        // Verify it can be serialized to JSON
        let json = serde_json::to_string(&err);
        assert!(json.is_ok());
    }
}
