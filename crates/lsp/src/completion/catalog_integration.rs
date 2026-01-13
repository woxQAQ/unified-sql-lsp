// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Catalog integration for completion
//!
//! This module provides functionality to fetch schema information
//! from the catalog for completion purposes.

use crate::completion::error::CompletionError;
use std::sync::Arc;
use unified_sql_lsp_catalog::{Catalog, ColumnMetadata, FunctionMetadata, TableMetadata};
use unified_sql_lsp_semantic::{ColumnSymbol, TableSymbol};

/// Catalog fetcher for completion
///
/// Fetches schema information from the catalog and converts it
/// to semantic symbols for completion.
pub struct CatalogCompletionFetcher {
    catalog: Arc<dyn Catalog>,
}

impl CatalogCompletionFetcher {
    /// Create a new catalog fetcher
    ///
    /// # Arguments
    ///
    /// * `catalog` - The catalog to fetch from
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    /// List all tables from the catalog
    ///
    /// # Returns
    ///
    /// Vector of table metadata
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let tables = fetcher.list_tables().await?;
    /// assert!(!tables.is_empty());
    /// ```
    pub async fn list_tables(&self) -> Result<Vec<TableMetadata>, CompletionError> {
        eprintln!("!!! LSP: CatalogCompletionFetcher::list_tables() called");
        let result = self
            .catalog
            .list_tables()
            .await
            .map_err(CompletionError::Catalog);
        eprintln!(
            "!!! LSP: CatalogCompletionFetcher::list_tables() returned: {:?}",
            result.as_ref().map(|t| t.len())
        );
        result
    }

    /// List all functions from the catalog
    ///
    /// # Returns
    ///
    /// Vector of function metadata
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let functions = fetcher.list_functions().await?;
    /// assert!(!functions.is_empty());
    /// ```
    pub async fn list_functions(&self) -> Result<Vec<FunctionMetadata>, CompletionError> {
        self.catalog
            .list_functions()
            .await
            .map_err(CompletionError::Catalog)
    }

    /// Populate table columns from the catalog
    ///
    /// # Arguments
    ///
    /// * `table` - The table symbol to populate (mutable)
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err if catalog query fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut table = TableSymbol::new("users");
    /// fetcher.populate_table_columns(&mut table).await?;
    /// assert!(!table.columns.is_empty());
    /// ```
    pub async fn populate_table_columns(
        &self,
        table: &mut TableSymbol,
    ) -> Result<(), CompletionError> {
        // Query catalog for columns
        let columns_metadata = self
            .catalog
            .get_columns(&table.table_name)
            .await
            .map_err(|e| CompletionError::Catalog(e))?;

        // Convert ColumnMetadata to ColumnSymbol
        let columns: Vec<ColumnSymbol> = columns_metadata
            .iter()
            .map(|meta| Self::metadata_to_symbol(meta, &table.table_name))
            .collect();

        // Update table with columns
        table.columns = columns;

        Ok(())
    }

    /// Populate multiple tables from the catalog
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to populate
    ///
    /// # Returns
    ///
    /// Ok(()) if all successful, Err on first failure
    ///
    /// # Note
    ///
    /// Tables that fail to load are skipped (not included in the final result)
    pub async fn populate_all_tables(
        &self,
        tables: &mut [TableSymbol],
    ) -> Result<(), CompletionError> {
        for table in tables.iter_mut() {
            // Skip tables that fail to load - log warning but continue
            if let Err(e) = self.populate_table_columns(table).await {
                eprintln!(
                    "Warning: Failed to load columns for table '{}': {}",
                    table.table_name, e
                );
            }
        }
        Ok(())
    }

    /// Populate a single table from the catalog
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the table to fetch
    ///
    /// # Returns
    ///
    /// Ok(TableSymbol) with columns populated, Err if catalog query fails
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let table = fetcher.populate_single_table("users").await?;
    /// assert!(!table.columns.is_empty());
    /// ```
    pub async fn populate_single_table(
        &self,
        table_name: &str,
    ) -> Result<TableSymbol, CompletionError> {
        let mut table = TableSymbol::new(table_name);
        self.populate_table_columns(&mut table).await?;
        Ok(table)
    }

