// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Scope building from CST
//!
//! This module builds semantic scopes from tree-sitter CST nodes for completion.
//!
//! ## What are Semantic Scopes?
//!
//! In SQL, a "scope" tracks which tables and columns are visible at a given position
//! in the query. For completion, we need to know:
//!
//! - **Table symbols**: Which tables are referenced in FROM clause (with aliases)
//! - **Column symbols**: Which columns exist on each table (fetched from catalog)
//! - **Visibility**: Which tables/columns are accessible at cursor position
//!
//! ## Example
//!
//! ```sql
//! SELECT u.↰              -- Cursor here: scope knows 'users' table with alias 'u'
//! FROM users AS u;
//! ```
//!
//! The scope builder extracts table references from the FROM clause and creates
//! `TableSymbol` entries that can later be populated with column metadata from
//! the database catalog.
//!
//! ## Architecture
//!
//! This is a simplified version of the full semantic analysis (SEMANTIC-002):
//! - Only builds scopes for SELECT statements (not INSERT/UPDATE/DELETE)
//! - Only extracts table references (not full column resolution)
//! - Populated incrementally by catalog during completion (not upfront)
//!
//! TODO: (SEMANTIC-002) Replace with full semantic analyzer when available

use crate::completion::error::CompletionError;
use std::collections::HashMap;
use tree_sitter::Node;
use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol};

/// Scope builder for completion
///
/// Builds semantic scopes from tree-sitter CST nodes.
pub struct ScopeBuilder;

impl ScopeBuilder {
    /// Build scopes from a SELECT statement
    ///
    /// # Arguments
    ///
    /// * `select_node` - The select_statement node
    /// * `source` - Source code text
    ///
    /// # Returns
    ///
    /// A ScopeManager with tables extracted from the FROM clause
    ///
    /// # Examples
    ///
    /// ```
    /// # use unified_sql_lsp_lsp::completion::scopes::ScopeBuilder;
    /// # use tree_sitter::Parser;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let source = "SELECT id FROM users";
    /// # let mut parser = Parser::new();
    /// # let tree = parser.parse(source, None).unwrap();
    /// # let select_node = tree.root_node();
    /// let manager = ScopeBuilder::build_from_select(&select_node, source)?;
    /// # let _ = manager;
    /// # Ok(())
    /// # }
    /// ```
    pub fn build_from_select(
        select_node: &Node,
        source: &str,
    ) -> Result<ScopeManager, CompletionError> {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        // Find the FROM clause
        let from_clause =
            Self::find_from_clause(select_node).ok_or(CompletionError::NoFromClause)?;

        // Extract table references
        let tables = Self::extract_table_references(&from_clause, source)?;

        // Add tables to scope
        let scope = manager.get_scope_mut(scope_id).unwrap();
        for table in tables {
            scope.add_table(table)?;
        }

        Ok(manager)
    }

    /// Find the FROM clause in a SELECT statement
    fn find_from_clause<'a>(select_node: &'a Node) -> Option<Node<'a>> {
        for child in select_node.children(&mut select_node.walk()) {
            if child.kind() == "from_clause" {
                return Some(child);
            }
        }
        None
    }

    /// Extract table references from a FROM clause
    ///
    /// Parses table_reference nodes and extracts table names and aliases.
    fn extract_table_references(
        from_clause: &Node,
        source: &str,
    ) -> Result<Vec<TableSymbol>, CompletionError> {
        let mut tables = Vec::new();
        let mut table_counts: HashMap<String, usize> = HashMap::new();

        // Find all table_reference nodes
        Self::extract_tables_recursive(from_clause, source, &mut tables, &mut table_counts)?;

        Ok(tables)
    }

    /// Recursively extract table references from a node
    fn extract_tables_recursive(
        node: &Node,
        source: &str,
        tables: &mut Vec<TableSymbol>,
        table_counts: &mut HashMap<String, usize>,
    ) -> Result<(), CompletionError> {
        match node.kind() {
            "table_reference" => {
                let table = Self::parse_table_reference(node, source)?;

                // Check for duplicate table names
                let display_name = table.display_name().to_string();
                *table_counts.entry(display_name.clone()).or_insert(0) += 1;

                if table_counts[&display_name] == 1 {
                    tables.push(table);
                } else {
                    return Err(CompletionError::ScopeBuild(format!(
                        "Duplicate table reference: {}",
                        display_name
                    )));
                }

                Ok(())
            }
            _ => {
                // Recurse into children
                for child in node.children(&mut node.walk()) {
                    Self::extract_tables_recursive(&child, source, tables, table_counts)?;
                }
                Ok(())
            }
        }
    }

    /// Parse a table_reference node
    ///
    /// Extracts the table name and optional alias.
    /// Supports formats:
    /// - `table_name`
    /// - `table_name AS alias`
    /// - `table_name alias` (implicit alias)
    /// - `schema.table_name`
    fn parse_table_reference(node: &Node, source: &str) -> Result<TableSymbol, CompletionError> {
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
                    // Extract alias
                    if let Some(a) = Self::extract_alias(&child, source) {
                        alias = Some(a);
                    }
                }
                "AS" => {
                    // AS keyword, next identifier is the alias
                    continue;
                }
                _ => {
                    // Check for identifier that might be an implicit alias
                    if table_name.is_some() && alias.is_none() && child.kind() == "identifier" {
                        // This might be an implicit alias
                        let text = Self::extract_node_text(&child, source);
                        if &text != table_name.as_ref().unwrap() {
                            alias = Some(text);
                        }
                    }
                }
            }
        }

        let table_name = table_name.ok_or_else(|| {
            CompletionError::ScopeBuild("Table name not found in table_reference".to_string())
        })?;

        let mut table = TableSymbol::new(&table_name);
        if let Some(a) = alias {
            table = table.with_alias(a);
        }
        Ok(table)
    }

    /// Extract text from a node
    fn extract_node_text(node: &Node, source: &str) -> String {
        let bytes = node.byte_range();
        source[bytes].to_string()
    }

    /// Extract alias from an alias node
    fn extract_alias(alias_node: &Node, source: &str) -> Option<String> {
        // Find the identifier in the alias node
        for child in alias_node.children(&mut alias_node.walk()) {
            if child.kind() == "identifier" {
                return Some(Self::extract_node_text(&child, source));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    // Note: Full integration tests with real tree-sitter parsing
    // will be in the tests module. Here we test the logic
    // with mock nodes where possible.
}
