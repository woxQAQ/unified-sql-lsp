// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Hover-oriented semantic helpers.

use std::sync::Arc;
use tree_sitter::Node;
use unified_sql_lsp_catalog::{Catalog, ColumnMetadata};

use crate::{AliasResolver, ResolutionResult};

/// Semantic hover helper service.
pub struct HoverService {
    catalog: Arc<dyn Catalog>,
}

impl HoverService {
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    /// Extract the word at a given node.
    pub fn extract_word_at_node(&self, node: &Node<'_>, source: &str) -> Option<String> {
        let node_text = self.node_text(node, source);

        if matches!(
            node.kind(),
            "identifier" | "table_name" | "column_name" | "function_name" | "alias"
        ) {
            let text = node_text
                .trim()
                .trim_start_matches('`')
                .trim_end_matches('`')
                .trim_start_matches('"')
                .trim_end_matches('"')
                .to_string();
            return Some(text);
        }

        if node.kind() == "object_reference" || node.kind() == "qualified_column_name" {
            return Some(node_text.trim().to_string());
        }

        None
    }

    /// Check if a node is in a FROM/JOIN clause.
    pub fn is_in_from_clause(&self, node: &Node<'_>) -> bool {
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

    /// Check if a node is in clauses where column references commonly appear.
    pub fn is_in_select_context(&self, node: &Node<'_>) -> bool {
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

    /// Resolve a visible table by name.
    pub async fn resolve_table_name(&self, word: &str) -> Option<String> {
        let tables = self.catalog.list_tables().await.ok()?;
        let word_lower = word.to_lowercase();
        for table in tables {
            if table.name.to_lowercase() == word_lower {
                return Some(table.name);
            }
        }
        None
    }

    /// Resolve alias to concrete table name.
    pub async fn resolve_alias_table(
        &self,
        alias: &str,
        visible_tables: &[String],
    ) -> Option<String> {
        if visible_tables.is_empty() {
            return None;
        }

        let resolver = AliasResolver::new(self.catalog.clone());
        match resolver.resolve(alias.to_string()).await.ok()? {
            ResolutionResult::Found(table) => Some(table.table_name),
            _ => None,
        }
    }

    /// Resolve unqualified column from visible tables.
    pub async fn resolve_column(
        &self,
        column_name: &str,
        visible_tables: &[String],
    ) -> Option<ColumnMetadata> {
        if visible_tables.is_empty() {
            return None;
        }

        let resolver = AliasResolver::new(self.catalog.clone());
        for table_name in visible_tables {
            match resolver.resolve(table_name.clone()).await.ok()? {
                ResolutionResult::Found(table) => {
                    if let Some(col) = self.lookup_column(&table.table_name, column_name).await {
                        return Some(col);
                    }
                }
                _ => continue,
            }
        }
        None
    }

    /// Resolve column using simple text fallback by extracting FROM table.
    pub async fn resolve_simple_column(
        &self,
        column_name: &str,
        source: &str,
    ) -> Option<ColumnMetadata> {
        let source_upper = source.to_uppercase();
        if let Some(from_pos) = source_upper.find(" FROM ") {
            let after_from = &source[from_pos + 6..];
            let table_name = after_from
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_')
                .to_lowercase();
            if !table_name.is_empty() {
                return self.lookup_column(&table_name, column_name).await;
            }
        }
        None
    }

    /// Resolve table-qualified column reference (`table.column`).
    pub async fn resolve_qualified_column(
        &self,
        node: &Node<'_>,
        source: &str,
    ) -> Option<ColumnMetadata> {
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

        let resolver = AliasResolver::new(self.catalog.clone());
        if let ResolutionResult::Found(table) =
            resolver.resolve(table_part.to_string()).await.ok()?
        {
            return self.lookup_column(&table.table_name, column_part).await;
        }
        None
    }

    fn node_text(&self, node: &Node<'_>, source: &str) -> String {
        let start_byte = node.start_byte();
        let end_byte = node.end_byte();
        source
            .get(start_byte..end_byte)
            .map(ToString::to_string)
            .unwrap_or_default()
    }

    async fn lookup_column(&self, table_name: &str, column_name: &str) -> Option<ColumnMetadata> {
        let columns = self.catalog.get_columns(table_name).await.ok()?;
        let column_lower = column_name.to_lowercase();
        columns
            .into_iter()
            .find(|column| column.name.to_lowercase() == column_lower)
    }
}
