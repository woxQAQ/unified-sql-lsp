// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Go-to-definition CST analysis helpers.

use crate::cst_utils::{
    NodeExt, Position, Range, extract_alias, extract_column_info, extract_identifier_name,
    extract_table_name, find_from_clause, find_node_at_position, find_parent_select,
    find_select_clause, node_to_range,
};
use tree_sitter::Node;

/// Definition lookup error.
#[derive(Debug, thiserror::Error)]
pub enum DefinitionError {
    #[error("No node found at cursor position")]
    NoNodeAtPosition,
}

/// Definition types represented with context-native range.
#[derive(Debug, Clone)]
pub enum Definition {
    Table(TableDefinition),
    Column(ColumnDefinition),
}

#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub table_name: String,
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub column_name: String,
    pub table_name: Option<String>,
    pub range: Range,
}

/// Finds symbol definitions in CST.
pub struct DefinitionFinder;

impl DefinitionFinder {
    pub fn find_at_position(
        root_node: &Node<'_>,
        source: &str,
        position: Position,
    ) -> Result<Option<Definition>, DefinitionError> {
        let cursor_node = find_node_at_position(root_node, position, source)
            .ok_or(DefinitionError::NoNodeAtPosition)?;

        let mut current = Some(cursor_node);
        while let Some(node) = current {
            match node.kind() {
                "table_reference" | "table_name" | "identifier" => {
                    if let Some((table_name, range)) = Self::find_table_definition(&node, source) {
                        return Ok(Some(Definition::Table(TableDefinition {
                            table_name,
                            range,
                        })));
                    }
                }
                "column_reference" | "column_name" => {
                    if let Some((column_name, table_name, range)) =
                        Self::find_column_definition(&node, source)
                    {
                        return Ok(Some(Definition::Column(ColumnDefinition {
                            column_name,
                            table_name,
                            range,
                        })));
                    }
                }
                _ => {}
            }
            current = node.parent();
        }

        Ok(None)
    }

    fn find_table_definition(cursor_node: &Node<'_>, source: &str) -> Option<(String, Range)> {
        let table_name = extract_identifier_name(cursor_node, source)?;
        let select_stmt = find_parent_select(cursor_node)?;
        let from_clause = find_from_clause(&select_stmt)?;

        if let Some(child) = from_clause.find_child(|c| c.kind() == "table_reference")
            && let Some(ref_name) = extract_table_name(&child, source)
            && ref_name == table_name
        {
            return Some((table_name, node_to_range(&child, source)));
        }

        None
    }

    fn find_column_definition(
        cursor_node: &Node<'_>,
        source: &str,
    ) -> Option<(String, Option<String>, Range)> {
        let (col_name, table_qualifier) = extract_column_info(cursor_node, source)?;
        let select_stmt = find_parent_select(cursor_node)?;
        let select_clause = find_select_clause(&select_stmt)?;

        for child in select_clause.iter_children() {
            match child.kind() {
                "column_reference" => {
                    let (ref_col, ref_table) = extract_column_info(&child, source)?;
                    if ref_col == col_name
                        && (table_qualifier.is_none() || ref_table == table_qualifier)
                    {
                        return Some((col_name, table_qualifier, node_to_range(&child, source)));
                    }
                }
                "expression" | "function_call" => {
                    if let Some(alias) = extract_alias(&child, source)
                        && alias == col_name
                    {
                        return Some((col_name, table_qualifier, node_to_range(&child, source)));
                    }
                }
                _ => {}
            }
        }
        None
    }
}
