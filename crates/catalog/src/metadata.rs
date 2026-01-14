// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Metadata types for database schema information
//!
//! This module re-exports metadata types from the `unified-sql-lsp-ir` crate.
//! These types define the schema information for tables, columns, and functions.

// Re-export all metadata types from the ir crate
pub use unified_sql_lsp_ir::{
    ColumnMetadata, DataType, FunctionMetadata, FunctionParameter, FunctionType, TableMetadata,
    TableReference, TableType,
};

/// Format a DataType to a display string
///
/// Converts the DataType enum to a human-readable string representation.
/// This is useful for displaying types in completion items, hover information,
/// and diagnostics.
///
/// # Examples
///
/// ```
/// use unified_sql_lsp_catalog::DataType;
/// use unified_sql_lsp_catalog::format_data_type;
///
/// assert_eq!(format_data_type(&DataType::Integer), "Integer");
/// assert_eq!(format_data_type(&DataType::Varchar(Some(255))), "VarChar(255)");
/// assert_eq!(format_data_type(&DataType::Varchar(None)), "VarChar");
/// ```
pub fn format_data_type(data_type: &DataType) -> String {
    match data_type {
        DataType::Integer => "Integer".to_string(),
        DataType::BigInt => "BigInt".to_string(),
        DataType::SmallInt => "SmallInt".to_string(),
        DataType::TinyInt => "TinyInt".to_string(),
        DataType::Decimal => "Decimal".to_string(),
        DataType::Float => "Float".to_string(),
        DataType::Double => "Double".to_string(),
        DataType::Text => "Text".to_string(),
        DataType::Varchar(length) => {
            if let Some(len) = length {
                format!("VarChar({})", len)
            } else {
                "VarChar".to_string()
            }
        }
        DataType::Char(length) => {
            if let Some(len) = length {
                format!("Char({})", len)
            } else {
                "Char".to_string()
            }
        }
        DataType::Binary => "Binary".to_string(),
        DataType::VarBinary(length) => {
            if let Some(len) = length {
                format!("VarBinary({})", len)
            } else {
                "VarBinary".to_string()
            }
        }
        DataType::Blob => "Blob".to_string(),
        DataType::Date => "Date".to_string(),
        DataType::Time => "Time".to_string(),
        DataType::DateTime => "DateTime".to_string(),
        DataType::Timestamp => "Timestamp".to_string(),
        DataType::Boolean => "Boolean".to_string(),
        DataType::Json => "JSON".to_string(),
        DataType::Uuid => "UUID".to_string(),
        DataType::Enum(values) => format!("Enum({})", values.join(", ")),
        DataType::Array(inner) => format!("{}[]", format_data_type(inner)),
        DataType::Other(name) => format!("Other({})", name),
        _ => "Unknown".to_string(),
    }
}

