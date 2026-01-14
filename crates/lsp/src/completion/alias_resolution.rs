// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Alias Resolution Module
//!
//! This module provides intelligent table alias resolution for SQL completion.
//!
//! When users write SQL queries with table aliases (e.g., `SELECT u.* FROM users AS u`),
//! the completion engine needs to resolve these aliases to actual table names to provide
//! accurate column completions.
//!
//! ## Resolution Strategies
//!
//! The resolver applies multiple strategies in order:
//!
//! 1. **Exact Match** - Try loading the alias as a real table name
//! 2. **Starts With** - Find tables where the name starts with the alias (e.g., "u" -> "users")
//! 3. **First Letter + Numeric** - Match first letter with numeric suffix (e.g., "e1" -> "employees")
//! 4. **Single Table Fallback** - If only one table exists, use it

use std::sync::Arc;
use tracing::{debug, instrument};
use unified_sql_lsp_semantic::TableSymbol;

use crate::completion::catalog_integration::CatalogCompletionFetcher;
use crate::completion::error::CompletionError;

/// Strategy for resolving table aliases to actual table names
#[derive(Debug, Clone, Copy)]
pub enum ResolutionStrategy {
    /// Try exact table name match first
    ExactMatch,

    /// Match tables where name starts with alias (case-insensitive)
    StartsWith,

    /// Match first letter + numeric suffix pattern (e.g., "e1" -> "employees")
    FirstLetterPlusNumeric,

    /// Use the only available table (for self-join scenarios)
    SingleTableFallback,
}

impl ResolutionStrategy {
    /// Get all strategies in order of precedence
    pub fn all() -> &'static [ResolutionStrategy] {
        use ResolutionStrategy::*;
        &[
            ExactMatch,
            StartsWith,
            FirstLetterPlusNumeric,
            SingleTableFallback,
        ]
    }
}

/// Result of an alias resolution attempt
#[derive(Debug)]
pub enum ResolutionResult {
    /// Successfully resolved to a table with columns
    Found(TableSymbol),
    /// Table exists but has no columns (might be a non-matching alias)
    EmptyTable(TableSymbol),
    /// No matching table found
    NotFound,
}

/// Resolver for table aliases in SQL completion
///
/// # Examples
///
/// ```rust,ignore
/// let resolver = AliasResolver::new(catalog_fetcher);
/// match resolver.resolve("u").await {
///     Ok(ResolutionResult::Found(table)) => {
///         // Use table for completion
///     }
///     Ok(ResolutionResult::NotFound) => {
///         // No table found
///     }
///     Err(e) => {
///         // Handle error
///     }
/// }
/// ```
pub struct AliasResolver {
    catalog_fetcher: Arc<CatalogCompletionFetcher>,
}

impl AliasResolver {
    /// Create a new alias resolver
    pub fn new(catalog_fetcher: Arc<CatalogCompletionFetcher>) -> Self {
        Self { catalog_fetcher }
    }

    /// Resolve a table alias to its actual table symbol
    ///
    /// Applies multiple resolution strategies in order to find the best match.
    #[instrument(skip(self), fields(alias = %alias))]
    pub async fn resolve(&self, alias: String) -> Result<ResolutionResult, CompletionError> {
        debug!("Starting alias resolution for '{}'", alias);

        // Try each strategy in order
        for strategy in ResolutionStrategy::all() {
            debug!(?strategy, "Attempting resolution strategy");

            match self.try_strategy(&alias, *strategy).await? {
                ResolutionResult::Found(table) => {
                    debug!(table_name = %table.table_name, columns = table.columns.len(), "Successfully resolved alias");
                    return Ok(ResolutionResult::Found(table));
                }
                ResolutionResult::EmptyTable(table) => {
                    debug!(table_name = %table.table_name, "Found empty table, continuing to next strategy");
                    // Continue to next strategy
                }
                ResolutionResult::NotFound => {
                    debug!("Strategy returned NotFound, continuing");
                    // Continue to next strategy
                }
            }
        }

        debug!("All strategies exhausted, alias not found");
        Ok(ResolutionResult::NotFound)
    }

    /// Try a specific resolution strategy
    async fn try_strategy(
        &self,
        alias: &str,
        strategy: ResolutionStrategy,
    ) -> Result<ResolutionResult, CompletionError> {
        match strategy {
            ResolutionStrategy::ExactMatch => self.try_exact_match(alias).await,
            ResolutionStrategy::StartsWith => self.try_starts_with(alias).await,
            ResolutionStrategy::FirstLetterPlusNumeric => {
                self.try_first_letter_numeric(alias).await
            }
            ResolutionStrategy::SingleTableFallback => self.try_single_table_fallback(alias).await,
        }
    }

    /// Strategy 1: Try exact table name match
    async fn try_exact_match(&self, alias: &str) -> Result<ResolutionResult, CompletionError> {
        match self.catalog_fetcher.populate_single_table(alias).await {
            Ok(table) => {
                if table.columns.is_empty() {
                    debug!("Exact match found but table has no columns");
                    Ok(ResolutionResult::EmptyTable(table))
                } else {
                    debug!("Exact match found with columns");
                    Ok(ResolutionResult::Found(table))
                }
            }
            Err(_) => {
                debug!("Exact match failed, table not found");
                Ok(ResolutionResult::NotFound)
            }
        }
    }

