// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Semantic Analyzer
//!
//! This module implements the core semantic analysis logic for SQL queries.
//!
//! The analyzer traverses IR queries, builds scope hierarchies, validates
//! column references, and integrates with the catalog for schema metadata.

use std::collections::HashMap;
use std::sync::Arc;

use unified_sql_lsp_catalog::Catalog;
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_ir::{ColumnRef, Expr, Query, SelectItem, SelectStatement, SetOp, TableRef};

use crate::error::{SemanticError, SemanticResult};
use crate::resolution::ColumnResolver;
use crate::scope::{ScopeManager, ScopeType};
use crate::symbol::{ColumnSymbol, TableSymbol};

/// Semantic analyzer for SQL queries
///
/// The analyzer traverses IR queries, builds scope hierarchies, validates
/// column references, and resolves table/column symbols.
pub struct SemanticAnalyzer {
    /// Catalog for fetching table and column metadata
    catalog: Arc<dyn Catalog>,

    /// SQL dialect for the query being analyzed
    dialect: Dialect,

    /// Scope manager for tracking tables across query hierarchy
    scope_manager: ScopeManager,

    /// Cache of column metadata per table (table_name -> columns)
    column_cache: HashMap<String, Vec<ColumnSymbol>>,

    /// Root scope ID for the most recently analyzed query
    root_scope_id: Option<usize>,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    ///
    /// # Arguments
    ///
    /// * `catalog` - Catalog for fetching table/column metadata
    /// * `dialect` - SQL dialect for validation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use unified_sql_lsp_semantic::SemanticAnalyzer;
    /// use unified_sql_lsp_ir::Dialect;
    ///
    /// let analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);
    /// ```
    pub fn new(catalog: Arc<dyn Catalog>, dialect: Dialect) -> Self {
        Self {
            catalog,
            dialect,
            scope_manager: ScopeManager::new(),
            column_cache: HashMap::new(),
            root_scope_id: None,
        }
    }

    /// Analyze a complete IR query and build scope hierarchy
    ///
    /// This is the main entry point for semantic analysis. It:
    /// 1. Extracts table names from the query
    /// 2. Pre-fetches column metadata from catalog
    /// 3. Builds CTE scopes (if any)
    /// 4. Builds main query scope
    /// 5. Validates all column references
    ///
    /// # Arguments
    ///
    /// * `query` - IR query to analyze
    ///
    /// # Returns
    ///
    /// The root scope ID for the analyzed query
    ///
    /// # Errors
    ///
    /// Returns `SemanticError` if:
    /// - Table not found in catalog
    /// - Column not found
    /// - Column reference is ambiguous
    /// - Duplicate table alias
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let root_scope = analyzer.analyze_query(&query).await?;
    /// let tables = analyzer.visible_tables(root_scope);
    /// ```
    pub async fn analyze_query(&mut self, query: &Query) -> SemanticResult<usize> {
        // Reset state for new query
        self.scope_manager = ScopeManager::new();
        self.column_cache.clear();
        self.root_scope_id = None;

        // Step 1: Extract all table names from query
        let table_names = self.extract_table_names(query)?;

        // Step 2: Pre-fetch column metadata from catalog
        self.prefetch_table_metadata(table_names).await?;

        // Step 3: Process CTEs if present
        // TODO: (SEMANTIC-006) Implement CTE scope creation and synthetic tables
        if !query.ctes.is_empty() {
            // For now, we'll skip CTE processing and add a placeholder
            // In the future, this will create CTE scopes and synthetic tables
        }

        // Step 4: Build main query scope
        let root_scope = match &query.body {
            SetOp::Select(select) => self.build_main_query_scope(select)?,
            // TODO: (SEMANTIC-006) Handle UNION, INTERSECT, EXCEPT
            _ => {
                // For set operations, create a basic scope
                // This is a placeholder for future implementation
                let scope_id = self.scope_manager.create_scope(ScopeType::Query, None);
                scope_id
            }
        };

        self.root_scope_id = Some(root_scope);
        Ok(root_scope)
    }

