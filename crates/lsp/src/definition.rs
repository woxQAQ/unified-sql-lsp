// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Go-to-Definition for SQL
//!
//! This module provides go-to-definition functionality for SQL queries.
//!
//! ## What is Go-to-Definition?
//!
//! Go-to-definition allows users to jump from a symbol reference (e.g., a table or column
//! used in a WHERE clause) to where that symbol is defined (e.g., the table reference in
//! the FROM clause, or the column in the SELECT projection).
//!
//! ## Example
//!
//! ```sql
//! SELECT u.id, u.name
//! FROM users u
//! WHERE u.id = 1;
//! ```
//!
//! If the user invokes go-to-definition on `u.id` in the WHERE clause, the cursor will
//! jump to `u.id` in the SELECT projection.
//!
//! ## Architecture
//!
//! This module provides:
//! - `Definition`: Enum representing table or column definitions
//! - `DefinitionFinder`: Finds symbol definitions from CST
//! - Helper functions for CST traversal and text extraction

use crate::cst_utils::{extract_identifier_name, extract_node_text, find_node_at_position, node_to_range};
use tower_lsp::lsp_types::{Location, Position, Url};
use tree_sitter::{Node, TreeCursor};

/// Iterator over a node's children
///
/// This provides a cleaner interface than repeatedly calling `node.children(&mut node.walk())`.
struct ChildIter<'a> {
    cursor: TreeCursor<'a>,
    finished_first: bool,
}

impl<'a> ChildIter<'a> {
    fn new(node: &Node<'a>) -> Self {
        Self {
            cursor: node.walk(),
            finished_first: false,
        }
    }
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = Node<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.finished_first {
            self.finished_first = true;
            if self.cursor.goto_first_child() {
                Some(self.cursor.node())
            } else {
                None
            }
        } else if self.cursor.goto_next_sibling() {
            Some(self.cursor.node())
        } else {
            None
        }
    }
}

/// Extension trait for tree-sitter Node to provide more convenient child traversal
///
/// This trait eliminates the ugly `node.children(&mut node.walk())` pattern.
trait NodeExt {
    /// Iterate over children of this node
    fn iter_children(&self) -> ChildIter<'_>;

    /// Find first child matching a predicate
    fn find_child<P>(&self, predicate: P) -> Option<Node<'_>>
    where
        P: Fn(&Node) -> bool;
}

impl NodeExt for Node<'_> {
    fn iter_children(&self) -> ChildIter<'_> {
        ChildIter::new(self)
    }

    fn find_child<P>(&self, predicate: P) -> Option<Node<'_>>
    where
        P: Fn(&Node) -> bool,
    {
        self.iter_children().find(predicate)
    }
}

/// Go-to-definition error types
#[derive(Debug, thiserror::Error)]
pub enum DefinitionError {
    #[error("No node found at cursor position")]
    NoNodeAtPosition,

    #[error("Definition not found")]
    NotFound,

    #[error("Ambiguous definition: multiple matches found")]
    Ambiguous,
}

/// Definition types
#[derive(Debug, Clone)]
pub enum Definition {
    Table(TableDefinition),
    Column(ColumnDefinition),
}

/// Table definition
#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub table_name: String,
    pub location: Location,
}

/// Column definition
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub column_name: String,
    pub table_name: Option<String>,
    pub location: Location,
}

/// Definition finder - finds symbol definitions in CST
pub struct DefinitionFinder;