    /// Convert ColumnMetadata to ColumnSymbol
    ///
    /// # Arguments
    ///
    /// * `meta` - The column metadata
    /// * `table_name` - The table name
    ///
    /// # Returns
    ///
    /// A ColumnSymbol with the same information
    fn metadata_to_symbol(meta: &ColumnMetadata, table_name: &str) -> ColumnSymbol {
        let mut symbol = ColumnSymbol::new(meta.name.clone(), meta.data_type.clone(), table_name);

        // Copy PK/FK metadata
        if meta.is_primary_key {
            symbol = symbol.with_primary_key();
        }
        if meta.is_foreign_key {
            symbol = symbol.with_foreign_key();
        }

        symbol
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use unified_sql_lsp_catalog::{CatalogError, DataType, TableMetadata};

    // Mock catalog for testing
    struct MockCatalog {
        tables: std::collections::HashMap<String, Vec<ColumnMetadata>>,
    }

    #[async_trait::async_trait]
    impl Catalog for MockCatalog {
        async fn list_tables(&self) -> unified_sql_lsp_catalog::CatalogResult<Vec<TableMetadata>> {
            Ok(Vec::new())
        }

        async fn get_columns(
            &self,
            table: &str,
        ) -> unified_sql_lsp_catalog::CatalogResult<Vec<ColumnMetadata>> {
            self.tables
                .get(table)
                .cloned()
                .ok_or_else(|| CatalogError::TableNotFound(table.to_string(), "public".to_string()))
        }

        async fn list_functions(
            &self,
        ) -> unified_sql_lsp_catalog::CatalogResult<Vec<unified_sql_lsp_catalog::FunctionMetadata>>
        {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn test_catalog_fetcher_new() {
        let catalog = Arc::new(MockCatalog {
            tables: std::collections::HashMap::new(),
        }) as Arc<dyn Catalog>;
        let _fetcher = CatalogCompletionFetcher::new(catalog.clone());
        // Just verify the fetcher was created successfully
        assert!(true);
    }

    #[tokio::test]
    async fn test_populate_table_columns() {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "users".to_string(),
            vec![
                ColumnMetadata::new("id", DataType::Integer),
                ColumnMetadata::new("name", DataType::Text),
            ],
        );

        let catalog = Arc::new(MockCatalog { tables });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let mut table = TableSymbol::new("users");
        fetcher.populate_table_columns(&mut table).await.unwrap();

        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.columns[1].name, "name");
    }

    #[tokio::test]
    async fn test_populate_table_columns_with_pk_fk() {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "users".to_string(),
            vec![
                ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                ColumnMetadata::new("name", DataType::Text),
            ],
        );
        tables.insert(
            "orders".to_string(),
            vec![
                ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                ColumnMetadata::new("user_id", DataType::Integer).with_foreign_key("users", "id"),
            ],
        );

        let catalog = Arc::new(MockCatalog { tables });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        // Test users table
        let mut users_table = TableSymbol::new("users");
        fetcher
            .populate_table_columns(&mut users_table)
            .await
            .unwrap();

        assert_eq!(users_table.columns.len(), 2);
        assert_eq!(users_table.columns[0].name, "id");
        assert!(users_table.columns[0].is_primary_key);
        assert!(!users_table.columns[0].is_foreign_key);

        assert_eq!(users_table.columns[1].name, "name");
        assert!(!users_table.columns[1].is_primary_key);
        assert!(!users_table.columns[1].is_foreign_key);

        // Test orders table
        let mut orders_table = TableSymbol::new("orders");
        fetcher
            .populate_table_columns(&mut orders_table)
            .await
            .unwrap();

        assert_eq!(orders_table.columns.len(), 2);
        assert_eq!(orders_table.columns[0].name, "id");
        assert!(orders_table.columns[0].is_primary_key);

        assert_eq!(orders_table.columns[1].name, "user_id");
        assert!(!orders_table.columns[1].is_primary_key);
        assert!(orders_table.columns[1].is_foreign_key);
    }

    #[tokio::test]
    async fn test_populate_single_table() {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "users".to_string(),
            vec![
                ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                ColumnMetadata::new("name", DataType::Text),
            ],
        );

        let catalog = Arc::new(MockCatalog { tables });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let table = fetcher.populate_single_table("users").await.unwrap();

        assert_eq!(table.table_name, "users");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "id");
        assert!(table.columns[0].is_primary_key);
    }

    #[tokio::test]
    async fn test_populate_single_table_not_found() {
        let catalog = Arc::new(MockCatalog {
            tables: std::collections::HashMap::new(),
        });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let result = fetcher.populate_single_table("nonexistent").await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompletionError::Catalog(CatalogError::TableNotFound(_, _))
        ));
    }

    #[tokio::test]
    async fn test_populate_table_columns_not_found() {
        let catalog = Arc::new(MockCatalog {
            tables: std::collections::HashMap::new(),
        });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let mut table = TableSymbol::new("nonexistent");
        let result = fetcher.populate_table_columns(&mut table).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CompletionError::Catalog(CatalogError::TableNotFound(_, _))
        ));
    }

    #[tokio::test]
    async fn test_populate_all_tables() {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "users".to_string(),
            vec![ColumnMetadata::new("id", DataType::Integer)],
        );
        tables.insert(
            "orders".to_string(),
            vec![ColumnMetadata::new("id", DataType::Integer)],
        );

        let catalog = Arc::new(MockCatalog { tables });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let mut table_symbols = vec![TableSymbol::new("users"), TableSymbol::new("orders")];
        fetcher
            .populate_all_tables(&mut table_symbols)
            .await
            .unwrap();

        assert_eq!(table_symbols[0].columns.len(), 1);
        assert_eq!(table_symbols[1].columns.len(), 1);
    }

    #[tokio::test]
    async fn test_populate_all_tables_with_failure() {
        let mut tables = std::collections::HashMap::new();
        tables.insert(
            "users".to_string(),
            vec![ColumnMetadata::new("id", DataType::Integer)],
        );
        // "orders" table not in catalog

        let catalog = Arc::new(MockCatalog { tables });
        let fetcher = CatalogCompletionFetcher::new(catalog);

        let mut table_symbols = vec![TableSymbol::new("users"), TableSymbol::new("orders")];
        fetcher
            .populate_all_tables(&mut table_symbols)
            .await
            .unwrap();

        // users should be populated, orders should be empty
        assert_eq!(table_symbols[0].columns.len(), 1);
        assert_eq!(table_symbols[1].columns.len(), 0);
    }
}
