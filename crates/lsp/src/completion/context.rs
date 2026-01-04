// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion context detection
//!
//! This module provides context detection for SQL completion.
//! It analyzes the tree-sitter CST to determine what kind of
//! completion should be provided based on cursor position.

use tower_lsp::lsp_types::Position;
use tree_sitter::Node;

/// Completion context types
///
/// Represents different SQL contexts where completion can be triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// SELECT clause projection
    ///
    /// User is typing in the SELECT projection list, e.g., `SELECT id, | FROM users`
    SelectProjection {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users." if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// FROM clause
    ///
    /// User is typing in the FROM clause, e.g., `SELECT * FROM |`
    FromClause,

    /// WHERE clause
    ///
    /// User is typing in the WHERE clause, e.g., `SELECT * FROM users WHERE |`
    WhereClause,

    /// JOIN ON condition
    ///
    /// User is typing in the JOIN ON condition, e.g., `SELECT * FROM users JOIN orders ON |`
    JoinCondition {
        /// Left table in the join
        left_table: Option<String>,
        /// Right table in the join
        right_table: Option<String>,
    },

    /// Unknown context
    ///
    /// Cursor is in a position that doesn't match known completion contexts
    Unknown,
}

impl CompletionContext {
    /// Check if this is a SELECT projection context
    pub fn is_select_projection(&self) -> bool {
        matches!(self, CompletionContext::SelectProjection { .. })
    }

    /// Check if this is a FROM clause context
    pub fn is_from_clause(&self) -> bool {
        matches!(self, CompletionContext::FromClause)
    }

    /// Check if this is a WHERE clause context
    pub fn is_where_clause(&self) -> bool {
        matches!(self, CompletionContext::WhereClause)
    }

    /// Check if this is a JOIN ON condition context
    pub fn is_join_condition(&self) -> bool {
        matches!(self, CompletionContext::JoinCondition { .. })
    }
}

/// Detect the completion context based on cursor position
///
/// # Arguments
///
/// * `root` - Root node of the parsed tree
/// * `position` - Cursor position (line, character)
/// * `source` - Source code text
///
/// # Returns
///
/// The detected completion context
///
/// # Examples
///
/// ```ignore
/// let tree = parser.parse(source, None).unwrap();
/// let ctx = detect_completion_context(
///     &tree.root_node(),
///     Position::new(0, 10),
///     source
/// );
/// ```
pub fn detect_completion_context(
    root: &Node,
    position: Position,
    source: &str,
) -> CompletionContext {
    // Find the node at the cursor position
    let node = match find_node_at_position(root, position, source) {
        Some(n) => n,
        None => return CompletionContext::Unknown,
    };

    // Walk up the parent chain to find the context
    let mut current = Some(node);
    let mut qualifier = None;

    while let Some(n) = current {
        match n.kind() {
            // Check if we're after a table qualifier (e.g., "users.")
            "table_reference" => {
                // Check if cursor is after a dot in a qualified reference
                if let Some(q) = extract_qualifier(&n, source, position) {
                    qualifier = Some(q);
                }
            }

            // SELECT clause
            "select_statement" => {
                // Check if we're in the projection list
                if is_in_projection(&n, position) {
                    // Extract table names from FROM clause
                    let tables = extract_tables_from_from_clause(&n, source);
                    return CompletionContext::SelectProjection { tables, qualifier };
                }
            }

            // FROM clause
            "from_clause" => {
                return CompletionContext::FromClause;
            }

            // WHERE clause
            "where_clause" => {
                return CompletionContext::WhereClause;
            }

            // JOIN ON clause
            "join_clause" => {
                // Extract left and right table names from the join
                let (left_table, right_table) = extract_join_tables(&n, source);
                return CompletionContext::JoinCondition {
                    left_table,
                    right_table,
                };
            }

            _ => {}
        }

        current = n.parent();
    }

    CompletionContext::Unknown
}

/// Find the node at the given position
fn find_node_at_position<'a>(root: &'a Node, position: Position, source: &str) -> Option<Node<'a>> {
    let byte_offset = position_to_byte_offset(source, position);
    // descendant_for_byte_range(start, end) returns the smallest node that
    // completely spans the range [start, end]. When start == end, it finds
    // the node at that exact byte position.
    root.descendant_for_byte_range(byte_offset, byte_offset)
}

/// Convert LSP Position to byte offset
fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let lines: Vec<&str> = source.lines().collect();
    // LSP Position uses u32 for line/character (per LSP specification),
    // but we need usize for indexing into lines array.
    // The cast is safe because cursor positions are always within document bounds.
    let mut byte_offset = 0;

    for (i, line) in lines.iter().enumerate() {
        if i == position.line as usize {
            byte_offset += position.character as usize;
            break;
        }
        byte_offset += line.len() + 1; // +1 for newline character
    }

    byte_offset
}