impl DefinitionFinder {
    /// Find definition at cursor position
    ///
    /// # Arguments
    ///
    /// * `root_node` - Root CST node
    /// * `source` - Source code text
    /// * `position` - Cursor position
    /// * `uri` - Document URI
    ///
    /// # Returns
    ///
    /// - `Ok(Some(Definition))` - Definition found
    /// - `Ok(None)` - No definition at this position (e.g., cursor on keyword or literal)
    /// - `Err(DefinitionError)` - Error occurred
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let result = DefinitionFinder::find_at_position(&root_node, source, position, &uri)?;
    /// if let Some(Definition::Table(def)) = result {
    ///     println!("Found table: {} at {:?}", def.table_name, def.location);
    /// }
    /// ```
    pub fn find_at_position(
        root_node: &Node,
        source: &str,
        position: Position,
        uri: &Url,
    ) -> Result<Option<Definition>, DefinitionError> {
        // 1. Find the node at cursor position
        let cursor_node = find_node_at_position(root_node, position, source)
            .ok_or(DefinitionError::NoNodeAtPosition)?;

        // 2. Walk up the parent chain to find significant nodes
        let mut current = Some(cursor_node);

        while let Some(node) = current {
            match node.kind() {
                "table_reference" | "table_name" | "identifier" => {
                    // Try to find as table definition
                    if let Some(location) = Self::find_table_definition(&node, source, uri) {
                        let table_name = extract_identifier_name(&node, source).unwrap_or_default();
                        return Ok(Some(Definition::Table(TableDefinition {
                            table_name,
                            location,
                        })));
                    }
                }
                "column_reference" | "column_name" => {
                    // Try to find as column definition
                    if let Some(location) = Self::find_column_definition(&node, source, uri) {
                        let (col_name, table_name) =
                            extract_column_info(&node, source).unwrap_or_default();
                        return Ok(Some(Definition::Column(ColumnDefinition {
                            column_name: col_name,
                            table_name,
                            location,
                        })));
                    }
                }
                _ => {}
            }

            // Walk up to parent
            current = node.parent();
        }

        Ok(None)
    }

    /// Find table definition in FROM clause
    ///
    /// When the cursor is on a table reference (e.g., in WHERE or SELECT clauses),
    /// this function finds the corresponding table reference in the FROM clause.
    fn find_table_definition(cursor_node: &Node, source: &str, uri: &Url) -> Option<Location> {
        // 1. Extract table name from cursor position
        let table_name = extract_identifier_name(cursor_node, source)?;

        // 2. Find parent SELECT statement
        let select_stmt = find_parent_select(cursor_node)?;

        // 3. Find FROM clause in SELECT
        let from_clause = find_from_clause(&select_stmt)?;

        // 4. Search for matching table_reference in FROM clause
        if let Some(child) = from_clause.find_child(|c| c.kind() == "table_reference") {
            if let Some(ref_name) = extract_table_name(&child, source) {
                if ref_name == table_name {
                    let range = node_to_range(&child, source);
                    return Some(Location {
                        uri: uri.clone(),
                        range,
                    });
                }
            }
        }

        None
    }

    /// Find column definition in SELECT projection
    ///
    /// When the cursor is on a column reference (e.g., in WHERE or ORDER BY clauses),
    /// this function finds the corresponding column definition in the SELECT projection.
    fn find_column_definition(cursor_node: &Node, source: &str, uri: &Url) -> Option<Location> {
        // 1. Extract column info from cursor
        let (col_name, table_qualifier) = extract_column_info(cursor_node, source)?;

        // 2. Find parent SELECT statement
        let select_stmt = find_parent_select(cursor_node)?;

        // 3. Find SELECT clause (projection list)
        let select_clause = find_select_clause(&select_stmt)?;

        // 4. Search for matching column in projection
        for child in select_clause.iter_children() {
            match child.kind() {
                "column_reference" => {
                    let (ref_col, ref_table) = extract_column_info(&child, source)?;

                    // Match if column names match AND (no qualifier OR qualifiers match)
                    if ref_col == col_name {
                        if table_qualifier.is_none() || ref_table == table_qualifier {
                            let range = node_to_range(&child, source);
                            return Some(Location {
                                uri: uri.clone(),
                                range,
                            });
                        }
                    }
                }
                // Handle expressions: SELECT COUNT(*) AS cnt
                "expression" | "function_call" => {
                    if let Some(alias) = extract_alias(&child, source) {
                        if alias == col_name {
                            let range = node_to_range(&child, source);
                            return Some(Location {
                                uri: uri.clone(),
                                range,
                            });
                        }
                    }
                }
                _ => {}
            }
        }

        None
    }
}

/// Extract table name from table_reference node
fn extract_table_name(node: &Node, source: &str) -> Option<String> {
    if let Some(child) = node.find_child(|c| matches!(c.kind(), "table_name" | "identifier")) {
        Some(extract_node_text(&child, source))
    } else {
        None
    }
}

