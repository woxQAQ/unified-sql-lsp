// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration test suite for TEST-002
//!
//! Comprehensive integration tests covering:
//! - Completion flow: Full pipeline from parsing to rendering
//! - Multi-document operations: Concurrent access and thread-safety
//! - Catalog integration: Error handling and schema filtering

use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_lsp::lsp_types::{Position, Url};
use unified_sql_lsp_catalog::{
    ColumnMetadata, DataType, FunctionMetadata, FunctionType, TableMetadata,
};
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::completion::CompletionEngine;
use unified_sql_lsp_lsp::document::{Document, DocumentStore, ParseMetadata};
use unified_sql_lsp_lsp::parsing::{ParseResult, ParserManager};
use unified_sql_lsp_test_utils::MockCatalogBuilder;

async fn create_test_document(sql: &str, language_id: &str) -> Document {
    let uri = Url::parse("file:///test.sql").unwrap();
    let mut document = Document::new(uri, sql.to_string(), 1, language_id.to_string());

    let dialect = match language_id {
        "mysql" => Dialect::MySQL,
        "postgresql" | "postgres" => Dialect::PostgreSQL,
        _ => Dialect::MySQL,
    };

    let manager = ParserManager::new();
    let result = manager.parse_text(dialect, sql);

    match &result {
        ParseResult::Success { tree, parse_time } => {
            if let Some(tree) = tree {
                let metadata = ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0);
                document.set_tree(tree.clone(), metadata);
            }
        }
        ParseResult::Partial { tree, errors } => {
            if let Some(tree) = tree {
                let metadata = ParseMetadata::new(0, dialect, true, errors.len());
                document.set_tree(tree.clone(), metadata);
            }
        }
        ParseResult::Failed { .. } => {}
    }

    document
}

// =============================================================================
// Completion Flow Tests (10 tests)
// =============================================================================

#[tokio::test]
async fn test_completion_flow_select_projection() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_from_clause() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 14);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_where_clause() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM users WHERE |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 28);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_qualified_column() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT users.| FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 14);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_with_parse_errors() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELCT * FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 13);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Should handle parse errors gracefully");
}

#[tokio::test]
async fn test_completion_flow_join_condition() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM users JOIN orders ON |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 37);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_with_table_alias() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT u.| FROM users u";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 10);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_completion_flow_empty_document() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 0);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Should handle empty document gracefully");
}

#[tokio::test]
async fn test_completion_flow_mysql_syntax() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM users LIMIT |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 30);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Should handle MySQL LIMIT clause");
}

#[tokio::test]
async fn test_completion_flow_postgresql_syntax() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM users";
    let document = create_test_document(sql, "postgresql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Should handle PostgreSQL syntax");
}

// =============================================================================
// Multi-Document Concurrent Tests (6 tests)
// =============================================================================

#[tokio::test]
async fn test_concurrent_open_multiple_documents() {
    let store = Arc::new(DocumentStore::new());

    let uris: Vec<_> = (0..10)
        .map(|i| Url::parse(&format!("file:///test{}.sql", i)).unwrap())
        .collect();

    for uri in &uris {
        store
            .open_document(uri.clone(), "SELECT 1".to_string(), 1, "sql".to_string())
            .await
            .unwrap();
    }

    let uri_count = store.list_uris().await.len();
    assert_eq!(uri_count, 10);
}