/// Check if the position is within the SELECT projection list
fn is_in_projection(select_node: &Node, position: Position) -> bool {
    // The projection is typically the first child after "SELECT" keyword
    for child in select_node.children(&mut select_node.walk()) {
        if child.kind() == "projection" {
            let start = child.start_position();
            let end = child.end_position();

            return position.line as usize >= start.row
                && position.line as usize <= end.row
                && position.character as usize >= start.column;
        }
    }

    false
}

/// Extract table names from the FROM clause
fn extract_tables_from_from_clause(select_node: &Node, source: &str) -> Vec<String> {
    let mut tables = Vec::new();

    for child in select_node.children(&mut select_node.walk()) {
        if child.kind() == "from_clause" {
            // Find table_reference nodes
            extract_table_names_recursive(&child, source, &mut tables);
            break;
        }
    }

    tables
}

/// Recursively extract table names from table_reference nodes
fn extract_table_names_recursive(node: &Node, source: &str, tables: &mut Vec<String>) {
    match node.kind() {
        "table_reference" | "table_name" => {
            if let Some(name) = extract_identifier(node, source) {
                tables.push(name);
            }
        }
        _ => {
            // Recurse into children
            for child in node.children(&mut node.walk()) {
                extract_table_names_recursive(&child, source, tables);
            }
        }
    }
}

/// Extract identifier text from a node
fn extract_identifier(node: &Node, source: &str) -> Option<String> {
    let bytes = node.byte_range();
    let text = &source[bytes];
    Some(text.trim().to_string())
}

/// Extract table qualifier if cursor is after a dot
fn extract_qualifier(node: &Node, source: &str, position: Position) -> Option<String> {
    // Check if the node contains a dot and cursor is after it
    let node_text = &source[node.byte_range()];
    let cursor_offset = position.character as usize;

    // Find dots in the node text
    if let Some(dot_pos) = node_text.rfind('.') {
        let dot_abs_pos = node.start_position().column + dot_pos;
        if cursor_offset > dot_abs_pos {
            // Cursor is after the dot, extract qualifier (text before dot)
            let qualifier = node_text[..dot_pos].trim();
            return Some(qualifier.to_string());
        }
    }

    None
}

/// Extract left and right table names from a join clause
///
/// For a JOIN like `users JOIN orders ON users.id = orders.user_id`,
/// this extracts ("users", "orders")
fn extract_join_tables(join_node: &Node, source: &str) -> (Option<String>, Option<String>) {
    // Get parent from_clause to find the left table
    let mut left_table = None;
    let mut right_table = None;

    // First, try to get the right table (the table being joined)
    // In the join_clause node, the table_name is typically the second child (after JOIN keyword)
    let mut walk = join_node.walk();
    let mut children = join_node.children(&mut walk);
    let mut found_join_keyword = false;

    for child in &mut children {
        match child.kind() {
            "JOIN" | "INNER" | "LEFT" | "RIGHT" | "FULL" => {
                found_join_keyword = true;
            }
            "table_name" | "table_reference" if found_join_keyword => {
                if let Some(name) = extract_identifier(&child, source) {
                    right_table = Some(name);
                    break;
                }
            }
            _ => {}
        }
    }

    // Now, try to get the left table from the parent context
    // Walk up to find the from_clause and get tables before this join
    if let Some(parent) = join_node.parent() {
        if parent.kind() == "from_clause" || parent.kind() == "select_statement" {
            // Look for table_reference nodes that come before this join
            let from_tables = extract_tables_from_from_clause(&parent, source);
            if !from_tables.is_empty() {
                // The last table before the join is typically the left table
                left_table = from_tables.into_iter().next();
            }
        }
    }

    (left_table, right_table)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    #[test]
    fn test_completion_context_is_select_projection() {
        let ctx = CompletionContext::SelectProjection {
            tables: vec!["users".to_string()],
            qualifier: None,
        };
        assert!(ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_from_clause() {
        let ctx = CompletionContext::FromClause;
        assert!(!ctx.is_select_projection());
        assert!(ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_where_clause() {
        let ctx = CompletionContext::WhereClause;
        assert!(!ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_join_condition() {
        let ctx = CompletionContext::JoinCondition {
            left_table: Some("users".to_string()),
            right_table: Some("orders".to_string()),
        };
        assert!(!ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(ctx.is_join_condition());
    }

    // Note: Full integration tests with real tree-sitter parsing
    // will be in the tests module
}
