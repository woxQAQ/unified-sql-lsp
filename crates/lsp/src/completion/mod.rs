// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion module
//!
//! This module provides intelligent SQL completion functionality.
//!
//! ## Architecture
//!
//! The completion system is organized into several modules:
//! - `context`: Detects the completion context (SELECT, FROM, WHERE, etc.)
//! - `scopes`: Builds semantic scopes from CST nodes
//! - `catalog_integration`: Fetches schema information from the catalog
//! - `render`: Converts semantic symbols to LSP completion items
//! - `error`: Error types for completion operations
//!
//! ## Flow
//!
//! ```text
//! 1. LSP Backend receives completion request
//!    ↓
//! 2. CompletionEngine.detect_context()
//!    ↓
//! 3. CompletionEngine.build_scopes()
//!    ↓
//! 4. CompletionEngine.fetch_columns()
//!    ↓
//! 5. CompletionEngine.render_completion()
//!    ↓
//! 6. Return CompletionResponse to client
//! ```

pub mod catalog_integration;
pub mod context;
pub mod error;
pub mod render;
pub mod scopes;

use std::collections::HashSet;
use std::sync::Arc;
use tower_lsp::lsp_types::{CompletionItem, Position};
use unified_sql_lsp_catalog::{Catalog, FunctionType};

use crate::completion::catalog_integration::CatalogCompletionFetcher;
use crate::completion::context::{CompletionContext, detect_completion_context};
use crate::completion::error::CompletionError;
use crate::completion::render::CompletionRenderer;
use crate::completion::scopes::ScopeBuilder;
use crate::document::Document;

pub use crate::completion::context::CompletionContext as SqlCompletionContext;

/// Completion engine
///
/// Orchestrates the completion flow from context detection to rendering.
pub struct CompletionEngine {
    catalog_fetcher: CatalogCompletionFetcher,
}

