// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Document Symbols
//!
//! This module provides LSP document symbols functionality for SQL queries.
//!
//! ## What are Document Symbols?
//!
//! Document symbols show the structure of a document in an outline view,
//! enabling quick navigation to tables, columns, and other SQL elements.
//!
//! ## Example
//!
//! ```sql
//! SELECT u.id, u.name, o.total
//! FROM users u
//! JOIN orders o ON u.id = o.user_id;
//! ```
//!
//! The outline view would show:
//! - **users** (table)
//!   - id (Integer, PK)
//!   - name (Text)
//! - **orders** (table)
//!   - id (Integer, PK)
//!   - user_id (Integer, FK)
//!   - total (Decimal)
//!
//! ## Architecture
//!
//! This module provides:
//! - `SymbolBuilder`: Extract symbols from CST
//! - `SymbolCatalogFetcher`: Enrich with catalog metadata
//! - `SymbolRenderer`: Convert to LSP format

use std::sync::Arc;
use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
use tree_sitter::Node;
use unified_sql_lsp_catalog::{Catalog, CatalogError, DataType};
use unified_sql_lsp_semantic::{ColumnSymbol, TableSymbol};

/// Symbol extraction error
#[derive(Debug, thiserror::Error)]
pub enum SymbolError {
    #[error("Document not parsed")]
    NotParsed,

    #[error("Invalid SQL syntax: {0}")]
    InvalidSyntax(String),

    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),

    #[error("Scope build error: {0}")]
    ScopeBuild(String),
}

/// Table symbol with range information
#[derive(Debug, Clone)]
pub struct TableSymbolWithRange {
    /// The table symbol
    pub symbol: TableSymbol,

    /// Full range of the table reference in the document
    pub range: Range,

    /// Range for selection (smaller than range for better UX)
    pub selection_range: Range,
}

/// Query symbol (represents a SELECT statement)
#[derive(Debug, Clone)]
pub struct QuerySymbol {
    /// Range of the entire SELECT statement
    pub range: Range,

    /// Tables in this query
    pub tables: Vec<TableSymbolWithRange>,
}

/// Symbol builder for extracting symbols from CST
pub struct SymbolBuilder;

impl SymbolBuilder {
    /// Build symbols from CST root node
    ///
    /// # Arguments
    ///
    /// * `root_node` - Root CST node
    /// * `source` - Source code text
    ///
    /// # Returns
    ///
    /// Vector of query symbols (one per SELECT statement)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let queries = SymbolBuilder::build_from_cst(&root_node, source)?;
    /// ```
    pub fn build_from_cst(root_node: &Node, source: &str) -> Result<Vec<QuerySymbol>, SymbolError> {
        let mut queries = Vec::new();

        // Find all SELECT statements
        Self::find_select_statements(root_node, &mut queries);

        // Extract tables from each SELECT
        for query in &mut queries {
            query.tables = Self::extract_tables_from_query(&query.range, root_node, source)?;
        }

        Ok(queries)
    }

    /// Find all SELECT statements in the CST
    fn find_select_statements(node: &Node, queries: &mut Vec<QuerySymbol>) {
        if node.kind() == "select_statement" || node.kind() == "statement" {
            // Check if this is a SELECT statement
            if node.kind() == "select_statement" || Self::is_select_statement(node) {
                let range = Self::node_to_range(node);
                queries.push(QuerySymbol {
                    range,
                    tables: Vec::new(),
                });
            }
        }

        // Recurse into children
        for child in node.children(&mut node.walk()) {
            Self::find_select_statements(&child, queries);
        }
    }

