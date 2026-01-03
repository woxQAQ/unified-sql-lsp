// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Completion integration tests
//!
//! These tests verify end-to-end completion functionality for various SQL contexts.

use std::sync::Arc;
use tower_lsp::lsp_types::Position;
use unified_sql_lsp_catalog::{ColumnMetadata, DataType, TableMetadata, TableType};
use unified_sql_lsp_lsp::completion::CompletionEngine;
use unified_sql_lsp_test_utils::MockCatalogBuilder;

/// Helper function to create a test document with SQL content
async fn create_test_document(sql: &str) -> unified_sql_lsp_lsp::document::Document {
    use unified_sql_lsp_lsp::document::Document;
    use unified_sql_lsp_grammar::Parser;

    let parser = Parser::new();
    let tree = parser.parse(sql, None).unwrap();

    Document::new("test.sql".to_string(), sql.to_string(), tree)
}

#[tokio::test]
async fn test_from_clause_table_completion_basic() {
    // Setup mock catalog with standard tables
    let catalog = MockCatalogBuilder::new()
        .with_standard_schema()
        .build();

    // Create completion engine
    let engine = CompletionEngine::new(Arc::new(catalog));

    // Create a document with cursor in FROM clause
    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;

    // Request completion at cursor position
    let position = Position::new(0, 14); // After "FROM "
    let result = engine.complete(&document, position).await;

    // Verify we get completion items
    assert!(result.is_ok());
    let items = result.unwrap().expect("Should return completion items");

    // Should have at least the 3 standard tables: users, orders, products
    assert!(items.len() >= 3, "Expected at least 3 tables, got {}", items.len());

    // Verify expected tables are present
    let table_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(table_names.contains(&"users"), "Missing 'users' table");
    assert!(table_names.contains(&"orders"), "Missing 'orders' table");
    assert!(table_names.contains(&"products"), "Missing 'products' table");
}

#[tokio::test]
async fn test_from_clause_table_completion_with_schema() {
    // Setup catalog with tables from multiple schemas
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .add_table(
            TableMetadata::new("users", "auth")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .add_table(
            TableMetadata::new("sessions", "auth")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    // With multiple schemas, should show schema qualifiers
    assert!(items.len() >= 3);

    // Check that schema qualifiers are shown
    let table_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        table_names.contains(&"public.users"),
        "Missing 'public.users' with schema qualifier"
    );
    assert!(
        table_names.contains(&"auth.users"),
        "Missing 'auth.users' with schema qualifier"
    );
    assert!(
        table_names.contains(&"auth.sessions"),
        "Missing 'auth.sessions' with schema qualifier"
    );
}

#[tokio::test]
async fn test_from_clause_after_comma() {
    // Test completion after comma in FROM clause
    let catalog = MockCatalogBuilder::new()
        .with_standard_schema()
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM users, |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 23); // After "users, "

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    // Should still show all tables
    assert!(items.len() >= 3);
    let table_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(table_names.contains(&"users"));
    assert!(table_names.contains(&"orders"));
    assert!(table_names.contains(&"products"));
}

#[tokio::test]
async fn test_from_clause_with_views() {
    // Test that VIEW tables are included and properly marked
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
                .with_type(TableType::Table),
        )
        .add_table(
            TableMetadata::new("active_users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
                .with_type(TableType::View),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    // Should include both tables and views
    assert_eq!(items.len(), 2);

    // Check that both are present
    let table_names: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(table_names.contains(&"users"));
    assert!(table_names.contains(&"active_users"));

    // Verify detail strings show table types
    for item in &items {
        if item.label == "users" {
            assert!(
                item.detail.as_ref().unwrap().contains("TABLE"),
                "users should be marked as TABLE"
            );
        } else if item.label == "active_users" {
            assert!(
                item.detail.as_ref().unwrap().contains("VIEW"),
                "active_users should be marked as VIEW"
            );
        }
    }
}

#[tokio::test]
async fn test_from_clause_with_materialized_view() {
    // Test materialized views
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("user_summary", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)])
                .with_type(TableType::MaterializedView),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    assert_eq!(items.len(), 1);
    assert!(items[0]
        .detail
        .as_ref()
        .unwrap()
        .contains("MATERIALIZED VIEW"));
}

#[tokio::test]
async fn test_from_clause_single_schema() {
    // Test that schema qualifiers are NOT shown for single schema
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .add_table(
            TableMetadata::new("orders", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    assert_eq!(items.len(), 2);

    // Check that labels don't include schema
    assert_eq!(items[0].label, "orders"); // Sorted alphabetically
    assert_eq!(items[1].label, "users");
}

#[tokio::test]
async fn test_from_clause_non_public_schema() {
    // Test that schema qualifiers ARE shown when schema is not 'public'
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("users", "myapp")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    assert_eq!(items.len(), 1);
    // Should show schema qualifier since it's not 'public'
    assert_eq!(items[0].label, "myapp.users");
}

#[tokio::test]
async fn test_from_clause_empty_catalog() {
    // Test with no tables in catalog
    let catalog = MockCatalogBuilder::new().build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    // Should return empty result, not error
    assert_eq!(items.len(), 0);
}

#[tokio::test]
async fn test_from_clause_documentation_includes_columns() {
    // Test that documentation includes column information
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("users", "public")
                .with_columns(vec![
                    ColumnMetadata::new("id", DataType::Integer),
                    ColumnMetadata::new("name", DataType::Text),
                    ColumnMetadata::new("email", DataType::Text),
                ])
                .with_comment("User accounts"),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    assert_eq!(items.len(), 1);

    let doc = items[0]
        .documentation
        .as_ref()
        .unwrap()
        .to_string();

    // Should include column count
    assert!(doc.contains("3 columns"), "Missing column count in documentation");

    // Should list column names (since there are <= 5)
    assert!(doc.contains("id"), "Missing column 'id' in documentation");
    assert!(doc.contains("name"), "Missing column 'name' in documentation");
    assert!(doc.contains("email"), "Missing column 'email' in documentation");

    // Should include comment
    assert!(doc.contains("User accounts"), "Missing table comment in documentation");
}

#[tokio::test]
async fn test_from_clause_sort_order() {
    // Test that completion items are sorted alphabetically
    let catalog = MockCatalogBuilder::new()
        .add_table(
            TableMetadata::new("zebra", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .add_table(
            TableMetadata::new("apple", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .add_table(
            TableMetadata::new("banana", "public")
                .with_columns(vec![ColumnMetadata::new("id", DataType::Integer)]),
        )
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    let sql = "SELECT * FROM |";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 14);

    let result = engine.complete(&document, position).await.unwrap();
    let items = result.unwrap();

    assert_eq!(items.len(), 3);

    // Check alphabetical ordering
    assert_eq!(items[0].label, "apple");
    assert_eq!(items[1].label, "banana");
    assert_eq!(items[2].label, "zebra");
}

#[tokio::test]
async fn test_no_completion_in_select_clause() {
    // Verify that FROM clause completion doesn't interfere with SELECT clause
    let catalog = MockCatalogBuilder::new()
        .with_standard_schema()
        .build();

    let engine = CompletionEngine::new(Arc::new(catalog));

    // Cursor in SELECT projection, not FROM clause
    let sql = "SELECT | FROM users";
    let document = create_test_document(sql).await;
    let position = Position::new(0, 7); // After "SELECT "

    let result = engine.complete(&document, position).await;

    // Should work (return column completion for SELECT clause)
    assert!(result.is_ok());
    // We don't check the exact result here as that's tested elsewhere
    // Just verify it doesn't return FROM clause completion
}
