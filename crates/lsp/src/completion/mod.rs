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

use std::sync::Arc;
use tower_lsp::lsp_types::{CompletionItem, Position};
use unified_sql_lsp_catalog::Catalog;

use crate::completion::catalog_integration::CatalogCompletionFetcher;
use crate::completion::context::{detect_completion_context, CompletionContext};
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
            let tree = document
                .tree()
                .ok_or(CompletionError::NotParsed)?;
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
                    let scope = scope_manager.get_scope_mut(scope_id).unwrap();

                    self.catalog_fetcher
                        .populate_all_tables(&mut scope.tables)
                        .await?;

                    // Render completion items
                    let force_qualifier = qualifier.is_some();
                    let items = CompletionRenderer::render_columns(&scope.tables, force_qualifier);

                    Ok(Some(items))
                } else {
                    Ok(None)
                }
            }
            CompletionContext::FromClause => {
                // Table completion (COMPLETION-002) - not implemented yet
                Ok(None)
            }
            CompletionContext::WhereClause => {
                // WHERE clause completion (COMPLETION-005) - not implemented yet
                Ok(None)
            }
            CompletionContext::Unknown => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_engine_new() {
        // This test requires a catalog, which we'd mock in real tests
        // Placeholder to show module structure
        assert!(true);
    }
}
