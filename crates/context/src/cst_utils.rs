// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # CST utility functions
//!
//! This module provides shared utility functions for working with tree-sitter CST nodes.
//! These functions are used across multiple LSP modules (completion, definition, etc.)

use tree_sitter::{Node, TreeCursor};

/// Position in a document (line, character)
///
/// This mirrors tower_lsp::lsp_types::Position but is defined here
/// to avoid the dependency on tower_lsp in the context crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    /// Line position in a document (zero-based)
    pub line: u32,
    /// Character offset on a line in a document (zero-based)
    pub character: u32,
}

impl Position {
    /// Create a new position
    pub fn new(line: u32, character: u32) -> Self {
        Self { line, character }
    }
}

/// Range in a document
///
/// This mirrors tower_lsp::lsp_types::Range but is defined here
/// to avoid the dependency on tower_lsp in the context crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Range {
    /// The range's start position
    pub start: Position,
    /// The range's end position
    pub end: Position,
}

impl Range {
    /// Create a new range
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }
}

/// Find the node at the given position
///
/// # Arguments
///
/// * `root` - Root node of the parsed tree
/// * `position` - Cursor position (line, character)
/// * `source` - Source code text
///
/// # Returns
///
/// The smallest node that contains the given position, or None if not found
///
/// # Examples
///
/// ```no_run
/// use unified_sql_lsp_context::cst_utils::{find_node_at_position, Position};
/// use tree_sitter::Parser;
/// use unified_sql_grammar::language_for_dialect;
/// use unified_sql_lsp_ir::Dialect;
///
/// let source = "SELECT id FROM users";
/// let mut parser = Parser::new();
/// let lang = language_for_dialect(Dialect::MySQL).unwrap();
/// parser.set_language(&lang).unwrap();
/// let tree = parser.parse(source, None).unwrap();
/// let root = tree.root_node();
/// let node = find_node_at_position(&root, Position::new(0, 10), source);
/// # let _ = node;
/// ```
pub fn find_node_at_position<'a>(
    root: &'a Node,
    position: Position,
    source: &str,
) -> Option<Node<'a>> {
    let byte_offset = position_to_byte_offset(source, position);
    // descendant_for_byte_range(start, end) returns the smallest node that
    // completely spans the range [start, end]. When start == end, it finds
    // the node at that exact byte position.
    root.descendant_for_byte_range(byte_offset, byte_offset)
}

/// Convert LSP Position to byte offset
///
/// # Arguments
///
/// * `source` - Source code text
/// * `position` - LSP position (line, character)
///
/// # Returns
///
/// Byte offset in the source string
///
/// # Examples
///
/// ```
/// use unified_sql_lsp_context::cst_utils::{position_to_byte_offset, Position};
///
/// let source = "SELECT *\nFROM users";
/// let offset = position_to_byte_offset(source, Position::new(1, 0));
/// assert_eq!(offset, 9); // After "SELECT *\n" (8 + 1 for newline)
/// ```
pub fn position_to_byte_offset(source: &str, position: Position) -> usize {
    let lines: Vec<&str> = source.lines().collect();
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

/// Convert byte offset to LSP Position (UTF-8 aware)
///
/// # Arguments
///
/// * `byte_offset` - Byte offset in source
/// * `source` - Source code text
///
/// # Returns
///
/// LSP Position with correct line and character numbers
///
/// # Examples
///
/// ```
/// use unified_sql_lsp_context::cst_utils::byte_to_position;
///
/// let source = "SELECT id\nFROM users";
/// let pos = byte_to_position(10, source);
/// assert_eq!(pos.line, 1);
/// assert_eq!(pos.character, 0);
/// ```
pub fn byte_to_position(byte_offset: usize, source: &str) -> Position {
    if source.is_empty() {
        return Position::new(0, 0);
    }

    // Clamp byte_offset to source length
    let safe_byte_offset = byte_offset.min(source.len());

    let mut line = 0;
    let mut char_in_line = 0;

    // Use char_indices() to correctly handle UTF-8
    for (byte_idx, ch) in source.char_indices() {
        if byte_idx >= safe_byte_offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            char_in_line = 0;
        } else {
            char_in_line += 1;
        }
    }

    Position::new(line as u32, char_in_line as u32)
}