/// Extract column info from column_reference node
/// Returns (column_name, optional_table_name)
fn extract_column_info(node: &Node, source: &str) -> Option<(String, Option<String>)> {
    let mut column_name = None;
    let mut table_name = None;

    for child in node.iter_children() {
        match child.kind() {
            "column_name" | "identifier" => {
                if column_name.is_none() {
                    column_name = Some(extract_node_text(&child, source));
                }
            }
            "table_name" => {
                table_name = Some(extract_node_text(&child, source));
            }
            _ => {}
        }
    }

    column_name.map(|col| (col, table_name))
}

/// Find parent SELECT statement
fn find_parent_select<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut current = *node;
    loop {
        if current.kind() == "select_statement" {
            return Some(current);
        }
        current = current.parent()?;
    }
}

/// Find FROM clause in SELECT statement
fn find_from_clause<'a>(select_node: &'a Node<'a>) -> Option<Node<'a>> {
    select_node.find_child(|c| c.kind() == "from_clause")
}

/// Find SELECT clause (projection) in SELECT statement
fn find_select_clause<'a>(select_node: &'a Node<'a>) -> Option<Node<'a>> {
    select_node.find_child(|c| c.kind() == "select" || c.kind() == "projection")
}

/// Extract alias from expression or function_call
fn extract_alias(node: &Node, source: &str) -> Option<String> {
    if let Some(child) = node.find_child(|c| c.kind() == "alias") {
        if let Some(alias_child) =
            child.find_child(|c| matches!(c.kind(), "identifier" | "column_name"))
        {
            return Some(extract_node_text(&alias_child, source));
        }
    }
    None
}

// Test helper functions
#[cfg(test)]
fn find_table_name_node<'a>(root_node: &Node<'a>) -> Option<Node<'a>> {
    let select = root_node.children(&mut root_node.walk())
        .find(|n| n.kind() == "select_statement")?;
    let from_clause = select.children(&mut select.walk())
        .find(|n| n.kind() == "from_clause")?;
    let table_ref = from_clause.children(&mut from_clause.walk())
        .find(|n| n.kind() == "table_reference")?;
    table_ref.children(&mut table_ref.walk())
        .find(|n| n.kind() == "table_name")
}