    /// Check if a statement node is a SELECT statement
    fn is_select_statement(node: &Node) -> bool {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "select_statement" {
                return true;
            }
        }
        false
    }

    /// Extract tables from a query
    fn extract_tables_from_query(
        _query_range: &Range,
        root_node: &Node,
        source: &str,
    ) -> Result<Vec<TableSymbolWithRange>, SymbolError> {
        let mut tables = Vec::new();

        // Find all table_reference nodes within the query range
        Self::find_table_references(root_node, source, &mut tables);

        Ok(tables)
    }

    /// Recursively find table references
    fn find_table_references(node: &Node, source: &str, tables: &mut Vec<TableSymbolWithRange>) {
        match node.kind() {
            "table_reference" => {
                if let Ok(table) = Self::parse_table_reference(node, source) {
                    tables.push(table);
                }
            }
            _ => {
                // Recurse into children
                for child in node.children(&mut node.walk()) {
                    Self::find_table_references(&child, source, tables);
                }
            }
        }
    }

    /// Parse a table_reference node
    fn parse_table_reference(
        node: &Node,
        source: &str,
    ) -> Result<TableSymbolWithRange, SymbolError> {
        let mut table_name = None;
        let mut alias = None;

        // Walk through children to find table name and alias
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "table_name" | "identifier" => {
                    if table_name.is_none() {
                        table_name = Some(Self::extract_node_text(&child, source));
                    }
                }
                "alias" => {
                    if let Some(a) = Self::extract_alias(&child, source) {
                        alias = Some(a);
                    }
                }
                _ => {
                    // Check for identifier that might be an implicit alias
                    if table_name.is_some() && alias.is_none() && child.kind() == "identifier" {
                        let text = Self::extract_node_text(&child, source);
                        if &text != table_name.as_ref().unwrap() {
                            alias = Some(text);
                        }
                    }
                }
            }
        }

        let table_name = table_name.ok_or_else(|| {
            SymbolError::InvalidSyntax("Table name not found in table_reference".to_string())
        })?;

        let mut symbol = TableSymbol::new(&table_name);
        if let Some(a) = alias {
            symbol = symbol.with_alias(a);
        }

        let range = Self::node_to_range(node);
        let selection_range = Self::node_to_selection_range(node, source);

        Ok(TableSymbolWithRange {
            symbol,
            range,
            selection_range,
        })
    }

    /// Extract text from a node
    fn extract_node_text(node: &Node, source: &str) -> String {
        let bytes = node.byte_range();
        source[bytes].to_string()
    }

    /// Extract alias from an alias node
    fn extract_alias(alias_node: &Node, source: &str) -> Option<String> {
        for child in alias_node.children(&mut alias_node.walk()) {
            if child.kind() == "identifier" {
                return Some(Self::extract_node_text(&child, source));
            }
        }
        None
    }

    /// Convert a tree-sitter node to LSP Range
    fn node_to_range(node: &Node) -> Range {
        let start = Self::byte_to_position(node.start_byte());
        let end = Self::byte_to_position(node.end_byte());
        Range { start, end }
    }

    /// Convert a tree-sitter node to LSP selection Range
    fn node_to_selection_range(node: &Node, _source: &str) -> Range {
        // For selection range, try to find the identifier
        for child in node.children(&mut node.walk()) {
            if child.kind() == "table_name" || child.kind() == "identifier" {
                return Self::node_to_range(&child);
            }
        }
        // Fallback to full range
        Self::node_to_range(node)
    }

    /// Convert byte offset to LSP Position
    fn byte_to_position(byte_offset: usize) -> Position {
        // Simple implementation - uses byte offset as character offset
        // Note: This is a simplification. For UTF-8 multi-byte characters,
        // we would need to use Ropey for accurate position calculation.
        // This is sufficient for basic functionality and can be improved later.
        Position::new(0, byte_offset as u32)
    }
}

/// Symbol catalog fetcher for populating column metadata
pub struct SymbolCatalogFetcher {
    catalog: Arc<dyn Catalog>,
}

impl SymbolCatalogFetcher {
    /// Create a new catalog fetcher
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    /// Populate columns for all tables
    ///
    /// # Arguments
    ///
    /// * `tables` - Tables to populate (modified in place)
    ///
    /// # Returns
    ///
    /// Ok if at least one table was populated, Err if all failed
    pub async fn populate_columns(
        &self,
        tables: &mut Vec<TableSymbolWithRange>,
    ) -> Result<(), CatalogError> {
        let mut any_success = false;
        let mut errors = Vec::new();

        for table in tables.iter_mut() {
            let table_name = &table.symbol.table_name;

            match self.catalog.get_columns(table_name).await {
                Ok(columns) => {
                    // Convert catalog columns to ColumnSymbol
                    let column_symbols: Vec<ColumnSymbol> = columns
                        .into_iter()
                        .map(|col| {
                            let mut col_symbol = ColumnSymbol::new(
                                col.name.clone(),
                                col.data_type.clone(),
                                table_name.clone(),
                            );
                            if col.is_primary_key {
                                col_symbol = col_symbol.with_primary_key();
                            }
                            if col.is_foreign_key {
                                col_symbol = col_symbol.with_foreign_key();
                            }
                            col_symbol
                        })
                        .collect();

                    table.symbol = table.symbol.clone().with_columns(column_symbols);
                    any_success = true;
                }
                Err(e) => {
                    errors.push(format!("{}: {}", table_name, e));
                    // Add empty columns vector to allow partial results
                    table.symbol = table.symbol.clone().with_columns(vec![]);
                    // Continue with other tables
                }
            }
        }

        if any_success {
            // At least one table succeeded - log warnings for partial failures but don't fail
            if !errors.is_empty() {
                eprintln!(
                    "Warning: Partial catalog failures occurred while fetching column metadata: {:?}",
                    errors
                );
            }
            Ok(())
        } else if errors.is_empty() {
            Ok(())
        } else {
            // All tables failed - return error with details
            let error_msg = if errors.len() == 1 {
                format!("Failed to fetch table metadata: {}", errors[0])
            } else {
                format!(
                    "Failed to fetch metadata for {} tables: {}",
                    errors.len(),
                    errors.join("; ")
                )
            };
            Err(CatalogError::TableNotFound(
                error_msg,
                "default".to_string(), // schema name
            ))
        }
    }
}

