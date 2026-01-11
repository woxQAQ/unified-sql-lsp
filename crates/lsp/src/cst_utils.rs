// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # CST utility functions
//!
//! This module provides shared utility functions for working with tree-sitter CST nodes.
//! These functions are used across multiple LSP modules (completion, definition, etc.)

use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::Node;

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
/// ```ignore
/// let tree = parser.parse(source, None).unwrap();
/// let node = find_node_at_position(&tree.root_node(), Position::new(0, 10), source);
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
/// use unified_sql_lsp_lsp::cst_utils::position_to_byte_offset;
/// use tower_lsp::lsp_types::Position;
///
/// let source = "SELECT *\nFROM users";
/// let offset = position_to_byte_offset(source, Position::new(1, 0));
/// assert_eq!(offset, 10); // After "SELECT *\n"
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
/// use unified_sql_lsp_lsp::cst_utils::byte_to_position;
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
            if let Some(child) = node
                .children(&mut node.walk())
                .find(|c| matches!(c.kind(), "table_name" | "column_name" | "identifier"))
            {
                Some(extract_node_text(&child, source))
            } else {
                None
            }
        }
    }
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
