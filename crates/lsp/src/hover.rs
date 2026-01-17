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

use unified_sql_lsp_semantic::{AliasResolver, ResolutionResult};

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
        let word = self.extract_word_at_node(&node, &source)?;
        if word.is_empty() {
            return None;
        }

        // Check for function names first
        if let Some(info) = self.hover_provider.get_function_hover(&word, &self.dialect) {
            return Some(info);
        }

        // Try to find parent SELECT statement to build scope
        let select_node_opt = find_parent_select(&node);

        // Check if we're in a FROM clause (hovering over table name or alias)
        if self.is_in_from_clause(&node) {
            // Try to resolve as table name first
            if let Some(table_hover) = self.try_table_hover(&word).await {
                return Some(table_hover);
            }

            // Try to resolve as table alias (if we have a select node)
            if let Some(select_node) = &select_node_opt
                && let Some(alias_hover) = self.try_alias_hover(select_node, &word, &source).await
            {
                return Some(alias_hover);
            }
        }

        // Check if we're in a SELECT/WHERE/JOIN clause (hovering over column)
        if self.is_in_select_context(&node) {
            // Try to resolve as column name (if we have a select node)
            if let Some(select_node) = &select_node_opt
                && let Some(column_hover) = self.try_column_hover(select_node, &word, &source).await
            {
                return Some(column_hover);
            }

            // Fallback: Try simple column lookup by parsing FROM clause
            if let Some(column_hover) = self.try_simple_column_hover(&word, &source).await {
                return Some(column_hover);
            }

            // Try to resolve as qualified column (table.column)
            if let Some(select_node) = &select_node_opt
                && let Some(qualified_hover) = self
                    .try_qualified_column_hover(&node, select_node, &source)
                    .await
            {
                return Some(qualified_hover);
            }
        }

        // Fallback: try as table name (for bare table references)
        if let Some(table_hover) = self.try_table_hover(&word).await {
            return Some(table_hover);
        }

        None
    }

    /// Extract the word at a given node
    fn extract_word_at_node(&self, node: &Node<'_>, source: &str) -> Option<String> {
        let node_text = self.node_text(node, source);

        // For identifier nodes, extract just the identifier part
        if matches!(
            node.kind(),
            "identifier" | "table_name" | "column_name" | "function_name" | "alias"
        ) {
            // Clean up the text
            let text = node_text
                .trim()
                .trim_start_matches('`')
                .trim_end_matches('`')
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_string();
            return Some(text);
        }

        // For qualified references (e.g., "u.username"), extract the relevant part
        if node.kind() == "object_reference" || node.kind() == "qualified_column_name" {
            // Get the child that contains the cursor
            // For now, just return the full text and let the caller handle it
            return Some(node_text.trim().to_string());
        }

        None
    }

    /// Try to get hover for a table name
    async fn try_table_hover(&self, word: &str) -> Option<String> {
        let tables = self.catalog.list_tables().await.ok()?;

        let word_lower = word.to_lowercase();
        for table in tables {
            if table.name.to_lowercase() == word_lower {
                return Some(self.hover_provider.get_table_hover(&table.name));
            }
        }

        None
    }

    /// Try to get hover for a column using simple FROM clause parsing
    /// This is a fallback when CST-based scope building fails
    async fn try_simple_column_hover(&self, column_name: &str, source: &str) -> Option<String> {
        use unified_sql_lsp_function_registry::hover::ColumnHoverInfo;

        let source_upper = source.to_uppercase();

        // Try to extract table name FROM clause
        if let Some(from_pos) = source_upper.find(" FROM ") {
            let after_from = &source[from_pos + 6..];
            let table_name = after_from
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_lowercase();

            if !table_name.is_empty() {
                // Try to get columns from catalog
                if let Ok(columns) = self.catalog.get_columns(&table_name).await {
                    let column_lower = column_name.to_lowercase();
                    for column in columns {
                        if column.name.to_lowercase() == column_lower {
                            let hover_info = ColumnHoverInfo {
                                name: column.name.clone(),
                                data_type: column.data_type.clone(),
                                is_primary_key: column.is_primary_key,
                                is_foreign_key: column.is_foreign_key,
                            };
                            return Some(self.hover_provider.get_column_hover(&hover_info));
                        }
                    }
                }
            }
        }

        None
    }

    /// Try to get hover for a table alias
    async fn try_alias_hover(
        &self,
        select_node: &Node<'_>,
        alias: &str,
        source: &str,
    ) -> Option<String> {
        // Build scope from SELECT statement
        let scope_manager = ScopeBuilder::build_from_select(select_node, source).ok()?;

        // Get the root scope (id 0)
        let scope = scope_manager.get_scope(0)?;

        // Collect all tables from the scope
        let tables: Vec<String> = scope.tables.iter().map(|t| t.table_name.clone()).collect();

        if tables.is_empty() {
            return None;
        }

        // Resolve the alias using AliasResolver
        let resolver = AliasResolver::new(self.catalog.clone());
        match resolver.resolve(alias.to_string()).await.ok()? {
            ResolutionResult::Found(table) => {
                // Return hover info showing both the alias and the actual table name
                Some(format!(
                    "```sql\n{}\n```\n\nTable alias for `{}`",
                    alias, table.table_name
                ))
            }
            _ => None,
        }
    }

    /// Try to get hover for a column name
    async fn try_column_hover(
        &self,
        select_node: &Node<'_>,
        column_name: &str,
        source: &str,
    ) -> Option<String> {
        // Build scope from SELECT statement to get visible tables
        let scope_manager = ScopeBuilder::build_from_select(select_node, source).ok()?;

        // Get the root scope (id 0)
        let scope = scope_manager.get_scope(0)?;

        // Collect all tables from the scope
        let tables: Vec<String> = scope.tables.iter().map(|t| t.table_name.clone()).collect();

        if tables.is_empty() {
            return None;
        }

        let resolver = AliasResolver::new(self.catalog.clone());

        // Try each table until we find the column
        for table_name in tables {
            match resolver.resolve(table_name.clone()).await.ok()? {
                ResolutionResult::Found(table) => {
                    // Check if this table has the column
                    if let Ok(columns) = self.catalog.get_columns(&table.table_name).await {
                        let column_lower = column_name.to_lowercase();
                        for column in columns {
                            if column.name.to_lowercase() == column_lower {
                                let hover_info = ColumnHoverInfo {
                                    name: column.name.clone(),
                                    data_type: column.data_type.clone(),
                                    is_primary_key: column.is_primary_key,
                                    is_foreign_key: column.is_foreign_key,
                                };
                                return Some(self.hover_provider.get_column_hover(&hover_info));
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        None
    }

    /// Try to get hover for a qualified column reference (table.column)
    async fn try_qualified_column_hover(
        &self,
        node: &Node<'_>,
        _select_node: &Node<'_>,
        source: &str,
    ) -> Option<String> {
        // Extract the table part and column part from the qualified reference
        let text = self.node_text(node, source);
        let parts: Vec<&str> = text.split('.').collect();

        if parts.len() != 2 {
            return None;
        }

        let table_part = parts[0].trim();
        let column_part = parts[1].trim();

        if table_part.is_empty() || column_part.is_empty() {
            return None;
        }

        // Resolve the table/alias part
        let resolver = AliasResolver::new(self.catalog.clone());
        if let ResolutionResult::Found(table) =
            resolver.resolve(table_part.to_string()).await.ok()?
        {
            // Get columns from the resolved table
            if let Ok(columns) = self.catalog.get_columns(&table.table_name).await {
                let column_lower = column_part.to_lowercase();
                for column in columns {
                    if column.name.to_lowercase() == column_lower {
                        let hover_info = ColumnHoverInfo {
                            name: column.name.clone(),
                            data_type: column.data_type.clone(),
                            is_primary_key: column.is_primary_key,
                            is_foreign_key: column.is_foreign_key,
                        };
                        return Some(self.hover_provider.get_column_hover(&hover_info));
                    }
                }
            }
        }

        None
    }

    /// Check if a node is in a FROM clause
    fn is_in_from_clause(&self, node: &Node<'_>) -> bool {
        // Walk up the tree to find if we're inside a FROM clause
        let mut current = *node;
        while current.is_named() {
            if current.kind() == "from_clause" || current.kind() == "join_clause" {
                return true;
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }
        false
    }

    /// Check if a node is in a SELECT, WHERE, or similar clause where columns appear
    fn is_in_select_context(&self, node: &Node<'_>) -> bool {
        // Walk up the tree to find if we're inside a SELECT/WHERE/HAVING/ORDER BY clause
        let mut current = *node;
        while current.is_named() {
            if matches!(
                current.kind(),
                "select_clause"
                    | "where_clause"
                    | "having_clause"
                    | "order_by_clause"
                    | "group_by_clause"
                    | "join_clause"
            ) {
                return true;
            }
            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                break;
            }
        }
        false
    }

    /// Get the text of a node
    fn node_text(&self, node: &Node<'_>, source: &str) -> String {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        if let Some(text) = source.get(start_byte..end_byte) {
            text.to_string()
        } else {
            String::new()
        }
    }
}
