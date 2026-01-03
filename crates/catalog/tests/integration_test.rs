// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for the catalog crate

use unified_sql_lsp_catalog::{
    Catalog, ColumnMetadata, DataType, FunctionMetadata, FunctionType, TableMetadata, TableType,
};

// Mock catalog implementation for integration testing
struct TestCatalog;

#[async_trait::async_trait]
impl Catalog for TestCatalog {
    async fn list_tables(&self) -> unified_sql_lsp_catalog::CatalogResult<Vec<TableMetadata>> {
        Ok(vec![
            TableMetadata::new("users", "myapp")
                .with_columns(vec![
                    ColumnMetadata::new("id", DataType::BigInt)
                        .with_nullable(false)
                        .with_primary_key(),
                    ColumnMetadata::new("email", DataType::Varchar(Some(255))).with_nullable(false),
                    ColumnMetadata::new("created_at", DataType::Timestamp).with_nullable(true),
                ])
                .with_row_count(50000)
                .with_comment("User account information"),
            TableMetadata::new("orders", "myapp")
                .with_columns(vec![
                    ColumnMetadata::new("id", DataType::BigInt)
                        .with_nullable(false)
                        .with_primary_key(),
                    ColumnMetadata::new("user_id", DataType::BigInt)
                        .with_nullable(false)
                        .with_foreign_key("users", "id"),
                    ColumnMetadata::new("total", DataType::Decimal).with_nullable(true),
                ])
                .with_row_count(100000)
                .with_type(TableType::Table),
        ])
    }

    async fn get_columns(
        &self,
        table: &str,
    ) -> unified_sql_lsp_catalog::CatalogResult<Vec<ColumnMetadata>> {
        match table {
            "users" => Ok(vec![
                ColumnMetadata::new("id", DataType::BigInt)
                    .with_nullable(false)
                    .with_primary_key(),
                ColumnMetadata::new("email", DataType::Varchar(Some(255))).with_nullable(false),
                ColumnMetadata::new("created_at", DataType::Timestamp).with_nullable(true),
            ]),
            "orders" => Ok(vec![
                ColumnMetadata::new("id", DataType::BigInt)
                    .with_nullable(false)
                    .with_primary_key(),
                ColumnMetadata::new("user_id", DataType::BigInt)
                    .with_nullable(false)
                    .with_foreign_key("users", "id"),
                ColumnMetadata::new("total", DataType::Decimal).with_nullable(true),
            ]),
            _ => Err(unified_sql_lsp_catalog::CatalogError::TableNotFound(
                table.to_string(),
                "myapp".to_string(),
            )),
        }
    }

    async fn list_functions(
        &self,
    ) -> unified_sql_lsp_catalog::CatalogResult<Vec<FunctionMetadata>> {
        Ok(vec![
            FunctionMetadata::new("count", DataType::BigInt)
                .with_type(FunctionType::Aggregate)
                .with_description("Count rows")
                .with_example("SELECT COUNT(*) FROM users"),
            FunctionMetadata::new("sum", DataType::Decimal)
                .with_type(FunctionType::Aggregate)
                .with_description("Calculate sum"),
            FunctionMetadata::new("abs", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Absolute value"),
            FunctionMetadata::new("row_number", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Row number within partition"),
        ])
    }
}

#[tokio::test]
async fn test_complete_table_metadata() {
    let catalog = TestCatalog;
    let tables = catalog.list_tables().await.unwrap();

    assert_eq!(tables.len(), 2);

    let users_table = &tables[0];
    assert_eq!(users_table.name, "users");
    assert_eq!(users_table.schema, "myapp");
    assert_eq!(users_table.columns.len(), 3);
    assert_eq!(users_table.row_count_estimate, Some(50000));
    assert_eq!(
        users_table.comment,
        Some("User account information".to_string())
    );
}

#[tokio::test]
async fn test_table_primary_keys() {
    let catalog = TestCatalog;
    let tables = catalog.list_tables().await.unwrap();

    let users_table = &tables[0];
    let pks = users_table.primary_keys();
    assert_eq!(pks.len(), 1);
    assert_eq!(pks[0].name, "id");
    assert!(pks[0].is_primary_key);
}

#[tokio::test]
async fn test_foreign_key_reference() {
    let catalog = TestCatalog;
    let tables = catalog.list_tables().await.unwrap();

    let orders_table = &tables[1];
    assert_eq!(orders_table.name, "orders");

    let user_id_col = orders_table.get_column("user_id").unwrap();
    assert!(user_id_col.is_foreign_key);
    assert!(user_id_col.references.is_some());
    let ref_table = user_id_col.references.as_ref().unwrap();
    assert_eq!(ref_table.table, "users");
    assert_eq!(ref_table.column, "id");
}

#[tokio::test]
async fn test_table_get_column() {
    let catalog = TestCatalog;
    let tables = catalog.list_tables().await.unwrap();

    let users_table = &tables[0];
    assert!(users_table.get_column("id").is_some());
    assert!(users_table.get_column("email").is_some());
    assert!(users_table.get_column("created_at").is_some());
    assert!(users_table.get_column("nonexistent").is_none());
}

#[tokio::test]
async fn test_get_columns_by_table() {
    let catalog = TestCatalog;

    let users_columns = catalog.get_columns("users").await.unwrap();
    assert_eq!(users_columns.len(), 3);

    let orders_columns = catalog.get_columns("orders").await.unwrap();
    assert_eq!(orders_columns.len(), 3);
}

#[tokio::test]
async fn test_get_columns_table_not_found() {
    let catalog = TestCatalog;
    let result = catalog.get_columns("nonexistent").await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        unified_sql_lsp_catalog::CatalogError::TableNotFound(_, _)
    ));
}

