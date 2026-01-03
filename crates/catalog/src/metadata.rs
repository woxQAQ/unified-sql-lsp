// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Metadata types for database schema information
//!
//! This module defines the types used to represent database schema metadata,
//! including tables, columns, and functions.

use serde::{Deserialize, Serialize};

/// SQL data types (unified across dialects)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DataType {
    // Numeric types
    Integer,
    BigInt,
    SmallInt,
    TinyInt,
    Decimal,
    Float,
    Double,

    // String types
    Varchar(Option<usize>),
    Char(Option<usize>),
    Text,

    // Binary types
    Binary,
    VarBinary(Option<usize>),
    Blob,

    // Date/Time types
    Date,
    Time,
    DateTime,
    Timestamp,

    // Boolean
    Boolean,

    // JSON
    Json,

    // Special types
    Uuid,
    Enum(Vec<String>),
    Array(Box<DataType>),

    // Unknown/Other (with original type name)
    Other(String),
}

/// Table type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TableType {
    Table,
    View,
    MaterializedView,
    Temporary,
    System,
}

/// Reference to a table (for foreign keys)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableReference {
    pub table: String,
    pub column: String,
}

/// Metadata for a database column
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnMetadata {
    /// Column name
    pub name: String,
    /// Data type
    pub data_type: DataType,
    /// Whether the column is nullable
    pub nullable: bool,
    /// Default value (as SQL expression string)
    pub default_value: Option<String>,
    /// Column comment/description
    pub comment: Option<String>,
    /// Whether this is a primary key column
    pub is_primary_key: bool,
    /// Whether this is a foreign key column
    pub is_foreign_key: bool,
    /// Referenced table (if foreign key)
    pub references: Option<TableReference>,
}

impl ColumnMetadata {
    /// Create a new column metadata with builder pattern
    pub fn new(name: impl Into<String>, data_type: DataType) -> Self {
        Self {
            name: name.into(),
            data_type,
            nullable: false,
            default_value: None,
            comment: None,
            is_primary_key: false,
            is_foreign_key: false,
            references: None,
        }
    }

    /// Builder method: set nullable
    pub fn with_nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Builder method: set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default_value = Some(default.into());
        self
    }

    /// Builder method: set comment
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Builder method: mark as primary key
    pub fn with_primary_key(mut self) -> Self {
        self.is_primary_key = true;
        self
    }

    /// Builder method: set foreign key reference
    pub fn with_foreign_key(mut self, table: impl Into<String>, column: impl Into<String>) -> Self {
        self.is_foreign_key = true;
        self.references = Some(TableReference {
            table: table.into(),
            column: column.into(),
        });
        self
    }
}

/// Metadata for a database table
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableMetadata {
    /// Table name
    pub name: String,
    /// Schema/database name
    pub schema: String,
    /// Column definitions
    pub columns: Vec<ColumnMetadata>,
    /// Estimated row count (for query planning)
    pub row_count_estimate: Option<u64>,
    /// Table comment/description
    pub comment: Option<String>,
    /// Table type (TABLE, VIEW, MATERIALIZED VIEW, etc.)
    pub table_type: TableType,
}

impl TableMetadata {
    /// Create new table metadata with builder pattern
    pub fn new(name: impl Into<String>, schema: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            schema: schema.into(),
            columns: Vec::new(),
            row_count_estimate: None,
            comment: None,
            table_type: TableType::Table,
        }
    }

    /// Builder method: add columns
    pub fn with_columns(mut self, columns: Vec<ColumnMetadata>) -> Self {
        self.columns = columns;
        self
    }

    /// Builder method: set row count estimate
    pub fn with_row_count(mut self, count: u64) -> Self {
        self.row_count_estimate = Some(count);
        self
    }

    /// Builder method: set comment
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Builder method: set table type
    pub fn with_type(mut self, table_type: TableType) -> Self {
        self.table_type = table_type;
        self
    }

    /// Get column by name
    pub fn get_column(&self, name: &str) -> Option<&ColumnMetadata> {
        self.columns.iter().find(|c| c.name == name)
    }

    /// Get primary key columns
    pub fn primary_keys(&self) -> Vec<&ColumnMetadata> {
        self.columns.iter().filter(|c| c.is_primary_key).collect()
    }
}

/// Function parameter definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionParameter {
    /// Parameter name
    pub name: String,
    /// Parameter data type
    pub data_type: DataType,
    /// Whether the parameter has a default value
    pub has_default: bool,
    /// Whether this is a variadic parameter (e.g., VARIADIC in PostgreSQL)
    pub is_variadic: bool,
}

