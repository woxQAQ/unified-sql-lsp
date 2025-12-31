// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Generic CST node wrapper for pre-Tree-sitter integration
//!
//! This module provides a generic abstraction over CST nodes that can work
//! with the actual Tree-sitter implementation when it's integrated.
//!
//! For now, it provides a mock implementation for testing the lowering trait.

use std::collections::HashMap;
use std::fmt;

/// Generic CST node trait
///
/// This trait abstracts over Tree-sitter nodes (or other parser outputs).
/// It provides a unified interface for the lowering layer to work with.
pub trait CstNode: fmt::Debug {
    /// Get the kind of node (e.g., "select_statement", "binary_expression")
    fn kind(&self) -> &str;

    /// Get child nodes by field name
    fn children(&self, field: &str) -> Vec<&Self>
    where
        Self: Sized;

    /// Get all child nodes (regardless of field)
    fn all_children(&self) -> Vec<&Self>
    where
        Self: Sized;

    /// Get the number of children
    fn child_count(&self) -> usize;

    /// Get the byte offset of this node in the source
    fn start_byte(&self) -> usize;

    /// Get the byte end offset of this node in the source
    fn end_byte(&self) -> usize;

    /// Get the text content of this node (if source is available)
    fn text(&self) -> Option<&str>;

    /// Check if this node is named (vs. anonymous nodes like parentheses)
    fn is_named(&self) -> bool;

    /// Get the parent node (if available)
    fn parent(&self) -> Option<&Self>
    where
        Self: Sized;
}

/// Mock CST node for testing (pre-Tree-sitter integration)
#[derive(Debug, Clone)]
pub struct MockCstNode {
    pub kind: String,
    pub children: Vec<MockCstNode>,
    pub field_map: HashMap<String, Vec<usize>>,
    pub start_byte: usize,
    pub end_byte: usize,
    pub text: Option<String>,
}

impl MockCstNode {
    /// Create a new mock CST node
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            children: Vec::new(),
            field_map: HashMap::new(),
            start_byte: 0,
            end_byte: 0,
            text: None,
        }
    }

    /// Add a child node with an optional field name
    pub fn with_child(mut self, field: Option<&str>, child: MockCstNode) -> Self {
        let idx = self.children.len();
        if let Some(field_name) = field {
            self.field_map
                .entry(field_name.to_string())
                .or_insert_with(Vec::new)
                .push(idx);
        }
        self.children.push(child);
        self
    }

    /// Set the byte range
    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.start_byte = start;
        self.end_byte = end;
        self
    }

    /// Set the text content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

impl CstNode for MockCstNode {
    fn kind(&self) -> &str {
        &self.kind
    }

    fn children(&self, field: &str) -> Vec<&Self> {
        self.field_map
            .get(field)
            .map(|indices| indices.iter().map(|&i| &self.children[i]).collect())
            .unwrap_or_default()
    }

    fn all_children(&self) -> Vec<&Self> {
        self.children.iter().collect()
    }

    fn child_count(&self) -> usize {
        self.children.len()
    }

    fn start_byte(&self) -> usize {
        self.start_byte
    }

    fn end_byte(&self) -> usize {
        self.end_byte
    }

    fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    fn is_named(&self) -> bool {
        !self.kind.starts_with('"')
    }

    fn parent(&self) -> Option<&Self> {
        None // Mock nodes don't track parents
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_node_creation() {
        let node = MockCstNode::new("select_statement")
            .with_child(Some("projection"), MockCstNode::new("select_list"))
            .with_child(Some("from"), MockCstNode::new("from_clause"));

        assert_eq!(node.kind(), "select_statement");
        assert_eq!(node.child_count(), 2);

        let projection_children = node.children("projection");
        assert_eq!(projection_children.len(), 1);
        assert_eq!(projection_children[0].kind(), "select_list");
    }

    #[test]
    fn test_field_access() {
        let select_list =
            MockCstNode::new("select_list").with_child(None, MockCstNode::new("column_ref"));

        let from_clause = MockCstNode::new("from_clause");

        let node = MockCstNode::new("select_statement")
            .with_child(Some("projection"), select_list)
            .with_child(Some("from"), from_clause);

        // Should have two named fields
        assert_eq!(node.children("projection").len(), 1);
        assert_eq!(node.children("from").len(), 1);

        // Non-existent field should return empty
        assert_eq!(node.children("where").len(), 0);
    }

    #[test]
    fn test_range_and_text() {
        let node = MockCstNode::new("column_ref")
            .with_range(10, 20)
            .with_text("user_id");

        assert_eq!(node.start_byte(), 10);
        assert_eq!(node.end_byte(), 20);
        assert_eq!(node.text(), Some("user_id"));
    }

    #[test]
    fn test_all_children() {
        let node = MockCstNode::new("select_list")
            .with_child(None, MockCstNode::new("column_ref"))
            .with_child(None, MockCstNode::new("column_ref"))
            .with_child(None, MockCstNode::new("literal"));

        let all = node.all_children();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_named_nodes() {
        let named = MockCstNode::new("select_statement");
        assert!(named.is_named());

        // Anonymous nodes in Tree-sitter start with quote
        let anonymous = MockCstNode::new("\"(\"");
        assert!(!anonymous.is_named());
    }
}