/// Convert a tree-sitter node to LSP Range
///
/// # Arguments
///
/// * `node` - Tree-sitter node
/// * `source` - Source code text
///
/// # Returns
///
/// LSP Range with start and end positions
pub fn node_to_range(node: &Node, source: &str) -> Range {
    let start = byte_to_position(node.start_byte(), source);
    let end = byte_to_position(node.end_byte(), source);
    Range { start, end }
}

/// Extract text from a node
///
/// # Arguments
///
/// * `node` - Tree-sitter node
/// * `source` - Source code text
///
/// # Returns
///
/// The text content of the node
pub fn extract_node_text(node: &Node, source: &str) -> String {
    let bytes = node.byte_range();
    source[bytes].to_string()
}

/// Extract identifier name from a node
///
/// This handles various node types that can represent identifiers:
/// - table_name
/// - column_name
/// - identifier
///
/// # Arguments
///
/// * `node` - Tree-sitter node
/// * `source` - Source code text
///
/// # Returns
///
/// The identifier text, or None if not found
pub fn extract_identifier_name(node: &Node, source: &str) -> Option<String> {
    match node.kind() {
        "table_name" | "column_name" | "identifier" => Some(extract_node_text(node, source)),
        _ => {
            // Try to find identifier child
            node.children(&mut node.walk())
                .find(|c| matches!(c.kind(), "table_name" | "column_name" | "identifier"))
                .map(|child| extract_node_text(&child, source))
        }
    }
}

/// Iterator over a node's children
///
/// This provides a cleaner interface than repeatedly calling `node.children(&mut node.walk())`.
pub struct ChildIter<'a> {
    cursor: TreeCursor<'a>,
    finished_first: bool,
}

impl<'a> ChildIter<'a> {
    pub fn new(node: &Node<'a>) -> Self {
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
pub trait NodeExt {
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

/// Find parent SELECT statement
///
/// Traverses up the tree from the given node to find the enclosing select_statement.
pub fn find_parent_select<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    let mut current = *node;
    loop {
        if current.kind() == "select_statement" {
            return Some(current);
        }
        current = current.parent()?;
    }
}

/// Find FROM clause in SELECT statement
pub fn find_from_clause<'a>(select_node: &'a Node<'a>) -> Option<Node<'a>> {
    select_node.find_child(|c| c.kind() == "from_clause")
}

/// Find SELECT clause (projection) in SELECT statement
pub fn find_select_clause<'a>(select_node: &'a Node<'a>) -> Option<Node<'a>> {
    select_node.find_child(|c| c.kind() == "select" || c.kind() == "projection")
}

/// Extract table name from table_reference node
pub fn extract_table_name(node: &Node, source: &str) -> Option<String> {
    node.find_child(|c| matches!(c.kind(), "table_name" | "identifier"))
        .map(|child| extract_node_text(&child, source))
}

/// Extract column info from column_reference node
/// Returns (column_name, optional_table_name)
pub fn extract_column_info(node: &Node, source: &str) -> Option<(String, Option<String>)> {
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

/// Extract alias from expression or function_call
pub fn extract_alias(node: &Node, source: &str) -> Option<String> {
    if let Some(child) = node.find_child(|c| c.kind() == "alias")
        && let Some(alias_child) =
            child.find_child(|c| matches!(c.kind(), "identifier" | "column_name"))
    {
        return Some(extract_node_text(&alias_child, source));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_byte_offset() {
        let source = "SELECT id FROM users";

        // Test position at start
        let pos = Position::new(0, 0);
        let offset = position_to_byte_offset(source, pos);
        assert_eq!(offset, 0);

        // Test position at "SELECT"
        let pos = Position::new(0, 6);
        let offset = position_to_byte_offset(source, pos);
        assert_eq!(offset, 6);
    }

    #[test]
    fn test_byte_to_position_multiline() {
        let source = "SELECT id\nFROM users\nWHERE id = 1";

        // Test position on line 1 (second line)
        let pos = byte_to_position(10, source);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        // Test position on line 2 (third line)
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
        assert_eq!(pos.character, 7);

        // Test position after UTF-8 characters
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
}
