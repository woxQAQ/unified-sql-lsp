// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Symbol types for semantic analysis
//!
//! This module defines symbol types representing tables and columns in SQL queries.

use serde::{Deserialize, Serialize};
use unified_sql_lsp_catalog::DataType;

/// Represents a table symbol in a SQL query
///
/// A table symbol can be the actual table name or an alias (e.g., "u" in "FROM users u").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableSymbol {
    /// The actual table name in the database
    pub table_name: String,

    /// Optional alias for the table (e.g., "u" for "FROM users u")
    pub alias: Option<String>,

    /// Columns available from this table
    pub columns: Vec<ColumnSymbol>,
}

impl TableSymbol {
    /// Create a new table symbol
    ///
    /// # Arguments
    ///
    /// * `table_name` - The actual table name in the database
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::TableSymbol;
    ///
    /// let table = TableSymbol::new("users");
    /// assert_eq!(table.table_name, "users");
    /// ```
    pub fn new(table_name: impl Into<String>) -> Self {
        Self {
            table_name: table_name.into(),
            alias: None,
            columns: Vec::new(),
        }
    }

    /// Set an alias for this table
    ///
    /// # Arguments
    ///
    /// * `alias` - The alias to use for this table
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::TableSymbol;
    ///
    /// let table = TableSymbol::new("users").with_alias("u");
    /// assert_eq!(table.alias, Some("u".to_string()));
    /// ```
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Set the columns for this table
    ///
    /// # Arguments
    ///
    /// * `columns` - Vector of column symbols
    pub fn with_columns(mut self, columns: Vec<ColumnSymbol>) -> Self {
        self.columns = columns;
        self
    }

    /// Check if this table matches the given name (by table_name or alias)
    ///
    /// # Arguments
    ///
    /// * `name` - The name to check against
    ///
    /// # Returns
    ///
    /// `true` if the name matches either the table_name or alias
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::TableSymbol;
    ///
    /// let table = TableSymbol::new("users").with_alias("u");
    /// assert!(table.matches("users"));
    /// assert!(table.matches("u"));
    /// assert!(!table.matches("orders"));
    /// ```
    pub fn matches(&self, name: &str) -> bool {
        self.table_name == name || self.alias.as_deref() == Some(name)
    }

    /// Get the display name (alias if present, otherwise table_name)
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::TableSymbol;
    ///
    /// let table1 = TableSymbol::new("users");
    /// assert_eq!(table1.display_name(), "users");
    ///
    /// let table2 = TableSymbol::new("users").with_alias("u");
    /// assert_eq!(table2.display_name(), "u");
    /// ```
    pub fn display_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.table_name)
    }

    /// Find a column by name in this table
    ///
    /// # Arguments
    ///
    /// * `name` - The column name to find
    ///
    /// # Returns
    ///
    /// `Some(&ColumnSymbol)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::{TableSymbol, ColumnSymbol};
    /// use unified_sql_lsp_catalog::DataType;
    ///
    /// let table = TableSymbol::new("users")
    ///     .with_columns(vec![
    ///         ColumnSymbol::new("id", DataType::Integer, "users"),
    ///         ColumnSymbol::new("name", DataType::Text, "users"),
    ///     ]);
    ///
    /// assert!(table.find_column("id").is_some());
    /// assert!(table.find_column("email").is_none());
    /// ```
    pub fn find_column(&self, name: &str) -> Option<&ColumnSymbol> {
        self.columns.iter().find(|c| c.name == name)
    }
}

/// Represents a column symbol in a SQL query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnSymbol {
    /// Column name
    pub name: String,

    /// Data type of the column
    pub data_type: DataType,

    /// Table this column belongs to
    pub table_name: String,

    /// Whether this column is a primary key
    #[serde(default)]
    pub is_primary_key: bool,

    /// Whether this column is a foreign key
    #[serde(default)]
    pub is_foreign_key: bool,
}

impl ColumnSymbol {
    /// Create a new column symbol
    ///
    /// # Arguments
    ///
    /// * `name` - Column name
    /// * `data_type` - Data type of the column
    /// * `table_name` - Table this column belongs to
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::ColumnSymbol;
    /// use unified_sql_lsp_catalog::DataType;
    ///
    /// let column = ColumnSymbol::new("id", DataType::Integer, "users");
    /// assert_eq!(column.name, "id");
    /// assert_eq!(column.table_name, "users");
    /// ```
    pub fn new(
        name: impl Into<String>,
        data_type: DataType,
        table_name: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            data_type,
            table_name: table_name.into(),
            is_primary_key: false,
            is_foreign_key: false,
        }
    }

    /// Mark this column as a primary key
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::ColumnSymbol;
    /// use unified_sql_lsp_catalog::DataType;
    ///
    /// let column = ColumnSymbol::new("id", DataType::Integer, "users")
    ///     .with_primary_key();
    /// assert!(column.is_primary_key);
    /// ```
    pub fn with_primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    /// Mark this column as a foreign key
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::ColumnSymbol;
    /// use unified_sql_lsp_catalog::DataType;
    ///
    /// let column = ColumnSymbol::new("user_id", DataType::Integer, "orders")
    ///     .with_foreign_key();
    /// assert!(column.is_foreign_key);
    /// ```
    pub fn with_foreign_key(mut self) -> Self {
        self.is_foreign_key = true;
        self
    }
}