    /// Resolve a column reference at a specific scope
    ///
    /// Handles both qualified (`u.id`) and unqualified (`id`) column references.
    ///
    /// # Arguments
    ///
    /// * `column_ref` - Column reference to resolve
    /// * `scope_id` - Scope ID for resolution
    ///
    /// # Returns
    ///
    /// Tuple of (table symbol, column symbol) if found
    ///
    /// # Errors
    ///
    /// - `ColumnNotFound` if column doesn't exist
    /// - `AmbiguousColumn` if unqualified column found in multiple tables
    pub fn resolve_column(
        &self,
        column_ref: &ColumnRef,
        scope_id: usize,
    ) -> SemanticResult<(&TableSymbol, &ColumnSymbol)> {
        // Qualified reference: u.id
        if let Some(table_qualifier) = &column_ref.table {
            let table = self
                .scope_manager
                .resolve_table(table_qualifier, scope_id)?;

            return table
                .find_column(&column_ref.column)
                .ok_or_else(|| SemanticError::ColumnNotFound(column_ref.qualified()))
                .map(|col| (table, col));
        }

        // Unqualified reference: id (search all visible tables)
        self.scope_manager
            .resolve_column(&column_ref.column, scope_id)
    }

    /// Get all visible tables at a scope (including parent scopes)
    ///
    /// # Arguments
    ///
    /// * `scope_id` - Scope ID to query
    ///
    /// # Returns
    ///
    /// Vector of visible table symbols
    pub fn visible_tables(&self, scope_id: usize) -> Vec<&TableSymbol> {
        let mut tables = Vec::new();
        let mut current_id = Some(scope_id);

        while let Some(id) = current_id {
            if let Some(scope) = self.scope_manager.get_scope(id) {
                tables.extend(scope.tables.iter().collect::<Vec<_>>());
                current_id = scope.parent_id;
            } else {
                break;
            }
        }

        tables
    }

    /// Get the root scope ID for the most recently analyzed query
    pub fn root_scope_id(&self) -> Option<usize> {
        self.root_scope_id
    }

    /// Get reference to scope manager for external queries
    pub fn scope_manager(&self) -> &ScopeManager {
        &self.scope_manager
    }

    /// Get a column resolver for enhanced resolution with fuzzy matching
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use unified_sql_lsp_semantic::ColumnResolver;
    ///
    /// let resolver = analyzer.column_resolver();
    /// let result = resolver.resolve_column(&column_ref, scope_id);
    /// ```
    pub fn column_resolver(&self) -> ColumnResolver {
        ColumnResolver::new(self.scope_manager.clone())
    }

    /// Resolve a column with enhanced error reporting and suggestions
    ///
    /// This method provides richer results than the basic `resolve_column`,
    /// including fuzzy matching and candidate suggestions for typos.
    ///
    /// # Arguments
    ///
    /// * `column_ref` - Column reference to resolve
    /// * `scope_id` - Scope ID for resolution
    ///
    /// # Returns
    ///
    /// Enhanced resolution result with suggestions if not found
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let result = analyzer.resolve_column_with_suggestions(&column_ref, scope_id);
    /// match result {
    ///     ColumnResolutionResult::Found { table, column } => {
    ///         println!("Found {} in {}", column.name, table.display_name());
    ///     }
    ///     ColumnResolutionResult::NotFoundWithSuggestions { suggestions } => {
    ///         println!("Did you mean: {}", suggestions[0].column.name);
    ///     }
    ///     ColumnResolutionResult::Ambiguous { candidates } => {
    ///         println!("Ambiguous, specify one of:");
    ///         for c in candidates {
    ///             println!("  - {}.{}", c.table.display_name(), c.column.name);
    ///         }
    ///     }
    /// }
    /// ```
    pub fn resolve_column_with_suggestions(
        &self,
        column_ref: &unified_sql_lsp_ir::ColumnRef,
        scope_id: usize,
    ) -> crate::resolution::ColumnResolutionResult {
        let resolver = self.column_resolver();
        resolver.resolve_column(column_ref, scope_id)
    }

