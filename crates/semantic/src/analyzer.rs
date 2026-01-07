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

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use unified_sql_lsp_catalog::{Catalog, FunctionMetadata, FunctionType};
use unified_sql_lsp_ir::{CommonTableExpr, Dialect};
use unified_sql_lsp_ir::{
    BinaryOp, ColumnRef, Expr, Literal, OrderBy, Query, SelectItem, SelectStatement, SetOp,
    TableRef,
};

use crate::error::{SemanticError, SemanticResult};
use crate::resolution::ColumnResolver;
use crate::scope::{ScopeManager, ScopeType};
use crate::symbol::{ColumnSymbol, TableSymbol};

/// Metadata for processed CTEs
struct CteMetadata {
    scope_id: usize,
    output_columns: Vec<ColumnSymbol>,
    is_recursive: bool,
}

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

    /// Cache of function metadata from catalog
    function_cache: Vec<FunctionMetadata>,

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
            function_cache: Vec::new(),
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
        self.function_cache.clear();
        self.root_scope_id = None;

        // Step 1: Extract all table names from query
        let table_names = self.extract_table_names(query)?;

        // Step 2: Pre-fetch column metadata from catalog
        self.prefetch_table_metadata(table_names).await?;

        // Step 2.5: Pre-fetch function metadata from catalog
        self.prefetch_function_metadata().await?;

        // Step 3: Process CTEs if present and get CTE scope ID
        let cte_scope_id = if !query.ctes.is_empty() {
            let (scope_id, _metadata) = self.process_ctes(&query.ctes, None).await?;
            Some(scope_id)
        } else {
            None
        };

        // Step 4: Build main query scope (handle set operations)
        // Main query scope should have CTE scope as parent if CTEs exist
        let root_scope = match &query.body {
            SetOp::Select(select) => {
                // Build scope with CTE scope as parent
                let scope_id = self.scope_manager.create_scope(ScopeType::Query, cte_scope_id);
                self.populate_query_scope(select, scope_id)?;
                scope_id
            }
            _ => {
                // Handle UNION, INTERSECT, EXCEPT
                let (scope_id, _columns) = self.analyze_set_operation_recursive(&query.body, cte_scope_id)?;
                scope_id
            }
        };

        // Step 5: Validate ORDER BY clause (if present)
        // ORDER BY is at the Query level, so we validate it after building the scope
        if let Some(order_by) = &query.order_by {
            self.validate_order_by(order_by, root_scope)?;
        }

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

        // Collect CTE names to filter them out later
        let cte_names: std::collections::HashSet<String> = query.ctes.iter()
            .map(|cte| cte.name.clone())
            .collect();

        match &query.body {
            SetOp::Select(select) => {
                // Extract from FROM clause
                for table_ref in &select.from {
                    // Skip CTE references
                    if !cte_names.contains(&table_ref.name) {
                        table_names.push(table_ref.name.clone());
                    }

                    // Extract from JOINs
                    for join in &table_ref.joins {
                        // Skip CTE references
                        if !cte_names.contains(&join.table.name) {
                            table_names.push(join.table.name.clone());
                        }
                    }
                }
            }
            SetOp::Union { left, right, .. }
            | SetOp::Intersect { left, right, .. }
            | SetOp::Except { left, right, .. } => {
                // Recursively extract tables from both sides
                let mut left_tables = self.extract_table_names(left)?;
                let mut right_tables = self.extract_table_names(right)?;
                table_names.append(&mut left_tables);
                table_names.append(&mut right_tables);
            }
        }

        // Also extract from CTEs (but not the CTEs themselves)
        for cte in &query.ctes {
            let mut cte_tables = self.extract_table_names(&cte.query)?;
            table_names.append(&mut cte_tables);
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

    /// Pre-fetch function metadata from catalog
    ///
    /// This loads all available functions from the catalog and caches them
    /// for use in aggregate function detection.
    async fn prefetch_function_metadata(&mut self) -> SemanticResult<()> {
        let functions = self.catalog.list_functions().await.map_err(|e| {
            SemanticError::InvalidScope(format!("Failed to fetch functions: {:?}", e))
        })?;

        self.function_cache = functions;
        Ok(())
    }

    /// Build the main query scope
    fn build_main_query_scope(&mut self, select: &SelectStatement) -> SemanticResult<usize> {
        let scope_id = self.scope_manager.create_scope(ScopeType::Query, None);
        self.populate_query_scope(select, scope_id)?;
        Ok(scope_id)
    }

    /// Populate a query scope with tables and validate clauses
    fn populate_query_scope(&mut self, select: &SelectStatement, scope_id: usize) -> SemanticResult<()> {
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

        // Validate WHERE clause
        if let Some(where_clause) = &select.where_clause {
            self.validate_expr(where_clause, scope_id)?;
        }

        // Validate GROUP BY clause
        self.validate_group_by(&select.group_by, scope_id)?;

        // Validate HAVING clause
        if let Some(having) = &select.having {
            self.validate_having(having, scope_id, &select.group_by)?;
        }

        // Validate projection (includes wildcard validation)
        self.validate_projection(&select.projection, scope_id)?;

        Ok(())
    }

    /// Process a table reference and add it to scope
    fn process_table_ref(&mut self, table_ref: &TableRef, scope_id: usize) -> SemanticResult<()> {
        // First check if this is a CTE (synthetic table in scope hierarchy)
        match self.scope_manager.resolve_table(&table_ref.name, scope_id) {
            Ok(cte_table) => {
                // This is a CTE, already in scope - just add it to current scope with alias if needed
                let mut table = cte_table.clone();
                if let Some(alias) = &table_ref.alias {
                    table = table.with_alias(alias);
                }

                let scope = self
                    .scope_manager
                    .get_scope_mut(scope_id)
                    .ok_or_else(|| SemanticError::InvalidScope(format!("scope {}", scope_id)))?;

                scope.add_table(table)?;
                return Ok(());
            }
            Err(_) => {
                // Not a CTE, continue to catalog lookup
            }
        }

        // Not a CTE, get column metadata from cache
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
                    // Validate that the table exists in the scope
                    self.scope_manager
                        .resolve_table(table, scope_id)
                        .map_err(|_| SemanticError::WildcardTableNotFound(table.clone()))?;
                    // Wildcard expansion is available via expand_projection_wildcards()
                }
                SelectItem::Wildcard => {
                    // Unqualified wildcard (*) is always valid
                    // Wildcard expansion is available via expand_projection_wildcards()
                }
            }
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // SEMANTIC-005: Advanced Query Clause Validation
    // -------------------------------------------------------------------------

    /// Check if a function name is an aggregate function
    ///
    /// Uses catalog function metadata to determine if a function is aggregate.
    fn is_aggregate_function(&self, name: &str) -> bool {
        self.function_cache.iter().any(|f| {
            f.name.to_uppercase() == name.to_uppercase()
                && f.function_type == FunctionType::Aggregate
        })
    }

    /// Check if an expression contains an aggregate function
    ///
    /// This recursively checks the expression tree for aggregate functions.
    fn expr_contains_aggregate(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Function { name, .. } => self.is_aggregate_function(name),
            Expr::BinaryOp { left, right, .. } => {
                self.expr_contains_aggregate(left) || self.expr_contains_aggregate(right)
            }
            Expr::UnaryOp { expr, .. } => self.expr_contains_aggregate(expr),
            Expr::Case {
                conditions,
                results,
                else_result,
            } => {
                conditions.iter().any(|c| self.expr_contains_aggregate(c))
                    || results.iter().any(|r| self.expr_contains_aggregate(r))
                    || else_result
                        .as_ref()
                        .map_or(false, |e| self.expr_contains_aggregate(e))
            }
            Expr::Cast { expr, .. } => self.expr_contains_aggregate(expr),
            Expr::Paren(inner) => self.expr_contains_aggregate(inner),
            _ => false,
        }
    }

    /// Validate GROUP BY clause
    ///
    /// Validates that all expressions in GROUP BY reference valid columns.
    fn validate_group_by(&self, group_by: &[Expr], scope_id: usize) -> SemanticResult<()> {
        for expr in group_by {
            self.validate_expr(expr, scope_id)?;
        }
        Ok(())
    }

    /// Validate HAVING clause
    ///
    /// HAVING clause validation rules:
    /// - With GROUP BY: can reference columns in GROUP BY and aggregate functions
    /// - Without GROUP BY: can only contain aggregate functions and literals
    fn validate_having(
        &self,
        having: &Expr,
        scope_id: usize,
        group_by: &[Expr],
    ) -> SemanticResult<()> {
        if group_by.is_empty() {
            self.validate_having_without_group(having, scope_id)?;
        } else {
            self.validate_expr(having, scope_id)?;
        }
        Ok(())
    }

    /// Validate HAVING clause when there's no GROUP BY
    ///
    /// When there's no GROUP BY, HAVING can only contain:
    /// - Aggregate functions
    /// - Literals
    /// - Expressions composed of the above
    ///
    /// Column references are NOT allowed without GROUP BY.
    fn validate_having_without_group(&self, expr: &Expr, scope_id: usize) -> SemanticResult<()> {
        match expr {
            Expr::Column(col_ref) => {
                // Column references in HAVING without GROUP BY are invalid
                // unless they're part of an aggregate function (handled below)
                Err(SemanticError::NonAggregateColumnInHaving(
                    col_ref.qualified(),
                ))
            }
            Expr::BinaryOp { left, right, .. } => {
                self.validate_having_without_group(left, scope_id)?;
                self.validate_having_without_group(right, scope_id)?;
                Ok(())
            }
            Expr::UnaryOp { expr, .. } => self.validate_having_without_group(expr, scope_id),
            Expr::Function { .. } => {
                // Validate function arguments (recursively check for columns)
                self.validate_expr(expr, scope_id)?;
                Ok(())
            }
            Expr::Case {
                conditions,
                results,
                else_result,
            } => {
                for condition in conditions {
                    self.validate_having_without_group(condition, scope_id)?;
                }
                for result in results {
                    self.validate_having_without_group(result, scope_id)?;
                }
                if let Some(else_res) = else_result {
                    self.validate_having_without_group(else_res, scope_id)?;
                }
                Ok(())
            }
            Expr::Cast { expr, .. } => self.validate_having_without_group(expr, scope_id),
            Expr::Paren(inner) => self.validate_having_without_group(inner, scope_id),
            Expr::List(items) => {
                for item in items {
                    self.validate_having_without_group(item, scope_id)?;
                }
                Ok(())
            }
            // Literals are always valid
            Expr::Literal(_) => Ok(()),
            // Catch-all for future Expr variants (non-exhaustive enum)
            _ => Ok(()),
        }
    }

    /// Validate ORDER BY clause
    ///
    /// Validates that all expressions in ORDER BY reference valid columns.
    ///
    /// # Note
    ///
    /// ORDER BY is at the Query level (not SelectStatement level),
    /// so this is called from `analyze_query`, not `build_main_query_scope`.
    fn validate_order_by(&self, order_by: &[OrderBy], scope_id: usize) -> SemanticResult<()> {
        for order_item in order_by {
            self.validate_expr(&order_item.expr, scope_id)?;
        }
        Ok(())
    }

    /// Expand a wildcard SelectItem to actual column references
    ///
    /// This is useful for completion and other features that need to know
    /// what columns a wildcard represents.
    ///
    /// # Arguments
    ///
    /// * `item` - The SelectItem to expand (can be Wildcard, QualifiedWildcard, or other)
    /// * `scope_id` - The scope ID to resolve tables from
    ///
    /// # Returns
    ///
    /// A vector of SelectItem with wildcards expanded to individual column references.
    fn expand_wildcard(
        &self,
        item: &SelectItem,
        scope_id: usize,
    ) -> SemanticResult<Vec<SelectItem>> {
        match item {
            SelectItem::QualifiedWildcard(table_name) => {
                let table = self.scope_manager.resolve_table(table_name, scope_id)?;
                let mut columns = Vec::new();
                for col in &table.columns {
                    let col_ref =
                        ColumnRef::new(&col.name).with_table(table.display_name().to_string());
                    columns.push(SelectItem::UnnamedExpr(Expr::Column(col_ref)));
                }
                Ok(columns)
            }
            SelectItem::Wildcard => {
                let tables = self.visible_tables(scope_id);
                let mut columns = Vec::new();
                for table in tables {
                    for col in &table.columns {
                        let col_ref =
                            ColumnRef::new(&col.name).with_table(table.display_name().to_string());
                        columns.push(SelectItem::UnnamedExpr(Expr::Column(col_ref)));
                    }
                }
                Ok(columns)
            }
            _ => Ok(vec![item.clone()]),
        }
    }

    /// Expand all wildcards in a projection to concrete columns
    ///
    /// This method takes a projection list and expands any wildcards to their
    /// actual column references. Non-wildcard items are preserved as-is.
    ///
    /// # Arguments
    ///
    /// * `projection` - The projection list from SELECT clause
    /// * `scope_id` - The scope ID to resolve tables from
    ///
    /// # Returns
    ///
    /// A new projection list with all wildcards expanded to actual columns.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // Given: SELECT * FROM users
    /// // Returns: SELECT users.id, users.name, users.email FROM users
    /// let expanded = analyzer.expand_projection_wildcards(&projection, scope_id)?;
    /// ```
    pub fn expand_projection_wildcards(
        &self,
        projection: &[SelectItem],
        scope_id: usize,
    ) -> SemanticResult<Vec<SelectItem>> {
        let mut result = Vec::new();
        for item in projection {
            match item {
                SelectItem::Wildcard | SelectItem::QualifiedWildcard(_) => {
                    let expanded = self.expand_wildcard(item, scope_id)?;
                    result.extend(expanded);
                }
                _ => result.push(item.clone()),
            }
        }
        Ok(result)
    }

    /// Check if a CTE is recursive
    ///
    /// A CTE is recursive if:
    /// 1. It contains UNION ALL
    /// 2. It references itself in the FROM clause
    ///
    /// # Arguments
    ///
    /// * `cte` - CTE to check
    ///
    /// # Returns
    ///
    /// true if the CTE is recursive, false otherwise
    fn is_recursive_cte(&self, cte: &CommonTableExpr) -> bool {
        // Check if CTE query has UNION ALL structure
        let has_union_all = matches!(&cte.query.body, SetOp::Union { all: true, .. });

        if !has_union_all {
            return false;
        }

        // Check if CTE references itself
        self.query_references_cte(&cte.query, &cte.name)
    }

    /// Process CTEs and create synthetic tables
    ///
    /// This method handles all CTEs in a query, creating a separate CTE scope
    /// and synthetic tables that can be referenced by the main query.
    ///
    /// # Arguments
    ///
    /// * `ctes` - CTEs to process
    /// * `parent_scope_id` - Parent scope ID (typically None for top-level CTEs)
    ///
    /// # Returns
    ///
    /// Tuple of (CTE scope ID, HashMap mapping CTE names to their metadata)
    async fn process_ctes(
        &mut self,
        ctes: &[CommonTableExpr],
        parent_scope_id: Option<usize>,
    ) -> SemanticResult<(usize, HashMap<String, CteMetadata>)> {
        let mut cte_map = HashMap::new();

        // Create CTE scope (will be parent of main query scope)
        let cte_scope_id = self.scope_manager.create_scope(ScopeType::CTE, parent_scope_id);

        // Process CTEs in declaration order
        for cte in ctes {
            // Check for circular dependencies
            self.detect_cte_cycles(ctes, &cte.name, &mut HashSet::new(), &mut HashSet::new())?;

            // Build CTE query scope directly (not using analyze_query to avoid recursion)
            let (cte_root_scope, output_columns) = match &cte.query.body {
                SetOp::Select(select) => {
                    let scope_id = self.build_main_query_scope(select)?;
                    let columns = self.infer_output_columns_from_select(select, scope_id)?;
                    (scope_id, columns)
                }
                _ => {
                    // Full support for set operations in CTEs
                    self.analyze_set_operation_recursive(&cte.query.body, Some(cte_scope_id))?
                }
            };

            // Validate column count if explicit column list provided
            if !cte.columns.is_empty() && cte.columns.len() != output_columns.len() {
                return Err(SemanticError::CteColumnCountMismatch {
                    cte: cte.name.clone(),
                    defined: cte.columns.len(),
                    returned: output_columns.len(),
                });
            }

            // Create synthetic table symbol
            let mut cte_table = TableSymbol::new(&cte.name).with_columns(output_columns.clone());

            // Use explicit column names if provided
            if !cte.columns.is_empty() {
                let renamed_columns: Vec<ColumnSymbol> = output_columns
                    .iter()
                    .enumerate()
                    .map(|(i, col)| {
                        let mut renamed = col.clone();
                        renamed.name = cte.columns[i].clone();
                        renamed
                    })
                    .collect();
                cte_table = TableSymbol::new(&cte.name).with_columns(renamed_columns.clone());
            }

            // Add CTE table to CTE scope
            let scope = self
                .scope_manager
                .get_scope_mut(cte_scope_id)
                .ok_or_else(|| SemanticError::InvalidScope("CTE scope not found".to_string()))?;
            scope.add_table(cte_table)?;

            // Store metadata
            cte_map.insert(
                cte.name.clone(),
                CteMetadata {
                    scope_id: cte_root_scope,
                    output_columns,
                    is_recursive: self.is_recursive_cte(cte),
                },
            );
        }

        Ok((cte_scope_id, cte_map))
    }

    /// Infer output columns from a CTE query
    ///
    /// Extracts column names and types from the CTE's SELECT projection.
    ///
    /// # Arguments
    ///
    /// * `cte` - CTE to analyze
    /// * `cte_scope_id` - Scope ID of the CTE query
    ///
    /// # Returns
    ///
    /// Vector of column symbols representing the CTE's output schema
    fn infer_cte_output_columns(
        &self,
        cte: &CommonTableExpr,
        cte_scope_id: usize,
    ) -> SemanticResult<Vec<ColumnSymbol>> {
        let mut columns = Vec::new();

        // Extract from CTE query's projection
        if let SetOp::Select(select) = &cte.query.body {
            for (i, item) in select.projection.iter().enumerate() {
                match item {
                    SelectItem::AliasedExpr { expr, alias } => {
                        // Use alias directly
                        let col_name = alias.clone();

                        // Infer data type from expression
                        let data_type = self.infer_expr_type(expr, cte_scope_id)?;

                        columns.push(ColumnSymbol::new(
                            col_name,
                            data_type,
                            &cte.name, // Table name is the CTE name
                        ));
                    }
                    SelectItem::UnnamedExpr(expr) => {
                        // No alias, generate column name
                        let col_name = self.extract_column_name(expr).unwrap_or(format!("col_{}", i + 1));

                        // Infer data type from expression
                        let data_type = self.infer_expr_type(expr, cte_scope_id)?;

                        columns.push(ColumnSymbol::new(
                            col_name,
                            data_type,
                            &cte.name,
                        ));
                    }
                    SelectItem::Wildcard | SelectItem::QualifiedWildcard(_) => {
                        // Expand wildcards
                        let expanded = self.expand_wildcard(item, cte_scope_id)?;
                        for expanded_item in expanded {
                            match expanded_item {
                                SelectItem::AliasedExpr { expr, alias } => {
                                    let col_name = alias.clone();
                                    let data_type = self.infer_expr_type(&expr, cte_scope_id)?;
                                    columns.push(ColumnSymbol::new(col_name, data_type, &cte.name));
                                }
                                SelectItem::UnnamedExpr(expr) => {
                                    let col_name = self.extract_column_name(&expr).unwrap_or("col".to_string());
                                    let data_type = self.infer_expr_type(&expr, cte_scope_id)?;
                                    columns.push(ColumnSymbol::new(col_name, data_type, &cte.name));
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(columns)
    }

    /// Detect circular dependencies in CTEs
    ///
    /// Uses DFS to detect cycles in the CTE reference graph.
    ///
    /// # Arguments
    ///
    /// * `ctes` - All CTEs in the query
    /// * `current_cte` - Current CTE being analyzed
    /// * `visited` - Set of visited CTE names
    /// * `rec_stack` - Current recursion stack
    fn detect_cte_cycles(
        &self,
        ctes: &[CommonTableExpr],
        current_cte: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
    ) -> SemanticResult<()> {
        visited.insert(current_cte.to_string());
        rec_stack.insert(current_cte.to_string());

        // Find CTE references in current CTE's query
        let referenced_ctes: Vec<String> = ctes
            .iter()
            .filter(|cte| cte.name != current_cte)
            .filter(|cte| self.query_references_cte(&ctes.iter().find(|c| c.name == current_cte).unwrap().query, &cte.name))
            .map(|cte| cte.name.clone())
            .collect();

        for ref_cte in referenced_ctes {
            if !visited.contains(&ref_cte) {
                self.detect_cte_cycles(ctes, &ref_cte, visited, rec_stack)?;
            } else if rec_stack.contains(&ref_cte) {
                // Cycle detected
                let cycle_path: Vec<String> = rec_stack.iter().cloned().collect();
                let cycle_str = cycle_path.join(" → ");
                return Err(SemanticError::CircularCteDependency(format!("{} → {}", cycle_str, ref_cte)));
            }
        }

        rec_stack.remove(current_cte);
        Ok(())
    }

    /// Check if a query references a specific CTE by name
    fn query_references_cte(&self, query: &Query, cte_name: &str) -> bool {
        match &query.body {
            SetOp::Select(select) => {
                // Check FROM clause
                for table_ref in &select.from {
                    if table_ref.name == cte_name {
                        return true;
                    }
                    // Check JOINs
                    for join in &table_ref.joins {
                        if join.table.name == cte_name {
                            return true;
                        }
                    }
                }
                false
            }
            SetOp::Union { left, right, .. }
            | SetOp::Intersect { left, right, .. }
            | SetOp::Except { left, right, .. } => {
                // Recursively check both sides
                self.query_references_cte(left, cte_name) || self.query_references_cte(right, cte_name)
            }
        }
    }

    /// Recursively analyze set operations (UNION, INTERSECT, EXCEPT)
    ///
    /// This method handles nested set operations, ensuring proper scope isolation
    /// and column count compatibility.
    ///
    /// # Arguments
    ///
    /// * `set_op` - Set operation to analyze
    /// * `parent_id` - Parent scope ID
    ///
    /// # Returns
    ///
    /// Tuple of (parent_scope_id, output_columns)
    fn analyze_set_operation_recursive(
        &mut self,
        set_op: &SetOp,
        parent_id: Option<usize>,
    ) -> SemanticResult<(usize, Vec<ColumnSymbol>)> {
        match set_op {
            SetOp::Select(select) => {
                // Base case: regular SELECT
                let scope_id = self.build_main_query_scope(select)?;
                let columns = self.infer_output_columns_from_select(select, scope_id)?;
                Ok((scope_id, columns))
            }
            SetOp::Union { left, right, .. } => {
                // Recursive case: analyze both sides
                let (left_id, left_cols) = self.analyze_set_operation_recursive(&left.body, parent_id)?;
                let (right_id, right_cols) = self.analyze_set_operation_recursive(&right.body, parent_id)?;

                // Validate column count matches
                if left_cols.len() != right_cols.len() {
                    return Err(SemanticError::SetOperationColumnCountMismatch {
                        left: left_cols.len(),
                        right: right_cols.len(),
                    });
                }

                // Create isolated parent scope
                let parent_scope = self.scope_manager.create_scope(ScopeType::Query, parent_id);
                Ok((parent_scope, left_cols)) // Left columns define output
            }
            SetOp::Intersect { left, right, .. } => {
                // Same logic as UNION
                let (left_id, left_cols) = self.analyze_set_operation_recursive(&left.body, parent_id)?;
                let (right_id, right_cols) = self.analyze_set_operation_recursive(&right.body, parent_id)?;

                if left_cols.len() != right_cols.len() {
                    return Err(SemanticError::SetOperationColumnCountMismatch {
                        left: left_cols.len(),
                        right: right_cols.len(),
                    });
                }

                let parent_scope = self.scope_manager.create_scope(ScopeType::Query, parent_id);
                Ok((parent_scope, left_cols))
            }
            SetOp::Except { left, right, .. } => {
                // Same logic as UNION and INTERSECT
                let (left_id, left_cols) = self.analyze_set_operation_recursive(&left.body, parent_id)?;
                let (right_id, right_cols) = self.analyze_set_operation_recursive(&right.body, parent_id)?;

                if left_cols.len() != right_cols.len() {
                    return Err(SemanticError::SetOperationColumnCountMismatch {
                        left: left_cols.len(),
                        right: right_cols.len(),
                    });
                }

                let parent_scope = self.scope_manager.create_scope(ScopeType::Query, parent_id);
                Ok((parent_scope, left_cols))
            }
        }
    }

    /// Infer output columns from a SELECT statement
    ///
    /// Similar to `infer_cte_output_columns` but used for set operations.
    fn infer_output_columns_from_select(
        &self,
        select: &SelectStatement,
        scope_id: usize,
    ) -> SemanticResult<Vec<ColumnSymbol>> {
        let mut columns = Vec::new();

        for (i, item) in select.projection.iter().enumerate() {
            match item {
                SelectItem::AliasedExpr { expr, alias } => {
                    let col_name = alias.clone();
                    let data_type = self.infer_expr_type(expr, scope_id)?;
                    // For set operations, we don't have a specific table name
                    columns.push(ColumnSymbol::new(col_name, data_type, ""));
                }
                SelectItem::UnnamedExpr(expr) => {
                    let col_name = self.extract_column_name(expr).unwrap_or(format!("col_{}", i + 1));
                    let data_type = self.infer_expr_type(expr, scope_id)?;
                    columns.push(ColumnSymbol::new(col_name, data_type, ""));
                }
                SelectItem::Wildcard | SelectItem::QualifiedWildcard(_) => {
                    // Expand wildcards - for set operations, use empty table name
                    let expanded = self.expand_wildcard(item, scope_id)?;
                    for expanded_item in expanded {
                        match expanded_item {
                            SelectItem::AliasedExpr { expr, alias } => {
                                let col_name = alias.clone();
                                let data_type = self.infer_expr_type(&expr, scope_id)?;
                                columns.push(ColumnSymbol::new(col_name, data_type, ""));
                            }
                            SelectItem::UnnamedExpr(expr) => {
                                let col_name = self.extract_column_name(&expr).unwrap_or("col".to_string());
                                let data_type = self.infer_expr_type(&expr, scope_id)?;
                                columns.push(ColumnSymbol::new(col_name, data_type, ""));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(columns)
    }

    /// Extract column name from an expression
    ///
    /// Returns None if the expression is not a simple column reference.
    fn extract_column_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Column(col_ref) => Some(col_ref.column.clone()),
            Expr::Literal(Literal::Integer(_)) => None,
            Expr::Literal(Literal::String(s)) => Some(s.clone()),
            _ => None, // Complex expressions don't have simple names
        }
    }

    /// Infer data type from an expression
    ///
    /// Traverses the expression tree and infers the type based on:
    /// - Literals have fixed types
    /// - Column references get type from symbol table
    /// - Binary operations follow type promotion rules
    /// - Functions return type from metadata
    /// - CAST expressions return the target type
    /// - CASE expressions return the most common type of all branches
    fn infer_expr_type(&self, expr: &Expr, scope_id: usize) -> SemanticResult<unified_sql_lsp_catalog::DataType> {
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_ir::Literal;

        match expr {
            // Literal types
            Expr::Literal(Literal::Integer(_)) => Ok(DataType::Integer),
            Expr::Literal(Literal::Float(_)) => Ok(DataType::Float),
            Expr::Literal(Literal::String(_)) => Ok(DataType::Text),
            Expr::Literal(Literal::Boolean(_)) => Ok(DataType::Boolean),
            Expr::Literal(Literal::Null) => Ok(DataType::Other("NULL".to_string())),

            // Column reference - resolve from symbol table
            Expr::Column(col_ref) => {
                match self.resolve_column(col_ref, scope_id) {
                    Ok((_table, column)) => Ok(column.data_type.clone()),
                    Err(_) => Ok(DataType::Other("UNKNOWN".to_string())),
                }
            }

            // Binary operations
            Expr::BinaryOp { left, op, right } => {
                self.infer_binaryop_type(left, op, right, scope_id)
            }

            // Unary operations
            Expr::UnaryOp { op, expr } => {
                match op {
                    unified_sql_lsp_ir::UnaryOp::Not => Ok(DataType::Boolean),
                    unified_sql_lsp_ir::UnaryOp::Neg => {
                        // Negation preserves the operand type
                        self.infer_expr_type(expr, scope_id)
                    }
                    unified_sql_lsp_ir::UnaryOp::Exists => Ok(DataType::Boolean),
                    _ => Ok(DataType::Other("UNKNOWN".to_string())),
                }
            }

            // Function calls
            Expr::Function { name, args, .. } => {
                self.infer_function_type(name, args, scope_id)
            }

            // CAST expression
            Expr::Cast { type_name, .. } => {
                self.parse_data_type_from_string(type_name)
            }

            // CASE expression - return most common type
            Expr::Case { results, else_result, .. } => {
                let mut types: Vec<DataType> = results
                    .iter()
                    .filter_map(|r| self.infer_expr_type(r, scope_id).ok())
                    .collect();

                if let Some(else_val) = else_result {
                    if let Ok(t) = self.infer_expr_type(else_val, scope_id) {
                        types.push(t);
                    }
                }

                Ok(self.find_most_common_type(&types))
            }

            // Parenthesized expression
            Expr::Paren(inner) => self.infer_expr_type(inner, scope_id),

            // List of expressions (for IN clause)
            Expr::List(items) => {
                // All items should have the same type
                if !items.is_empty() {
                    self.infer_expr_type(&items[0], scope_id)
                } else {
                    Ok(DataType::Other("UNKNOWN".to_string()))
                }
            }

            // Catch-all for future expression variants
            _ => Ok(DataType::Other("UNKNOWN".to_string())),
        }
    }

    /// Infer type for binary operations
    fn infer_binaryop_type(
        &self,
        left: &Expr,
        op: &unified_sql_lsp_ir::BinaryOp,
        right: &Expr,
        scope_id: usize,
    ) -> SemanticResult<unified_sql_lsp_catalog::DataType> {
        use unified_sql_lsp_catalog::DataType;
        use unified_sql_lsp_ir::BinaryOp;

        let left_type = self.infer_expr_type(left, scope_id)?;
        let right_type = self.infer_expr_type(right, scope_id)?;

        match op {
            // Arithmetic operations - use type promotion
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => {
                Ok(Self::promote_types(&left_type, &right_type))
            }

            // Comparison operations - always return Boolean
            BinaryOp::Eq | BinaryOp::NotEq | BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                Ok(DataType::Boolean)
            }

            // Logical operations - always return Boolean
            BinaryOp::And | BinaryOp::Or => Ok(DataType::Boolean),

            // String operations
            BinaryOp::Like | BinaryOp::NotLike | BinaryOp::ILike | BinaryOp::NotILike => {
                Ok(DataType::Boolean)
            }

            // Other operations
            BinaryOp::In | BinaryOp::NotIn => Ok(DataType::Boolean),
            BinaryOp::Is | BinaryOp::IsNot => Ok(DataType::Boolean),

            // Catch-all for future operators
            _ => Ok(DataType::Other("UNKNOWN".to_string())),
        }
    }

    /// Infer type for function calls
    fn infer_function_type(
        &self,
        name: &str,
        args: &[Expr],
        scope_id: usize,
    ) -> SemanticResult<unified_sql_lsp_catalog::DataType> {
        use unified_sql_lsp_catalog::DataType;

        let name_lower = name.to_lowercase();

        // Try to find function metadata in cache
        if let Some(func) = self.function_cache.iter().find(|f| f.name.eq_ignore_ascii_case(name)) {
            return Ok(func.return_type.clone());
        }

        // Built-in aggregate function type rules
        match name_lower.as_str() {
            "count" => Ok(DataType::Integer),
            "sum" => {
                if !args.is_empty() {
                    self.infer_expr_type(&args[0], scope_id)
                } else {
                    Ok(DataType::Float)
                }
            }
            "avg" => Ok(DataType::Float),
            "min" | "max" => {
                if !args.is_empty() {
                    self.infer_expr_type(&args[0], scope_id)
                } else {
                    Ok(DataType::Other("UNKNOWN".to_string()))
                }
            }
            // String functions
            "upper" | "lower" | "trim" | "substring" => Ok(DataType::Text),
            // Date functions
            "current_date" => Ok(DataType::Date),
            "current_timestamp" => Ok(DataType::Timestamp),
            _ => Ok(DataType::Other("UNKNOWN".to_string())),
        }
    }

    /// Parse data type from CAST string
    fn parse_data_type_from_string(&self, type_name: &str) -> SemanticResult<unified_sql_lsp_catalog::DataType> {
        use unified_sql_lsp_catalog::DataType;

        let type_lower = type_name.to_lowercase();

        match type_lower.as_str() {
            "int" | "integer" => Ok(DataType::Integer),
            "bigint" => Ok(DataType::BigInt),
            "smallint" => Ok(DataType::SmallInt),
            "tinyint" => Ok(DataType::TinyInt),
            "float" | "double" => Ok(DataType::Float),
            "decimal" => Ok(DataType::Decimal),
            "varchar" | "text" => Ok(DataType::Text),
            "char" => Ok(DataType::Char(None)),
            "boolean" | "bool" => Ok(DataType::Boolean),
            "date" => Ok(DataType::Date),
            "timestamp" => Ok(DataType::Timestamp),
            "json" => Ok(DataType::Json),
            _ => Ok(DataType::Other(type_name.to_string())),
        }
    }

    /// Find the most common type from a list of types
    fn find_most_common_type(&self, types: &[unified_sql_lsp_catalog::DataType]) -> unified_sql_lsp_catalog::DataType {
        use unified_sql_lsp_catalog::DataType;

        if types.is_empty() {
            return DataType::Other("UNKNOWN".to_string());
        }

        // If all types are the same, return that type
        if types.iter().all(|t| t == &types[0]) {
            return types[0].clone();
        }

        // Type precedence: Float > Integer > Text > Unknown
        if types.contains(&DataType::Float) {
            return DataType::Float;
        }
        if types.contains(&DataType::Integer) {
            return DataType::Integer;
        }
        if types.contains(&DataType::Text) {
            return DataType::Text;
        }

        DataType::Other("UNKNOWN".to_string())
    }

    /// Promote types for binary operations (static helper method)
    fn promote_types(
        left: &unified_sql_lsp_catalog::DataType,
        right: &unified_sql_lsp_catalog::DataType,
    ) -> unified_sql_lsp_catalog::DataType {
        use unified_sql_lsp_catalog::DataType;

        match (left, right) {
            (DataType::Float, _) | (_, DataType::Float) => DataType::Float,
            (DataType::BigInt, _) | (_, DataType::BigInt) => DataType::BigInt,
            (DataType::Integer, DataType::Integer) => DataType::Integer,
            (DataType::Text, _) | (_, DataType::Text) => DataType::Text,
            (DataType::Boolean, DataType::Boolean) => DataType::Boolean,
            _ => DataType::Other("UNKNOWN".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use unified_sql_lsp_catalog::{
        CatalogError, ColumnMetadata, DataType, FunctionMetadata, TableMetadata,
    };
    use unified_sql_lsp_ir::{Join, JoinCondition, JoinType, SortDirection};

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
            // Return mix of aggregate and scalar functions for testing
            Ok(vec![
                // Aggregate functions
                FunctionMetadata::new("count", DataType::Integer)
                    .with_type(FunctionType::Aggregate),
                FunctionMetadata::new("sum", DataType::Integer).with_type(FunctionType::Aggregate),
                FunctionMetadata::new("avg", DataType::Integer).with_type(FunctionType::Aggregate),
                FunctionMetadata::new("min", DataType::Integer).with_type(FunctionType::Aggregate),
                FunctionMetadata::new("max", DataType::Integer).with_type(FunctionType::Aggregate),
                // Scalar functions
                FunctionMetadata::new("abs", DataType::Integer).with_type(FunctionType::Scalar),
                FunctionMetadata::new("upper", DataType::Text).with_type(FunctionType::Scalar),
                FunctionMetadata::new("lower", DataType::Text).with_type(FunctionType::Scalar),
            ])
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

    // -------------------------------------------------------------------------
    // SEMANTIC-005: Advanced Query Clause Validation Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_group_by_valid_column() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a simple query with GROUP BY
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new(
                "name",
            ))));
        select.group_by.push(Expr::Column(ColumnRef::new("name")));

        query.body = SetOp::Select(Box::new(select));

        // Should succeed
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_group_by_invalid_column() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with GROUP BY on non-existent column
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .group_by
            .push(Expr::Column(ColumnRef::new("invalid_column")));

        query.body = SetOp::Select(Box::new(select));

        // Should fail
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_having_with_group_by() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with GROUP BY and HAVING
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new(
                "name",
            ))));
        select.group_by.push(Expr::Column(ColumnRef::new("name")));
        select.having = Some(Expr::BinaryOp {
            left: Box::new(Expr::Function {
                name: "COUNT".to_string(),
                args: vec![Expr::Literal(Literal::Integer(1))],
                distinct: false,
            }),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(Literal::Integer(5))),
        });

        query.body = SetOp::Select(Box::new(select));

        // Should succeed
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_having_without_group_by_aggregate_only() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with HAVING but no GROUP BY (only aggregates)
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select.having = Some(Expr::BinaryOp {
            left: Box::new(Expr::Function {
                name: "COUNT".to_string(),
                args: vec![Expr::Literal(Literal::Integer(1))],
                distinct: false,
            }),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(Literal::Integer(5))),
        });

        query.body = SetOp::Select(Box::new(select));

        // Should succeed - HAVING with aggregates is OK without GROUP BY
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_having_without_group_by_invalid_column() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with HAVING but no GROUP BY, referencing a column
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select.having = Some(Expr::Column(ColumnRef::new("name")));

        query.body = SetOp::Select(Box::new(select));

        // Should fail - column reference in HAVING without GROUP BY
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("HAVING"));
    }

    #[tokio::test]
    async fn test_order_by_valid_column() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with ORDER BY
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new(
                "name",
            ))));

        query.body = SetOp::Select(Box::new(select));
        query.order_by = Some(vec![OrderBy {
            expr: Expr::Column(ColumnRef::new("name")),
            direction: Some(SortDirection::Asc),
        }]);

        // Should succeed
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_order_by_invalid_column() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with ORDER BY on non-existent column
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });

        query.body = SetOp::Select(Box::new(select));
        query.order_by = Some(vec![OrderBy {
            expr: Expr::Column(ColumnRef::new("invalid_column")),
            direction: Some(SortDirection::Desc),
        }]);

        // Should fail
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test]
    async fn test_wildcard_unqualified() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT *
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select.projection.push(SelectItem::Wildcard);

        query.body = SetOp::Select(Box::new(select));

        // Should succeed - wildcard is always valid
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wildcard_qualified_valid_table() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT users.*
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::QualifiedWildcard("users".to_string()));

        query.body = SetOp::Select(Box::new(select));

        // Should succeed
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_wildcard_qualified_invalid_table() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT invalid_table.*
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::QualifiedWildcard("invalid_table".to_string()));

        query.body = SetOp::Select(Box::new(select));

        // Should fail - table not found
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    // -------------------------------------------------------------------------
    // SEMANTIC-005: Wildcard Expansion Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_expand_wildcard_unqualified() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT *
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select.projection.push(SelectItem::Wildcard);

        // Clone projection before move
        let projection = select.projection.clone();

        query.body = SetOp::Select(Box::new(select));

        // Analyze the query
        let scope_id = analyzer.analyze_query(&query).await.unwrap();

        // Expand the wildcard
        let expanded = analyzer
            .expand_projection_wildcards(&projection, scope_id)
            .unwrap();

        // Should expand to 3 columns (id, name, email)
        assert_eq!(expanded.len(), 3);

        // Verify all expanded items are column references
        for item in &expanded {
            match item {
                SelectItem::UnnamedExpr(Expr::Column(col_ref)) => {
                    assert!(col_ref.table.as_ref().is_some());
                    assert!(matches!(col_ref.column.as_str(), "id" | "name" | "email"));
                }
                _ => panic!("Expected column references, got {:?}", item),
            }
        }
    }

    #[tokio::test]
    async fn test_expand_wildcard_qualified() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT users.*
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::QualifiedWildcard("users".to_string()));

        // Clone projection before move
        let projection = select.projection.clone();

        query.body = SetOp::Select(Box::new(select));

        // Analyze the query
        let scope_id = analyzer.analyze_query(&query).await.unwrap();

        // Expand the wildcard
        let expanded = analyzer
            .expand_projection_wildcards(&projection, scope_id)
            .unwrap();

        // Should expand to 3 columns (id, name, email)
        assert_eq!(expanded.len(), 3);

        // Verify all columns are qualified with "users"
        for item in &expanded {
            match item {
                SelectItem::UnnamedExpr(Expr::Column(col_ref)) => {
                    assert_eq!(col_ref.table.as_ref().unwrap(), "users");
                }
                _ => panic!("Expected column references, got {:?}", item),
            }
        }
    }

    #[tokio::test]
    async fn test_expand_wildcard_mixed_projection() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT id, users.*, name
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("id"))));
        select
            .projection
            .push(SelectItem::QualifiedWildcard("users".to_string()));
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new(
                "name",
            ))));

        // Clone projection before move
        let projection = select.projection.clone();

        query.body = SetOp::Select(Box::new(select));

        // Analyze the query
        let scope_id = analyzer.analyze_query(&query).await.unwrap();

        // Expand the wildcards
        let expanded = analyzer
            .expand_projection_wildcards(&projection, scope_id)
            .unwrap();

        // Should have: id + 3 columns from users.* + name = 5 items
        assert_eq!(expanded.len(), 5);

        // First item should be "id" (unqualified)
        match &expanded[0] {
            SelectItem::UnnamedExpr(Expr::Column(col_ref)) => {
                assert!(col_ref.table.is_none());
                assert_eq!(col_ref.column, "id");
            }
            _ => panic!("Expected unqualified id column"),
        }

        // Next 3 items should be qualified columns from users.*
        for i in 1..4 {
            match &expanded[i] {
                SelectItem::UnnamedExpr(Expr::Column(col_ref)) => {
                    assert_eq!(col_ref.table.as_ref().unwrap(), "users");
                }
                _ => panic!("Expected qualified column from users.*"),
            }
        }

        // Last item should be "name" (unqualified)
        match &expanded[4] {
            SelectItem::UnnamedExpr(Expr::Column(col_ref)) => {
                assert!(col_ref.table.is_none());
                assert_eq!(col_ref.column, "name");
            }
            _ => panic!("Expected unqualified name column"),
        }
    }

    #[tokio::test]
    async fn test_expand_wildcard_multiple_tables() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT * FROM users JOIN orders
        let mut query = build_test_query();
        let mut select = SelectStatement::default();

        let mut users_table = TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        };
        users_table.joins.push(Join {
            join_type: unified_sql_lsp_ir::JoinType::Inner,
            table: TableRef {
                name: "orders".to_string(),
                alias: None,
                joins: Vec::new(),
            },
            condition: JoinCondition::On(Expr::Literal(unified_sql_lsp_ir::Literal::Integer(1))),
        });

        select.from.push(users_table);
        select.projection.push(SelectItem::Wildcard);

        // Clone projection before move
        let projection = select.projection.clone();

        query.body = SetOp::Select(Box::new(select));

        // Analyze the query
        let scope_id = analyzer.analyze_query(&query).await.unwrap();

        // Expand the wildcard
        let expanded = analyzer
            .expand_projection_wildcards(&projection, scope_id)
            .unwrap();

        // Should expand to 6 columns (3 from users + 3 from orders)
        assert_eq!(expanded.len(), 6);
    }

    #[tokio::test]
    async fn test_expand_wildcard_no_wildcards() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with SELECT id, name (no wildcards)
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("id"))));
        select
            .projection
            .push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new(
                "name",
            ))));

        // Clone projection before move
        let projection = select.projection.clone();

        query.body = SetOp::Select(Box::new(select));

        // Analyze the query
        let scope_id = analyzer.analyze_query(&query).await.unwrap();

        // Expand (should be no-op)
        let expanded = analyzer
            .expand_projection_wildcards(&projection, scope_id)
            .unwrap();

        // Should remain unchanged
        assert_eq!(expanded.len(), 2);
        assert_eq!(
            expanded[0],
            SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("id")))
        );
        assert_eq!(
            expanded[1],
            SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("name")))
        );
    }

    // -------------------------------------------------------------------------
    // Catalog-based Aggregate Function Detection Tests
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_is_aggregate_function_from_catalog() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Analyze a simple query to populate function cache
        let query = build_test_query();
        analyzer.analyze_query(&query).await.unwrap();

        // Test aggregate functions
        assert!(analyzer.is_aggregate_function("count"));
        assert!(analyzer.is_aggregate_function("COUNT"));
        assert!(analyzer.is_aggregate_function("sum"));
        assert!(analyzer.is_aggregate_function("avg"));
        assert!(analyzer.is_aggregate_function("min"));
        assert!(analyzer.is_aggregate_function("max"));

        // Test scalar functions (should return false)
        assert!(!analyzer.is_aggregate_function("abs"));
        assert!(!analyzer.is_aggregate_function("upper"));
        assert!(!analyzer.is_aggregate_function("lower"));

        // Test unknown function
        assert!(!analyzer.is_aggregate_function("unknown"));
    }

    #[tokio::test]
    async fn test_expr_contains_aggregate_with_catalog() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Analyze a simple query to populate function cache
        let query = build_test_query();
        analyzer.analyze_query(&query).await.unwrap();

        // Test aggregate function detection
        let count_expr = Expr::Function {
            name: "count".to_string(),
            args: vec![Expr::Literal(Literal::Integer(1))],
            distinct: false,
        };
        assert!(analyzer.expr_contains_aggregate(&count_expr));

        // Test scalar function (should not be detected as aggregate)
        let abs_expr = Expr::Function {
            name: "abs".to_string(),
            args: vec![Expr::Literal(Literal::Integer(-5))],
            distinct: false,
        };
        assert!(!analyzer.expr_contains_aggregate(&abs_expr));

        // Test nested expression with aggregate
        let nested_expr = Expr::BinaryOp {
            left: Box::new(Expr::Function {
                name: "sum".to_string(),
                args: vec![Expr::Column(ColumnRef::new("amount"))],
                distinct: false,
            }),
            op: BinaryOp::Add,
            right: Box::new(Expr::Literal(Literal::Integer(10))),
        };
        assert!(analyzer.expr_contains_aggregate(&nested_expr));
    }

    #[tokio::test]
    async fn test_having_validation_with_catalog_aggregates() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Build a query with HAVING and aggregate functions
        let mut query = build_test_query();
        let mut select = SelectStatement::default();
        select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });

        // Add aggregate to HAVING clause
        select.having = Some(Expr::Function {
            name: "count".to_string(),
            args: vec![Expr::Literal(Literal::Integer(1))],
            distinct: false,
        });

        query.body = SetOp::Select(Box::new(select));

        // Should not error - HAVING with aggregate is allowed
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok());
    }

    // =========================================================================
    // CTE Tests (SEMANTIC-006)
    // =========================================================================

    #[tokio::test]
    async fn test_simple_cte() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Create a simple CTE query: WITH user_counts AS (SELECT user_id FROM orders) SELECT * FROM user_counts
        let mut query = Query::new(Dialect::MySQL);

        // Add CTE
        let mut cte_query = Query::new(Dialect::MySQL);
        let mut cte_select = SelectStatement::default();
        cte_select.from.push(TableRef {
            name: "orders".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        cte_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "user_id".to_string(),
        })));

        cte_query.body = SetOp::Select(Box::new(cte_select));

        query.ctes.push(CommonTableExpr {
            name: "user_counts".to_string(),
            columns: Vec::new(),
            query: Box::new(cte_query),
            materialized: None,
        });

        // Main query selects from CTE
        let mut main_select = SelectStatement::default();
        main_select.from.push(TableRef {
            name: "user_counts".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        main_select.projection.push(SelectItem::Wildcard);

        query.body = SetOp::Select(Box::new(main_select));

        // Should analyze successfully
        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok(), "CTE analysis should succeed: {:?}", result.err());
    }

    #[tokio::test]
    async fn test_cte_with_column_list() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // CTE with explicit column list: WITH cte (col1, col2) AS (SELECT id, name FROM users) SELECT * FROM cte
        let mut query = Query::new(Dialect::MySQL);

        let mut cte_query = Query::new(Dialect::MySQL);
        let mut cte_select = SelectStatement::default();
        cte_select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        cte_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));
        cte_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "name".to_string(),
        })));

        cte_query.body = SetOp::Select(Box::new(cte_select));

        query.ctes.push(CommonTableExpr {
            name: "cte".to_string(),
            columns: vec!["col1".to_string(), "col2".to_string()],
            query: Box::new(cte_query),
            materialized: None,
        });

        let mut main_select = SelectStatement::default();
        main_select.from.push(TableRef {
            name: "cte".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        main_select.projection.push(SelectItem::Wildcard);

        query.body = SetOp::Select(Box::new(main_select));

        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok(), "CTE with column list should succeed");
    }

    #[tokio::test]
    async fn test_multiple_ctes() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // Multiple CTEs: WITH cte1 AS (...), cte2 AS (...) SELECT * FROM cte1 JOIN cte2
        let mut query = Query::new(Dialect::MySQL);

        // CTE 1
        let mut cte1_query = Query::new(Dialect::MySQL);
        let mut cte1_select = SelectStatement::default();
        cte1_select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        cte1_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));
        cte1_query.body = SetOp::Select(Box::new(cte1_select));

        query.ctes.push(CommonTableExpr {
            name: "cte1".to_string(),
            columns: Vec::new(),
            query: Box::new(cte1_query),
            materialized: None,
        });

        // CTE 2
        let mut cte2_query = Query::new(Dialect::MySQL);
        let mut cte2_select = SelectStatement::default();
        cte2_select.from.push(TableRef {
            name: "orders".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        cte2_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));
        cte2_query.body = SetOp::Select(Box::new(cte2_select));

        query.ctes.push(CommonTableExpr {
            name: "cte2".to_string(),
            columns: Vec::new(),
            query: Box::new(cte2_query),
            materialized: None,
        });

        // Main query joins both CTEs
        let mut main_select = SelectStatement::default();
        main_select.from.push(TableRef {
            name: "cte1".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        main_select.from[0].joins.push(Join {
            join_type: JoinType::Inner,
            table: TableRef {
                name: "cte2".to_string(),
                alias: None,
                joins: Vec::new(),
            },
            condition: JoinCondition::On(Expr::Literal(Literal::Integer(1))), // Dummy condition
        });
        main_select.projection.push(SelectItem::Wildcard);

        query.body = SetOp::Select(Box::new(main_select));

        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok(), "Multiple CTEs should succeed");
    }

    #[tokio::test]
    async fn test_cte_column_count_mismatch() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // CTE defines 2 columns but query returns 1
        let mut query = Query::new(Dialect::MySQL);

        let mut cte_query = Query::new(Dialect::MySQL);
        let mut cte_select = SelectStatement::default();
        cte_select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        cte_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));

        cte_query.body = SetOp::Select(Box::new(cte_select));

        query.ctes.push(CommonTableExpr {
            name: "cte".to_string(),
            columns: vec!["col1".to_string(), "col2".to_string()], // 2 columns defined
            query: Box::new(cte_query),
            materialized: None,
        });

        let mut main_select = SelectStatement::default();
        main_select.from.push(TableRef {
            name: "cte".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        main_select.projection.push(SelectItem::Wildcard);

        query.body = SetOp::Select(Box::new(main_select));

        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err(), "CTE column count mismatch should error");

        if let Err(SemanticError::CteColumnCountMismatch { defined, returned, .. }) = result {
            assert_eq!(defined, 2);
            assert_eq!(returned, 1);
        } else {
            panic!("Expected CteColumnCountMismatch error");
        }
    }

    // =========================================================================
    // Set Operation Tests (SEMANTIC-006)
    // =========================================================================

    #[tokio::test]
    async fn test_simple_union() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // SELECT id FROM users UNION SELECT id FROM orders
        let mut query = Query::new(Dialect::MySQL);

        let mut left_select = SelectStatement::default();
        left_select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        left_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));

        let mut right_select = SelectStatement::default();
        right_select.from.push(TableRef {
            name: "orders".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        right_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));

        query.body = SetOp::Union {
            left: Box::new(Query {
                ctes: Vec::new(),
                body: SetOp::Select(Box::new(left_select)),
                order_by: None,
                limit: None,
                offset: None,
                dialect: Dialect::MySQL,
            }),
            right: Box::new(Query {
                ctes: Vec::new(),
                body: SetOp::Select(Box::new(right_select)),
                order_by: None,
                limit: None,
                offset: None,
                dialect: Dialect::MySQL,
            }),
            all: false,
        };

        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_ok(), "UNION with matching columns should succeed");
    }

    #[tokio::test]
    async fn test_set_operation_column_mismatch() {
        let catalog = Arc::new(MockCatalog::new());
        let mut analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);

        // SELECT id, name FROM users UNION SELECT id FROM orders (2 vs 1 columns)
        let mut query = Query::new(Dialect::MySQL);

        let mut left_select = SelectStatement::default();
        left_select.from.push(TableRef {
            name: "users".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        left_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));
        left_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "name".to_string(),
        })));

        let mut right_select = SelectStatement::default();
        right_select.from.push(TableRef {
            name: "orders".to_string(),
            alias: None,
            joins: Vec::new(),
        });
        right_select.projection.push(SelectItem::UnnamedExpr(Expr::Column(ColumnRef {
            table: None,
            column: "id".to_string(),
        })));

        query.body = SetOp::Union {
            left: Box::new(Query {
                ctes: Vec::new(),
                body: SetOp::Select(Box::new(left_select)),
                order_by: None,
                limit: None,
                offset: None,
                dialect: Dialect::MySQL,
            }),
            right: Box::new(Query {
                ctes: Vec::new(),
                body: SetOp::Select(Box::new(right_select)),
                order_by: None,
                limit: None,
                offset: None,
                dialect: Dialect::MySQL,
            }),
            all: false,
        };

        let result = analyzer.analyze_query(&query).await;
        assert!(result.is_err(), "UNION with column count mismatch should error");

        if let Err(SemanticError::SetOperationColumnCountMismatch { left, right }) = result {
            assert_eq!(left, 2);
            assert_eq!(right, 1);
        } else {
            panic!("Expected SetOperationColumnCountMismatch error");
        }
    }
}
