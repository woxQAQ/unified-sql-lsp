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

use crate::TableSymbol;
use std::sync::Arc;
use tracing::{debug, instrument};
use unified_sql_lsp_catalog::Catalog;

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

/// Alias resolution error
#[derive(Debug, thiserror::Error)]
pub enum AliasResolutionError {
    #[error("Catalog error: {0}")]
    Catalog(#[from] unified_sql_lsp_catalog::CatalogError),

    #[error("No matching table found for alias: {0}")]
    NotFound(String),
}

/// Resolver for table aliases in SQL completion
///
/// # Examples
///
/// ```rust,ignore
/// let resolver = AliasResolver::new(catalog);
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
    catalog: Arc<dyn Catalog>,
}

impl AliasResolver {
    /// Create a new alias resolver
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    /// Resolve a table alias to its actual table symbol
    ///
    /// Applies multiple resolution strategies in order to find the best match.
    #[instrument(skip(self), fields(alias = %alias))]
    pub async fn resolve(&self, alias: String) -> Result<ResolutionResult, AliasResolutionError> {
        debug!("Starting alias resolution for '{}'", alias);

        // Try each strategy in order
        for strategy in ResolutionStrategy::all() {
            debug!(?strategy, "Attempting resolution strategy");

            match self.try_strategy(&alias, *strategy).await? {
                ResolutionResult::Found(table) => {
                    debug!(
                        table_name = %table.table_name,
                        columns = table.columns.len(),
                        "Successfully resolved alias"
                    );
                    return Ok(ResolutionResult::Found(table));
                }
                ResolutionResult::EmptyTable(table) => {
                    debug!(
                        table_name = %table.table_name,
                        "Found empty table, continuing to next strategy"
                    );
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
    ) -> Result<ResolutionResult, AliasResolutionError> {
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
    async fn try_exact_match(&self, alias: &str) -> Result<ResolutionResult, AliasResolutionError> {
        match self.catalog.get_columns(alias).await {
            Ok(columns) => {
                let mut table = TableSymbol::new(alias);
                table = table.with_columns(
                    columns
                        .into_iter()
                        .map(|c| {
                            crate::ColumnSymbol::new(
                                c.name.clone(),
                                c.data_type.clone(),
                                alias.to_string(),
                            )
                            .with_primary_key_if(c.is_primary_key)
                            .with_foreign_key_if(c.is_foreign_key)
                        })
                        .collect(),
                );

                if table.columns.is_empty() {
                    debug!("Exact match found but table has no columns");
                    Ok(ResolutionResult::EmptyTable(table))
                } else {
                    debug!("Exact match found with columns");
                    Ok(ResolutionResult::Found(table))
                }
            }
            Err(_e) => {
                debug!("Exact match failed, table not found");
                Ok(ResolutionResult::NotFound)
            }
        }
    }

    /// Strategy 2: Find tables that start with the alias
    ///
    /// Prefers exact word matches and shorter table names:
    /// - Exact match (table name equals alias) - highest priority
    /// - Word boundary match (ends with alias or followed by _) - high priority
    /// - Shortest table name (for same prefix) - fallback priority
    async fn try_starts_with(&self, alias: &str) -> Result<ResolutionResult, AliasResolutionError> {
        let all_tables = self.catalog.list_tables().await?;

        // First pass: exact match (table name equals alias)
        for table in &all_tables {
            if table.name.eq_ignore_ascii_case(alias) {
                debug!(found_table = %table.name, "Found exact match for alias");
                match self.catalog.get_columns(&table.name).await {
                    Ok(columns) => {
                        let mut table_symbol = TableSymbol::new(&table.name);
                        table_symbol = table_symbol.with_alias(alias);
                        table_symbol = table_symbol.with_columns(
                            columns
                                .into_iter()
                                .map(|c| {
                                    crate::ColumnSymbol::new(
                                        c.name.clone(),
                                        c.data_type.clone(),
                                        table.name.clone(),
                                    )
                                    .with_primary_key_if(c.is_primary_key)
                                    .with_foreign_key_if(c.is_foreign_key)
                                })
                                .collect(),
                        );
                        return Ok(ResolutionResult::Found(table_symbol));
                    }
                    Err(_) => continue,
                }
            }
        }

        // Second pass: word boundary match (e.g., "ord" -> "orders" not "order_items")
        let alias_lower = alias.to_lowercase();
        let mut best_match: Option<(String, usize)> = None; // (table_name, length)

        for table in &all_tables {
            if table.name.to_lowercase().starts_with(&alias_lower) {
                let next_char = table.name[alias.len()..].chars().next();
                // Check if we're at a word boundary (end of string or followed by _)
                if next_char.is_none() || next_char == Some('_') {
                    debug!(found_table = %table.name, "Found table with word boundary match");
                    let table_len = table.name.len();
                    match &best_match {
                        None => {
                            best_match = Some((table.name.clone(), table_len));
                        }
                        Some((_, current_len)) => {
                            // Prefer shorter table names (closer match)
                            if table_len < *current_len {
                                best_match = Some((table.name.clone(), table_len));
                            }
                        }
                    }
                }
            }
        }

        if let Some((table_name, _)) = best_match {
            if let Ok(columns) = self.catalog.get_columns(&table_name).await {
                let mut table_symbol = TableSymbol::new(&table_name);
                table_symbol = table_symbol.with_alias(alias);
                table_symbol = table_symbol.with_columns(
                    columns
                        .into_iter()
                        .map(|c| {
                            crate::ColumnSymbol::new(
                                c.name.clone(),
                                c.data_type.clone(),
                                table_name.clone(),
                            )
                            .with_primary_key_if(c.is_primary_key)
                            .with_foreign_key_if(c.is_foreign_key)
                        })
                        .collect(),
                );
                return Ok(ResolutionResult::Found(table_symbol));
            }
        }

        // Third pass: any match, prefer shorter table names
        let mut best_match: Option<(String, usize)> = None;

        for table in &all_tables {
            if table.name.to_lowercase().starts_with(&alias_lower) {
                let table_len = table.name.len();
                match &best_match {
                    None => {
                        best_match = Some((table.name.clone(), table_len));
                    }
                    Some((_, current_len)) => {
                        // Prefer shorter table names (closer match)
                        if table_len < *current_len {
                            best_match = Some((table.name.clone(), table_len));
                        }
                    }
                }
            }
        }

        if let Some((table_name, _)) = best_match {
            debug!(found_table = %table_name, "Found shortest table starting with alias");
            if let Ok(columns) = self.catalog.get_columns(&table_name).await {
                let mut table_symbol = TableSymbol::new(&table_name);
                table_symbol = table_symbol.with_alias(alias);
                table_symbol = table_symbol.with_columns(
                    columns
                        .into_iter()
                        .map(|c| {
                            crate::ColumnSymbol::new(
                                c.name.clone(),
                                c.data_type.clone(),
                                table_name.clone(),
                            )
                            .with_primary_key_if(c.is_primary_key)
                            .with_foreign_key_if(c.is_foreign_key)
                        })
                        .collect(),
                );
                return Ok(ResolutionResult::Found(table_symbol));
            }
        }

        debug!("No tables found starting with alias");
        Ok(ResolutionResult::NotFound)
    }

    /// Strategy 3: Match first letter + numeric suffix (e.g., "e1" -> "employees")
    async fn try_first_letter_numeric(
        &self,
        alias: &str,
    ) -> Result<ResolutionResult, AliasResolutionError> {
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

        let all_tables = self.catalog.list_tables().await?;

        for table in all_tables {
            let table_first_char = match table.name.chars().next() {
                Some(c) => c,
                None => continue,
            };
            if table_first_char.eq_ignore_ascii_case(&first_char) {
                debug!(
                    found_table = %table.name,
                    "Found table matching first letter pattern"
                );
                match self.catalog.get_columns(&table.name).await {
                    Ok(columns) => {
                        let mut table_symbol = TableSymbol::new(&table.name);
                        table_symbol = table_symbol.with_alias(alias);
                        table_symbol = table_symbol.with_columns(
                            columns
                                .into_iter()
                                .map(|c| {
                                    crate::ColumnSymbol::new(
                                        c.name.clone(),
                                        c.data_type.clone(),
                                        table.name.clone(),
                                    )
                                    .with_primary_key_if(c.is_primary_key)
                                    .with_foreign_key_if(c.is_foreign_key)
                                })
                                .collect(),
                        );
                        return Ok(ResolutionResult::Found(table_symbol));
                    }
                    Err(_) => continue,
                }
            }
        }

        debug!("No tables found matching first letter pattern");
        Ok(ResolutionResult::NotFound)
    }

    /// Strategy 4: If only one table exists, use it (for self-join scenarios)
    async fn try_single_table_fallback(
        &self,
        alias: &str,
    ) -> Result<ResolutionResult, AliasResolutionError> {
        let all_tables = self.catalog.list_tables().await?;

        if all_tables.len() == 1 {
            let table_name = &all_tables[0].name;
            debug!(table = %table_name, "Using single table fallback");
            match self.catalog.get_columns(table_name).await {
                Ok(columns) => {
                    let mut table_symbol = TableSymbol::new(table_name);
                    table_symbol = table_symbol.with_alias(alias);
                    table_symbol = table_symbol.with_columns(
                        columns
                            .into_iter()
                            .map(|c| {
                                crate::ColumnSymbol::new(
                                    c.name.clone(),
                                    c.data_type.clone(),
                                    table_name.clone(),
                                )
                                .with_primary_key_if(c.is_primary_key)
                                .with_foreign_key_if(c.is_foreign_key)
                            })
                            .collect(),
                    );
                    Ok(ResolutionResult::Found(table_symbol))
                }
                Err(e) => Err(AliasResolutionError::Catalog(e)),
            }
        } else {
            debug!(
                table_count = all_tables.len(),
                "Multiple tables available, skipping single table fallback"
            );
            Ok(ResolutionResult::NotFound)
        }
    }

    /// Resolve multiple aliases concurrently
    ///
    /// Useful for join conditions where both left and right tables need resolution.
    pub async fn resolve_multiple(
        &self,
        aliases: Vec<String>,
    ) -> Result<Vec<TableSymbol>, AliasResolutionError> {
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

        let resolver = AliasResolver::new(Arc::new(catalog));

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

        let resolver = AliasResolver::new(Arc::new(catalog));

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

        let resolver = AliasResolver::new(Arc::new(catalog));

        match resolver.resolve("e1".to_string()).await.unwrap() {
            ResolutionResult::Found(table) => {
                assert_eq!(table.table_name, "employees");
                assert_eq!(table.alias.as_ref().unwrap(), "e1");
            }
            _ => panic!("Expected Found result"),
        }
    }
}
