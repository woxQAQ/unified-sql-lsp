// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! CST symbol extraction helpers for document symbols.

use crate::{Range, extract_alias, extract_node_text, node_to_range as context_node_to_range};
use tree_sitter::Node;
use unified_sql_lsp_semantic::TableSymbol;

/// Symbol extraction error
#[derive(Debug, thiserror::Error)]
pub enum SymbolError {
    #[error("Invalid SQL syntax: {0}")]
    InvalidSyntax(String),
}

/// Table symbol with context-native range information.
#[derive(Debug, Clone)]
pub struct TableSymbolWithRange {
    pub symbol: TableSymbol,
    pub range: Range,
    pub selection_range: Range,
}

/// Query symbol (represents a SELECT statement).
#[derive(Debug, Clone)]
pub struct QuerySymbol {
    pub range: Range,
    pub tables: Vec<TableSymbolWithRange>,
}

/// Symbol builder for extracting symbols from CST.
pub struct SymbolBuilder;

impl SymbolBuilder {
    pub fn build_from_cst(
        root_node: &Node<'_>,
        source: &str,
    ) -> Result<Vec<QuerySymbol>, SymbolError> {
        let mut queries = Vec::new();
        Self::find_select_statements(root_node, source, &mut queries);

        for query in &mut queries {
            query.tables = Self::extract_tables_from_query(root_node, source)?;
        }

        Ok(queries)
    }

    fn find_select_statements(node: &Node<'_>, source: &str, queries: &mut Vec<QuerySymbol>) {
        if (node.kind() == "select_statement" || node.kind() == "statement")
            && (node.kind() == "select_statement" || Self::is_select_statement(node))
        {
            let range = context_node_to_range(node, source);
            queries.push(QuerySymbol {
                range,
                tables: Vec::new(),
            });
        }

        for child in node.children(&mut node.walk()) {
            Self::find_select_statements(&child, source, queries);
        }
    }

    fn is_select_statement(node: &Node<'_>) -> bool {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "select_statement" {
                return true;
            }
        }
        false
    }

    fn extract_tables_from_query(
        root_node: &Node<'_>,
        source: &str,
    ) -> Result<Vec<TableSymbolWithRange>, SymbolError> {
        let mut tables = Vec::new();
        Self::find_table_references(root_node, source, &mut tables);
        Ok(tables)
    }

    fn find_table_references(
        node: &Node<'_>,
        source: &str,
        tables: &mut Vec<TableSymbolWithRange>,
    ) {
        if node.kind() == "table_reference" {
            if let Ok(table) = Self::parse_table_reference(node, source) {
                tables.push(table);
            }
            return;
        }

        for child in node.children(&mut node.walk()) {
            Self::find_table_references(&child, source, tables);
        }
    }

    fn parse_table_reference(
        node: &Node<'_>,
        source: &str,
    ) -> Result<TableSymbolWithRange, SymbolError> {
        let mut table_name = None;
        let mut alias = None;

        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "table_name" | "identifier" => {
                    if table_name.is_none() {
                        table_name = Some(extract_node_text(&child, source));
                    }
                }
                "alias" => {
                    if let Some(a) = extract_alias(&child, source) {
                        alias = Some(a);
                    }
                }
                _ => {
                    if alias.is_none() && child.kind() == "identifier" {
                        let text = extract_node_text(&child, source);
                        if let Some(ref name) = table_name
                            && &text != name
                        {
                            alias = Some(text);
                        }
                    }
                }
            }
        }

        let table_name = table_name.ok_or_else(|| {
            SymbolError::InvalidSyntax("Table name not found in table_reference".to_string())
        })?;

        let mut symbol = TableSymbol::new(&table_name);
        if let Some(a) = alias {
            symbol = symbol.with_alias(a);
        }

        let range = context_node_to_range(node, source);
        let selection_range = Self::node_to_selection_range(node, source);

        Ok(TableSymbolWithRange {
            symbol,
            range,
            selection_range,
        })
    }

    fn node_to_selection_range(node: &Node<'_>, source: &str) -> Range {
        for child in node.children(&mut node.walk()) {
            if child.kind() == "table_name" || child.kind() == "identifier" {
                return context_node_to_range(&child, source);
            }
        }
        context_node_to_range(node, source)
    }
}