    // -------------------------------------------------------------------------
    // Private Helper Methods
    // -------------------------------------------------------------------------

    /// Extract all table names from a query
    fn extract_table_names(&self, query: &Query) -> SemanticResult<Vec<String>> {
        let mut table_names = Vec::new();

        match &query.body {
            SetOp::Select(select) => {
                // Extract from FROM clause
                for table_ref in &select.from {
                    table_names.push(table_ref.name.clone());

                    // Extract from JOINs
                    for join in &table_ref.joins {
                        table_names.push(join.table.name.clone());
                    }
                }
            }
            // TODO: (SEMANTIC-006) Extract tables from set operations
            _ => {}
        }

        Ok(table_names)
    }

    /// Pre-fetch column metadata for all tables in the query
    async fn prefetch_table_metadata(&mut self, table_names: Vec<String>) -> SemanticResult<()> {
        for table_name in table_names {
            // Skip if already cached
            if self.column_cache.contains_key(&table_name) {
                continue;
            }

            // Fetch from catalog
            let columns = self
                .catalog
                .get_columns(&table_name)
                .await
                .map_err(|_| SemanticError::TableNotFound(table_name.clone()))?;

            // Convert to ColumnSymbol
            let symbols: Vec<ColumnSymbol> = columns
                .into_iter()
                .map(|col| ColumnSymbol::new(col.name.clone(), col.data_type, &table_name))
                .collect();

            self.column_cache.insert(table_name, symbols);
        }

        Ok(())
    }

    /// Build the main query scope
    fn build_main_query_scope(&mut self, select: &SelectStatement) -> SemanticResult<usize> {
        let scope_id = self.scope_manager.create_scope(ScopeType::Query, None);

        // Process FROM clause tables
        for table_ref in &select.from {
            self.process_table_ref(table_ref, scope_id)?;
        }

        // Process JOINs (they add more tables to the same scope)
        for table_ref in &select.from {
            for join in &table_ref.joins {
                self.process_table_ref(&join.table, scope_id)?;
            }
        }

        // Validate column references in projection
        self.validate_projection(&select.projection, scope_id)?;

        // Validate WHERE clause
        if let Some(where_clause) = &select.where_clause {
            self.validate_expr(where_clause, scope_id)?;
        }

        // TODO: (SEMANTIC-005) Validate GROUP BY, HAVING, ORDER BY clauses

        Ok(scope_id)
    }

    /// Process a table reference and add it to scope
    fn process_table_ref(&mut self, table_ref: &TableRef, scope_id: usize) -> SemanticResult<()> {
        // Get column metadata from cache
        let columns = self
            .column_cache
            .get(&table_ref.name)
            .ok_or_else(|| SemanticError::TableNotFound(table_ref.name.clone()))?;

        // Create TableSymbol with alias
        let mut table = TableSymbol::new(&table_ref.name).with_columns(columns.clone());

        if let Some(alias) = &table_ref.alias {
            table = table.with_alias(alias);
        }

        // Add to scope
        let scope = self
            .scope_manager
            .get_scope_mut(scope_id)
            .ok_or_else(|| SemanticError::InvalidScope(format!("scope {}", scope_id)))?;

        scope.add_table(table)?;

        Ok(())
    }