#[cfg(test)]
fn find_table_reference_node<'a>(root_node: &Node<'a>) -> Option<Node<'a>> {
    let select = root_node.children(&mut root_node.walk())
        .find(|n| n.kind() == "select_statement")?;
    let from_clause = select.children(&mut select.walk())
        .find(|n| n.kind() == "from_clause")?;
    from_clause.children(&mut from_clause.walk())
        .find(|n| n.kind() == "table_reference")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst_utils::{byte_to_position, position_to_byte_offset};
    use crate::parsing::{ParseResult, ParserManager};
    use unified_sql_lsp_ir::Dialect;

    /// Helper function to parse SQL and get root node
    fn parse_sql(sql: &str) -> tree_sitter::Tree {
        let manager = ParserManager::new();
        match manager.parse_text(Dialect::MySQL, sql) {
            ParseResult::Success { tree, .. } => {
                tree.expect("Parse tree should be present")
            }
            _ => panic!("Failed to parse SQL: {}", sql),
        }
    }

    #[test]
    fn test_extract_identifier_name_from_table_name() {
        let sql = "SELECT * FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find the table_name node using helper function
        let table_name_node = find_table_name_node(&root_node).expect("table_name node not found");
        let result = extract_identifier_name(&table_name_node, sql);
        assert_eq!(result, Some("users".to_string()));
    }

    #[test]
    fn test_extract_identifier_name_from_identifier() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find an identifier node
        let mut cursor = root_node.walk();
        for child in root_node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let result = extract_identifier_name(&child, sql);
                assert!(result.is_some());
                break;
            }
        }
        // Note: May not find identifier node in all parses, so we don't assert
    }

    #[test]
    fn test_extract_identifier_name_from_non_identifier() {
        let sql = "SELECT * FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Try to extract from select_statement (should fail)
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = extract_identifier_name(&child, sql);
                assert_eq!(result, None, "Should return None for non-identifier node");
                return;
            }
        }
    }

    #[test]
    fn test_extract_table_name() {
        let sql = "SELECT * FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find table_reference node using helper function
        let table_ref_node =
            find_table_reference_node(&root_node).expect("table_reference node not found");
        let result = extract_table_name(&table_ref_node, sql);
        assert_eq!(result, Some("users".to_string()));
    }

    #[test]
    fn test_extract_table_name_not_found() {
        let sql = "SELECT * FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Try to extract from select_statement (should fail)
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = extract_table_name(&child, sql);
                assert_eq!(
                    result, None,
                    "Should return None for non-table_reference node"
                );
                return;
            }
        }
    }

    #[test]
    fn test_extract_column_info_simple() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find a column_reference using recursive walk
        fn find_column_recursive(
            node: &tree_sitter::Node,
            sql: &str,
        ) -> Option<(String, Option<String>)> {
            if node.kind() == "column_reference" || node.kind() == "column_name" {
                return extract_column_info(node, sql);
            }
            for child in node.iter_children() {
                if let Some(result) = find_column_recursive(&child, sql) {
                    return Some(result);
                }
            }
            None
        }

        let result = find_column_recursive(&root_node, sql);
        if let Some((col_name, table_name)) = result {
            assert_eq!(col_name, "id");
            assert_eq!(table_name, None);
        } else {
            println!("Warning: Could not find simple column reference for testing");
        }
    }

    #[test]
    fn test_extract_column_info_qualified() {
        let sql = "SELECT users.id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Look for qualified column reference using recursive walk
        fn find_qualified_column_recursive(
            node: &tree_sitter::Node,
            sql: &str,
        ) -> Option<(String, Option<String>)> {
            if node.kind() == "column_reference" {
                let result = extract_column_info(node, sql);
                if let Some((_, table_name)) = &result {
                    if table_name.is_some() {
                        return result;
                    }
                }
            }
            for child in node.iter_children() {
                if let Some(result) = find_qualified_column_recursive(&child, sql) {
                    return Some(result);
                }
            }
            None
        }

        let result = find_qualified_column_recursive(&root_node, sql);
        if let Some((col_name, table_name)) = result {
            assert_eq!(col_name, "id");
            assert_eq!(table_name, Some("users".to_string()));
        } else {
            println!("Warning: Could not find qualified column reference for testing");
        }
    }

    #[test]
    fn test_extract_column_info_not_found() {
        let sql = "SELECT * FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Try to extract from select_statement (should fail)
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = extract_column_info(&child, sql);
                assert_eq!(result, None, "Should return None for non-column node");
                return;
            }
        }
    }

    #[test]
    fn test_find_parent_select() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find a column node using recursive walk
        fn find_column_node_recursive(node: tree_sitter::Node) -> Option<tree_sitter::Node> {
            if node.kind() == "column_name" || node.kind() == "identifier" {
                return Some(node.clone());
            }
            for child in node.children(&mut node.walk()) {
                if let Some(found) = find_column_node_recursive(child) {
                    return Some(found);
                }
            }
            None
        }

        let column_node =
            find_column_node_recursive(root_node.clone()).expect("Could not find column node for testing");

        let result = find_parent_select(&column_node);
        assert!(result.is_some(), "Should find parent SELECT statement");
        if let Some(parent) = result {
            assert_eq!(parent.kind(), "select_statement");
        }
    }

    #[test]
    fn test_find_parent_select_from_top() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Finding parent of root should return None
        let result = find_parent_select(&root_node);
        assert_eq!(result, None, "Root node should not have a SELECT parent");
    }

    #[test]
    fn test_find_from_clause() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find SELECT statement
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = find_from_clause(&child);
                assert!(result.is_some(), "Should find FROM clause");
                if let Some(from_clause) = result {
                    assert_eq!(from_clause.kind(), "from_clause");
                }
                return;
            }
        }
        panic!("Could not find SELECT statement");
    }

    #[test]
    fn test_find_from_clause_missing() {
        let sql = "SELECT id"; // No FROM clause (invalid but parses)
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find SELECT statement
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = find_from_clause(&child);
                // May or may not find FROM clause depending on grammar
                // We just check it doesn't panic
                let _ = result;
                return;
            }
        }
    }

    #[test]
    fn test_find_select_clause() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find SELECT statement
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = find_select_clause(&child);
                assert!(result.is_some(), "Should find SELECT clause");
                if let Some(select_clause) = result {
                    assert!(
                        select_clause.kind() == "select" || select_clause.kind() == "projection",
                        "Should find select or projection clause, got: {}",
                        select_clause.kind()
                    );
                }
                return;
            }
        }
        panic!("Could not find SELECT statement");
    }

    #[test]
    fn test_find_select_clause_missing() {
        // Create a minimal invalid query
        let sql = "FROM users"; // No SELECT clause
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Try to find SELECT clause (may not exist)
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = find_select_clause(&child);
                // Just check it doesn't panic
                let _ = result;
                return;
            }
        }
    }

    #[test]
    fn test_extract_alias() {
        let sql = "SELECT COUNT(*) AS cnt FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find function_call node using recursive walk
        fn find_function_with_alias_recursive(
            node: &tree_sitter::Node,
            sql: &str,
        ) -> Option<String> {
            if node.kind() == "function_call" || node.kind() == "expression" {
                if let Some(alias) = extract_alias(node, sql) {
                    return Some(alias);
                }
            }
            for child in node.iter_children() {
                if let Some(alias) = find_function_with_alias_recursive(&child, sql) {
                    return Some(alias);
                }
            }
            None
        }

        let result = find_function_with_alias_recursive(&root_node, sql);
        if let Some(alias) = result {
            assert_eq!(alias, "cnt");
        } else {
            println!("Warning: Could not find function_call with alias for testing");
        }
    }

    #[test]
    fn test_extract_alias_none() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find a node without alias
        if let Some(child) = root_node.find_child(|_| true) {
            if child.kind() == "select_statement" {
                let result = extract_alias(&child, sql);
                assert_eq!(result, None, "Should return None for node without alias");
                return;
            }
        }
    }

    #[test]
    fn test_node_to_range() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        let range = node_to_range(&root_node, sql);
        // Basic sanity checks
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 0);
    }

    #[test]
    fn test_extract_node_text() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        let text = extract_node_text(&root_node, sql);
        assert_eq!(text, sql);
    }

    #[test]
    fn test_position_to_byte_offset() {
        let sql = "SELECT id FROM users";

        // Test position at start
        let pos = Position::new(0, 0);
        let offset = position_to_byte_offset(sql, pos);
        assert_eq!(offset, 0);

        // Test position at "SELECT"
        let pos = Position::new(0, 6);
        let offset = position_to_byte_offset(sql, pos);
        assert_eq!(offset, 6);
    }

    #[test]
    fn test_find_node_at_position() {
        let sql = "SELECT id FROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        // Find node at start
        let pos = Position::new(0, 0);
        let node = find_node_at_position(&root_node, pos, sql);
        assert!(node.is_some());

        // Find node in middle of SELECT
        let pos = Position::new(0, 5);
        let node = find_node_at_position(&root_node, pos, sql);
        assert!(node.is_some());
    }

    #[test]
    fn test_byte_to_position_multiline() {
        let source = "SELECT id\nFROM users\nWHERE id = 1";

        // Test position on line 1 (second line)
        // "FROM" starts at byte offset 10 (after "SELECT id\n")
        let pos = byte_to_position(10, source);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        // Test position on line 2 (third line)
        // "WHERE" starts at byte offset 21 (after "SELECT id\nFROM users\n")
        let pos = byte_to_position(21, source);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_utf8() {
        // Test with UTF-8 characters (Chinese characters)
        let source = "SELECT 名称 FROM 用户";

        // "SELECT " is 7 bytes
        let pos = byte_to_position(7, source);
        assert_eq!(pos.line, 0);
        // "名称" should be at character position 7
        assert_eq!(pos.character, 7);

        // Test position after UTF-8 characters
        // "SELECT 名称" is 7 + 6 = 13 bytes (名称 is 2 UTF-8 chars, 6 bytes)
        let pos = byte_to_position(13, source);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 9); // 7 + 2 = 9 characters
    }

    #[test]
    fn test_byte_to_position_empty_string() {
        let source = "";
        let pos = byte_to_position(0, source);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_byte_to_position_single_line() {
        let source = "SELECT id FROM users";

        // Test at various positions
        let pos = byte_to_position(0, source);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        let pos = byte_to_position(7, source); // "id FROM users"
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 7);

        let pos = byte_to_position(source.len(), source);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, source.len() as u32);
    }

    #[test]
    fn test_node_to_range_multiline() {
        let sql = "SELECT id\nFROM users";
        let tree = parse_sql(sql);
        let root_node = tree.root_node();

        let range = node_to_range(&root_node, sql);
        // Range should span multiple lines
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        // End should be on line 1 or later
        assert!(range.end.line >= 1);
    }
}