/// Function classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FunctionType {
    Scalar,
    Aggregate,
    Window,
    Table,
}

/// Metadata for a database function
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FunctionMetadata {
    /// Function name
    pub name: String,
    /// Return type
    pub return_type: DataType,
    /// Function parameters
    pub parameters: Vec<FunctionParameter>,
    /// Function type (scalar, aggregate, window)
    pub function_type: FunctionType,
    /// Function description/documentation
    pub description: Option<String>,
    /// Example usage
    pub example: Option<String>,
    /// Whether this is a built-in function
    pub is_builtin: bool,
}

impl FunctionMetadata {
    /// Create new function metadata with builder pattern
    pub fn new(name: impl Into<String>, return_type: DataType) -> Self {
        Self {
            name: name.into(),
            return_type,
            parameters: Vec::new(),
            function_type: FunctionType::Scalar,
            description: None,
            example: None,
            is_builtin: true,
        }
    }

    /// Builder method: add parameters
    pub fn with_parameters(mut self, params: Vec<FunctionParameter>) -> Self {
        self.parameters = params;
        self
    }

    /// Builder method: set function type
    pub fn with_type(mut self, function_type: FunctionType) -> Self {
        self.function_type = function_type;
        self
    }

    /// Builder method: set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Builder method: set example
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Get function signature (for display in completion)
    pub fn signature(&self) -> String {
        let params: Vec<String> = self
            .parameters
            .iter()
            .map(|p| format!("{} {:?}", p.name, p.data_type))
            .collect();
        format!("{}({}) -> {:?}", self.name, params.join(", "), self.return_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_get_column() {
        let col1 = ColumnMetadata::new("id", DataType::Integer);
        let col2 = ColumnMetadata::new("name", DataType::Text);

        let table = TableMetadata::new("users", "public")
            .with_columns(vec![col1, col2]);

        assert!(table.get_column("id").is_some());
        assert!(table.get_column("name").is_some());
        assert!(table.get_column("email").is_none());
    }

    #[test]
    fn test_table_primary_keys() {
        let col1 = ColumnMetadata::new("id", DataType::Integer).with_primary_key();
        let col2 = ColumnMetadata::new("user_id", DataType::Integer).with_primary_key();
        let col3 = ColumnMetadata::new("name", DataType::Text);

        let table = TableMetadata::new("user_roles", "public")
            .with_columns(vec![col1, col2, col3]);

        let pks = table.primary_keys();
        assert_eq!(pks.len(), 2);
        assert_eq!(pks[0].name, "id");
        assert_eq!(pks[1].name, "user_id");
    }

    #[test]
    fn test_function_metadata_signature() {
        let func = FunctionMetadata::new("abs", DataType::Integer);
        let signature = func.signature();
        assert!(signature.contains("abs"));
        assert!(signature.contains("Integer"));
    }

    #[test]
    fn test_data_type_equality() {
        assert_eq!(DataType::Integer, DataType::Integer);
        assert_eq!(DataType::Varchar(Some(255)), DataType::Varchar(Some(255)));
        assert_ne!(DataType::Varchar(Some(100)), DataType::Varchar(Some(255)));
    }

    #[test]
    fn test_data_type_serialization() {
        let dt = DataType::Varchar(Some(255));
        let json = serde_json::to_string(&dt).unwrap();
        let deserialized: DataType = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, deserialized);
    }

    #[test]
    fn test_json_roundtrip() {
        let col = ColumnMetadata::new("data", DataType::Json)
            .with_nullable(true)
            .with_comment("JSON payload");

        let json = serde_json::to_string(&col).unwrap();
        let deserialized: ColumnMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(col, deserialized);
    }

    #[test]
    fn test_table_type_variants() {
        assert_eq!(TableType::Table, TableType::Table);
        assert_eq!(TableType::View, TableType::View);
        assert_ne!(TableType::Table, TableType::View);
    }

    #[test]
    fn test_function_type_variants() {
        assert_eq!(FunctionType::Scalar, FunctionType::Scalar);
        assert_eq!(FunctionType::Aggregate, FunctionType::Aggregate);
        assert_ne!(FunctionType::Scalar, FunctionType::Aggregate);
    }

    #[test]
    fn test_complex_data_type() {
        let array_type = DataType::Array(Box::new(DataType::Integer));
        assert!(matches!(array_type, DataType::Array(_)));

        let json = serde_json::to_string(&array_type).unwrap();
        let deserialized: DataType = serde_json::from_str(&json).unwrap();
        assert_eq!(array_type, deserialized);
    }
}
