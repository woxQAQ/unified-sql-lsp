// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Mock catalog implementation for testing
//!
//! Provides an in-memory catalog with builder pattern for easy test setup

use unified_sql_lsp_catalog::{
    Catalog, CatalogError, CatalogResult, ColumnMetadata, DataType, FunctionMetadata,
    FunctionType, TableMetadata, TableType,
};
use std::collections::HashMap;

/// In-memory mock catalog for testing
#[derive(Debug, Clone)]
pub struct MockCatalog {
    tables: HashMap<String, TableMetadata>,
    functions: Vec<FunctionMetadata>,
}

impl Default for MockCatalog {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCatalog {
    /// Create a new empty mock catalog
    pub fn new() -> Self {
        Self {
            tables: HashMap::new(),
            functions: Vec::new(),
        }
    }

    /// Add a table to the catalog
    pub fn add_table(mut self, table: TableMetadata) -> Self {
        let key = format!("{}.{}", table.schema, table.name);
        self.tables.insert(key, table);
        self
    }

    /// Add a function to the catalog
    pub fn add_function(mut self, function: FunctionMetadata) -> Self {
        self.functions.push(function);
        self
    }

    /// Get a table by name (any schema)
    pub fn get_table(&self, name: &str) -> Option<&TableMetadata> {
        self.tables.values().find(|t| t.name == name)
    }
}

#[async_trait::async_trait]
impl Catalog for MockCatalog {
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
        Ok(self.tables.values().cloned().collect())
    }

    async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
        if let Some(table_metadata) = self.get_table(table) {
            Ok(table_metadata.columns.clone())
        } else {
            Err(CatalogError::TableNotFound(table.to_string(), "mock".to_string()))
        }
    }

    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
        Ok(self.functions.clone())
    }
}

/// Builder for creating mock catalogs with a fluent API
pub struct MockCatalogBuilder {
    catalog: MockCatalog,
}

impl Default for MockCatalogBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockCatalogBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            catalog: MockCatalog::new(),
        }
    }

    /// Add the standard test schema (users and orders tables)
    pub fn with_standard_schema(mut self) -> Self {
        self.catalog = self
            .catalog
            .add_table(
                TableMetadata::new("users", "myapp")
                    .with_columns(vec![
                        ColumnMetadata::new("id", DataType::BigInt)
                            .with_nullable(false)
                            .with_primary_key(),
                        ColumnMetadata::new("email", DataType::Varchar(Some(255)))
                            .with_nullable(false),
                        ColumnMetadata::new("name", DataType::Varchar(Some(100)))
                            .with_nullable(true),
                        ColumnMetadata::new("created_at", DataType::Timestamp)
                            .with_nullable(true),
                    ])
                    .with_row_count(50000)
                    .with_comment("User account information"),
            )
            .add_table(
                TableMetadata::new("orders", "myapp")
                    .with_columns(vec![
                        ColumnMetadata::new("id", DataType::BigInt)
                            .with_nullable(false)
                            .with_primary_key(),
                        ColumnMetadata::new("user_id", DataType::BigInt)
                            .with_nullable(false)
                            .with_foreign_key("users", "id"),
                        ColumnMetadata::new("total", DataType::Decimal).with_nullable(true),
                        ColumnMetadata::new("status", DataType::Varchar(Some(50)))
                            .with_nullable(false),
                        ColumnMetadata::new("created_at", DataType::Timestamp)
                            .with_nullable(true),
                    ])
                    .with_row_count(100000)
                    .with_type(TableType::Table),
            )
            .add_table(
                TableMetadata::new("products", "myapp")
                    .with_columns(vec![
                        ColumnMetadata::new("id", DataType::BigInt)
                            .with_nullable(false)
                            .with_primary_key(),
                        ColumnMetadata::new("name", DataType::Varchar(Some(255)))
                            .with_nullable(false),
                        ColumnMetadata::new("price", DataType::Decimal).with_nullable(false),
                        ColumnMetadata::new("stock", DataType::Integer).with_nullable(true),
                    ])
                    .with_row_count(10000),
            )
            .add_function(
                FunctionMetadata::new("count", DataType::BigInt)
                    .with_type(FunctionType::Aggregate)
                    .with_description("Count rows")
                    .with_example("SELECT COUNT(*) FROM users"),
            )
            .add_function(
                FunctionMetadata::new("sum", DataType::Decimal)
                    .with_type(FunctionType::Aggregate)
                    .with_description("Calculate sum"),
            )
            .add_function(
                FunctionMetadata::new("avg", DataType::Decimal)
                    .with_type(FunctionType::Aggregate)
                    .with_description("Calculate average"),
            )
            .add_function(
                FunctionMetadata::new("min", DataType::Decimal)
                    .with_type(FunctionType::Aggregate)
                    .with_description("Find minimum value"),
            )
            .add_function(
                FunctionMetadata::new("max", DataType::Decimal)
                    .with_type(FunctionType::Aggregate)
                    .with_description("Find maximum value"),
            )
            .add_function(
                FunctionMetadata::new("abs", DataType::Integer)
                    .with_type(FunctionType::Scalar)
                    .with_description("Absolute value"),
            )
            .add_function(
                FunctionMetadata::new("upper", DataType::Varchar(None))
                    .with_type(FunctionType::Scalar)
                    .with_description("Convert to uppercase"),
            )
            .add_function(
                FunctionMetadata::new("lower", DataType::Varchar(None))
                    .with_type(FunctionType::Scalar)
                    .with_description("Convert to lowercase"),
            )
            .add_function(
                FunctionMetadata::new("row_number", DataType::BigInt)
                    .with_type(FunctionType::Window)
                    .with_description("Row number within partition"),
            )
            .add_function(
                FunctionMetadata::new("rank", DataType::BigInt)
                    .with_type(FunctionType::Window)
                    .with_description("Rank within partition"),
            );

        self
    }

    /// Add a custom table
    pub fn with_table(mut self, table: TableMetadata) -> Self {
        self.catalog = self.catalog.add_table(table);
        self
    }

    /// Add a custom function
    pub fn with_function(mut self, function: FunctionMetadata) -> Self {
        self.catalog = self.catalog.add_function(function);
        self
    }

    /// Build the mock catalog
    pub fn build(self) -> MockCatalog {
        self.catalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_catalog_list_tables() {
        let catalog = MockCatalogBuilder::new()
            .with_standard_schema()
            .build();

        let tables = catalog.list_tables().await.unwrap();
        assert_eq!(tables.len(), 3);

        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        assert!(table_names.contains(&"users"));
        assert!(table_names.contains(&"orders"));
        assert!(table_names.contains(&"products"));
    }

    #[tokio::test]
    async fn test_mock_catalog_get_columns() {
        let catalog = MockCatalogBuilder::new()
            .with_standard_schema()
            .build();

        let columns = catalog.get_columns("users").await.unwrap();
        assert_eq!(columns.len(), 4);

        let column_names: Vec<&str> = columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"email"));
        assert!(column_names.contains(&"name"));
        assert!(column_names.contains(&"created_at"));
    }

    #[tokio::test]
    async fn test_mock_catalog_list_functions() {
        let catalog = MockCatalogBuilder::new()
            .with_standard_schema()
            .build();

        let functions = catalog.list_functions().await.unwrap();
        assert!(!functions.is_empty());

        let has_count = functions.iter().any(|f| f.name == "count");
        assert!(has_count);
    }
}
