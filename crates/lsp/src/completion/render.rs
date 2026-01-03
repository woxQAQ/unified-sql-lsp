// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion rendering
//!
//! This module provides functionality to render LSP completion items
//! from semantic symbols.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation};
use unified_sql_lsp_catalog::DataType;
use unified_sql_lsp_semantic::{ColumnSymbol, TableSymbol};

/// Completion renderer
///
/// Converts semantic symbols to LSP CompletionItem representations.
pub struct CompletionRenderer;

impl CompletionRenderer {
    /// Render column completion items
    ///
    /// # Arguments
    ///
    /// * `tables` - Tables with their columns
    /// * `force_qualifier` - Whether to force table qualifier (e.g., "users.id")
    ///
    /// # Returns
    ///
    /// Vector of completion items
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let items = CompletionRenderer::render_columns(&tables, false);
    /// assert!(items.iter().any(|i| i.label == "id"));
    /// ```
    pub fn render_columns(tables: &[TableSymbol], force_qualifier: bool) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Add wildcard (*) completion
        items.push(Self::wildcard_item());

        // Group columns by name to detect ambiguity
        let mut column_map: std::collections::HashMap<String, Vec<&ColumnSymbol>> =
            std::collections::HashMap::new();

        for table in tables {
            for column in &table.columns {
                column_map
                    .entry(column.name.clone())
                    .or_insert_with(Vec::new)
                    .push(column);
            }
        }

        // Generate completion items
        for table in tables {
            for column in &table.columns {
                // Check if column is ambiguous
                let is_ambiguous = column_map[&column.name].len() > 1;

                // Force qualifier if ambiguous or explicitly requested
                let needs_qualifier = force_qualifier || is_ambiguous;

                items.push(Self::column_item(column, table, needs_qualifier));
            }
        }

