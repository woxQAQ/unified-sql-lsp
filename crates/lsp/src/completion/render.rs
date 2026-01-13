// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion rendering
//!
//! This module provides functionality to render LSP completion items
//! from semantic symbols.

use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation};
use unified_sql_lsp_catalog::{DataType, FunctionMetadata, FunctionType, TableMetadata, TableType};
use unified_sql_lsp_semantic::{ColumnSymbol, TableSymbol};

// Import keyword types
use crate::completion::keywords::SqlKeyword;

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

    /// Render JOIN condition column completion items with PK/FK prioritization
    ///
    /// # Arguments
    ///
    /// * `tables` - Tables with their columns (typically 2 tables for JOIN)
    /// * `force_qualifier` - Whether to force table qualifier (always true for JOINs)
    ///
    /// # Returns
    ///
    /// Vector of completion items sorted by PK/FK priority
    ///
    /// # Sorting Strategy
    ///
    /// - Tier 1: Primary keys (sort: "00_pk_<name>", preselect: true)
    /// - Tier 2: Foreign keys (sort: "01_fk_<name>", preselect: true)
    /// - Tier 3: Regular columns (sort: "02_<name>", alphabetical)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let items = CompletionRenderer::render_join_columns(&[left_table, right_table], true);
    /// assert!(items[0].preselect.unwrap()); // PK/FK columns should be preselected
    /// ```
    pub fn render_join_columns(
        tables: &[TableSymbol],
        force_qualifier: bool,
    ) -> Vec<CompletionItem> {
        let mut pk_columns: Vec<CompletionItem> = Vec::new();
        let mut fk_columns: Vec<CompletionItem> = Vec::new();
        let mut regular_columns: Vec<CompletionItem> = Vec::new();

        for table in tables {
            for column in &table.columns {
                let item = Self::column_item(column, table, force_qualifier);

                if column.is_primary_key {
                    // Mark as preselect (top suggestion)
                    let mut item = item;
                    item.preselect = Some(true);
                    item.sort_text = Some(format!("00_pk_{}", column.name));
                    pk_columns.push(item);
                } else if column.is_foreign_key {
                    // Mark as preselect (top suggestion)
                    let mut item = item;
                    item.preselect = Some(true);
                    item.sort_text = Some(format!("01_fk_{}", column.name));
                    fk_columns.push(item);
                } else {
                    // Regular columns
                    let mut item = item;
                    item.sort_text = Some(format!("02_{}", column.name));
                    regular_columns.push(item);
                }
            }
        }

        // Concatenate in priority order: PK → FK → Regular
        let mut items = Vec::new();
        items.extend(pk_columns);
        items.extend(fk_columns);

        // Sort regular columns alphabetically by label
        regular_columns.sort_by(|a, b| a.label.cmp(&b.label));
        items.extend(regular_columns);

        items
    }

    /// Render table completion items
    ///
    /// # Arguments
    ///
    /// * `tables` - Vector of table metadata from catalog
    /// * `show_schema` - Whether to show schema qualifier (e.g., "public.users")
    ///
    /// # Returns
    ///
    /// Vector of completion items
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let items = CompletionRenderer::render_tables(&tables, false);
    /// assert!(items.iter().any(|i| i.label == "users"));
    /// ```
    pub fn render_tables(tables: &[TableMetadata], show_schema: bool) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for table in tables {
            items.push(Self::table_item(table, show_schema));
        }

        items
    }

    /// Render a single table completion item
    ///
    /// # Arguments
    ///
    /// * `table` - The table metadata
    /// * `show_schema` - Whether to include schema qualifier in label
    fn table_item(table: &TableMetadata, show_schema: bool) -> CompletionItem {
        let label = if show_schema {
            format!("{}.{}", table.schema, table.name)
        } else {
            table.name.clone()
        };

        let detail = Self::format_table_detail(table);
        let documentation = Self::format_table_documentation(table);

        CompletionItem {
            label,
            kind: Some(CompletionItemKind::CLASS),
            detail: Some(detail),
            documentation: Some(Documentation::String(documentation)),
            deprecated: Some(false),
            preselect: Some(false),
            sort_text: Some(Self::table_sort_text(table, show_schema)),
            filter_text: Some(table.name.clone()),
            insert_text: Some(if show_schema {
                format!("{}.{}", table.schema, table.name)
            } else {
                table.name.clone()
            }),
            ..Default::default()
        }
    }

    /// Format the detail string for a table
    ///
    /// Shows the schema name and table type
    fn format_table_detail(table: &TableMetadata) -> String {
        let type_str = match &table.table_type {
            TableType::Table => "TABLE",
            TableType::View => "VIEW",
            TableType::MaterializedView => "MATERIALIZED VIEW",
            TableType::Temporary => "TEMPORARY",
            TableType::System => "SYSTEM",
            TableType::Other(s) => s,
        };
        format!("{}.{} [{}]", table.schema, table.name, type_str)
    }

    /// Format the documentation string for a table
    ///
    /// Shows column count and comment if available
    fn format_table_documentation(table: &TableMetadata) -> String {
        let mut parts = Vec::new();

        // Add column count
        let column_count = table.columns.len();
        if column_count > 0 {
            parts.push(format!("{} columns", column_count));

            // List column names if there are few (<= 5)
            if column_count <= 5 {
                let column_names: Vec<&str> =
                    table.columns.iter().map(|c| c.name.as_str()).collect();
                parts.push(format!("Columns: {}", column_names.join(", ")));
            }
        }

        // Add comment if available
        if let Some(comment) = &table.comment {
            parts.push(comment.clone());
        }

        // Add row count estimate if available
        if let Some(row_count) = table.row_count_estimate {
            parts.push(format!("~{} rows", row_count));
        }

        if parts.is_empty() {
            "Database table".to_string()
        } else {
            parts.join("\n\n")
        }
    }

    /// Generate sort text for a table
    ///
    /// Tables are sorted alphabetically by schema.table name
    fn table_sort_text(table: &TableMetadata, show_schema: bool) -> String {
        if show_schema {
            format!("{}_.{}", table.schema, table.name)
        } else {
            format!("{}_{}", table.schema, table.name)
        }
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

    /// Render function completion items
    ///
    /// # Arguments
    ///
    /// * `functions` - Vector of function metadata
    /// * `filter` - Optional function type filter (None = show all)
    ///
    /// # Returns
    ///
    /// Vector of completion items
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Show all functions
    /// let items = CompletionRenderer::render_functions(&functions, None);
    ///
    /// // Show only aggregate functions
    /// let items = CompletionRenderer::render_functions(
    ///     &functions,
    ///     Some(FunctionType::Aggregate)
    /// );
    /// ```
    pub fn render_functions(
        functions: &[FunctionMetadata],
        filter: Option<FunctionType>,
    ) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for function in functions {
            // Apply filter if specified
            if let Some(ft) = &filter {
                if &function.function_type != ft {
                    continue;
                }
            }

            items.push(Self::function_item(function));
        }

        // Sort by function type priority, then alphabetically
        items.sort_by(|a, b| {
            let a_sort = a.sort_text.as_ref().unwrap();
            let b_sort = b.sort_text.as_ref().unwrap();
            a_sort.cmp(b_sort)
        });

        items
    }

    /// Render a single function completion item
    ///
    /// # Arguments
    ///
    /// * `function` - The function metadata
    fn function_item(function: &FunctionMetadata) -> CompletionItem {
        let label = function.name.clone();
        let detail = Self::format_function_detail(function);
        let documentation = Self::format_function_documentation(function);

        // Determine sort priority based on function type
        let sort_prefix = match function.function_type {
            FunctionType::Aggregate => "00_aggregate_",
            FunctionType::Window => "01_window_",
            FunctionType::Table => "02_table_",
            FunctionType::Scalar => "03_scalar_",
        };

        CompletionItem {
            label,
            kind: Some(CompletionItemKind::CLASS), // TODO: (COMPLETION-006) Use Function when tower-lsp upgrades to LSP 3.17+
            detail: Some(detail),
            documentation: Some(Documentation::String(documentation)),
            deprecated: Some(false),
            preselect: Some(false),
            sort_text: Some(format!("{}{}", sort_prefix, function.name)),
            filter_text: Some(function.name.clone()),
            insert_text: Some(format!("{}(", function.name)), // Add opening paren
            ..Default::default()
        }
    }

    /// Format the detail string for a function
    ///
    /// Shows the function signature with parameters and return type
    fn format_function_detail(function: &FunctionMetadata) -> String {
        // Use the existing signature() method from FunctionMetadata
        function.signature()
    }

    /// Format the documentation string for a function
    ///
    /// Shows description, example, and parameter details
    fn format_function_documentation(function: &FunctionMetadata) -> String {
        let mut parts = Vec::new();

        // Add description
        if let Some(desc) = &function.description {
            parts.push(desc.clone());
        }

        // Add parameter details
        if !function.parameters.is_empty() {
            let params: Vec<String> = function
                .parameters
                .iter()
                .map(|p| {
                    let default = if p.has_default { " = default" } else { "" };
                    let variadic = if p.is_variadic { "..." } else { "" };
                    format!("- `{} {:?}{}{}`", p.name, p.data_type, variadic, default)
                })
                .collect();
            parts.push(format!("Parameters:\n{}", params.join("\n")));
        }

        // Add example if available
        if let Some(example) = &function.example {
            parts.push(format!("Example:\n```sql\n{}\n```", example));
        }

        if parts.is_empty() {
            "SQL function".to_string()
        } else {
            parts.join("\n\n")
        }
    }

    /// Render keyword completion items
    ///
    /// # Arguments
    ///
    /// * `keywords` - Vector of SQL keywords
    ///
    /// # Returns
    ///
    /// Vector of completion items
    pub fn render_keywords(keywords: &[SqlKeyword]) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        for keyword in keywords {
            items.push(Self::keyword_item(keyword));
        }

        // Sort by priority
        items.sort_by(|a, b| {
            let a_sort = a.sort_text.as_ref().unwrap();
            let b_sort = b.sort_text.as_ref().unwrap();
            a_sort.cmp(b_sort)
        });

        items
    }

    /// Render a single keyword completion item
    ///
    /// # Arguments
    ///
    /// * `keyword` - The SQL keyword
    fn keyword_item(keyword: &SqlKeyword) -> CompletionItem {
        let label = keyword.label.clone();
        let documentation = if let Some(desc) = &keyword.description {
            Documentation::String(desc.clone())
        } else {
            Documentation::String("SQL keyword".to_string())
        };

        CompletionItem {
            label,
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("SQL keyword".to_string()),
            documentation: Some(documentation),
            deprecated: Some(false),
            preselect: Some(false),
            sort_text: Some(format!("{:05}_{}", keyword.sort_priority, keyword.label)),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::{ColumnMetadata, DataType, TableType};

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
        let table = TableSymbol::new("users").with_columns(vec![ColumnSymbol::new(
            "id",
            DataType::Integer,
            "users",
        )]);

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
        let table1 = TableSymbol::new("users").with_columns(vec![ColumnSymbol::new(
            "id",
            DataType::Integer,
            "users",
        )]);

        let table2 = TableSymbol::new("orders").with_columns(vec![ColumnSymbol::new(
            "id",
            DataType::Integer,
            "orders",
        )]);

        let items = CompletionRenderer::render_columns(&[table1, table2], false);

        // Both columns are named "id", so both should be qualified
        let id_items: Vec<_> = items.iter().filter(|i| i.label.contains("id")).collect();
        assert_eq!(id_items.len(), 2);
        assert!(id_items.iter().any(|i| i.label == "users.id"));
        assert!(id_items.iter().any(|i| i.label == "orders.id"));
    }

    #[test]
    fn test_format_data_type() {
        assert_eq!(
            CompletionRenderer::format_data_type(&DataType::Integer),
            "INTEGER"
        );
        assert_eq!(
            CompletionRenderer::format_data_type(&DataType::Varchar(Some(255))),
            "VARCHAR(255)"
        );
        assert_eq!(
            CompletionRenderer::format_data_type(&DataType::Text),
            "TEXT"
        );
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

    #[test]
    fn test_render_tables_simple() {
        let table = TableMetadata::new("users", "public")
            .with_columns(vec![
                ColumnMetadata::new("id", DataType::Integer).with_primary_key(),
                ColumnMetadata::new("name", DataType::Text),
            ])
            .with_row_count(100);

        let items = CompletionRenderer::render_tables(&[table], false);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "users");
        assert_eq!(items[0].kind, Some(CompletionItemKind::CLASS));
        assert!(items[0].detail.as_ref().unwrap().contains("TABLE"));
        match items[0].documentation.as_ref().unwrap() {
            Documentation::String(s) => assert!(s.contains("2 columns")),
            Documentation::MarkupContent(m) => assert!(m.value.contains("2 columns")),
        }
    }

    #[test]
    fn test_render_tables_with_schema() {
        let table = TableMetadata::new("users", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]);

        let items = CompletionRenderer::render_tables(&[table], true);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "public.users");
        assert!(items[0].detail.as_ref().unwrap().contains("public"));
    }

    #[test]
    fn test_render_tables_multiple_schemas() {
        let table1 = TableMetadata::new("users", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]);
        let table2 = TableMetadata::new("users", "myapp")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]);

        let items = CompletionRenderer::render_tables(&[table1, table2], true);

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.label == "public.users"));
        assert!(items.iter().any(|i| i.label == "myapp.users"));
    }

    #[test]
    fn test_render_tables_with_view() {
        let view = TableMetadata::new("active_users", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
            .with_type(TableType::View);

        let items = CompletionRenderer::render_tables(&[view], false);

        assert_eq!(items.len(), 1);
        assert!(items[0].detail.as_ref().unwrap().contains("VIEW"));
    }

    #[test]
    fn test_render_tables_with_materialized_view() {
        let mv = TableMetadata::new("user_summary", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
            .with_type(TableType::MaterializedView);

        let items = CompletionRenderer::render_tables(&[mv], false);

        assert_eq!(items.len(), 1);
        assert!(
            items[0]
                .detail
                .as_ref()
                .unwrap()
                .contains("MATERIALIZED VIEW")
        );
    }

    #[test]
    fn test_render_tables_with_comment() {
        let table = TableMetadata::new("users", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
            .with_comment("User accounts table");

        let items = CompletionRenderer::render_tables(&[table], false);

        assert_eq!(items.len(), 1);
        match items[0].documentation.as_ref().unwrap() {
            Documentation::String(s) => assert!(s.contains("User accounts table")),
            Documentation::MarkupContent(m) => assert!(m.value.contains("User accounts table")),
        }
    }

    #[test]
    fn test_render_tables_few_columns_lists_names() {
        let table = TableMetadata::new("users", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("name", DataType::Text),
            ColumnMetadata::new("email", DataType::Text),
        ]);

        let items = CompletionRenderer::render_tables(&[table], false);

        assert_eq!(items.len(), 1);
        match items[0].documentation.as_ref().unwrap() {
            Documentation::String(s) => assert!(s.contains("id, name, email")),
            Documentation::MarkupContent(m) => assert!(m.value.contains("id, name, email")),
        }
    }

    #[test]
    fn test_render_tables_many_columns_hides_names() {
        // Create a table with more than 5 columns
        let columns: Vec<_> = (0..10)
            .map(|i| ColumnMetadata::new(&format!("col{}", i), DataType::Text))
            .collect();

        let table = TableMetadata::new("wide_table", "public").with_columns(columns);

        let items = CompletionRenderer::render_tables(&[table], false);

        assert_eq!(items.len(), 1);
        // Should not list column names for wide tables
        match items[0].documentation.as_ref().unwrap() {
            Documentation::String(s) => {
                assert!(s.contains("10 columns"));
                assert!(!s.contains("col0"));
            }
            Documentation::MarkupContent(m) => {
                assert!(m.value.contains("10 columns"));
                assert!(!m.value.contains("col0"));
            }
        }
    }

    #[test]
    fn test_render_tables_sort_order() {
        let table1 = TableMetadata::new("zebra", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]);
        let table2 = TableMetadata::new("apple", "public")
            .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]);

        let items = CompletionRenderer::render_tables(&[table1, table2], false);

        assert_eq!(items.len(), 2);
        // Items should be sorted alphabetically
        assert_eq!(items[0].label, "apple");
        assert_eq!(items[1].label, "zebra");
    }

    #[test]
    fn test_render_join_columns_basic() {
        let left = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users").with_primary_key(),
            ColumnSymbol::new("name", DataType::Text, "users"),
        ]);

        let right = TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders").with_primary_key(),
            ColumnSymbol::new("user_id", DataType::Integer, "orders").with_foreign_key(),
        ]);

        let items = CompletionRenderer::render_join_columns(&[left, right], true);

        // Should have 4 columns total (no wildcard for JOINs)
        assert_eq!(items.len(), 4);

        // First items should be PK columns (preselect: true)
        assert!(items[0].preselect.unwrap());
        assert!(items[1].preselect.unwrap());

        // PK/FK columns should come before regular columns
        assert!(items[0].sort_text.as_ref().unwrap().starts_with("00_pk_"));
        assert!(items[1].sort_text.as_ref().unwrap().starts_with("00_pk_"));
        assert!(items[2].sort_text.as_ref().unwrap().starts_with("01_fk_"));
        assert!(items[3].sort_text.as_ref().unwrap().starts_with("02_"));

        // All columns should be qualified
        assert!(items.iter().all(|i| i.label.contains('.')));
    }

    #[test]
    fn test_render_join_columns_pk_fk_priority() {
        let users = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users").with_primary_key(),
            ColumnSymbol::new("name", DataType::Text, "users"),
        ]);

        let orders = TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders").with_primary_key(),
            ColumnSymbol::new("user_id", DataType::Integer, "orders").with_foreign_key(),
            ColumnSymbol::new("total", DataType::Decimal, "orders"),
        ]);

        let items = CompletionRenderer::render_join_columns(&[users, orders], true);

        // Verify ordering: PKs first, then FKs, then regular
        let pk_items: Vec<_> = items
            .iter()
            .filter(|i| i.sort_text.as_ref().unwrap().starts_with("00_pk_"))
            .collect();
        let fk_items: Vec<_> = items
            .iter()
            .filter(|i| i.sort_text.as_ref().unwrap().starts_with("01_fk_"))
            .collect();
        let regular_items: Vec<_> = items
            .iter()
            .filter(|i| i.sort_text.as_ref().unwrap().starts_with("02_"))
            .collect();

        assert_eq!(pk_items.len(), 2); // users.id, orders.id
        assert_eq!(fk_items.len(), 1); // orders.user_id
        assert_eq!(regular_items.len(), 2); // users.name, orders.total (sorted alphabetically)

        // Verify PK/FK items are preselected
        assert!(pk_items.iter().all(|i| i.preselect.unwrap()));
        assert!(fk_items.iter().all(|i| i.preselect.unwrap()));
        assert!(!regular_items.iter().all(|i| i.preselect.unwrap()));
    }

    #[test]
    fn test_render_join_columns_qualified() {
        let table = TableSymbol::new("users").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users").with_primary_key(),
            ColumnSymbol::new("name", DataType::Text, "users"),
        ]);

        let items = CompletionRenderer::render_join_columns(&[table], true);

        // All items should be qualified
        assert!(items.iter().all(|i| i.label.contains('.')));
        assert!(items.iter().any(|i| i.label == "users.id"));
        assert!(items.iter().any(|i| i.label == "users.name"));
    }

    #[test]
    fn test_render_join_columns_with_alias() {
        let table = TableSymbol::new("users").with_alias("u").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users").with_primary_key(),
            ColumnSymbol::new("name", DataType::Text, "users"),
        ]);

        let items = CompletionRenderer::render_join_columns(&[table], true);

        // Should use alias instead of table name
        assert!(items.iter().any(|i| i.label == "u.id"));
        assert!(items.iter().any(|i| i.label == "u.name"));
        assert!(!items.iter().any(|i| i.label.starts_with("users.")));
    }

    #[test]
    fn test_render_join_columns_composite_pk() {
        let table = TableSymbol::new("order_items").with_columns(vec![
            ColumnSymbol::new("order_id", DataType::Integer, "order_items").with_primary_key(),
            ColumnSymbol::new("item_id", DataType::Integer, "order_items").with_primary_key(),
            ColumnSymbol::new("quantity", DataType::Integer, "order_items"),
        ]);

        let items = CompletionRenderer::render_join_columns(&[table], true);

        // Both PK columns should be marked as preselect
        let pk_items: Vec<_> = items.iter().filter(|i| i.preselect.unwrap()).collect();

        assert_eq!(pk_items.len(), 2);
        assert!(
            pk_items
                .iter()
                .all(|i| i.sort_text.as_ref().unwrap().starts_with("00_pk_"))
        );
    }

    #[test]
    fn test_render_functions_all() {
        use unified_sql_lsp_catalog::FunctionMetadata;

        let functions = vec![
            FunctionMetadata::new("count", DataType::BigInt)
                .with_type(FunctionType::Aggregate)
                .with_description("Count rows"),
            FunctionMetadata::new("abs", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Absolute value"),
        ];

        let items = CompletionRenderer::render_functions(&functions, None);

        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.label == "count"));
        assert!(items.iter().any(|i| i.label == "abs"));
    }

    #[test]
    fn test_render_functions_filtered() {
        use unified_sql_lsp_catalog::FunctionMetadata;

        let functions = vec![
            FunctionMetadata::new("count", DataType::BigInt).with_type(FunctionType::Aggregate),
            FunctionMetadata::new("abs", DataType::Integer).with_type(FunctionType::Scalar),
        ];

        let items = CompletionRenderer::render_functions(&functions, Some(FunctionType::Aggregate));

        // Should only show aggregate functions
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "count");
    }

    #[test]
    fn test_function_item_signature() {
        use unified_sql_lsp_catalog::{FunctionMetadata, FunctionParameter};

        let func = FunctionMetadata::new("count", DataType::BigInt).with_parameters(vec![
            FunctionParameter {
                name: "expr".to_string(),
                data_type: DataType::Integer,
                has_default: false,
                is_variadic: false,
            },
        ]);

        let item = CompletionRenderer::function_item(&func);

        assert_eq!(item.label, "count");
        assert!(item.detail.as_ref().unwrap().contains("count"));
        assert_eq!(item.kind, Some(CompletionItemKind::CLASS)); // Using CLASS for functions
        assert!(item.insert_text.as_ref().unwrap().ends_with("("));
    }

    #[test]
    fn test_function_sort_order() {
        use unified_sql_lsp_catalog::FunctionMetadata;

        let functions = vec![
            FunctionMetadata::new("abs", DataType::Integer).with_type(FunctionType::Scalar),
            FunctionMetadata::new("count", DataType::BigInt).with_type(FunctionType::Aggregate),
            FunctionMetadata::new("row_number", DataType::BigInt).with_type(FunctionType::Window),
        ];

        let items = CompletionRenderer::render_functions(&functions, None);

        // Aggregates should come first
        assert!(
            items[0]
                .sort_text
                .as_ref()
                .unwrap()
                .starts_with("00_aggregate_")
        );
        assert_eq!(items[0].label, "count");

        // Window functions second
        assert!(
            items[1]
                .sort_text
                .as_ref()
                .unwrap()
                .starts_with("01_window_")
        );
        assert_eq!(items[1].label, "row_number");

        // Scalar functions last
        assert!(
            items[2]
                .sort_text
                .as_ref()
                .unwrap()
                .starts_with("03_scalar_")
        );
        assert_eq!(items[2].label, "abs");
    }

    #[test]
    fn test_function_item_with_parameters() {
        use unified_sql_lsp_catalog::{FunctionMetadata, FunctionParameter};

        let func = FunctionMetadata::new("concat", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Concatenate strings")
            .with_parameters(vec![
                FunctionParameter {
                    name: "str1".to_string(),
                    data_type: DataType::Text,
                    has_default: false,
                    is_variadic: false,
                },
                FunctionParameter {
                    name: "str2".to_string(),
                    data_type: DataType::Text,
                    has_default: false,
                    is_variadic: false,
                },
            ])
            .with_example("SELECT CONCAT(first, ' ', last) FROM users");

        let item = CompletionRenderer::function_item(&func);

        assert_eq!(item.label, "concat");
        assert_eq!(item.kind, Some(CompletionItemKind::CLASS)); // Using CLASS for functions

        // Check documentation contains parameter details
        match item.documentation.as_ref().unwrap() {
            Documentation::String(doc) => {
                assert!(doc.contains("Concatenate strings"));
                assert!(doc.contains("str1"));
                assert!(doc.contains("str2"));
                assert!(doc.contains("CONCAT"));
            }
            _ => panic!("Expected string documentation"),
        }
    }

    #[test]
    fn test_function_item_insert_text() {
        use unified_sql_lsp_catalog::FunctionMetadata;

        let func =
            FunctionMetadata::new("count", DataType::BigInt).with_type(FunctionType::Aggregate);

        let item = CompletionRenderer::function_item(&func);

        // Insert text should include opening paren for easier typing
        assert_eq!(item.insert_text.as_ref().unwrap(), "count(");
    }
}
