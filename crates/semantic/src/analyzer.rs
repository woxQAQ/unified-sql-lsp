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

use unified_sql_lsp_catalog::{Catalog, FunctionMetadata, FunctionType};
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_ir::{
    BinaryOp, ColumnRef, Expr, Literal, OrderBy, Query, SelectItem, SelectStatement, SetOp,
    TableRef,
};

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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use unified_sql_lsp_catalog::{
        CatalogError, ColumnMetadata, DataType, FunctionMetadata, TableMetadata,
    };
    use unified_sql_lsp_ir::{Join, JoinCondition, SortDirection};

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
}
