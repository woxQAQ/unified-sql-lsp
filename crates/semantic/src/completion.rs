// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Completion-oriented semantic helpers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::debug;
use unified_sql_lsp_catalog::Catalog;

use crate::{AliasResolutionError, AliasResolver, ColumnSymbol, ScopeManager, TableSymbol};

/// Resolution output for completion contexts that only provide table names.
#[derive(Debug, Clone)]
pub struct ContextTableResolution {
    /// Tables filtered for rendering based on current qualifier.
    pub tables_to_render: Vec<TableSymbol>,
    /// All resolved tables before qualifier filtering.
    pub resolved_tables: Vec<TableSymbol>,
}

/// Semantic completion helper service.
pub struct CompletionService {
    catalog: Arc<dyn Catalog>,
}

/// Text-level completion heuristics shared across adapters.
pub struct CompletionTextHeuristics;

impl CompletionService {
    pub fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self { catalog }
    }

    /// Resolve context tables and apply qualifier filtering.
    ///
    /// Returns `Ok(None)` when qualifier is invalid and does not match any known context name.
    pub async fn resolve_context_tables(
        &self,
        context_tables: Vec<String>,
        qualifier: Option<&str>,
    ) -> Result<Option<ContextTableResolution>, AliasResolutionError> {
        let (table_names_only, alias_to_table) =
            Self::split_table_and_aliases(context_tables.clone());

        debug!(
            ?table_names_only,
            ?alias_to_table,
            "Resolved table/alias candidates for completion"
        );

        let resolver = AliasResolver::new(Arc::clone(&self.catalog));
        let mut resolved_tables = resolver.resolve_multiple(table_names_only).await?;

        for table in &mut resolved_tables {
            if let Some(alias) = alias_to_table
                .iter()
                .find(|(_, table_name)| table_name.eq_ignore_ascii_case(&table.table_name))
                .map(|(alias, _)| alias.clone())
            {
                *table = table.clone().with_alias(alias);
            }
        }

        let tables_to_render = match qualifier {
            Some(q) => {
                let exact_match: Vec<_> = resolved_tables
                    .iter()
                    .filter(|t| t.table_name.eq_ignore_ascii_case(q))
                    .cloned()
                    .collect();
                if !exact_match.is_empty() {
                    exact_match
                } else {
                    let alias_match: Vec<_> = resolved_tables
                        .iter()
                        .filter(|t| t.alias.as_ref().is_some_and(|a| a.eq_ignore_ascii_case(q)))
                        .cloned()
                        .collect();
                    if !alias_match.is_empty() {
                        alias_match
                    } else {
                        let qualifier_matches_context = context_tables
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(q));
                        if qualifier_matches_context {
                            Vec::new()
                        } else {
                            return Ok(None);
                        }
                    }
                }
            }
            None => resolved_tables.clone(),
        };

        Ok(Some(ContextTableResolution {
            tables_to_render,
            resolved_tables,
        }))
    }

    /// Resolve JOIN tables and apply optional qualifier filtering.
    ///
    /// Returns `Ok(None)` when no tables are resolved or qualifier filters out all tables.
    pub async fn resolve_join_tables(
        &self,
        table_names: Vec<String>,
        qualifier: Option<&str>,
    ) -> Result<Option<ContextTableResolution>, AliasResolutionError> {
        let resolver = AliasResolver::new(Arc::clone(&self.catalog));
        let resolved_tables = resolver.resolve_multiple(table_names).await?;

        if resolved_tables.is_empty() {
            return Ok(None);
        }

        let tables_to_render = match qualifier {
            Some(q) => resolved_tables
                .iter()
                .filter(|t| t.alias.as_ref().map(|a| a == q).unwrap_or(false) || t.table_name == q)
                .cloned()
                .collect(),
            None => resolved_tables.clone(),
        };

        if tables_to_render.is_empty() {
            return Ok(None);
        }

        Ok(Some(ContextTableResolution {
            tables_to_render,
            resolved_tables,
        }))
    }

    /// Populate scope tables from catalog and resolve tables for rendering.
    ///
    /// Returns:
    /// - `None` if the scope does not exist.
    /// - `Some(vec![])` if qualifier is invalid.
    /// - `Some(tables)` otherwise.
    pub async fn resolve_scope_tables(
        &self,
        scope_manager: &mut ScopeManager,
        scope_id: usize,
        qualifier: Option<&str>,
    ) -> Option<Vec<TableSymbol>> {
        {
            let scope = scope_manager.get_scope_mut(scope_id)?;
            for table in &mut scope.tables {
                match self.catalog.get_columns(&table.table_name).await {
                    Ok(columns_metadata) => {
                        table.columns = columns_metadata
                            .iter()
                            .map(|meta| Self::metadata_to_symbol(meta, &table.table_name))
                            .collect();
                    }
                    Err(e) => {
                        debug!(
                            "Warning: Failed to load columns for table '{}': {}",
                            table.table_name, e
                        );
                    }
                }
            }
        }

        match qualifier {
            Some(q) => {
                let scope = scope_manager.get_scope(scope_id)?;
                Some(
                    scope
                        .find_table(q)
                        .map(|qualified_table| vec![qualified_table.clone()])
                        .unwrap_or_default(),
                )
            }
            None => {
                let scope = scope_manager.get_scope(scope_id)?;
                Some(scope.tables.clone())
            }
        }
    }

    fn split_table_and_aliases(
        context_tables: Vec<String>,
    ) -> (Vec<String>, HashMap<String, String>) {
        let mut table_names_only = Vec::new();
        let mut alias_to_table = HashMap::new();

        let mut sorted_tables = context_tables;
        sorted_tables.sort_by_key(|a| std::cmp::Reverse(a.len()));
        sorted_tables.dedup();

        for (i, table) in sorted_tables.iter().enumerate() {
            let mut is_alias = false;
            if table.len() < 6 {
                for other in sorted_tables.iter().take(i) {
                    if other.len() > table.len()
                        && other.to_lowercase().starts_with(&table.to_lowercase())
                    {
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

        (table_names_only, alias_to_table)
    }

    fn metadata_to_symbol(
        meta: &unified_sql_lsp_catalog::ColumnMetadata,
        table_name: &str,
    ) -> ColumnSymbol {
        let mut symbol = ColumnSymbol::new(meta.name.clone(), meta.data_type.clone(), table_name);

        if meta.is_primary_key {
            symbol = symbol.with_primary_key();
        }
        if meta.is_foreign_key {
            symbol = symbol.with_foreign_key();
        }

        symbol
    }
}

impl CompletionTextHeuristics {
    /// Extract selected column names from a SELECT projection list (upper-cased).
    pub fn selected_projection_columns_upper(source: &str) -> HashSet<String> {
        let mut selected_columns = HashSet::new();
        if let Some(select_pos) = source.to_uppercase().find("SELECT") {
            let after_select = &source[select_pos + 6..];
            if let Some(from_pos) = after_select.to_uppercase().find("FROM") {
                let select_clause = &after_select[..from_pos];
                for part in select_clause.split(',') {
                    let part = part.trim();
                    if part.is_empty() || part.starts_with('(') {
                        continue;
                    }

                    let col_name = if let Some(as_pos) = part.to_uppercase().find(" AS ") {
                        &part[..as_pos]
                    } else {
                        part
                    }
                    .trim();

                    let final_col = if let Some(dot_pos) = col_name.find('.') {
                        &col_name[dot_pos + 1..]
                    } else {
                        col_name
                    };

                    if !final_col.is_empty() && !final_col.ends_with('*') {
                        selected_columns.insert(final_col.to_uppercase());
                    }
                }
            }
        }
        selected_columns
    }

    /// Detect whether source currently ends with a CASE keyword boundary.
    pub fn ends_with_case_expression(source: &str) -> bool {
        let text_upper = source.to_uppercase();
        text_upper.ends_with("CASE ")
            || text_upper.ends_with("CASE\t")
            || text_upper.ends_with(" CASE ")
            || text_upper.ends_with(" CASE\t")
    }

    /// Decide whether JOIN column completion should force table qualifier.
    ///
    /// USING clause does not use qualifiers, so it always returns false.
    pub fn should_force_join_qualifier(source: &str, resolved_table_count: usize) -> bool {
        if source.to_uppercase().contains("USING") {
            false
        } else {
            resolved_table_count > 1
        }
    }
}