impl CompletionEngine {
    /// Create a new completion engine
    ///
    /// # Arguments
    ///
    /// * `catalog` - The catalog to use for fetching schema information
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self {
            catalog_fetcher: CatalogCompletionFetcher::new(catalog),
        }
    }

    /// Perform completion at the given position
    ///
    /// # Arguments
    ///
    /// * `document` - The document to complete in
    /// * `position` - The cursor position
    ///
    /// # Returns
    ///
    /// - `Ok(Some(items))` - Completion items available
    /// - `Ok(None)` - No completion (wrong context)
    /// - `Err(CompletionError)` - Error occurred
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let engine = CompletionEngine::new(catalog);
    /// match engine.complete(&document, Position::new(0, 10)).await {
    ///     Ok(Some(items)) => {
    ///         // Show completion items to user
    ///     }
    ///     Ok(None) => {
    ///         // No completion available (wrong context)
    ///     }
    ///     Err(e) => {
    ///         // Handle error
    ///     }
    /// }
    /// ```
    pub async fn complete(
        &self,
        document: &Document,
        position: Position,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        // Clone source to avoid holding document reference
        let source = document.get_content().to_string();

        // Get the parsed tree and do all synchronous parsing
        let (ctx, scope_manager) = {
            let tree = document.tree().ok_or(CompletionError::NotParsed)?;
            let tree_lock = tree.try_lock().map_err(|_| CompletionError::NotParsed)?;
            let tree = tree_lock.clone();

            // Extract root node - all operations using it must be within this block
            let root_node = tree.root_node();

            // Detect completion context (synchronous)
            let ctx = detect_completion_context(&root_node, position, &source);

            // Build scope synchronously if needed
            let scope_manager = match &ctx {
                CompletionContext::SelectProjection { .. }
                | CompletionContext::WhereClause { .. } => {
                    Some(ScopeBuilder::build_from_select(&root_node, &source)?)
                }
                _ => None,
            };

            (ctx, scope_manager)
        }; // root_node and tree_lock dropped here

        // Now handle async operations with only owned data
        match ctx {
            CompletionContext::SelectProjection { qualifier, .. } => {
                self.complete_with_scope(
                    &scope_manager,
                    qualifier,
                    false, // include_wildcard
                    None,  // function_filter
                )
                .await
            }
            CompletionContext::FromClause => {
                // Fetch all tables from catalog
                let tables = self.catalog_fetcher.list_tables().await?;

                // Check if we should show schema qualifier
                // Show if multiple schemas exist or schema is not 'public'
                let schemas: HashSet<&str> = tables.iter().map(|t| t.schema.as_str()).collect();
                let show_schema = schemas.len() > 1 || !schemas.iter().any(|&s| s == "public");

                // Render completion items
                let items = CompletionRenderer::render_tables(&tables, show_schema);
                Ok(Some(items))
            }
            CompletionContext::WhereClause { qualifier } => {
                self.complete_with_scope(
                    &scope_manager,
                    qualifier,
                    true, // exclude_wildcard
                    None, // function_filter
                )
                .await
            }
            CompletionContext::JoinCondition {
                left_table,
                right_table,
            } => {
                // Handle incomplete context (still typing table names)
                let (left_name, right_name) = match (left_table, right_table) {
                    (Some(l), Some(r)) => (l, r),
                    _ => return Ok(None), // Not ready for completion yet
                };

                // Fetch both tables from catalog
                let left_table_symbol =
                    match self.catalog_fetcher.populate_single_table(&left_name).await {
                        Ok(table) => table,
                        Err(e) => {
                            eprintln!("Warning: Failed to load left table '{}': {}", left_name, e);
                            return Ok(None);
                        }
                    };

                let right_table_symbol =
                    match self.catalog_fetcher.populate_single_table(&right_name).await {
                        Ok(table) => table,
                        Err(e) => {
                            eprintln!(
                                "Warning: Failed to load right table '{}': {}",
                                right_name, e
                            );
                            return Ok(None);
                        }
                    };

                // Fetch functions from catalog (scalar functions only for JOINs)
                let functions = self.catalog_fetcher.list_functions().await?;

                // Always force qualification for JOINs (best practice)
                let force_qualifier = true;

                // Render with PK/FK prioritization
                let mut items = CompletionRenderer::render_join_columns(
                    &[left_table_symbol, right_table_symbol],
                    force_qualifier,
                );

                // Add function completion items (scalar functions only for JOINs)
                let function_items =
                    CompletionRenderer::render_functions(&functions, Some(FunctionType::Scalar));
                items.extend(function_items);

                Ok(Some(items))
            }
            CompletionContext::Unknown => Ok(None),
        }
    }

    /// Shared completion logic for contexts with scope (SELECT/WHERE)
    ///
    /// This consolidates the duplicate logic between SelectProjection and WhereClause.
    async fn complete_with_scope(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        qualifier: Option<String>,
        exclude_wildcard: bool,
        function_filter: Option<FunctionType>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        let mut scope_manager = match scope_manager {
            Some(manager) => manager.clone(),
            None => return Ok(None),
        };

        let scope_id = 0; // Main query scope

        // Populate all tables with columns from catalog
        {
            let scope = scope_manager.get_scope_mut(scope_id).unwrap();
            self.catalog_fetcher
                .populate_all_tables(&mut scope.tables)
                .await?;
        }

        // Fetch functions from catalog
        let functions = self.catalog_fetcher.list_functions().await?;

        // Resolve qualifier if present to filter tables
        let tables_to_render = match &qualifier {
            Some(q) => {
                let scope = scope_manager.get_scope(scope_id).unwrap();
                match scope.find_table(q) {
                    Some(qualified_table) => vec![qualified_table.clone()],
                    None => return Ok(Some(vec![])), // Invalid qualifier
                }
            }
            None => {
                let scope = scope_manager.get_scope(scope_id).unwrap();
                scope.tables.clone()
            }
        };

        // Render completion items
        let force_qualifier = qualifier.is_some();
        let mut items = CompletionRenderer::render_columns(&tables_to_render, force_qualifier);

        // Filter out wildcard if needed
        if exclude_wildcard {
            items.retain(|i| i.label != "*");
        }

        // Add function completion items
        let function_items = CompletionRenderer::render_functions(&functions, function_filter);
        items.extend(function_items);

        Ok(Some(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::ParseMetadata;
    use crate::parsing::{ParseResult, ParserManager};
    use std::sync::Arc;
    use tower_lsp::lsp_types::{Position, Url};
    use unified_sql_lsp_ir::Dialect;

    /// Helper function to create a parsed document for testing
    async fn create_test_document(sql: &str, language_id: &str) -> Document {
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
            ParseResult::Success { tree, parse_time } => {
                if let Some(tree) = tree {
                    let metadata =
                        ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0);
                    document.set_tree(tree.clone(), metadata);
                }
            }
            ParseResult::Partial { tree, .. } => {
                if let Some(tree) = tree {
                    let metadata = ParseMetadata::new(0, dialect, true, 0);
                    document.set_tree(tree.clone(), metadata);
                }
            }
            ParseResult::Failed { .. } => {
                // No tree to set
            }
        }

        document
    }

    #[test]
    fn test_completion_engine_new() {
        // This test requires a catalog, which we'd mock in real tests
        // Placeholder to show module structure
        assert!(true);
    }

    #[tokio::test]
    async fn test_qualified_column_completion_with_table_name() {
        use unified_sql_lsp_catalog::{DataType, TableMetadata};
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        // Create mock catalog
        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
                unified_sql_lsp_catalog::ColumnMetadata::new("email", DataType::Text),
            ]))
            .with_table(TableMetadata::new("orders", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("user_id", DataType::Integer),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT users. FROM users;
        let source = r#"SELECT users. FROM users;"#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 14))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should only show users columns, all qualified
        assert!(items.iter().any(|i| i.label == "users.id"));
        assert!(items.iter().any(|i| i.label == "users.name"));
        assert!(items.iter().any(|i| i.label == "users.email"));
        // Should NOT show orders columns
        assert!(!items.iter().any(|i| i.label.contains("orders")));
    }

    #[tokio::test]
    async fn test_qualified_column_completion_with_alias() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT u. FROM users AS u;
        let source = r#"SELECT u. FROM users AS u;"#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 10))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should show columns with alias "u"
        assert!(items.iter().any(|i| i.label == "u.id"));
        assert!(items.iter().any(|i| i.label == "u.name"));
    }

    #[tokio::test]
    async fn test_qualified_column_completion_invalid_qualifier() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT nonexistent. FROM users;
        let source = r#"SELECT nonexistent. FROM users;"#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 22))
            .await
            .unwrap();
        assert!(items.is_some());
        // Should return empty completion for invalid qualifier
        assert_eq!(items.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_unqualified_column_completion_still_works() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT  FROM users; (no qualifier)
        let source = r#"SELECT  FROM users;"#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 8))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should show all columns without qualifier
        assert!(items.iter().any(|i| i.label == "id"));
        assert!(items.iter().any(|i| i.label == "name"));
    }

    #[tokio::test]
    async fn test_where_clause_unqualified_completion() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
                unified_sql_lsp_catalog::ColumnMetadata::new("email", DataType::Text),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT * FROM users WHERE |
        let source = r#"SELECT * FROM users WHERE "#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 28))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should show all columns without qualifier
        assert!(items.iter().any(|i| i.label == "id"));
        assert!(items.iter().any(|i| i.label == "name"));
        assert!(items.iter().any(|i| i.label == "email"));
        // Should NOT show wildcard
        assert!(!items.iter().any(|i| i.label == "*"));
    }

    #[tokio::test]
    async fn test_where_clause_qualified_completion() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
            ]))
            .with_table(TableMetadata::new("orders", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("user_id", DataType::Integer),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users.|
        let source = r#"SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE users."#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 83))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should only show users columns, all qualified
        assert!(items.iter().any(|i| i.label == "users.id"));
        assert!(items.iter().any(|i| i.label == "users.name"));
        // Should NOT show orders columns
        assert!(!items.iter().any(|i| i.label.contains("orders")));
    }

    #[tokio::test]
    async fn test_where_clause_qualified_with_alias() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT * FROM users AS u WHERE u.|
        let source = r#"SELECT * FROM users AS u WHERE u."#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 37))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Should use alias "u" instead of table name "users"
        assert!(items.iter().any(|i| i.label == "u.id"));
        assert!(items.iter().any(|i| i.label == "u.name"));
    }

    #[tokio::test]
    async fn test_where_clause_invalid_qualifier() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT * FROM users WHERE nonexistent.|
        let source = r#"SELECT * FROM users WHERE nonexistent."#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 43))
            .await
            .unwrap();
        assert!(items.is_some());
        // Should return empty completion for invalid qualifier
        assert_eq!(items.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_where_clause_ambiguous_column() {
        use unified_sql_lsp_catalog::TableMetadata;
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_test_utils::MockCatalogBuilder;

        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .with_table(TableMetadata::new("orders", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE |
        let source = r#"SELECT * FROM users JOIN orders ON users.id = orders.user_id WHERE "#;
        let document = create_test_document(source, "mysql").await;

        let items = engine
            .complete(&document, Position::new(0, 73))
            .await
            .unwrap();
        assert!(items.is_some());
        let items = items.unwrap();

        // Both tables have "id", so both should be qualified
        let id_items: Vec<_> = items.iter().filter(|i| i.label.contains("id")).collect();
        assert_eq!(id_items.len(), 2);
        assert!(id_items.iter().any(|i| i.label == "users.id"));
        assert!(id_items.iter().any(|i| i.label == "orders.id"));
    }
}
