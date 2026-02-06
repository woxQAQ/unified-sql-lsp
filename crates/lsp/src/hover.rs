// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Hover Information Provider
//!
//! This module provides hover functionality for SQL queries using CST analysis.
//!
//! ## Architecture
//!
//! The hover engine:
//! 1. Parses the source to get a CST
//! 2. Finds the node at cursor position
//! 3. Builds a scope from the FROM clause using ScopeBuilder
//! 4. Uses AliasResolver to resolve aliases to actual tables
//! 5. Returns appropriate hover information based on node type
//!
//! ## Example
//!
//! ```sql
//! SELECT u.|username| FROM users u
//! ```
//!
//! When hovering over `username`, the engine:
//! - Finds the identifier node at cursor
//! - Extracts "username" as the word
//! - Builds scope from FROM clause, finding alias "u" -> "users"
//! - Queries catalog for users.username column
//! - Returns column type information

use std::sync::Arc;
use tower_lsp::lsp_types::Position;
use tree_sitter::Node;
use unified_sql_lsp_catalog::Catalog;
use unified_sql_lsp_function_registry::HoverInfoProvider;
use unified_sql_lsp_function_registry::hover::ColumnHoverInfo;
use unified_sql_lsp_ir::Dialect;

use unified_sql_lsp_context::{
    Position as ContextPosition, ScopeBuilder,
    find_node_at_position as context_find_node_at_position, find_parent_select,
};

use unified_sql_lsp_semantic::HoverService;

use crate::document::Document;

/// Hover engine for SQL queries
///
/// Provides context-aware hover information using CST analysis.
pub struct HoverEngine {
    /// Catalog for fetching table and column metadata
    catalog: Arc<dyn Catalog>,

    /// SQL dialect
    dialect: Dialect,

    /// Hover info provider for formatting responses
    hover_provider: HoverInfoProvider,
}

impl HoverEngine {
    /// Create a new hover engine
    pub fn new(catalog: Arc<dyn Catalog>, dialect: Dialect) -> Self {
        Self {
            catalog,
            dialect,
            hover_provider: HoverInfoProvider::new(),
        }
    }

    /// Get hover information for a position in a document
    ///
    /// # Arguments
    ///
    /// * `document` - The document being hovered over
    /// * `position` - The cursor position
    ///
    /// # Returns
    ///
    /// Markdown-formatted hover text, or None if no information available
    pub async fn get_hover(&self, document: &Document, position: Position) -> Option<String> {
        let semantic_hover = HoverService::new(self.catalog.clone());

        // Get the CST from the document
        let tree_arc = document.tree()?;
        let tree_guard = tree_arc.blocking_lock();
        let root = tree_guard.root_node();

        // Get source text
        let source = document.get_content();

        // Find the node at cursor position
        let context_pos = ContextPosition::new(position.line, position.character);
        let node = context_find_node_at_position(&root, context_pos, &source)?;

        // Extract the word at cursor
        let word = semantic_hover.extract_word_at_node(&node, &source)?;
        if word.is_empty() {
            return None;
        }

        // Check for function names first
        if let Some(info) = self.hover_provider.get_function_hover(&word, &self.dialect) {
            return Some(info);
        }

        // Try to find parent SELECT statement to build scope
        let select_node_opt = find_parent_select(&node);
        let visible_tables = select_node_opt
            .as_ref()
            .map(|n| Self::extract_visible_tables(n, &source))
            .unwrap_or_default();

        // Check if we're in a FROM clause (hovering over table name or alias)
        if semantic_hover.is_in_from_clause(&node) {
            // Try to resolve as table name first
            if let Some(table_name) = semantic_hover.resolve_table_name(&word).await {
                return Some(self.hover_provider.get_table_hover(&table_name));
            }

            // Try to resolve as table alias.
            if let Some(actual_table) = semantic_hover
                .resolve_alias_table(&word, &visible_tables)
                .await
            {
                return Some(format!(
                    "```sql\n{}\n```\n\nTable alias for `{}`",
                    word, actual_table
                ));
            }
        }

        // Check if we're in a SELECT/WHERE/JOIN clause (hovering over column)
        if semantic_hover.is_in_select_context(&node) {
            // Try to resolve as column name.
            if let Some(column) = semantic_hover.resolve_column(&word, &visible_tables).await {
                return Some(
                    self.hover_provider
                        .get_column_hover(&to_column_hover_info(&column)),
                );
            }

            // Fallback: Try simple column lookup by parsing FROM clause
            if let Some(column) = semantic_hover.resolve_simple_column(&word, &source).await {
                return Some(
                    self.hover_provider
                        .get_column_hover(&to_column_hover_info(&column)),
                );
            }

            // Try to resolve as qualified column (table.column)
            if select_node_opt.is_some()
                && let Some(column) = semantic_hover
                    .resolve_qualified_column(&node, &source)
                    .await
            {
                return Some(
                    self.hover_provider
                        .get_column_hover(&to_column_hover_info(&column)),
                );
            }
        }

        // Fallback: try as table name (for bare table references)
        if let Some(table_name) = semantic_hover.resolve_table_name(&word).await {
            return Some(self.hover_provider.get_table_hover(&table_name));
        }

        None
    }

    fn extract_visible_tables(select_node: &Node<'_>, source: &str) -> Vec<String> {
        ScopeBuilder::build_from_select(select_node, source)
            .ok()
            .and_then(|scope_manager| scope_manager.get_scope(0).cloned())
            .map(|scope| scope.tables.into_iter().map(|t| t.table_name).collect())
            .unwrap_or_default()
    }
}

fn to_column_hover_info(column: &unified_sql_lsp_catalog::ColumnMetadata) -> ColumnHoverInfo {
    ColumnHoverInfo {
        name: column.name.clone(),
        data_type: column.data_type.clone(),
        is_primary_key: column.is_primary_key,
        is_foreign_key: column.is_foreign_key,
    }
}