    /// Strategy 2: Find tables that start with the alias
    async fn try_starts_with(&self, alias: &str) -> Result<ResolutionResult, CompletionError> {
        let all_tables = self.catalog_fetcher.list_tables().await?;

        for table in &all_tables {
            if table.name.to_lowercase().starts_with(&alias.to_lowercase()) {
                debug!(found_table = %table.name, "Found table starting with alias");
                return self
                    .catalog_fetcher
                    .populate_single_table(&table.name)
                    .await
                    .map(|mut t| {
                        t.alias = Some(alias.to_string());
                        ResolutionResult::Found(t)
                    });
            }
        }

        debug!("No tables found starting with alias");
        Ok(ResolutionResult::NotFound)
    }

    /// Strategy 3: Match first letter + numeric suffix (e.g., "e1" -> "employees")
    async fn try_first_letter_numeric(
        &self,
        alias: &str,
    ) -> Result<ResolutionResult, CompletionError> {
        if alias.is_empty() {
            return Ok(ResolutionResult::NotFound);
        }

        // Extract first character and check if rest is numeric
        let first_char = alias.chars().next().unwrap();
        let rest: String = alias.chars().skip(1).collect();

        if !rest.chars().all(|c| c.is_numeric()) {
            debug!("Alias does not match first-letter + numeric pattern");
            return Ok(ResolutionResult::NotFound);
        }

        let all_tables = self.catalog_fetcher.list_tables().await?;

        for table in &all_tables {
            if let Some(table_first_char) = table.name.chars().next()
                && table_first_char.eq_ignore_ascii_case(&first_char)
            {
                debug!(found_table = %table.name, "Found table matching first letter pattern");
                return self
                    .catalog_fetcher
                    .populate_single_table(&table.name)
                    .await
                    .map(|mut t| {
                        t.alias = Some(alias.to_string());
                        ResolutionResult::Found(t)
                    });
            }
        }

        debug!("No tables found matching first letter pattern");
        Ok(ResolutionResult::NotFound)
    }

    /// Strategy 4: If only one table exists, use it (for self-join scenarios)
    async fn try_single_table_fallback(
        &self,
        alias: &str,
    ) -> Result<ResolutionResult, CompletionError> {
        let all_tables = self.catalog_fetcher.list_tables().await?;

        if all_tables.len() == 1 {
            let table_name = &all_tables[0].name;
            debug!(table = %table_name, "Using single table fallback");
            return self
                .catalog_fetcher
                .populate_single_table(table_name)
                .await
                .map(|mut t| {
                    t.alias = Some(alias.to_string());
                    ResolutionResult::Found(t)
                });
        }

        debug!(
            table_count = all_tables.len(),
            "Multiple tables available, skipping single table fallback"
        );
        Ok(ResolutionResult::NotFound)
    }

    /// Resolve multiple aliases concurrently
    ///
    /// Useful for join conditions where both left and right tables need resolution.
    pub async fn resolve_multiple(
        &self,
        aliases: Vec<String>,
    ) -> Result<Vec<TableSymbol>, CompletionError> {
        let mut results = Vec::new();

        for alias in aliases {
            match self.resolve(alias).await? {
                ResolutionResult::Found(table) => results.push(table),
                ResolutionResult::EmptyTable(_) => {
                    debug!("Skipping empty table for alias");
                }
                ResolutionResult::NotFound => {
                    debug!("No table found for alias, skipping");
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::{DataType, TableMetadata};
    use unified_sql_lsp_test_utils::MockCatalogBuilder;

    #[tokio::test]
    async fn test_exact_match_resolution() {
        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
                unified_sql_lsp_catalog::ColumnMetadata::new("name", DataType::Text),
            ]))
            .build();

        let fetcher = Arc::new(CatalogCompletionFetcher::new(Arc::new(catalog)));
        let resolver = AliasResolver::new(fetcher);

        match resolver.resolve("users".to_string()).await.unwrap() {
            ResolutionResult::Found(table) => {
                assert_eq!(table.table_name, "users");
                assert_eq!(table.columns.len(), 2);
            }
            _ => panic!("Expected Found result"),
        }
    }

    #[tokio::test]
    async fn test_starts_with_resolution() {
        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("users", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .build();

        let fetcher = Arc::new(CatalogCompletionFetcher::new(Arc::new(catalog)));
        let resolver = AliasResolver::new(fetcher);

        match resolver.resolve("u".to_string()).await.unwrap() {
            ResolutionResult::Found(table) => {
                assert_eq!(table.table_name, "users");
                assert_eq!(table.alias.as_ref().unwrap(), "u");
            }
            _ => panic!("Expected Found result"),
        }
    }

    #[tokio::test]
    async fn test_first_letter_numeric_resolution() {
        let catalog = MockCatalogBuilder::new()
            .with_table(TableMetadata::new("employees", "public").with_columns(vec![
                unified_sql_lsp_catalog::ColumnMetadata::new("id", DataType::Integer),
            ]))
            .build();

        let fetcher = Arc::new(CatalogCompletionFetcher::new(Arc::new(catalog)));
        let resolver = AliasResolver::new(fetcher);

        match resolver.resolve("e1".to_string()).await.unwrap() {
            ResolutionResult::Found(table) => {
                assert_eq!(table.table_name, "employees");
                assert_eq!(table.alias.as_ref().unwrap(), "e1");
            }
            _ => panic!("Expected Found result"),
        }
    }
}