        items
    }

    /// Render a single column completion item
    ///
    /// # Arguments
    ///
    /// * `column` - The column symbol
    /// * `table` - The table symbol
    /// * `qualified` - Whether to use qualified name (e.g., "users.id")
    fn column_item(column: &ColumnSymbol, table: &TableSymbol, qualified: bool) -> CompletionItem {
        let label = if qualified {
            format!("{}.{}", table.display_name(), column.name)
        } else {
            column.name.clone()
        };

        let detail = Self::format_column_detail(column);

        CompletionItem {
            label,
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(detail),
            documentation: None,
            deprecated: Some(false),
            preselect: Some(false),
            sort_text: Some(Self::sort_text(column)),
            filter_text: Some(column.name.clone()),
            insert_text: Some(if qualified {
                format!("{}.{}", table.display_name(), column.name)
            } else {
                column.name.clone()
            }),
            ..Default::default()
        }
    }

    /// Create a wildcard (*) completion item
    fn wildcard_item() -> CompletionItem {
        CompletionItem {
            label: "*".to_string(),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some("All columns".to_string()),
            documentation: Some(Documentation::String(
                "Selects all columns from all tables in the FROM clause".to_string(),
            )),
            sort_text: Some("00_wildcard".to_string()),
            ..Default::default()
        }
    }

    /// Format the detail string for a column
    ///
    /// Shows the data type and whether it's nullable
    fn format_column_detail(column: &ColumnSymbol) -> String {
        let type_str = Self::format_data_type(&column.data_type);
        type_str
    }

    /// Format a DataType to a display string
    fn format_data_type(data_type: &DataType) -> String {
        match data_type {
            DataType::Integer => "INTEGER".to_string(),
            DataType::BigInt => "BIGINT".to_string(),
            DataType::SmallInt => "SMALLINT".to_string(),
            DataType::TinyInt => "TINYINT".to_string(),
            DataType::Decimal => "DECIMAL".to_string(),
            DataType::Float => "FLOAT".to_string(),
            DataType::Double => "DOUBLE".to_string(),
            DataType::Varchar(Some(len)) => format!("VARCHAR({})", len),
            DataType::Varchar(None) => "VARCHAR".to_string(),
            DataType::Char(Some(len)) => format!("CHAR({})", len),
            DataType::Char(None) => "CHAR".to_string(),
            DataType::Text => "TEXT".to_string(),
            DataType::Binary => "BINARY".to_string(),
            DataType::VarBinary(None) => "VARBINARY".to_string(),
            DataType::VarBinary(Some(len)) => format!("VARBINARY({})", len),
            DataType::Blob => "BLOB".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Time => "TIME".to_string(),
            DataType::DateTime => "DATETIME".to_string(),
            DataType::Timestamp => "TIMESTAMP".to_string(),
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::Json => "JSON".to_string(),
            DataType::Uuid => "UUID".to_string(),
            DataType::Enum(values) => format!("ENUM({})", values.join(", ")),
            DataType::Array(inner) => format!("{}[]", Self::format_data_type(inner)),
            DataType::Other(name) => name.clone(),
            _ => "UNKNOWN".to_string(), // Handle non-exhaustive enum
        }
    }

    /// Generate sort text for a column
    ///
    /// Columns are sorted alphabetically by name
    fn sort_text(column: &ColumnSymbol) -> String {
        format!("01_{}", column.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::DataType;

    #[test]
    fn test_render_columns_simple() {
        let table = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users"),
            ColumnSymbol::new("name", DataType::Text, "users"),
        ]);

        let items = CompletionRenderer::render_columns(&[table], false);

        // Should have wildcard + 2 columns
        assert_eq!(items.len(), 3);
        assert!(items.iter().any(|i| i.label == "id"));
        assert!(items.iter().any(|i| i.label == "name"));
        assert!(items.iter().any(|i| i.label == "*"));
    }

    #[test]
    fn test_render_columns_qualified() {
        let table = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users"),
        ]);

        let items = CompletionRenderer::render_columns(&[table], true);

        assert!(items.iter().any(|i| i.label == "users.id"));
    }

    #[test]
    fn test_render_columns_with_alias() {
        let table = TableSymbol::new("users")
            .with_alias("u")
            .with_columns(vec![ColumnSymbol::new("id", DataType::Integer, "users")]);

        let items = CompletionRenderer::render_columns(&[table], true);

        // Should use alias "u" instead of table name "users"
        assert!(items.iter().any(|i| i.label == "u.id"));
    }

    #[test]
    fn test_render_columns_ambiguous() {
        let table1 = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users"),
        ]);

        let table2 = TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders"),
        ]);

        let items = CompletionRenderer::render_columns(&[table1, table2], false);

        // Both columns are named "id", so both should be qualified
        let id_items: Vec<_> = items.iter().filter(|i| i.label.contains("id")).collect();
        assert_eq!(id_items.len(), 2);
        assert!(id_items.iter().any(|i| i.label == "users.id"));
        assert!(id_items.iter().any(|i| i.label == "orders.id"));
    }

    #[test]
    fn test_wildcard_item() {
        let item = CompletionRenderer::wildcard_item();
        assert_eq!(item.label, "*");
        assert_eq!(item.kind, Some(CompletionItemKind::FIELD));
        assert_eq!(item.detail, Some("All columns".to_string()));
    }

    #[test]
    fn test_format_data_type() {
        assert_eq!(CompletionRenderer::format_data_type(&DataType::Integer), "INTEGER");
        assert_eq!(
            CompletionRenderer::format_data_type(&DataType::Varchar(Some(255))),
            "VARCHAR(255)"
        );
        assert_eq!(CompletionRenderer::format_data_type(&DataType::Text), "TEXT");
        assert_eq!(
            CompletionRenderer::format_data_type(&DataType::Array(Box::new(DataType::Integer))),
            "INTEGER[]"
        );
    }

    #[test]
    fn test_sort_text() {
        let pk_col = ColumnSymbol::new("id", DataType::Integer, "users");
        let regular_col = ColumnSymbol::new("name", DataType::Text, "users");

        let pk_sort = CompletionRenderer::sort_text(&pk_col);
        let regular_sort = CompletionRenderer::sort_text(&regular_col);

        // Sort alphabetically by name
        assert_eq!(pk_sort, "01_id");
        assert_eq!(regular_sort, "01_name");
        assert!(pk_sort < regular_sort);
    }
}
