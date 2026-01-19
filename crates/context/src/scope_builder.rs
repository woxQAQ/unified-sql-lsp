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

use std::collections::HashMap;
use tree_sitter::Node;
use tracing::warn;
use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol};

/// Scope builder error
#[derive(Debug, thiserror::Error)]
pub enum ScopeBuildError {
    #[error("No FROM clause found")]
    NoFromClause,

    #[error("Scope build error: {0}")]
    ScopeBuild(String),

    #[error("Semantic error: {0}")]
    SemanticError(#[from] unified_sql_lsp_semantic::SemanticError),
}

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
    /// # use unified_sql_lsp_context::scope_builder::ScopeBuilder;
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
    ) -> Result<ScopeManager, ScopeBuildError> {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        // Find the FROM clause
        let from_clause =
            Self::find_from_clause(select_node).ok_or(ScopeBuildError::NoFromClause)?;

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
    pub fn find_from_clause<'a>(select_node: &'a Node) -> Option<Node<'a>> {
        select_node
            .children(&mut select_node.walk())
            .find(|&child| child.kind() == "from_clause")
    }

    /// Extract table references from a FROM clause
    ///
    /// Parses table_reference nodes and extracts table names and aliases.
    pub fn extract_table_references(
        from_clause: &Node,
        source: &str,
    ) -> Result<Vec<TableSymbol>, ScopeBuildError> {
        let mut tables = Vec::new();
        let mut table_counts: HashMap<String, usize> = HashMap::new();

        // Find all table_reference nodes
        Self::extract_tables_recursive(from_clause, source, &mut tables, &mut table_counts)?;

        Ok(tables)
    }

    /// Recursively extract table references from a node
    ///
    /// Phase 3: Allows duplicate table references (self-joins) as long as they have different aliases
    fn extract_tables_recursive(
        node: &Node,
        source: &str,
        tables: &mut Vec<TableSymbol>,
        table_counts: &mut HashMap<String, usize>,
    ) -> Result<(), ScopeBuildError> {
        if node.kind() == "table_reference" {
            let table = Self::parse_table_reference(node, source)?;
            let display_name = table.display_name().to_string();
            *table_counts.entry(display_name.clone()).or_insert(0) += 1;

            // Phase 3: Remove duplicate restriction - allow self-joins with different aliases
            // For example: "FROM users AS u1 JOIN users AS u2" should be allowed
            // The duplicate check is removed because different aliases make them distinct
            tables.push(table);
            return Ok(());
        }

        // Handle join_clause nodes (Phase 1: JOIN alias support)
        if node.kind() == "join_clause" {
            match Self::parse_join_clause(node, source) {
                Ok(table) => {
                    let display_name = table.display_name().to_string();
                    *table_counts.entry(display_name.clone()).or_insert(0) += 1;
                    tables.push(table);
                }
                Err(e) => {
                    warn!("Failed to parse JOIN clause: {}", e);
                    // Continue - don't fail entire scope build
                }
            }
            return Ok(());
        }

        // Recurse into children
        for child in node.children(&mut node.walk()) {
            Self::extract_tables_recursive(&child, source, tables, table_counts)?;
        }
        Ok(())
    }

    /// Parse a table_reference node
    ///
    /// Extracts the table name and optional alias.
    /// Supports formats:
    /// - `table_name`
    /// - `table_name AS alias`
    /// - `table_name alias` (implicit alias)
    /// - `schema.table_name`
    pub fn parse_table_reference(
        node: &Node,
        source: &str,
    ) -> Result<TableSymbol, ScopeBuildError> {
        let mut table_name = None;
        let mut alias = None;

        // Walk through children to find table name and alias
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "table_name" | "identifier" => {
                    let text = Self::extract_node_text(&child, source);
                    if table_name.is_none() {
                        table_name = Some(text);
                    } else if alias.is_none() {
                        // Second identifier is the implicit alias
                        alias = Some(text);
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
                    // Ignore other nodes
                }
            }
        }

        let table_name = table_name.ok_or_else(|| {
            ScopeBuildError::ScopeBuild("Table name not found in table_reference".to_string())
        })?;

        let mut table = TableSymbol::new(&table_name);
        if let Some(a) = alias {
            table = table.with_alias(a);
        }
        Ok(table)
    }

    /// Parse a join_clause node to extract table and alias
    ///
    /// # Phase 1: JOIN Alias Support
    ///
    /// Supports formats:
    /// - `JOIN table_name [AS alias]`
    /// - `JOIN (subquery) [AS alias]` (future: Phase 2)
    ///
    /// # Grammar Structure
    ///
    /// From `crates/grammar/src/grammar/grammar.js`:
    /// ```text
    /// join_clause: seq(
    ///   optional($.join_type),      // INNER, LEFT, RIGHT, FULL, CROSS
    ///   /[Jj][Oo][Ii][Nn]/,
    ///   $.table_name,               // The table being joined
    ///   optional(seq(/[Aa][Ss]/, $.alias)),  // Optional AS alias
    ///   /[Oo][Nn]/,                 // ON keyword
    ///   $.expression                // Join condition
    /// )
    /// ```
    ///
    /// # Examples
    ///
    /// ```sql
    /// -- Simple join with alias
    /// JOIN orders o ON u.id = o.user_id
    ///
    /// -- Join without alias
    /// JOIN orders ON users.id = orders.user_id
    ///
    /// -- Different join types
    /// LEFT JOIN orders o ON u.id = o.user_id
    /// INNER JOIN orders o ON u.id = o.user_id
    /// ```
    pub fn parse_join_clause(
        node: &Node,
        source: &str,
    ) -> Result<TableSymbol, ScopeBuildError> {
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
                // Ignore: join_type, "ON", expression, etc.
                _ => {}
            }
        }

        let table_name = table_name.ok_or_else(|| {
            ScopeBuildError::ScopeBuild("Table name not found in join_clause".to_string())
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
    use super::*;
    use tree_sitter::Parser;
    use unified_sql_grammar::{DialectVersion, language_for_dialect_with_version};
    use unified_sql_lsp_ir::Dialect;

    /// Helper to find select_statement node anywhere in tree
    fn find_select_statement<'a>(node: &Node<'a>) -> Option<Node<'a>> {
        if node.kind() == "select_statement" {
            return Some(node.clone());
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_select_statement(&child) {
                return Some(found);
            }
        }

        None
    }

    #[test]
    fn test_extract_join_with_alias() {
        let sql = "SELECT u.id FROM users AS u JOIN orders AS o ON u.id = o.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        // Find the select_statement node anywhere in the tree
        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        // Use the public API
        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        // Should have 2 tables: users (u) and orders (o)
        assert_eq!(tables.len(), 2, "Expected 2 tables, found {}", tables.len());

        // Check users table with alias
        let users_table = tables
            .iter()
            .find(|t| t.table_name == "users")
            .expect("Users table not found");
        assert_eq!(users_table.alias, Some("u".to_string()));

        // Check orders table with alias
        let orders_table = tables
            .iter()
            .find(|t| t.table_name == "orders")
            .expect("Orders table not found");
        assert_eq!(orders_table.alias, Some("o".to_string()));
    }

    #[test]
    fn test_extract_join_without_alias() {
        let sql = "SELECT users.id, orders.total FROM users JOIN orders ON users.id = orders.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        assert_eq!(tables.len(), 2);

        // Both tables should have no alias
        let users_table = tables
            .iter()
            .find(|t| t.table_name == "users")
            .expect("Users table not found");
        assert_eq!(users_table.alias, None);

        let orders_table = tables
            .iter()
            .find(|t| t.table_name == "orders")
            .expect("Orders table not found");
        assert_eq!(orders_table.alias, None);
    }

    #[test]
    fn test_multiple_joins() {
        let sql = "SELECT u.id, o.total, oi.quantity \
                   FROM users AS u \
                   JOIN orders AS o ON u.id = o.user_id \
                   JOIN order_items AS oi ON o.id = oi.order_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        assert_eq!(tables.len(), 3, "Expected 3 tables");

        // Check all three tables with aliases
        let table_names: Vec<&str> = tables.iter().map(|t| t.table_name.as_str()).collect();
        assert!(table_names.contains(&"users"));
        assert!(table_names.contains(&"orders"));
        assert!(table_names.contains(&"order_items"));

        let aliases: Vec<Option<&String>> = tables.iter().map(|t| t.alias.as_ref()).collect();
        assert!(aliases.iter().all(|a| a.is_some()));
    }

    #[test]
    fn test_left_join_with_alias() {
        let sql = "SELECT u.*, o.* FROM users AS u LEFT JOIN orders AS o ON u.id = o.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        assert_eq!(tables.len(), 2);

        let users = tables.iter().find(|t| t.table_name == "users").unwrap();
        assert_eq!(users.alias, Some("u".to_string()));

        let orders = tables.iter().find(|t| t.table_name == "orders").unwrap();
        assert_eq!(orders.alias, Some("o".to_string()));
    }

    #[test]
    fn test_inner_join_with_explicit_as() {
        let sql = "SELECT u.id FROM users AS u INNER JOIN orders AS o ON u.id = o.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        assert_eq!(tables.len(), 2);

        let orders = tables.iter().find(|t| t.table_name == "orders").unwrap();
        assert_eq!(orders.alias, Some("o".to_string()));
    }

    #[test]
    fn test_mixed_from_and_join() {
        let sql = "SELECT u.id, o.total FROM users AS u JOIN orders AS o ON u.id = o.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        // users from FROM clause, orders from JOIN
        assert_eq!(tables.len(), 2);

        let users = tables.iter().find(|t| t.table_name == "users").unwrap();
        assert_eq!(users.alias, Some("u".to_string()));

        let orders = tables.iter().find(|t| t.table_name == "orders").unwrap();
        assert_eq!(orders.alias, Some("o".to_string()));
    }

    #[test]
    fn test_extract_subquery_with_alias() {
        let sql = "SELECT u.id FROM (SELECT id, name FROM users) AS u JOIN orders AS o ON u.id = o.user_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        // CST-based scope building doesn't work for subqueries due to grammar limitations
        // The tree-sitter grammar creates ERROR nodes for subquery aliases
        // Subquery completion relies on text-based extraction in completion.rs instead
        let result = ScopeBuilder::build_from_select(&select_stmt, sql);

        // Verify it fails (as expected due to CST limitations)
        assert!(result.is_err(), "CST-based scope building should fail for subqueries");

        // The workaround is that text-based extraction in completion.rs handles subqueries
        // This is documented as a known limitation
    }

    #[test]
    fn test_self_join_with_different_aliases() {
        // Phase 3: Test that self-joins with different aliases are allowed
        let sql = "SELECT u1.id, u2.name FROM users AS u1 JOIN users AS u2 ON u1.id = u2.manager_id";
        let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
            .expect("Failed to get MySQL 8.0 language");

        let mut parser = Parser::new();
        parser.set_language(&lang).expect("Failed to set language");
        let tree = parser.parse(sql, None).expect("Failed to parse SQL");
        let root = tree.root_node();

        let select_stmt = find_select_statement(&root)
            .expect("No SELECT statement found in parsed tree");

        let manager = ScopeBuilder::build_from_select(&select_stmt, sql)
            .expect("Failed to build scope");

        let scope = manager.get_scope(0).expect("No scope found");
        let tables = &scope.tables;

        // Should have 2 tables: both "users" but with different aliases "u1" and "u2"
        assert_eq!(tables.len(), 2, "Expected 2 tables for self-join");

        let users1 = tables
            .iter()
            .find(|t| t.alias == Some("u1".to_string()))
            .expect("First users table with alias u1 not found");
        assert_eq!(users1.table_name, "users");

        let users2 = tables
            .iter()
            .find(|t| t.alias == Some("u2".to_string()))
            .expect("Second users table with alias u2 not found");
        assert_eq!(users2.table_name, "users");
    }
}
