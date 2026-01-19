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
pub mod error;
pub mod render;

// Note: alias_resolution and scopes modules are now provided by semantic and context crates
// Note: context and keywords modules are now provided by unified_sql-lsp-context crate

use std::collections::HashSet;
use std::sync::Arc;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat, Position};
use tracing::{debug, instrument};
use unified_sql_lsp_catalog::{Catalog, FunctionType};
use unified_sql_lsp_ir::Dialect;

// Import from semantic crate (moved from LSP)
use unified_sql_lsp_semantic::AliasResolver;

// Import from context crate (moved from LSP)
use unified_sql_lsp_context::ScopeBuilder;

use crate::completion::catalog_integration::CatalogCompletionFetcher;
use crate::completion::error::CompletionError;
use crate::completion::render::CompletionRenderer;
use crate::document::Document;

// Use context crate for keywords
use unified_sql_lsp_context::KeywordProvider;

// Use context crate for context detection
// Re-export the context types for backward compatibility
pub use unified_sql_lsp_context::{CompletionContext, CompletionContext as SqlCompletionContext};

/// Convert tower_lsp Position to context Position
fn to_context_pos(pos: Position) -> unified_sql_lsp_context::Position {
    unified_sql_lsp_context::Position::new(pos.line, pos.character)
}

/// Completion engine
///
/// Orchestrates the completion flow from context detection to rendering.
pub struct CompletionEngine {
    catalog_fetcher: Arc<CatalogCompletionFetcher>,
    dialect: Dialect,
}

impl CompletionEngine {
    /// Create a new completion engine
    ///
    /// # Arguments
    ///
    /// * `catalog` - The catalog to use for fetching schema information
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        let dialect = Dialect::MySQL; // Default dialect
        Self {
            catalog_fetcher: Arc::new(CatalogCompletionFetcher::new(catalog)),
            dialect,
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
    /// ```text,ignore
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
    #[instrument(skip(self, document), fields(position = ?position))]
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
            // Convert tower_lsp Position to context Position
            let ctx = unified_sql_lsp_context::detect_completion_context(
                &root_node,
                to_context_pos(position),
                &source,
            );

            debug!(?ctx, "Detected completion context");
            if let CompletionContext::FromClause { exclude_tables } = &ctx {
                debug!(?exclude_tables, "FromClause context with excluded tables");
            }

            // Build scope synchronously if needed
            let scope_manager = match &ctx {
                CompletionContext::SelectProjection { .. }
                | CompletionContext::WhereClause { .. } => {
                    // Try to build scope from CST, but don't fail if it's incomplete
                    match ScopeBuilder::build_from_select(&root_node, &source) {
                        Ok(scope) => Some(scope),
                        Err(e) => {
                            debug!(error = ?e, "Failed to build scope from CST, will use context_tables");
                            None
                        }
                    }
                }
                _ => None,
            };

            (ctx, scope_manager)
        }; // root_node and tree_lock dropped here

        debug!(
            scope_manager_exists = scope_manager.is_some(),
            is_join_condition = matches!(ctx, CompletionContext::JoinCondition { .. }),
            "Context detection complete"
        );

