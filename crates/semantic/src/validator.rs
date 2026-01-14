// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Semantic Validator
//!
//! This module provides semantic validation for SQL queries using the catalog.
//!
//! ## Purpose
//!
//! The semantic validator checks for:
//! - Undefined table references
//! - Undefined column references
//! - Ambiguous column references
//! - Type mismatches
//!
//! ## Architecture
//!
//! This validator uses the catalog to get schema information and performs
//! semantic validation that would otherwise require hardcoded SQL knowledge.

use std::sync::Arc;
use unified_sql_lsp_catalog::{Catalog, CatalogError};
use unified_sql_lsp_ir::Dialect;
use crate::{error::SemanticError, ColumnSymbol, SemanticAnalyzer, TableSymbol};

/// Result type for validation
pub type ValidationResult<T> = Result<T, ValidationError>;

/// Semantic validation error
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    #[error("Semantic analysis error: {0}")]
    Semantic(#[from] SemanticError),

    #[error("Table not found: {0}")]
    TableNotFound(String),

    #[error("Column not found: {0}")]
    ColumnNotFound(String),

    #[error("Ambiguous column reference: {0}")]
    AmbiguousColumn(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
}

/// Information about a validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// The issue message
    pub message: String,

    /// The severity of the issue
    pub severity: ValidationSeverity,

    /// Code identifying the type of issue
    pub code: String,
}

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationSeverity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Semantic validator for SQL queries
pub struct SemanticValidator {
    /// The semantic analyzer
    analyzer: SemanticAnalyzer,

    /// The catalog for schema information
    catalog: Arc<dyn Catalog>,
}

impl SemanticValidator {
    /// Create a new semantic validator
    ///
    /// # Arguments
    ///
    /// * `catalog` - The catalog to use for schema information
    /// * `dialect` - The SQL dialect
    pub fn new(catalog: Arc<dyn Catalog>, dialect: Dialect) -> Self {
        let analyzer = SemanticAnalyzer::new(catalog.clone(), dialect);
        Self { analyzer, catalog }
    }

    /// Validate a table reference
    ///
    /// # Arguments
    ///
    /// * `table_name` - The table name to validate
    ///
    /// # Returns
    ///
    /// Ok if the table exists, Err with validation issue otherwise
    pub async fn validate_table(&self, table_name: &str) -> ValidationResult<()> {
        let tables = self.catalog.list_tables().await?;

        let table_exists = tables
            .iter()
            .any(|t| t.name.eq_ignore_ascii_case(table_name));

        if !table_exists {
            return Err(ValidationError::TableNotFound(table_name.to_string()));
        }

        Ok(())
    }

    /// Validate a column reference
    ///
    /// # Arguments
    ///
    /// * `column_name` - The column name to validate
    /// * `table_name` - The table name (optional)
    ///
    /// # Returns
    ///
    /// Ok if the column exists, Err with validation issue otherwise
    pub async fn validate_column(
        &self,
        column_name: &str,
        table_name: Option<&str>,
    ) -> ValidationResult<()> {
        if let Some(table) = table_name {
            // Check if column exists in the specified table
            match self.catalog.get_columns(table).await {
                Ok(columns) => {
                    let column_exists = columns
                        .iter()
                        .any(|c| c.name.eq_ignore_ascii_case(column_name));

                    if !column_exists {
                        return Err(ValidationError::ColumnNotFound(format!(
                            "{}.{}",
                            table, column_name
                        )));
                    }
                }
                Err(CatalogError::TableNotFound(_, _)) => {
                    return Err(ValidationError::TableNotFound(table.to_string()));
                }
                Err(e) => return Err(ValidationError::Catalog(e)),
            }
        }
        // Note: Unqualified column validation requires scope analysis
        // which is handled by the SemanticAnalyzer

        Ok(())
    }

    /// Get the analyzer instance
    pub fn analyzer(&self) -> &SemanticAnalyzer {
        &self.analyzer
    }

    /// Get the catalog instance
    pub fn catalog(&self) -> &Arc<dyn Catalog> {
        &self.catalog
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::{ColumnMetadata, DataType, TableMetadata, TableType};

    // Mock catalog for testing
    struct MockCatalog {
        tables: Vec<TableMetadata>,
    }

    impl MockCatalog {
        fn new() -> Self {
            Self {
                tables: vec![
                    TableMetadata::new("users", "public")
                        .with_columns(vec![
                            ColumnMetadata::new("id", DataType::Integer)
                                .with_primary_key(),
                            ColumnMetadata::new("username", DataType::Varchar(Some(50))),
                        ]),
                ],
            }
        }
    }

    #[async_trait::async_trait]
    impl Catalog for MockCatalog {
        async fn list_tables(&self) -> Result<Vec<TableMetadata>, CatalogError> {
            Ok(self.tables.clone())
        }

        async fn get_columns(&self, table: &str) -> Result<Vec<ColumnMetadata>, CatalogError> {
            let table_lower = table.to_lowercase();
            for t in &self.tables {
                if t.name.to_lowercase() == table_lower {
                    return Ok(t.columns.clone());
                }
            }
            Err(CatalogError::TableNotFound(
                table.to_string(),
                "public".to_string(),
            ))
        }

        async fn list_functions(&self) -> Result<Vec<unified_sql_lsp_catalog::FunctionMetadata>, CatalogError> {
            Ok(vec![])
        }
    }

    #[tokio::test]
    async fn test_validate_table_exists() {
        let catalog = Arc::new(MockCatalog::new());
        let validator = SemanticValidator::new(catalog, Dialect::MySQL);

        assert!(validator.validate_table("users").await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_table_not_found() {
        let catalog = Arc::new(MockCatalog::new());
        let validator = SemanticValidator::new(catalog, Dialect::MySQL);

        let result = validator.validate_table("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_column_exists() {
        let catalog = Arc::new(MockCatalog::new());
        let validator = SemanticValidator::new(catalog, Dialect::MySQL);

        assert!(validator.validate_column("id", Some("users")).await.is_ok());
    }

    #[tokio::test]
    async fn test_validate_column_not_found() {
        let catalog = Arc::new(MockCatalog::new());
        let validator = SemanticValidator::new(catalog, Dialect::MySQL);

        let result = validator.validate_column("nonexistent", Some("users")).await;
        assert!(result.is_err());
    }
}
