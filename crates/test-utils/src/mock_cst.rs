// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Mock CST builder for testing lowering layer
//!
//! Provides a fluent API for building CST trees without requiring tree-sitter

use std::collections::HashMap;
use unified_sql_lsp_lowering::CstNode;

/// Mock CST node for testing
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

/// Fluent builder for creating CST trees
pub struct MockCstBuilder {
    current: MockCstNode,
}

impl MockCstBuilder {
    /// Start building a new CST node
    pub fn new(kind: impl Into<String>) -> Self {
        Self {
            current: MockCstNode::new(kind),
        }
    }

    /// Add a child node with a field name
    pub fn with_field(mut self, field: &str, child: MockCstNode) -> Self {
        let idx = self.current.children.len();
        self.current
            .field_map
            .entry(field.to_string())
            .or_insert_with(Vec::new)
            .push(idx);
        self.current.children.push(child);
        self
    }

    /// Add a child without a field name
    pub fn with_child(mut self, child: MockCstNode) -> Self {
        self.current.children.push(child);
        self
    }

    /// Set the text content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.current.text = Some(text.into());
        self
    }

    /// Set the byte range
    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.current.start_byte = start;
        self.current.end_byte = end;
        self
    }

    /// Build the final CST node
    pub fn build(self) -> MockCstNode {
        self.current
    }
}

/// Helper functions for creating common SQL CST structures
pub struct SqlCstHelpers;

impl SqlCstHelpers {
    /// Create a simple SELECT statement: SELECT col1, col2 FROM table
    pub fn simple_select(columns: Vec<&str>, table: &str) -> MockCstNode {
        let select_list = columns
            .into_iter()
            .fold(MockCstNode::new("select_list"), |node, col| {
                node.with_child(None, MockCstNode::new("column_ref").with_text(col))
            });

        let from_clause = MockCstNode::new("from_clause").with_child(
            Some("table"),
            MockCstNode::new("table_ref").with_text(table),
        );

        MockCstNode::new("select_statement")
            .with_child(Some("projection"), select_list)
            .with_child(Some("from"), from_clause)
    }

    /// Create a SELECT with WHERE clause
    pub fn select_with_where(columns: Vec<&str>, table: &str, condition: &str) -> MockCstNode {
        let mut select_stmt = Self::simple_select(columns, table);
        select_stmt = select_stmt.with_child(
            Some("where"),
            MockCstNode::new("where_clause").with_text(condition),
        );
        select_stmt
    }

    /// Create a SELECT with JOIN
    pub fn select_with_join(
        columns: Vec<&str>,
        table1: &str,
        table2: &str,
        join_type: &str,
    ) -> MockCstNode {
        let select_list = columns
            .into_iter()
            .fold(MockCstNode::new("select_list"), |node, col| {
                node.with_child(None, MockCstNode::new("column_ref").with_text(col))
            });

        let from_clause = MockCstNode::new("from_clause")
            .with_child(
                Some("table"),
                MockCstNode::new("table_ref").with_text(table1),
            )
            .with_child(
                Some("join"),
                MockCstNode::new("join_clause")
                    .with_text(join_type)
                    .with_child(None, MockCstNode::new("table_ref").with_text(table2)),
            );

        MockCstNode::new("select_statement")
            .with_child(Some("projection"), select_list)
            .with_child(Some("from"), from_clause)
    }

    /// Create a column reference
    pub fn column(name: &str) -> MockCstNode {
        MockCstNode::new("column_ref").with_text(name)
    }

    /// Create a literal value
    pub fn literal(value: &str) -> MockCstNode {
        MockCstNode::new("literal").with_text(value)
    }

    /// Create a table reference
    pub fn table_ref(name: &str) -> MockCstNode {
        MockCstNode::new("table_ref").with_text(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_select() {
        let node = SqlCstHelpers::simple_select(vec!["id", "name"], "users");

        assert_eq!(node.kind(), "select_statement");
        assert_eq!(node.children("projection").len(), 1);
        assert_eq!(node.children("from").len(), 1);
    }

    #[test]
    fn test_select_with_where() {
        let node = SqlCstHelpers::select_with_where(vec!["id"], "users", "id > 10");

        assert_eq!(node.kind(), "select_statement");
        assert!(node.children("where").len() > 0);
    }

    #[test]
    fn test_builder_pattern() {
        let node = MockCstBuilder::new("select_statement")
            .with_field(
                "projection",
                MockCstBuilder::new("select_list")
                    .with_child(SqlCstHelpers::column("id"))
                    .with_child(SqlCstHelpers::column("name"))
                    .build(),
            )
            .with_field("from", SqlCstHelpers::table_ref("users"))
            .build();

        assert_eq!(node.kind(), "select_statement");
        assert_eq!(node.child_count(), 2);
    }
}
