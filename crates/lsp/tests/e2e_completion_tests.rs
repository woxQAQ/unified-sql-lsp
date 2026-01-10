// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! End-to-end completion integration tests
//!
//! Tests the full pipeline from parsing to completion.

use std::sync::Arc;
use tower_lsp::lsp_types::{Position, Range, TextDocumentContentChangeEvent, Url};
use unified_sql_lsp_catalog::{ColumnMetadata, DataType, TableMetadata};
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::completion::CompletionEngine;
use unified_sql_lsp_lsp::document::{Document, ParseMetadata};
use unified_sql_lsp_lsp::parsing::ParserManager;
use unified_sql_lsp_test_utils::MockCatalogBuilder;

async fn create_and_parse_document(sql: &str, language_id: &str) -> Document {
    let uri = Url::parse("file:///test.sql").unwrap();
    let mut document = Document::new(uri, sql.to_string(), 1, language_id.to_string());

    let dialect = match language_id {
        "mysql" => Dialect::MySQL,
        "postgresql" => Dialect::PostgreSQL,
        _ => Dialect::MySQL,
    };

    let manager = ParserManager::new();
    let result = manager.parse_text(dialect, sql);

    match &result {
        unified_sql_lsp_lsp::parsing::ParseResult::Success { tree, parse_time } => {
            if let Some(tree) = tree {
                let metadata = ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0);
                document.set_tree(tree.clone(), metadata);
            }
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Partial { tree, errors } => {
            if let Some(tree) = tree {
                let metadata = ParseMetadata::new(0, dialect, true, errors.len());
                document.set_tree(tree.clone(), metadata);
            }
        }
        unified_sql_lsp_lsp::parsing::ParseResult::Failed { .. } => {
            // No tree to set
        }
    }

    document
}

#[tokio::test]
async fn test_e2e_completion_from_clause() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 14);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().expect("Expected completion items");

    assert!(
        items.len() >= 2,
        "Expected at least 2 tables, got {}",
        items.len()
    );

    let table_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(table_names.iter().any(|t| t.contains("users")));
    assert!(table_names.iter().any(|t| t.contains("orders")));
}

#[tokio::test]
async fn test_e2e_completion_with_mysql_syntax() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "myapp")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    // MySQL-specific LIMIT syntax
    let sql = "SELECT * FROM users LIMIT |";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 30);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok());
    // TODO: (COMPLETION-005) Implement LIMIT/OFFSET clause completion context detection
    // LIMIT clause completion not implemented yet, so we just verify it doesn't crash
}

#[tokio::test]
async fn test_e2e_completion_with_postgresql_syntax() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "myapp")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    // PostgreSQL-specific DISTINCT ON syntax
    let sql = "SELECT DISTINCT ON (|) id FROM users";
    let document = create_and_parse_document(sql, "postgresql").await;

    let position = Position::new(0, 20);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_e2e_document_sync_flow() {
    use std::sync::Arc;
    use tokio::sync::RwLock;

    let config = Arc::new(RwLock::new(None));
    let sync = unified_sql_lsp_lsp::sync::DocumentSync::new(config);

    let uri = Url::parse("file:///test.sql").unwrap();
    let doc = Document::new(uri, "SELECT 1".to_string(), 1, "mysql".to_string());

    // Open document
    let open_result = sync.on_document_open(&doc);
    assert!(!open_result.is_failed(), "Failed to parse on open");

    // Update document
    let changes = vec![TextDocumentContentChangeEvent {
        range: Some(Range {
            start: Position {
                line: 0,
                character: 7,
            },
            end: Position {
                line: 0,
                character: 8,
            },
        }),
        range_length: Some(1),
        text: "*".to_string(),
    }];

    let change_result = sync.on_document_change(&doc, None, &changes);
    assert!(!change_result.is_failed(), "Failed to parse on change");
}

// =============================================================================
// WHERE Clause Completion Tests (COMPLETION-005)
// =============================================================================

#[tokio::test]
async fn test_e2e_completion_where_clause_unqualified() {
    let catalog = MockCatalogBuilder::new()
        .with_table(TableMetadata::new("users", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("name", DataType::Varchar(None)),
            ColumnMetadata::new("email", DataType::Varchar(None)),
        ]))
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));
    let sql = "SELECT * FROM users WHERE |";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 28);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().expect("Expected completion items");

    assert!(
        items.len() >= 3,
        "Expected at least 3 columns, got {}",
        items.len()
    );
    let column_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(column_names.contains(&"id"));
    assert!(column_names.contains(&"name"));
    assert!(column_names.contains(&"email"));
}

#[tokio::test]
async fn test_e2e_completion_where_clause_qualified() {
    let catalog = MockCatalogBuilder::new()
        .with_table(TableMetadata::new("users", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("name", DataType::Varchar(None)),
        ]))
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));
    let sql = "SELECT * FROM users WHERE users.|";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 34);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().expect("Expected completion items");

    assert!(
        items.len() >= 2,
        "Expected at least 2 columns, got {}",
        items.len()
    );
    let column_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(column_names.contains(&"id"));
    assert!(column_names.contains(&"name"));

    // Verify that items are properly qualified
    for item in &items {
        assert!(
            item.label.starts_with("users."),
            "Expected qualified column name, got: {}",
            item.label
        );
    }
}

#[tokio::test]
async fn test_e2e_completion_where_clause_multiple_tables() {
    let catalog = MockCatalogBuilder::new()
        .with_table(TableMetadata::new("users", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("name", DataType::Varchar(None)),
        ]))
        .with_table(TableMetadata::new("orders", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("user_id", DataType::Integer),
            ColumnMetadata::new("total", DataType::Decimal),
        ]))
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));
    let sql = "SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE |";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 70);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().expect("Expected completion items");

    // Should have columns from both tables
    assert!(
        items.len() >= 5,
        "Expected at least 5 columns, got {}",
        items.len()
    );
    let column_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();

    // Verify we have columns from both tables
    assert!(column_names.contains(&"users.id") || column_names.contains(&"id"));
    assert!(column_names.contains(&"orders.id") || column_names.contains(&"id"));
}

#[tokio::test]
async fn test_e2e_completion_where_clause_invalid_qualifier() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));
    let sql = "SELECT * FROM users WHERE nonexistent.|";
    let document = create_and_parse_document(sql, "mysql").await;

    let position = Position::new(0, 41);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().unwrap_or_default();

    // Should return empty completion for invalid qualifier
    assert_eq!(
        items.len(),
        0,
        "Expected empty completion for invalid qualifier"
    );
}

#[tokio::test]
async fn test_e2e_completion_where_clause_with_postgresql() {
    let catalog = MockCatalogBuilder::new()
        .with_table(TableMetadata::new("products", "public").with_columns(vec![
            ColumnMetadata::new("id", DataType::Integer),
            ColumnMetadata::new("price", DataType::Decimal),
            ColumnMetadata::new("name", DataType::Varchar(None)),
        ]))
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));
    let sql = "SELECT * FROM products WHERE |";
    let document = create_and_parse_document(sql, "postgresql").await;

    let position = Position::new(0, 30);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
    let items = result.unwrap().expect("Expected completion items");

    assert!(
        items.len() >= 3,
        "Expected at least 3 columns, got {}",
        items.len()
    );
    let column_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(column_names.contains(&"id"));
    assert!(column_names.contains(&"price"));
    assert!(column_names.contains(&"name"));
}