/// Symbol renderer for converting to LSP format
pub struct SymbolRenderer;

impl SymbolRenderer {
    /// Render document symbols
    ///
    /// # Arguments
    ///
    /// * `queries` - Query symbols to render
    ///
    /// # Returns
    ///
    /// Vector of LSP DocumentSymbol
    pub fn render_document(queries: Vec<QuerySymbol>) -> Vec<DocumentSymbol> {
        queries
            .into_iter()
            .map(|query| Self::render_query(query))
            .collect()
    }

    /// Render a single query symbol
    fn render_query(query: QuerySymbol) -> DocumentSymbol {
        let children = query
            .tables
            .into_iter()
            .map(|table| Self::render_table(table))
            .collect();

        #[allow(deprecated)]
        DocumentSymbol {
            name: "SELECT".to_string(),
            kind: SymbolKind::FUNCTION,
            range: query.range,
            selection_range: query.range,
            detail: Some("Query".to_string()),
            children: Some(children),
            deprecated: None,
            tags: None,
        }
    }

    /// Render a table symbol
    fn render_table(table: TableSymbolWithRange) -> DocumentSymbol {
        let children = Self::render_columns(&table.symbol.columns);

        #[allow(deprecated)]
        DocumentSymbol {
            name: table.symbol.display_name().to_string(),
            kind: SymbolKind::OBJECT,
            range: table.range,
            selection_range: table.selection_range,
            detail: Some("Table".to_string()),
            children: Some(children),
            deprecated: None,
            tags: None,
        }
    }

    /// Render column symbols
    fn render_columns(columns: &[ColumnSymbol]) -> Vec<DocumentSymbol> {
        // Sort columns: PK → FK → alphabetical
        let mut sorted: Vec<&ColumnSymbol> = columns.iter().collect();
        sorted.sort_by(|a, b| {
            // Tier 1: Primary keys first
            match (a.is_primary_key, b.is_primary_key) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Tier 2: Foreign keys next
            match (a.is_foreign_key, b.is_foreign_key) {
                (true, false) => return std::cmp::Ordering::Less,
                (false, true) => return std::cmp::Ordering::Greater,
                _ => {}
            }

            // Tier 3: Alphabetical
            a.name.cmp(&b.name)
        });

        sorted
            .into_iter()
            .map(|col| Self::render_column(col))
            .collect()
    }

    /// Render a single column symbol
    fn render_column(column: &ColumnSymbol) -> DocumentSymbol {
        let detail = Self::format_column_detail(column);

        #[allow(deprecated)]
        DocumentSymbol {
            name: column.name.clone(),
            kind: SymbolKind::FIELD,
            detail: Some(detail),
            tags: None,
            range: Range::default(),
            selection_range: Range::default(),
            deprecated: None,
            children: None,
        }
    }

    /// Format column detail with data type and PK/FK indicators
    fn format_column_detail(column: &ColumnSymbol) -> String {
        let mut detail = Self::format_data_type(&column.data_type);

        if column.is_primary_key {
            detail.push_str(" PK");
        }
        if column.is_foreign_key {
            detail.push_str(" FK");
        }

        detail
    }

    /// Format data type for display
    fn format_data_type(data_type: &DataType) -> String {
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
            DataType::Array(inner) => format!("{}[]", Self::format_data_type(inner)),
            DataType::Other(name) => format!("Other({})", name),
            // Handle any future variants added to the non-exhaustive enum
            _ => "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_data_type() {
        assert_eq!(
            SymbolRenderer::format_data_type(&DataType::Integer),
            "Integer"
        );
        assert_eq!(SymbolRenderer::format_data_type(&DataType::Text), "Text");
        assert_eq!(
            SymbolRenderer::format_data_type(&DataType::Varchar(Some(255))),
            "VarChar(255)"
        );
        assert_eq!(
            SymbolRenderer::format_data_type(&DataType::Varchar(None)),
            "VarChar"
        );
    }

    #[test]
    fn test_format_column_detail() {
        let col1 = ColumnSymbol::new("id", DataType::Integer, "users").with_primary_key();
        assert_eq!(SymbolRenderer::format_column_detail(&col1), "Integer PK");

        let col2 = ColumnSymbol::new("user_id", DataType::Integer, "orders").with_foreign_key();
        assert_eq!(SymbolRenderer::format_column_detail(&col2), "Integer FK");

        let col3 = ColumnSymbol::new("name", DataType::Text, "users");
        assert_eq!(SymbolRenderer::format_column_detail(&col3), "Text");
    }
}
