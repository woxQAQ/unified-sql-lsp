// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Hover Information Provider
//!
//! This module provides hover information for SQL functions and columns.
//!
//! ## Examples
//!
//! ```rust,ignore
//! use unified_sql_lsp_function_registry::HoverInfoProvider;
//!
//! let provider = HoverInfoProvider::new();
//! let info = provider.get_function_hover("COUNT", &Dialect::MySQL);
//! assert!(info.is_some());
//! ```

use crate::{Dialect, FunctionRegistry};
use unified_sql_lsp_ir::DataType;

/// Column metadata needed for hover information
pub struct ColumnHoverInfo {
    pub name: String,
    pub data_type: DataType,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
}

/// Hover information provider for SQL completion
///
/// Provides hover text for:
/// - SQL functions (e.g., COUNT, SUM, AVG)
/// - Columns (with type information)
pub struct HoverInfoProvider {
    function_registry: FunctionRegistry,
}

impl HoverInfoProvider {
    /// Create a new hover info provider
    pub fn new() -> Self {
        Self {
            function_registry: FunctionRegistry::new(),
        }
    }

    /// Get hover information for a SQL function
    ///
    /// # Arguments
    ///
    /// * `name` - Function name (case-insensitive)
    /// * `dialect` - SQL dialect
    ///
    /// # Returns
    ///
    /// Markdown-formatted hover text, or None if function not found
    ///
    /// # Examples
    ///
    /// ```
    /// # use unified_sql_lsp_function_registry::HoverInfoProvider;
    /// # use unified_sql_lsp_ir::Dialect;
    /// let provider = HoverInfoProvider::new();
    /// let info = provider.get_function_hover("COUNT", &Dialect::MySQL);
    /// assert!(info.is_some());
    /// ```
    pub fn get_function_hover(&self, name: &str, dialect: &Dialect) -> Option<String> {
        let functions = self.function_registry.get_functions(*dialect);
        let func = functions
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))?;

        let desc = func.description.as_ref().map(|s| s.as_str()).unwrap_or("SQL function");
        let params: Vec<String> = func.parameters.iter().map(|p| p.name.clone()).collect();
        let params_str = if params.is_empty() {
            String::new()
        } else {
            format!("({})", params.join(", "))
        };

        Some(format!(
            "```sql\n{}{}{}\n```\n\n{}",
            func.name, params_str,
            if func.return_type == DataType::Other("void".to_string()) { "" } else { " -> ..." },
            desc
        ))
    }

    /// Get hover information for a column
    ///
    /// # Arguments
    ///
    /// * `column_info` - Column metadata
    ///
    /// # Returns
    ///
    /// Markdown-formatted hover text with column type
    pub fn get_column_hover(&self, column_info: &ColumnHoverInfo) -> String {
        let mut detail = format!(
            "```sql\n{}\n```\n\nColumn type: {}",
            column_info.name,
            self.format_data_type(&column_info.data_type)
        );

        if column_info.is_primary_key {
            detail.push_str("\n\n**Primary Key**");
        }
        if column_info.is_foreign_key {
            detail.push_str("\n\n**Foreign Key**");
        }

        detail
    }

    /// Format a data type for display
    fn format_data_type(&self, data_type: &DataType) -> String {
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
            DataType::Array(inner) => format!("{}[]", self.format_data_type(inner)),
            DataType::Other(name) => format!("Other({})", name),
            _ => "Unknown".to_string(),
        }
    }

    /// Get hover information for a table alias
    ///
    /// # Arguments
    ///
    /// * `alias` - Table alias
    ///
    /// # Returns
    ///
    /// Markdown-formatted hover text indicating it's a table alias
    pub fn get_table_alias_hover(&self, alias: &str) -> String {
        format!("```sql\n{}\n```\n\nTable alias", alias)
    }

    /// Get hover information for a table
    ///
    /// # Arguments
    ///
    /// * `table_name` - Table name
    ///
    /// # Returns
    ///
    /// Markdown-formatted hover text indicating it's a table
    pub fn get_table_hover(&self, table_name: &str) -> String {
        format!("```sql\n{}\n```\n\nTable", table_name)
    }

    /// Check if a word is likely a SQL function
    ///
    /// # Arguments
    ///
    /// * `word` - Word to check
    /// * `dialect` - SQL dialect
    ///
    /// # Returns
    ///
    /// true if the word matches a known function name
    pub fn is_function(&self, word: &str, dialect: &Dialect) -> bool {
        let functions = self.function_registry.get_functions(*dialect);
        functions
            .iter()
            .any(|f| f.name.eq_ignore_ascii_case(word))
    }
}

impl Default for HoverInfoProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_ir::Dialect;

    #[test]
    fn test_get_function_hover() {
        let provider = HoverInfoProvider::new();
        let info = provider.get_function_hover("COUNT", &Dialect::MySQL);
        assert!(info.is_some());
        assert!(info.unwrap().contains("COUNT"));
    }

    #[test]
    fn test_is_function() {
        let provider = HoverInfoProvider::new();
        assert!(provider.is_function("COUNT", &Dialect::MySQL));
        assert!(provider.is_function("SUM", &Dialect::MySQL));
        assert!(!provider.is_function("NOTAREALFUNCTION", &Dialect::MySQL));
    }

    #[test]
    fn test_format_data_type() {
        let provider = HoverInfoProvider::new();
        assert_eq!(provider.format_data_type(&DataType::Integer), "Integer");
        assert_eq!(provider.format_data_type(&DataType::Text), "Text");
        assert_eq!(
            provider.format_data_type(&DataType::Varchar(Some(255))),
            "VarChar(255)"
        );
    }

    #[test]
    fn test_table_alias_hover() {
        let provider = HoverInfoProvider::new();
        let info = provider.get_table_alias_hover("u");
        assert!(info.contains("Table alias"));
    }

    #[test]
    fn test_column_hover() {
        let provider = HoverInfoProvider::new();
        let column_info = ColumnHoverInfo {
            name: "id".to_string(),
            data_type: DataType::Integer,
            is_primary_key: true,
            is_foreign_key: false,
        };
        let info = provider.get_column_hover(&column_info);
        assert!(info.contains("id"));
        assert!(info.contains("Integer"));
        assert!(info.contains("Primary Key"));
    }
}