        // Now handle async operations with only owned data
        match ctx {
            CompletionContext::SelectProjection { tables, qualifier } => {
                eprintln!(
                    "!!! LSP: SelectProjection with tables={:?}, qualifier={:?}",
                    tables, qualifier
                );
                debug!(?tables, ?qualifier, "Matched SelectProjection context");
                self.complete_select_projection(
                    &scope_manager,
                    tables,
                    qualifier,
                    &source,
                    document,
                )
                .await
            }
            CompletionContext::FromClause { exclude_tables } => {
                self.complete_from_clause(document, position, exclude_tables)
                    .await
            }
            CompletionContext::WhereClause { tables, qualifier } => {
                self.complete_where_clause(&scope_manager, tables, qualifier)
                    .await
            }
            CompletionContext::JoinCondition {
                left_table,
                right_table,
                qualifier,
            } => {
                debug!(?left_table, ?right_table, "Matched JoinCondition context");
                // We can complete if we have at least one table
                if let (None, None) = (left_table.as_ref(), right_table.as_ref()) {
                    return Ok(None);
                }

                // Use AliasResolver to resolve table names
                let resolver = AliasResolver::new(self.catalog_fetcher.catalog());

                // Collect table names to resolve
                let table_names: Vec<_> = left_table
                    .iter()
                    .chain(right_table.iter())
                    .cloned()
                    .collect();

                debug!(?table_names, "Resolving table aliases for JOIN");

                // Resolve all tables
                let tables_with_columns = resolver.resolve_multiple(table_names).await?;

                debug!(
                    table_count = tables_with_columns.len(),
                    "Resolved tables for JOIN"
                );

                if tables_with_columns.is_empty() {
                    debug!("No tables loaded for JOIN condition");
                    return Ok(None);
                }

                // Determine if we should force qualification
                // Force qualification when multiple tables are in the JOIN
                // BUT: For USING clause, never force qualification (USING syntax doesn't use qualifiers)
                let is_using_clause = source.to_uppercase().contains("USING");
                let force_qualifier = if is_using_clause {
                    false // USING clause doesn't use table qualifiers
                } else {
                    tables_with_columns.len() > 1
                };

                // Fetch functions from catalog (scalar functions only for JOINs)
                let functions = self.catalog_fetcher.list_functions().await?;

                // Filter tables based on qualifier if provided
                let tables_to_render = if let Some(ref q) = qualifier {
                    // Only show columns from the table matching the qualifier
                    tables_with_columns
                        .into_iter()
                        .filter(|t| {
                            t.alias.as_ref().map(|a| a == q).unwrap_or(false) || t.table_name == *q
                        })
                        .collect()
                } else {
                    tables_with_columns
                };

                debug!(
                    tables_count = tables_to_render.len(),
                    "Filtered tables for rendering"
                );

                if tables_to_render.is_empty() {
                    debug!("No tables after qualifier filtering");
                    return Ok(None);
                }

                // Render with PK/FK prioritization
                let mut items =
                    CompletionRenderer::render_join_columns(&tables_to_render, force_qualifier);

                // Add function completion items (scalar functions only for JOINs)
                let function_items =
                    CompletionRenderer::render_functions(&functions, Some(FunctionType::Scalar));
                items.extend(function_items);

                debug!(
                    item_count = items.len(),
                    "Rendered JOIN condition completion"
                );
                Ok(Some(items))
            }
            CompletionContext::Keywords {
                statement_type,
                existing_clauses,
            } => {
                // Handle keyword completion
                // Get dialect from document metadata, fallback to stored dialect
                let dialect = document
                    .parse_metadata()
                    .map(|m| m.dialect)
                    .unwrap_or(self.dialect);
                let provider = KeywordProvider::new(dialect);

                // Render completion items
                let items = if let Some(stmt_type) = &statement_type {
                    // Show keywords based on statement type
                    match stmt_type.as_str() {
                        "SELECT" => {
                            // Show SELECT clause keywords, excluding existing ones
                            let all = provider.select_clause_keywords();
                            let exclude: HashSet<String> = existing_clauses.into_iter().collect();
                            let keywords = all.exclude(&exclude);
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        "UPDATE" => {
                            // For UPDATE, we need both table names and SET keyword
                            // SQL syntax: UPDATE table_name SET ...
                            let keywords = provider.update_keywords().keywords;

                            // Get table names for UPDATE statement
                            let tables =
                                self.catalog_fetcher.list_tables().await.unwrap_or_default();
                            let table_items = CompletionRenderer::render_tables(&tables, false);

                            // Render keywords
                            let keyword_items = CompletionRenderer::render_keywords(&keywords);

                            // Combine: tables first (higher priority), then keywords
                            let mut all_items = table_items;
                            all_items.extend(keyword_items);
                            all_items
                        }
                        "DELETE" => {
                            // For DELETE, we need both table names and FROM keyword
                            // SQL syntax: DELETE FROM table_name ...
                            // But also support: DELETE table_name (MySQL syntax)
                            let keywords = provider.delete_keywords().keywords;

                            // Get table names for DELETE statement
                            let tables =
                                self.catalog_fetcher.list_tables().await.unwrap_or_default();
                            let table_items = CompletionRenderer::render_tables(&tables, false);

                            // Render keywords
                            let keyword_items = CompletionRenderer::render_keywords(&keywords);

                            // Combine: tables first (higher priority), then keywords
                            let mut all_items = table_items;
                            all_items.extend(keyword_items);
                            all_items
                        }
                        "INSERT" => {
                            let keywords = provider.insert_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        "CREATE" => {
                            let keywords = provider.create_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        "ALTER" => {
                            let keywords = provider.alter_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        "DROP" => {
                            let keywords = provider.drop_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        "UNION" => {
                            let keywords = provider.union_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                        _ => {
                            let keywords = provider.select_clause_keywords().keywords;
                            CompletionRenderer::render_keywords(&keywords)
                        }
                    }
                } else {
                    // No statement type, show statement keywords
                    let keywords = provider.statement_keywords().keywords;
                    CompletionRenderer::render_keywords(&keywords)
                };

                Ok(Some(items))
            }
            CompletionContext::OrderByClause { tables, qualifier } => {
                self.complete_order_by_clause(&scope_manager, tables, qualifier)
                    .await
            }
            CompletionContext::GroupByClause { tables, qualifier } => {
                self.complete_group_by_clause(&scope_manager, tables, qualifier)
                    .await
            }
            CompletionContext::LimitClause => self.complete_limit_clause().await,
            CompletionContext::HavingClause { tables, qualifier } => {
                self.complete_having_clause(&scope_manager, tables, qualifier)
                    .await
            }
            CompletionContext::CteDefinition {
                available_tables,
                defined_ctes,
            } => {
                self.complete_cte_definition(
                    &scope_manager,
                    document,
                    position,
                    available_tables,
                    defined_ctes,
                )
                .await
            }
            CompletionContext::WindowFunctionClause {
                tables,
                window_part,
            } => {
                self.complete_window_function_clause(&scope_manager, tables, window_part)
                    .await
            }
            CompletionContext::ReturningClause { tables, qualifier } => {
                self.complete_returning_clause(&scope_manager, tables, qualifier)
                    .await
            }
            CompletionContext::Unknown => Ok(None),
        }
    }

    /// Complete SELECT projection with columns, functions, and SELECT modifiers
    ///
    /// This is specialized for SELECT clause completion.
    #[instrument(skip(self, document, source))]
    async fn complete_select_projection(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
        source: &str,
        document: &Document,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting SELECT projection completion");

        // Get columns and functions using the shared scope completion logic
        let items_result = self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                false, // include_wildcard
                None,  // function_filter (show all)
            )
            .await?;

        // If we have an invalid qualifier (returned None or empty items from context_tables path), return empty
        // But if we have no CST scope (None from scope_manager path), still show keywords
        let has_invalid_qualifier = qualifier.is_some()
            && (items_result.is_none() || items_result.as_ref().is_some_and(|i| i.is_empty()));

        if has_invalid_qualifier {
            return Ok(Some(vec![]));
        }

        let mut items = items_result.unwrap_or_default();

        // Exclude columns that are already selected in the SELECT clause
        // Pattern: "SELECT id, username, | FROM users" -> exclude "id" and "username"
        if let Some(select_pos) = source.to_uppercase().find("SELECT") {
            // Get text between SELECT and FROM
            let after_select = &source[select_pos + 6..]; // +6 for "SELECT"
            if let Some(from_pos) = after_select.to_uppercase().find("FROM") {
                let select_clause = &after_select[..from_pos];
                debug!(select_clause = %select_clause, "Processing SELECT clause");

                // Extract selected column names
                let mut selected_columns = std::collections::HashSet::new();
                for part in select_clause.split(',') {
                    let part = part.trim();
                    if !part.is_empty() && !part.starts_with('(') {
                        // Simple column name (not a function call)
                        // Remove any alias (AS xxx)
                        let col_name = if let Some(as_pos) = part.to_uppercase().find(" AS ") {
                            &part[..as_pos]
                        } else {
                            part
                        }
                        .trim();

                        // Extract just the column name (without table qualifier)
                        let final_col = if let Some(dot_pos) = col_name.find('.') {
                            &col_name[dot_pos + 1..]
                        } else {
                            col_name
                        };

                        if !final_col.is_empty() && !final_col.ends_with('*') {
                            selected_columns.insert(final_col.to_uppercase());
                            debug!(column = %final_col, "Excluding already selected column");
                        }
                    }
                }

                // Filter out selected columns
                if !selected_columns.is_empty() {
                    items.retain(|item| !selected_columns.contains(&item.label.to_uppercase()));
                }
            }
        }

        // Add SELECT clause keywords (DISTINCT, ALL, etc.)
        // Get dialect from document metadata, fallback to stored dialect
        let dialect = document
            .parse_metadata()
            .map(|m| m.dialect)
            .unwrap_or(self.dialect);
        let provider = KeywordProvider::new(dialect);
        let select_keywords = provider.select_clause_keywords().keywords;
        let keyword_items = CompletionRenderer::render_keywords(&select_keywords);
        items.extend(keyword_items);

        // Add expression keywords (WHEN, THEN, ELSE, etc.) if we're in a CASE expression
        // Check if source text ends with "CASE " (or CASE followed by whitespace)
        let text_upper = source.to_uppercase();
        let ends_with_case = text_upper.ends_with("CASE ")
            || text_upper.ends_with("CASE\t")
            || text_upper.ends_with(" CASE ")
            || text_upper.ends_with(" CASE\t");

        if ends_with_case {
            debug!("Detected CASE expression, adding expression keywords");
            let expr_keywords = provider.expression_keywords().keywords;
            let expr_items = CompletionRenderer::render_keywords(&expr_keywords);
            items.extend(expr_items);
        }

        Ok(Some(items))
    }

    /// Complete WHERE clause with columns, operators, and clause keywords
    ///
    /// This is specialized for WHERE clause completion.
    #[instrument(skip(self))]
    async fn complete_where_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting WHERE clause completion");

        // Get columns using the shared scope completion logic
        let mut items = match self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                true, // exclude_wildcard
                None, // function_filter (show all)
            )
            .await?
        {
            Some(items) => items,
            None => return Ok(None),
        };

        // If items are empty (e.g., invalid qualifier), return None
        if items.is_empty() {
            return Ok(None);
        }

        // When we have a table qualifier (e.g., "u."), filter to only items with that qualifier
        // This removes functions and other items that don't have the qualifier prefix
        if let Some(ref q) = qualifier {
            let qualifier_prefix = format!("{}.", q);
            debug!(qualifier_prefix = %qualifier_prefix, item_count = items.len(), "Filtering items by qualifier");
            items.retain(|i| i.label.starts_with(&qualifier_prefix));
            debug!(
                item_count_after = items.len(),
                "Items after qualifier filter"
            );
        }

        // Add WHERE clause keywords (AND, OR, etc.) and subsequent clauses
        // But NOT if we have a table qualifier (e.g., "u.") - in that case, only show columns
        if qualifier.is_none() {
            let dialect = self.dialect;
            let provider = KeywordProvider::new(dialect);

            // Get expression keywords (AND, OR, NOT, etc.)
            let expr_keywords = provider.expression_keywords().keywords;
            let expr_items = CompletionRenderer::render_keywords(&expr_keywords);
            items.extend(expr_items);

            // Get keywords after WHERE clause (GROUP BY, ORDER BY, etc.)
            let clause_keywords = provider.keywords_after_clause("WHERE");
            let clause_items = CompletionRenderer::render_keywords(&clause_keywords);
            items.extend(clause_items);
        }

        Ok(Some(items))
    }

    /// Shared completion logic for contexts with scope (SELECT/WHERE)
    ///
    /// This consolidates the duplicate logic between SelectProjection and WhereClause.
    #[instrument(skip(self))]
    async fn complete_with_scope(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        context_tables: Vec<String>,
        qualifier: Option<String>,
        exclude_wildcard: bool,
        function_filter: Option<FunctionType>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!(
            ?context_tables,
            scope_manager_exists = scope_manager.is_some(),
            "Starting scoped completion"
        );

        // Check if we should use context_tables instead of CST-based scope
        eprintln!(
            "!!! LSP: complete_with_scope: context_tables={:?}",
            context_tables
        );
        let use_context_tables = if !context_tables.is_empty() {
            // Check if the CST-based scope exists and has tables
            if let Some(manager) = scope_manager {
                let scope = manager.get_scope(0);
                match scope {
                    Some(s) => {
                        debug!(cst_table_count = s.tables.len(), "CST scope has tables");
                        s.tables.is_empty()
                    }
                    None => {
                        debug!("CST scope is None");
                        true
                    }
                }
            } else {
                debug!("scope_manager is None");
                true
            }
        } else {
            debug!("context_tables is empty");
            false
        };

        debug!(use_context_tables, "Decision on using context tables");

        // If we have context tables (from text-based fallback) but no scope or empty scope,
        // fetch tables directly from catalog using the context table names
        if use_context_tables {
            debug!(?context_tables, "Using context tables for completion");

            // Store a copy of CTE names for later use
            let context_tables_copy = context_tables.clone();

            // Build alias-to-table mapping from context_tables
            // When context_tables contains both table names and aliases (e.g., ["users", "u", "orders", "o"]),
            // we need to identify which are aliases and filter them out before resolution.
            // Heuristic: if a short string is a prefix of a longer string, it's likely an alias.
            let mut table_names_only = Vec::new();
            let mut alias_to_table: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();

            // Sort by length (longer first) to identify aliases
            let mut sorted_tables = context_tables.clone();
            sorted_tables.sort_by_key(|a| std::cmp::Reverse(a.len()));
            sorted_tables.dedup(); // Remove duplicates

            for (i, table) in sorted_tables.iter().enumerate() {
                // Check if any longer string starts with this shorter string
                // If "users" starts with "u", then "u" is an alias for "users"
                let mut is_alias = false;
                if table.len() < 6 {
                    // Only check short strings (likely aliases) against longer ones
                    for other in sorted_tables.iter().take(i) {
                        // Only check against longer strings that come before us
                        if other.len() > table.len()
                            && other.to_lowercase().starts_with(&table.to_lowercase())
                        {
                            // 'table' is a prefix of 'other', so 'table' is likely an alias
                            alias_to_table.insert(table.clone(), other.clone());
                            is_alias = true;
                            break;
                        }
                    }
                }

                if !is_alias {
                    table_names_only.push(table.clone());
                }
            }

            debug!(
                ?table_names_only,
                ?alias_to_table,
                "Filtered table names and built alias mapping"
            );

            // Use AliasResolver to resolve only table names (not aliases)
            let resolver = AliasResolver::new(self.catalog_fetcher.catalog());
            let mut tables_with_columns = resolver.resolve_multiple(table_names_only).await?;

            debug!(
                table_count = tables_with_columns.len(),
                "Resolved tables from context"
            );

            // Set aliases on resolved tables using our mapping
            for table in &mut tables_with_columns {
                if let Some(alias) = alias_to_table
                    .iter()
                    .find(|(_, table_name)| table_name.eq_ignore_ascii_case(&table.table_name))
                    .map(|(alias, _)| alias.clone())
                {
                    *table = table.clone().with_alias(alias);
                }
            }

            // Also check if the qualifier is an alias, and if so, add it to the mapping
            if let Some(ref q) = qualifier {
                if let Some(table_name) = alias_to_table.get(q) {
                    // Qualifier is an alias, make sure it maps to a table
                    debug!("Qualifier '{}' is an alias for table '{}'", q, table_name);
                }
            }

            // Clone tables_with_columns for later use (after potential move in match)
            let tables_with_columns_clone = tables_with_columns.clone();

            // Fetch functions from catalog
            let functions = self.catalog_fetcher.list_functions().await?;

            // Resolve qualifier if present to filter tables
            let tables_to_render = match &qualifier {
                Some(q) => {
                    // The qualifier could be:
                    // 1. An actual table name (e.g., "users")
                    // 2. A table alias (e.g., "u" for "users")
                    debug!(qualifier = %q, "Filtering tables by qualifier");

                    // First try to match by exact table name
                    let exact_match: Vec<_> = tables_with_columns
                        .iter()
                        .filter(|t| t.table_name.eq_ignore_ascii_case(q))
                        .cloned()
                        .collect();

                    if !exact_match.is_empty() {
                        debug!("Found exact match for qualifier");
                        exact_match
                    } else {
                        // Try to match by alias
                        let alias_match: Vec<_> = tables_with_columns
                            .iter()
                            .filter(|t| t.alias.as_ref().is_some_and(|a| a.eq_ignore_ascii_case(q)))
                            .cloned()
                            .collect();

                        if !alias_match.is_empty() {
                            debug!("Found alias match for qualifier");
                            alias_match
                        } else {
                            // Qualifier doesn't match any resolved table - might be a CTE
                            // Check if qualifier matches any name in context_tables (which includes CTEs)
                            let qualifier_matches_cte = context_tables_copy
                                .iter()
                                .any(|name| name.eq_ignore_ascii_case(q));

                            if qualifier_matches_cte {
                                debug!("Qualifier matches a CTE name, continuing to CTE rendering");
                                vec![] // Return empty list so CTE rendering logic can handle it
                            } else {
                                // Qualifier doesn't match any table or CTE - return empty
                                debug!("Qualifier doesn't match any table or CTE");
                                return Ok(None);
                            }
                        }
                    }
                }
                None => tables_with_columns,
            };

            debug!(tables_count = tables_to_render.len(), "Tables to render");

            // Render completion items
            // Force qualifier if there are multiple tables or if an explicit qualifier was provided
            let force_qualifier = qualifier.is_some() || tables_to_render.len() > 1;
            let mut items = CompletionRenderer::render_columns(&tables_to_render, force_qualifier);

            debug!(item_count = items.len(), "Rendered column items");

            // Check if context_tables contains names that weren't resolved from catalog (likely CTEs)
            // Get the set of resolved table names
            let resolved_table_names: std::collections::HashSet<String> = tables_with_columns_clone
                .iter()
                .map(|t| t.table_name.clone())
                .collect();

            // Find names in context_tables that weren't resolved (these are likely CTEs)
            let cte_names: Vec<_> = context_tables_copy
                .iter()
                .filter(|name| !resolved_table_names.contains(*name))
                .collect();

            // If we have CTEs, add them as completion items
            if !cte_names.is_empty() {
                debug!(
                    "Found {} potential CTE names not in catalog: {:?}",
                    cte_names.len(),
                    cte_names
                );
                for cte_name in cte_names {
                    // Create a simple completion item for the CTE
                    items.push(CompletionItem {
                        label: cte_name.clone(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some(format!("CTE: {}", cte_name)),
                        insert_text: Some(format!("{}.*", cte_name)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }

            // Filter out wildcard if needed
            if exclude_wildcard {
                items.retain(|i| i.label != "*");
            }

            // Add function completion items
            let function_items = CompletionRenderer::render_functions(&functions, function_filter);
            items.extend(function_items);

            debug!(
                total_item_count = items.len(),
                "Total items after adding functions"
            );
            return Ok(Some(items));
        }

        // Original logic for CST-based scope
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

    /// Extract the prefix being typed before cursor for filtering
    ///
    /// For example, in "SELECT * FROM or|", returns "or"
    fn extract_prefix_from_document(document: &Document, position: Position) -> Option<String> {
        use ropey::Rope;

        let content = document.get_content();
        let rope = Rope::from_str(content.as_str());
        let line = rope.get_line(position.line as usize)?;

        // Convert line to string
        let line_text = line.to_string();

        // Get text up to cursor position
        // Clamp to string length to avoid out-of-bounds panic
        let char_index = (position.character as usize).min(line_text.len());
        let up_to_cursor = &line_text[..char_index];

        // If cursor is immediately after a comma or space, there's no prefix
        // Check BEFORE trimming to detect trailing delimiters
        if up_to_cursor.ends_with(',')
            || up_to_cursor.ends_with(' ')
            || up_to_cursor.ends_with('\t')
        {
            return None;
        }

        // Find the last word before cursor
        let last_word = up_to_cursor.split_whitespace().last()?;

        // Strip trailing non-alphanumeric characters (commas, semicolons, etc.)
        // For example: "users," -> "users" for comma-style joins
        let last_word_clean = last_word.trim_end_matches(|c: char| !c.is_alphanumeric());

        Some(last_word_clean.to_string())
    }

    /// Complete ORDER BY clause with columns and sort directions
    #[instrument(skip(self))]
    async fn complete_order_by_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting ORDER BY clause completion");

        // Get columns using the shared scope completion logic
        let mut items: Vec<CompletionItem> = self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                true, // exclude_wildcard
                None, // function_filter (show all)
            )
            .await?
            .unwrap_or_default();

        // Add sort direction keywords (ASC, DESC)
        let dialect = self.dialect;
        let provider = KeywordProvider::new(dialect);
        let sort_keywords = provider.sort_direction_keywords().keywords;
        let sort_items = CompletionRenderer::render_keywords(&sort_keywords);
        items.extend(sort_items);

        Ok(Some(items))
    }

    /// Complete GROUP BY clause with columns and HAVING
    #[instrument(skip(self))]
    async fn complete_group_by_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting GROUP BY clause completion");

        // Get columns using the shared scope completion logic
        let mut items: Vec<CompletionItem> = self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                true, // exclude_wildcard
                None, // function_filter (show all)
            )
            .await?
            .unwrap_or_default();

        // Add HAVING keyword
        let dialect = self.dialect;
        let provider = KeywordProvider::new(dialect);
        let having_keywords = provider.having_keywords().keywords;
        let having_items = CompletionRenderer::render_keywords(&having_keywords);
        items.extend(having_items);

        Ok(Some(items))
    }

    /// Complete LIMIT clause with numbers and OFFSET
    #[instrument(skip(self))]
    async fn complete_limit_clause(&self) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting LIMIT clause completion");

        // Add common LIMIT numbers and OFFSET keyword
        let dialect = self.dialect;
        let provider = KeywordProvider::new(dialect);
        let limit_keywords = provider.limit_keywords().keywords;
        let items = CompletionRenderer::render_keywords(&limit_keywords);

        Ok(Some(items))
    }

    /// Complete HAVING clause with columns and aggregations
    #[instrument(skip(self))]
    async fn complete_having_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting HAVING clause completion");

        // Get columns using the shared scope completion logic
        let items: Vec<CompletionItem> = self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                true, // exclude_wildcard
                None, // function_filter (show all)
            )
            .await?
            .unwrap_or_default();

        Ok(Some(items))
    }

    /// Complete RETURNING clause with columns
    ///
    /// Provides column completion for PostgreSQL RETURNING clause after INSERT/UPDATE/DELETE
    #[instrument(skip(self))]
    async fn complete_returning_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        qualifier: Option<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!("Starting RETURNING clause completion");

        // Get columns using the shared scope completion logic
        let mut items: Vec<CompletionItem> = self
            .complete_with_scope(
                scope_manager,
                tables,
                qualifier.clone(),
                false, // exclude_wildcard (RETURNING * is valid, so don't exclude)
                None,  // function_filter (show all)
            )
            .await?
            .unwrap_or_default();

        // Ensure wildcard is included (it should be from complete_with_scope, but add it manually if not)
        if !items.iter().any(|i| i.label == "*") {
            debug!("Adding wildcard (*) to RETURNING clause completion");
            items.push(CompletionRenderer::wildcard_item());
        }

        Ok(Some(items))
    }