    /// Validate expression (recursive)
    fn validate_expr(&self, expr: &Expr, scope_id: usize) -> SemanticResult<()> {
        match expr {
            Expr::Column(col_ref) => {
                self.resolve_column(col_ref, scope_id)?;
                Ok(())
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_expr(left, scope_id)?;
                self.validate_expr(right, scope_id)?;
                Ok(())
            }
            Expr::UnaryOp { expr, .. } => {
                self.validate_expr(expr, scope_id)?;
                Ok(())
            }
            Expr::Function { name: _, args, .. } => {
                // TODO: (SEMANTIC-004) Validate function calls against catalog
                for arg in args {
                    self.validate_expr(arg, scope_id)?;
                }
                Ok(())
            }
            Expr::Case {
                conditions,
                results,
                else_result,
            } => {
                for condition in conditions {
                    self.validate_expr(condition, scope_id)?;
                }
                for result in results {
                    self.validate_expr(result, scope_id)?;
                }
                if let Some(else_res) = else_result {
                    self.validate_expr(else_res, scope_id)?;
                }
                Ok(())
            }
            Expr::Cast { expr, .. } => {
                self.validate_expr(expr, scope_id)?;
                Ok(())
            }
            Expr::Paren(inner) => {
                self.validate_expr(inner, scope_id)?;
                Ok(())
            }
            Expr::List(items) => {
                for item in items {
                    self.validate_expr(item, scope_id)?;
                }
                Ok(())
            }
            // Literals are always valid
            Expr::Literal(_) => Ok(()),
            // Catch-all for future Expr variants (non-exhaustive enum)
            _ => Ok(()),
        }
    }

    /// Validate projection (SELECT clause)
    fn validate_projection(
        &self,
        projection: &[SelectItem],
        scope_id: usize,
    ) -> SemanticResult<()> {
        for item in projection {
            match item {
                SelectItem::UnnamedExpr(expr) => {
                    self.validate_expr(expr, scope_id)?;
                }
                SelectItem::AliasedExpr { expr, .. } => {
                    self.validate_expr(expr, scope_id)?;
                }
                SelectItem::QualifiedWildcard(table) => {
                    // TODO: (SEMANTIC-005) Validate table exists and expand wildcard
                    // For now, just check that the table exists
                    self.scope_manager.resolve_table(table, scope_id)?;
                }
                SelectItem::Wildcard => {
                    // Wildcard (*) is always valid
                    // TODO: (SEMANTIC-005) Expand to all columns
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use unified_sql_lsp_catalog::{
        CatalogError, ColumnMetadata, DataType, FunctionMetadata, TableMetadata,
    };

    // Mock catalog for testing
    struct MockCatalog {
        tables: HashMap<String, Vec<ColumnMetadata>>,
    }

    impl MockCatalog {
        fn new() -> Self {
            let mut tables = HashMap::new();

            // Add "users" table
            tables.insert(
                "users".to_string(),
                vec![
                    ColumnMetadata::new("id", DataType::Integer),
                    ColumnMetadata::new("name", DataType::Text),
                    ColumnMetadata::new("email", DataType::Text),
                ],
            );

            // Add "orders" table
            tables.insert(
                "orders".to_string(),
                vec![
                    ColumnMetadata::new("id", DataType::Integer),
                    ColumnMetadata::new("user_id", DataType::Integer),
                    ColumnMetadata::new("total", DataType::Integer),
                ],
            );

            Self { tables }
        }
    }

    #[async_trait::async_trait]
    impl Catalog for MockCatalog {
        async fn list_tables(&self) -> unified_sql_lsp_catalog::CatalogResult<Vec<TableMetadata>> {
            Ok(self
                .tables
                .keys()
                .map(|name| TableMetadata::new(name, "public"))
                .collect())
        }

        async fn get_columns(
            &self,
            table: &str,
        ) -> unified_sql_lsp_catalog::CatalogResult<Vec<ColumnMetadata>> {
            self.tables
                .get(table)
                .cloned()
                .ok_or_else(|| CatalogError::TableNotFound(table.to_string(), "public".to_string()))
        }

        async fn list_functions(
            &self,
        ) -> unified_sql_lsp_catalog::CatalogResult<Vec<FunctionMetadata>> {
            Ok(vec![])
        }
    }

    fn build_test_query() -> Query {
        Query::new(Dialect::MySQL)
    }

    #[tokio::test]
    async fn test_analyzer_creation() {
        let catalog = Arc::new(MockCatalog::new());
        let analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        assert!(analyzer.root_scope_id().is_none());
        assert_eq!(analyzer.scope_manager().scope_count(), 0);
    }
}
