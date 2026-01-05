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
use unified_sql_lsp_catalog::Catalog;

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
                CompletionContext::SelectProjection { .. } => {
                    Some(ScopeBuilder::build_from_select(&root_node, &source)?)
                }
                _ => None,
            };

            (ctx, scope_manager)
        }; // root_node and tree_lock dropped here

        // Now handle async operations with only owned data
        match ctx {
            CompletionContext::SelectProjection { qualifier, .. } => {
                if let Some(mut scope_manager) = scope_manager {
                    let scope_id = 0; // Main query scope

                    // Populate all tables with columns from catalog
                    {
                        let scope = scope_manager.get_scope_mut(scope_id).unwrap();
                        self.catalog_fetcher
                            .populate_all_tables(&mut scope.tables)
                            .await?;
                    }

                    // Resolve qualifier if present to filter tables
                    let tables_to_render = if let Some(q) = &qualifier {
                        // Resolve qualifier to actual table
                        let scope = scope_manager.get_scope(scope_id).unwrap();
                        match scope.find_table(q) {
                            Some(qualified_table) => {
                                // Found the table - create filtered list with just this table
                                vec![qualified_table.clone()]
                            }
                            None => {
                                // Invalid qualifier - return empty completion
                                // User might be typing a wrong table name
                                return Ok(Some(vec![]));
                            }
                        }
                    } else {
                        // No qualifier - show all columns from all tables
                        let scope = scope_manager.get_scope(scope_id).unwrap();
                        scope.tables.clone()
                    };

                    // Render completion items
                    let force_qualifier = qualifier.is_some();
                    let items =
                        CompletionRenderer::render_columns(&tables_to_render, force_qualifier);

                    Ok(Some(items))
                } else {
                    Ok(None)
                }
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
            CompletionContext::WhereClause => {
                // WHERE clause completion (COMPLETION-005) - not implemented yet
                // TODO: (COMPLETION-005) Implement WHERE clause column completion
                Ok(None)
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
                let left_table_symbol = match self
                    .catalog_fetcher
                    .populate_single_table(&left_name)
                    .await
                {
                    Ok(table) => table,
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load left table '{}': {}",
                            left_name, e
                        );
                        return Ok(None);
                    }
                };

                let right_table_symbol = match self
                    .catalog_fetcher
                    .populate_single_table(&right_name)
                    .await
                {
                    Ok(table) => table,
                    Err(e) => {
                        eprintln!(
                            "Warning: Failed to load right table '{}': {}",
                            right_name, e
                        );
                        return Ok(None);
                    }
                };

                // Always force qualification for JOINs (best practice)
                let force_qualifier = true;

                // Render with PK/FK prioritization
                let items =
                    CompletionRenderer::render_join_columns(&[left_table_symbol, right_table_symbol], force_qualifier);

                Ok(Some(items))
            }
            CompletionContext::Unknown => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_completion_engine_new() {
        // This test requires a catalog, which we'd mock in real tests
        // Placeholder to show module structure
        assert!(true);
    }

    #[tokio::test]
    async fn test_qualified_column_completion_with_table_name() {
        use tower_lsp::lsp_types::Position;
        use unified_sql_lsp_test_utils::{MockCatalogBuilder, catalog::DataType};

        // Create mock catalog
        let catalog = MockCatalogBuilder::new()
            .with_table(
                "users",
                vec![
                    ("id", DataType::Integer),
                    ("name", DataType::Text),
                    ("email", DataType::Text),
                ],
            )
            .with_table(
                "orders",
                vec![("id", DataType::Integer), ("user_id", DataType::Integer)],
            )
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT users. FROM users;
        let source = r#"SELECT users. FROM users;"#;
        let document = Document::new(source, "test.sql");
        document.parse().await;

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
        use tower_lsp::lsp_types::Position;
        use unified_sql_lsp_test_utils::{MockCatalogBuilder, catalog::DataType};

        let catalog = MockCatalogBuilder::new()
            .with_table(
                "users",
                vec![("id", DataType::Integer), ("name", DataType::Text)],
            )
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT u. FROM users AS u;
        let source = r#"SELECT u. FROM users AS u;"#;
        let document = Document::new(source, "test.sql");
        document.parse().await;

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
        use tower_lsp::lsp_types::Position;
        use unified_sql_lsp_test_utils::{MockCatalogBuilder, catalog::DataType};

        let catalog = MockCatalogBuilder::new()
            .with_table("users", vec![("id", DataType::Integer)])
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT nonexistent. FROM users;
        let source = r#"SELECT nonexistent. FROM users;"#;
        let document = Document::new(source, "test.sql");
        document.parse().await;

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
        use tower_lsp::lsp_types::Position;
        use unified_sql_lsp_test_utils::{MockCatalogBuilder, catalog::DataType};

        let catalog = MockCatalogBuilder::new()
            .with_table(
                "users",
                vec![("id", DataType::Integer), ("name", DataType::Text)],
            )
            .build();

        let engine = CompletionEngine::new(Arc::new(catalog));

        // Test: SELECT  FROM users; (no qualifier)
        let source = r#"SELECT  FROM users;"#;
        let document = Document::new(source, "test.sql");
        document.parse().await;

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
}
