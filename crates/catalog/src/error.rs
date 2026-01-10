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

    /// Query execution failed
    #[error("Query execution failed: {0}")]
    QueryFailed(String),

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

