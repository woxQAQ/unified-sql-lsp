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
/// // TODO: (CATALOG-001) Implement actual catalog to make this example testable
/// // This example requires a working catalog implementation
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
    /// // TODO: (CATALOG-001) Implement actual catalog to make this example testable
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
    /// // TODO: (CATALOG-001) Implement actual catalog to make this example testable
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
    /// // TODO: (CATALOG-001) Implement actual catalog to make this example testable
    /// let functions = catalog.list_functions().await?;
    /// let aggregate_funcs: Vec<_> = functions.into_iter()
    ///     .filter(|f| matches!(f.function_type, FunctionType::Aggregate))
    ///     .collect();
    /// ```
    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CatalogError,
        metadata::{ColumnMetadata, DataType, FunctionMetadata, FunctionType, TableMetadata},
    };

    // Mock implementation for testing
    struct MockCatalog;

    #[async_trait::async_trait]
    impl Catalog for MockCatalog {
        async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
            Ok(vec![
                TableMetadata::new("users", "public")
                    .with_columns(vec![
                        ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                        ColumnMetadata::new("name", DataType::Text),
                    ])
                    .with_row_count(100),
            ])
        }

        async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
            if table == "users" {
                Ok(vec![
                    ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                    ColumnMetadata::new("name", DataType::Text),
                ])
            } else {
                Err(CatalogError::TableNotFound(
                    table.to_string(),
                    "public".to_string(),
                ))
            }
        }

        async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
            Ok(vec![
                FunctionMetadata::new("count", DataType::BigInt).with_type(FunctionType::Aggregate),
                FunctionMetadata::new("abs", DataType::Integer).with_type(FunctionType::Scalar),
            ])
        }
    }

    #[tokio::test]
    async fn test_mock_catalog_list_tables() {
        let catalog = MockCatalog;
        let tables = catalog.list_tables().await.unwrap();
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");
    }

    #[tokio::test]
    async fn test_mock_catalog_get_columns() {
        let catalog = MockCatalog;
        let columns = catalog.get_columns("users").await.unwrap();
        assert_eq!(columns.len(), 2);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[1].name, "name");
    }

    #[tokio::test]
    async fn test_mock_catalog_get_columns_not_found() {
        let catalog = MockCatalog;
        let result = catalog.get_columns("nonexistent").await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CatalogError::TableNotFound(_, _)
        ));
    }

    #[tokio::test]
    async fn test_mock_catalog_list_functions() {
        let catalog = MockCatalog;
        let functions = catalog.list_functions().await.unwrap();
        assert_eq!(functions.len(), 2);
        assert_eq!(functions[0].name, "count");
        assert_eq!(functions[1].name, "abs");
    }
}