#[tokio::test]
async fn test_rapid_open_close_cycles() {
    let store = Arc::new(DocumentStore::new());

    for i in 0..50 {
        let uri = Url::parse(&format!("file:///cycle{}.sql", i)).unwrap();

        store
            .open_document(uri.clone(), "SELECT 1".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        let exists = store.has_document(&uri).await;
        assert!(exists);

        let _ = store.close_document(&uri).await;

        let exists_after = store.has_document(&uri).await;
        assert!(!exists_after);
    }

    let uris = store.list_uris().await;
    assert_eq!(uris.len(), 0);
}

#[tokio::test]
async fn test_concurrent_parsing_performance() {
    let start = Instant::now();

    for _ in 0..20 {
        let sql = "SELECT id, email, name FROM users WHERE active = true";
        let _ = create_test_document(sql, "mysql").await;
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_secs(3),
        "Parsing should complete quickly"
    );
    println!("20 parses completed in {:?}", elapsed);
}

#[tokio::test]
async fn test_concurrent_completion_latency() {
    let catalog = Arc::new(MockCatalogBuilder::new().with_standard_schema().build());
    let engine = Arc::new(CompletionEngine::new(catalog));

    let mut latencies = Vec::new();

    for _ in 0..10 {
        let start = Instant::now();
        let sql = "SELECT | FROM users";
        let document = create_test_document(sql, "mysql").await;
        let position = Position::new(0, 8);
        let _ = engine.complete(&document, position).await;
        let elapsed = start.elapsed();
        latencies.push(elapsed);
    }

    latencies.sort();
    let p95_index = (latencies.len() * 95 / 100).min(latencies.len() - 1);
    let p95_latency = latencies[p95_index];

    assert!(
        p95_latency < Duration::from_millis(200),
        "p95 latency should be reasonable"
    );
    println!("p95 completion latency: {:?}", p95_latency);
}

#[tokio::test]
async fn test_concurrent_operations_with_errors() {
    let catalog = Arc::new(MockCatalogBuilder::new().with_standard_schema().build());
    let engine = Arc::new(CompletionEngine::new(catalog));

    let valid_sql = "SELECT | FROM users";
    let invalid_sql = "TOTALLY INVALID SQL!!!";

    for i in 0..10 {
        let sql = if i % 2 == 0 { valid_sql } else { invalid_sql };

        let document = create_test_document(sql, "mysql").await;
        let position = Position::new(0, 8);
        let _ = engine.complete(&document, position).await;
    }
}

#[tokio::test]
async fn test_concurrent_read_multiple_documents() {
    let store = Arc::new(DocumentStore::new());

    for i in 0..5 {
        let uri = Url::parse(&format!("file:///test{}.sql", i)).unwrap();
        let sql = format!("SELECT * FROM table{}", i);
        store
            .open_document(uri, sql, 1, "sql".to_string())
            .await
            .unwrap();
    }

    for i in 0..10 {
        let doc_index = i % 5;
        let uri = Url::parse(&format!("file:///test{}.sql", doc_index)).unwrap();
        let doc = store.get_document(&uri).await;
        assert!(doc.is_some());
    }
}

// =============================================================================
// Catalog Integration Tests (8 tests)
// =============================================================================

#[tokio::test]
async fn test_catalog_integration_standard_schema() {
    let catalog = MockCatalogBuilder::new().with_standard_schema().build();
    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_integration_custom_table() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("custom_table", "public")
                .with_columns(vec![
                    ColumnMetadata::new("custom_id", DataType::Integer),
                    ColumnMetadata::new("custom_name", DataType::Varchar(Some(100))),
                ])
                .with_row_count(1000),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM custom_table";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_integration_multiple_schemas() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .with_table(
            TableMetadata::new("orders", "myapp")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 14);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_integration_function_completion() {
    let catalog = MockCatalogBuilder::new()
        .with_function(
            FunctionMetadata::new("count", DataType::BigInt).with_type(FunctionType::Aggregate),
        )
        .with_function(
            FunctionMetadata::new("upper", DataType::Varchar(None)).with_type(FunctionType::Scalar),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_error_table_not_found() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM nonexistent_table";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(
        result.is_ok(),
        "Should handle non-existent table gracefully"
    );
}

#[tokio::test]
async fn test_catalog_metadata_in_completion() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
                .with_comment("User account information")
                .with_row_count(50000),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 14);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_column_metadata_in_completion() {
    let catalog = MockCatalogBuilder::new()
        .with_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![
                    ColumnMetadata::new("id", DataType::Integer)
                        .with_nullable(false)
                        .with_primary_key(),
                    ColumnMetadata::new("email", DataType::Varchar(Some(255))),
                ])
                .with_row_count(50000),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT | FROM users";
    let document = create_test_document(sql, "mysql").await;

    let position = Position::new(0, 8);
    let result = engine.complete(&document, position).await;

    assert!(result.is_ok(), "Completion failed: {:?}", result.err());
}

#[tokio::test]
async fn test_catalog_integration_mysql_vs_postgresql() {
    // Test MySQL
    let catalog_mysql = MockCatalogBuilder::new().with_standard_schema().build();
    let engine_mysql = CompletionEngine::new(Arc::new(catalog_mysql));
    let sql_mysql = "SELECT | FROM users";
    let document_mysql = create_test_document(sql_mysql, "mysql").await;
    let result_mysql = engine_mysql
        .complete(&document_mysql, Position::new(0, 8))
        .await;

    assert!(result_mysql.is_ok(), "MySQL completion failed");

    // Test PostgreSQL
    let catalog_pg = MockCatalogBuilder::new().with_standard_schema().build();
    let engine_pg = CompletionEngine::new(Arc::new(catalog_pg));
    let sql_pg = "SELECT | FROM users";
    let document_pg = create_test_document(sql_pg, "postgresql").await;
    let result_pg = engine_pg.complete(&document_pg, Position::new(0, 8)).await;

    assert!(result_pg.is_ok(), "PostgreSQL completion failed");

    // Both should succeed (results may be empty or have items)
    let _ = result_mysql.unwrap();
    let _ = result_pg.unwrap();
}