#[tokio::test]
async fn test_aggregate_function_metadata() {
    let catalog = TestCatalog;
    let functions = catalog.list_functions().await.unwrap();

    let count_func = functions
        .iter()
        .find(|f| f.name == "count")
        .expect("count function not found");

    assert!(matches!(count_func.function_type, FunctionType::Aggregate));
    assert!(count_func.description.is_some());
    assert!(count_func.example.is_some());
}

#[tokio::test]
async fn test_function_types() {
    let catalog = TestCatalog;
    let functions = catalog.list_functions().await.unwrap();

    let aggregate_count = functions
        .iter()
        .filter(|f| matches!(f.function_type, FunctionType::Aggregate))
        .count();
    assert_eq!(aggregate_count, 2);

    let scalar_count = functions
        .iter()
        .filter(|f| matches!(f.function_type, FunctionType::Scalar))
        .count();
    assert_eq!(scalar_count, 1);

    let window_count = functions
        .iter()
        .filter(|f| matches!(f.function_type, FunctionType::Window))
        .count();
    assert_eq!(window_count, 1);
}

#[tokio::test]
async fn test_function_signature() {
    let catalog = TestCatalog;
    let functions = catalog.list_functions().await.unwrap();

    let count_func = functions
        .iter()
        .find(|f| f.name == "count")
        .expect("count function not found");

    let signature = count_func.signature();
    assert!(signature.contains("count"));
    assert!(signature.contains("BigInt"));
}

#[tokio::test]
async fn test_json_serialization_roundtrip() {
    let col = ColumnMetadata::new("data", DataType::Json)
        .with_nullable(true)
        .with_comment("JSON payload");

    let json = serde_json::to_string(&col).unwrap();
    let deserialized: ColumnMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(col, deserialized);
}

#[tokio::test]
async fn test_error_display() {
    use unified_sql_lsp_catalog::CatalogError;

    let err = CatalogError::TableNotFound("test_table".to_string(), "public".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test_table"));
    assert!(msg.contains("public"));
    assert!(msg.contains("not found"));
}

#[tokio::test]
async fn test_complete_metadata_workflow() {
    let catalog = TestCatalog;

    // List tables
    let tables = catalog.list_tables().await.unwrap();
    assert!(!tables.is_empty());

    // Get columns for first table
    let first_table = &tables[0];
    let columns = catalog.get_columns(&first_table.name).await.unwrap();
    assert!(!columns.is_empty());

    // Verify primary keys exist
    let pks = first_table.primary_keys();
    assert!(!pks.is_empty());

    // List functions
    let functions = catalog.list_functions().await.unwrap();
    assert!(!functions.is_empty());

    // Verify we have aggregate functions
    let has_aggregate = functions
        .iter()
        .any(|f| matches!(f.function_type, FunctionType::Aggregate));
    assert!(has_aggregate);
}
