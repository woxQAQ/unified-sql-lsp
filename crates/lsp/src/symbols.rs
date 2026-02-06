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
use tracing::debug;
use unified_sql_lsp_catalog::{Catalog, CatalogError, format_data_type};
use unified_sql_lsp_context::{
    QuerySymbol as ContextQuerySymbol, SymbolBuilder as ContextSymbolBuilder,
};
use unified_sql_lsp_semantic::{ColumnSymbol, TableSymbol};

/// Symbol extraction error
#[derive(Debug, thiserror::Error)]
pub enum SymbolError {
    #[error("Invalid SQL syntax: {0}")]
    InvalidSyntax(String),

    #[error("Catalog error: {0}")]
    Catalog(#[from] CatalogError),
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
    /// ```
    /// # use unified_sql_lsp_lsp::symbols::SymbolBuilder;
    /// # use tree_sitter::Parser;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let source = "SELECT id FROM users";
    /// # let mut parser = Parser::new();
    /// # let tree = parser.parse(source, None).unwrap();
    /// let queries = SymbolBuilder::build_from_cst(&tree.root_node(), source)?;
    /// # let _ = queries;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_from_cst(
        root_node: &tree_sitter::Node<'_>,
        source: &str,
    ) -> Result<Vec<QuerySymbol>, SymbolError> {
        let context_queries = ContextSymbolBuilder::build_from_cst(root_node, source)
            .map_err(|e| SymbolError::InvalidSyntax(e.to_string()))?;
        Ok(context_queries
            .into_iter()
            .map(Self::from_context_query)
            .collect())
    }

    fn from_context_query(query: ContextQuerySymbol) -> QuerySymbol {
        QuerySymbol {
            range: Self::context_range_to_lsp_range(query.range),
            tables: query
                .tables
                .into_iter()
                .map(|t| TableSymbolWithRange {
                    symbol: t.symbol,
                    range: Self::context_range_to_lsp_range(t.range),
                    selection_range: Self::context_range_to_lsp_range(t.selection_range),
                })
                .collect(),
        }
    }

    fn context_range_to_lsp_range(range: unified_sql_lsp_context::Range) -> Range {
        Range {
            start: Position::new(range.start.line, range.start.character),
            end: Position::new(range.end.line, range.end.character),
        }
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
        tables: &mut [TableSymbolWithRange],
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
                debug!(
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
        queries.into_iter().map(Self::render_query).collect()
    }

    /// Render a single query symbol
    fn render_query(query: QuerySymbol) -> DocumentSymbol {
        let children = query.tables.into_iter().map(Self::render_table).collect();

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

        sorted.into_iter().map(Self::render_column).collect()
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
        let mut detail = format_data_type(&column.data_type);

        if column.is_primary_key {
            detail.push_str(" PK");
        }
        if column.is_foreign_key {
            detail.push_str(" FK");
        }

        detail
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::DataType;

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
