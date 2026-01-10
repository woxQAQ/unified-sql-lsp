// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Catalog trait for database schema abstraction
//!
//! This module defines the async Catalog trait used for querying database schema information.

use crate::error::CatalogResult;
use crate::metadata::{ColumnMetadata, FunctionMetadata, TableMetadata};

/// Catalog trait for database schema abstraction
///
/// This trait provides an async interface for querying database schema information.
/// Implementations can connect to live databases, read from static files, or use caches.
///
/// # Examples
///
/// ```rust,ignore
/// use unified_sql_lsp_catalog::{Catalog, CatalogError};
///
/// async fn list_user_tables(catalog: &impl Catalog) -> Result<Vec<String>, CatalogError> {
///     let tables = catalog.list_tables().await?;
///     Ok(tables.into_iter()
///         .filter(|t| t.schema == "users")
///         .map(|t| t.name)
///         .collect())
/// }
/// ```
#[async_trait::async_trait]
pub trait Catalog: Send + Sync {
    /// List all tables in the database
    ///
    /// Returns metadata for all tables accessible to the current connection.
    /// This includes base tables, views, and materialized views.
    ///
    /// # Returns
    ///
    /// A vector of `TableMetadata` containing table information.
    ///
    /// # Errors
    ///
    /// Returns `CatalogError::ConnectionFailed` if database connection fails.
    /// Returns `CatalogError::QueryTimeout` if the query exceeds timeout.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let tables = catalog.list_tables().await?;
    /// for table in tables {
    ///     println!("{}.{}", table.schema, table.name);
    /// }
    /// ```
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>>;

    /// Get column metadata for a specific table
    ///
    /// # Arguments
    ///
    /// * `table` - Table name (may include schema qualifier like "schema.table")
    ///
    /// # Returns
    ///
    /// A vector of `ColumnMetadata` containing column information.
    ///
    /// # Errors
    ///
    /// Returns `CatalogError::TableNotFound` if the table doesn't exist.
    /// Returns `CatalogError::PermissionDenied` if access is denied.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let columns = catalog.get_columns("users").await?;
    /// for column in columns {
    ///     println!("{}: {:?}", column.name, column.data_type);
    /// }
    /// ```
    async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>>;

    /// List all available functions
    ///
    /// Returns metadata for all functions available in the database,
    /// including both built-in and user-defined functions.
    ///
    /// # Returns
    ///
    /// A vector of `FunctionMetadata` containing function information.
    ///
    /// # Errors
    ///
    /// Returns `CatalogError::ConnectionFailed` if database connection fails.
    /// Returns `CatalogError::QueryTimeout` if the query exceeds timeout.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let functions = catalog.list_functions().await?;
    /// let aggregate_funcs: Vec<_> = functions.into_iter()
    ///     .filter(|f| matches!(f.function_type, FunctionType::Aggregate))
    ///     .collect();
    /// ```
    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>>;
}