    /// Complete CTE (Common Table Expression) definition
    ///
    /// Suggests table names that can be used as sources for CTEs
    #[instrument(skip(self, _scope_manager))]
    async fn complete_cte_definition(
        &self,
        _scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        _document: &Document,
        _position: Position,
        available_tables: Vec<String>,
        defined_ctes: Vec<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!(
            "Starting CTE definition completion: available_tables={:?}, defined_ctes={:?}",
            available_tables, defined_ctes
        );

        let mut items = Vec::new();

        // If available_tables is empty, fetch all tables from catalog
        if available_tables.is_empty() {
            debug!("available_tables is empty, fetching from catalog");
            // Fetch all tables directly from catalog
            let catalog_tables = self.catalog_fetcher.list_tables().await?;
            debug!("catalog returned {} tables", catalog_tables.len());

            // Filter out already defined CTEs
            let exclude_lower: Vec<String> =
                defined_ctes.iter().map(|n| n.to_lowercase()).collect();

            for table in catalog_tables {
                // Skip if table name matches a defined CTE
                if exclude_lower.contains(&table.name.to_lowercase()) {
                    continue;
                }

                items.push(CompletionItem {
                    label: table.name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some(format!("Table: {}", table.name)),
                    documentation: None,
                    deprecated: None,
                    preselect: None,
                    sort_text: Some(format!("0_{}", table.name)),
                    filter_text: Some(table.name.clone()),
                    insert_text: Some(table.name.clone()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    insert_text_mode: None,
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: None,
                    label_details: None,
                });
            }
        } else {
            // Use the provided tables
            debug!("!!! LSP: using {} provided tables", available_tables.len());
            for table_name in available_tables {
                // Skip already defined CTEs
                if defined_ctes.contains(&table_name) {
                    continue;
                }

                items.push(CompletionItem {
                    label: table_name.clone(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some(format!("Table: {}", table_name)),
                    documentation: None,
                    deprecated: None,
                    preselect: None,
                    sort_text: Some(format!("0_{}", table_name)),
                    filter_text: Some(table_name.clone()),
                    insert_text: Some(table_name.clone()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    insert_text_mode: None,
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: None,
                    label_details: None,
                });
            }
        }

        // Add already defined CTEs
        for cte_name in &defined_ctes {
            items.push(CompletionItem {
                label: cte_name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                detail: Some(format!("CTE: {}", cte_name)),
                documentation: None,
                deprecated: None,
                preselect: None,
                sort_text: Some(format!("1_{}", cte_name)),
                filter_text: Some(cte_name.clone()),
                insert_text: Some(cte_name.clone()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                insert_text_mode: None,
                text_edit: None,
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
                label_details: None,
            });
        }

        Ok(Some(items))
    }

    /// Complete window function (OVER clause) specifications
    ///
    /// Provides column completion for PARTITION BY and ORDER BY within OVER clauses,
    /// and window function keywords at OVER clause start
    #[instrument(skip(self, scope_manager))]
    async fn complete_window_function_clause(
        &self,
        scope_manager: &Option<unified_sql_lsp_semantic::ScopeManager>,
        tables: Vec<String>,
        window_part: unified_sql_lsp_context::WindowFunctionPart,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        debug!(
            "Starting window function clause completion: tables={:?}, window_part={:?}",
            tables, window_part
        );

        match window_part {
            unified_sql_lsp_context::WindowFunctionPart::OverStart => {
                // At OVER (|), suggest window function keywords
                let provider = KeywordProvider::new(self.dialect);
                let keywords = provider.window_function_keywords().keywords;
                let items = CompletionRenderer::render_keywords(&keywords);
                Ok(Some(items))
            }
            unified_sql_lsp_context::WindowFunctionPart::PartitionBy
            | unified_sql_lsp_context::WindowFunctionPart::OrderBy => {
                // In PARTITION BY or ORDER BY, suggest columns
                let items: Vec<CompletionItem> = self
                    .complete_with_scope(
                        scope_manager,
                        tables,
                        None, // qualifier (no table prefix expected in OVER clauses)
                        true, // exclude_wildcard
                        None, // function_filter (show all)
                    )
                    .await?
                    .unwrap_or_default();

                Ok(Some(items))
            }
            unified_sql_lsp_context::WindowFunctionPart::WindowFrame => {
                // Window frame specification - suggest frame keywords
                let provider = KeywordProvider::new(self.dialect);
                let keywords = provider.window_frame_keywords().keywords;
                let items = CompletionRenderer::render_keywords(&keywords);
                Ok(Some(items))
            }
        }
    }

    /// Complete FROM clause with table names
    ///
    /// Filters out already-included tables and SQL keywords from the completion list.
    #[instrument(skip(self, document))]
    async fn complete_from_clause(
        &self,
        document: &Document,
        position: Position,
        exclude_tables: Vec<String>,
    ) -> Result<Option<Vec<CompletionItem>>, CompletionError> {
        // Extract prefix and filter out SQL keywords
        let prefix = Self::extract_prefix_from_document(document, position)
            .filter(|p| !Self::is_sql_keyword(p));

        let mut tables = self.catalog_fetcher.list_tables().await?;

        // Filter out excluded tables
        if !exclude_tables.is_empty() {
            let exclude_lower: Vec<String> =
                exclude_tables.iter().map(|n| n.to_lowercase()).collect();
            tables.retain(|t| !exclude_lower.contains(&t.name.to_lowercase()));
        }

        // Filter by prefix if present
        if let Some(ref p) = prefix
            && !p.is_empty()
        {
            tables.retain(|t| t.name.to_lowercase().starts_with(&p.to_lowercase()));
        }

        // Show schema qualifier if multiple schemas
        let schemas: HashSet<&str> = tables.iter().map(|t| t.schema.as_str()).collect();
        let items = CompletionRenderer::render_tables(&tables, schemas.len() > 1);

        Ok(Some(items))
    }

    /// Check if a word is a SQL keyword that should be filtered from table completion
    fn is_sql_keyword(word: &str) -> bool {
        matches!(
            word.to_uppercase().as_str(),
            "FROM"
                | "JOIN"
                | "INNER"
                | "LEFT"
                | "RIGHT"
                | "FULL"
                | "CROSS"
                | "STRAIGHT"
                | "UPDATE"
                | "INSERT"
                | "DELETE"
                | "CREATE"
                | "ALTER"
                | "DROP"
                | "INTO"
        )
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
        // Should return None for invalid qualifier (no completions available)
        assert!(items.is_none());
    }

    #[tokio::test]
    async fn test_where_clause_ambiguous_column() {
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_catalog::TableMetadata;
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
