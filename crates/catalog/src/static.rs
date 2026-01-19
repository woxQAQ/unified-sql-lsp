// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Static Catalog
//!
//! This module provides a static catalog implementation that uses predefined schema data.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_catalog::StaticCatalog;
//!
//! let catalog = StaticCatalog::new();
//! let tables = catalog.list_tables().await?;
//! ```

use async_trait::async_trait;
use std::collections::HashMap;

use crate::metadata::{ColumnMetadata, DataType, FunctionMetadata, TableMetadata, TableType};
use crate::{Catalog, CatalogError, CatalogResult};

/// Static catalog with predefined schema data
///
/// This is used for the playground and testing without requiring a real database.
pub struct StaticCatalog {
    tables: HashMap<String, TableMetadata>,
}

impl StaticCatalog {
    /// Create a new static catalog with default playground schema
    pub fn new() -> Self {
        let mut tables = HashMap::new();

        // Users table
        tables.insert(
            "users".to_string(),
            TableMetadata {
                schema: "playground".to_string(),
                name: "users".to_string(),
                table_type: TableType::Table,
                columns: vec![
                    ColumnMetadata {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: true,
                        is_foreign_key: false,
                        default_value: Some("AUTO_INCREMENT".to_string()),
                        comment: None,
                        references: None,
                    },
                    ColumnMetadata {
                        name: "name".to_string(),
                        data_type: DataType::Varchar(Some(100)),
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("User name".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "email".to_string(),
                        data_type: DataType::Varchar(Some(255)),
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("User email address".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "created_at".to_string(),
                        data_type: DataType::Timestamp,
                        nullable: true,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: Some("CURRENT_TIMESTAMP".to_string()),
                        comment: Some("Account creation timestamp".to_string()),
                        references: None,
                    },
                ],
                row_count_estimate: Some(3),
                comment: Some("User accounts table".to_string()),
            },
        );

        // Orders table
        tables.insert(
            "orders".to_string(),
            TableMetadata {
                schema: "playground".to_string(),
                name: "orders".to_string(),
                table_type: TableType::Table,
                columns: vec![
                    ColumnMetadata {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: true,
                        is_foreign_key: false,
                        default_value: Some("AUTO_INCREMENT".to_string()),
                        comment: None,
                        references: None,
                    },
                    ColumnMetadata {
                        name: "user_id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: true,
                        default_value: None,
                        comment: Some("Foreign key to users.id".to_string()),
                        references: Some(unified_sql_lsp_ir::TableReference {
                            table: "users".to_string(),
                            column: "id".to_string(),
                        }),
                    },
                    ColumnMetadata {
                        name: "total".to_string(),
                        data_type: DataType::Decimal,
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("Order total amount".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "status".to_string(),
                        data_type: DataType::Varchar(Some(20)),
                        nullable: true,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: Some("'pending'".to_string()),
                        comment: Some("Order status".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "created_at".to_string(),
                        data_type: DataType::Timestamp,
                        nullable: true,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: Some("CURRENT_TIMESTAMP".to_string()),
                        comment: Some("Order creation timestamp".to_string()),
                        references: None,
                    },
                ],
                row_count_estimate: Some(3),
                comment: Some("Customer orders table".to_string()),
            },
        );

        // Order items table
        tables.insert(
            "order_items".to_string(),
            TableMetadata {
                schema: "playground".to_string(),
                name: "order_items".to_string(),
                table_type: TableType::Table,
                columns: vec![
                    ColumnMetadata {
                        name: "id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: true,
                        is_foreign_key: false,
                        default_value: Some("AUTO_INCREMENT".to_string()),
                        comment: None,
                        references: None,
                    },
                    ColumnMetadata {
                        name: "order_id".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: true,
                        default_value: None,
                        comment: Some("Foreign key to orders.id".to_string()),
                        references: Some(unified_sql_lsp_ir::TableReference {
                            table: "orders".to_string(),
                            column: "id".to_string(),
                        }),
                    },
                    ColumnMetadata {
                        name: "product_name".to_string(),
                        data_type: DataType::Varchar(Some(255)),
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("Product name".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "quantity".to_string(),
                        data_type: DataType::Integer,
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("Item quantity".to_string()),
                        references: None,
                    },
                    ColumnMetadata {
                        name: "price".to_string(),
                        data_type: DataType::Decimal,
                        nullable: false,
                        is_primary_key: false,
                        is_foreign_key: false,
                        default_value: None,
                        comment: Some("Item price".to_string()),
                        references: None,
                    },
                ],
                row_count_estimate: Some(4),
                comment: Some("Order line items table".to_string()),
            },
        );

        Self { tables }
    }

    /// Load static catalog from SQL file
    ///
    /// For now, this just returns the default catalog.
    /// TODO: Parse the SQL file and extract schema information.
    pub fn from_file(_path: &str) -> Result<Self, CatalogError> {
        // For now, just return the default catalog
        // A full implementation would parse the SQL file
        Ok(Self::new())
    }
}

impl Default for StaticCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Catalog for StaticCatalog {
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
        Ok(self.tables.values().cloned().collect())
    }

    async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
        self.tables
            .get(table)
            .map(|t| t.columns.clone())
            .ok_or_else(|| {
                CatalogError::TableNotFound(
                    format!("Table '{}' not found in static catalog", table),
                    "playground".to_string(),
                )
            })
    }

    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
        // Return empty list for now
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_static_catalog_new() {
        let catalog = StaticCatalog::new();
        let tables = catalog.list_tables().await.unwrap();
        assert_eq!(tables.len(), 3);
    }

    #[tokio::test]
    async fn test_static_catalog_get_columns() {
        let catalog = StaticCatalog::new();
        let columns = catalog.get_columns("users").await.unwrap();
        assert_eq!(columns.len(), 4);
        assert_eq!(columns[0].name, "id");
        assert_eq!(columns[1].name, "name");
    }

    #[tokio::test]
    async fn test_static_catalog_table_not_found() {
        let catalog = StaticCatalog::new();
        let result = catalog.get_columns("nonexistent").await;
        assert!(matches!(result, Err(CatalogError::TableNotFound(_, _))));
    }

    #[tokio::test]
    async fn test_static_catalog_from_file() {
        let catalog = StaticCatalog::from_file("test.sql").unwrap();
        let tables = catalog.list_tables().await.unwrap();
        assert_eq!(tables.len(), 3);
    }
}
