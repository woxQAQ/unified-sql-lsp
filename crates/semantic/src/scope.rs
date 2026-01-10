// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Scope management for semantic analysis
//!
//! This module defines scope types and the scope manager for tracking
//! tables and their visibility across nested SQL queries.

use crate::error::{SemanticError, SemanticResult};
use crate::symbol::{ColumnSymbol, TableSymbol};
use serde::{Deserialize, Serialize};

/// Type of scope in a SQL query
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ScopeType {
    /// Top-level query scope
    Query,
    /// Subquery scope
    Subquery,
    /// Common Table Expression (CTE) scope
    CTE,
    /// JOIN clause scope
    Join,
}

/// Represents a lexical scope in a SQL query
///
/// Scopes form a hierarchy where child scopes can access symbols from parent scopes.
/// For example, a subquery can access tables from its parent query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    /// Unique identifier for this scope
    pub id: usize,

    /// Parent scope ID (if any)
    pub parent_id: Option<usize>,

    /// Tables visible in this scope
    pub tables: Vec<TableSymbol>,

    /// Type of this scope
    pub scope_type: ScopeType,
}

impl Scope {
    /// Create a new scope
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for this scope
    /// * `scope_type` - Type of scope (Query, Subquery, CTE, Join)
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::{Scope, ScopeType};
    ///
    /// let scope = Scope::new(0, ScopeType::Query);
    /// assert_eq!(scope.id, 0);
    /// assert!(scope.tables.is_empty());
    /// ```
    pub fn new(id: usize, scope_type: ScopeType) -> Self {
        Self {
            id,
            parent_id: None,
            tables: Vec::new(),
            scope_type,
        }
    }

    /// Set the parent scope
    ///
    /// # Arguments
    ///
    /// * `parent_id` - ID of the parent scope
    pub fn with_parent(mut self, parent_id: usize) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Set the tables for this scope
    ///
    /// # Arguments
    ///
    /// * `tables` - Vector of table symbols
    pub fn with_tables(mut self, tables: Vec<TableSymbol>) -> Self {
        self.tables = tables;
        self
    }

    /// Find a table by name or alias in this scope only
    ///
    /// # Arguments
    ///
    /// * `name` - Table name or alias to find
    ///
    /// # Returns
    ///
    /// `Some(&TableSymbol)` if found, `None` otherwise
    pub fn find_table(&self, name: &str) -> Option<&TableSymbol> {
        self.tables.iter().find(|t| t.matches(name))
    }

    /// Add a table to this scope
    ///
    /// # Arguments
    ///
    /// * `table` - Table symbol to add
    ///
    /// # Returns
    ///
    /// `Ok(())` if successful, `Err(SemanticError)` if duplicate alias
    pub fn add_table(&mut self, table: TableSymbol) -> SemanticResult<()> {
        // Check for duplicate alias
        let display_name = table.display_name();
        if self.tables.iter().any(|t| t.display_name() == display_name) {
            return Err(SemanticError::DuplicateAlias(display_name.to_string()));
        }

        self.tables.push(table);
        Ok(())
    }
}

/// Manages hierarchical scopes and symbol resolution
///
/// The ScopeManager maintains a forest of scopes and provides methods for
/// resolving tables and columns across the scope hierarchy.
#[derive(Debug, Clone)]
pub struct ScopeManager {
    /// All scopes managed by this manager
    scopes: Vec<Scope>,

    /// Next scope ID to assign
    next_id: usize,
}

impl ScopeManager {
    /// Create a new scope manager
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::ScopeManager;
    ///
    /// let manager = ScopeManager::new();
    /// assert_eq!(manager.scope_count(), 0);
    /// ```
    pub fn new() -> Self {
        Self {
            scopes: Vec::new(),
            next_id: 0,
        }
    }

    /// Create a new scope
    ///
    /// # Arguments
    ///
    /// * `scope_type` - Type of scope to create
    /// * `parent_id` - Optional parent scope ID
    ///
    /// # Returns
    ///
    /// The ID of the newly created scope
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::{ScopeManager, ScopeType};
    ///
    /// let mut manager = ScopeManager::new();
    /// let parent_id = manager.create_scope(ScopeType::Query, None);
    /// let child_id = manager.create_scope(ScopeType::Subquery, Some(parent_id));
    ///
    /// assert!(child_id > parent_id);
    /// ```
    pub fn create_scope(&mut self, scope_type: ScopeType, parent_id: Option<usize>) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let mut scope = Scope::new(id, scope_type);
        if let Some(parent) = parent_id {
            scope = scope.with_parent(parent);
        }

        self.scopes.push(scope);
        id
    }

    /// Get a scope by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Scope ID
    ///
    /// # Returns
    ///
    /// `Some(&Scope)` if found, `None` otherwise
    pub fn get_scope(&self, id: usize) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Get a mutable reference to a scope by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Scope ID
    ///
    /// # Returns
    ///
    /// `Some(&mut Scope)` if found, `None` otherwise
    pub fn get_scope_mut(&mut self, id: usize) -> Option<&mut Scope> {
        self.scopes.get_mut(id)
    }

    /// Resolve a table by name, searching through the current scope and all parent scopes
    ///
    /// # Arguments
    ///
    /// * `name` - Table name or alias to resolve
    /// * `scope_id` - Starting scope ID for the search
    ///
    /// # Returns
    ///
    /// `Ok(&TableSymbol)` if found, `Err(SemanticError)` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol};
    /// use unified_sql_lsp_catalog::DataType;
    ///
    /// let mut manager = ScopeManager::new();
    /// let parent_id = manager.create_scope(ScopeType::Query, None);
    /// let child_id = manager.create_scope(ScopeType::Subquery, Some(parent_id));
    ///
    /// // Add table to parent scope
    /// let table = TableSymbol::new("users");
    /// manager.get_scope_mut(parent_id).unwrap().add_table(table);
    ///
    /// // Can resolve from child scope
    /// let resolved = manager.resolve_table("users", child_id);
    /// assert!(resolved.is_ok());
    /// ```
    pub fn resolve_table(&self, name: &str, scope_id: usize) -> SemanticResult<&TableSymbol> {
        let mut current_id = Some(scope_id);

        while let Some(id) = current_id {
            if let Some(scope) = self.get_scope(id) {
                if let Some(table) = scope.find_table(name) {
                    return Ok(table);
                }
                current_id = scope.parent_id;
            } else {
                return Err(SemanticError::InvalidScope(format!("scope {}", id)));
            }
        }

        Err(SemanticError::TableNotFound(name.to_string()))
    }

    /// Resolve a column by name, searching through all visible tables
    ///
    /// # Arguments
    ///
    /// * `name` - Column name to resolve
    /// * `scope_id` - Starting scope ID for the search
    ///
    /// # Returns
    ///
    /// `Ok((&TableSymbol, &ColumnSymbol))` if found uniquely
    /// `Err(SemanticError::ColumnNotFound)` if not found
    /// `Err(SemanticError::AmbiguousColumn)` if found in multiple tables
    pub fn resolve_column(
        &self,
        name: &str,
        scope_id: usize,
    ) -> SemanticResult<(&TableSymbol, &ColumnSymbol)> {
        let mut current_id = Some(scope_id);
        let mut found = Vec::new();

        // Collect all matching columns from visible scopes
        while let Some(id) = current_id {
            if let Some(scope) = self.get_scope(id) {
                for table in &scope.tables {
                    if let Some(column) = table.find_column(name) {
                        found.push((table, column));
                    }
                }
                current_id = scope.parent_id;
            } else {
                return Err(SemanticError::InvalidScope(format!("scope {}", id)));
            }
        }

        match found.len() {
            0 => Err(SemanticError::ColumnNotFound(name.to_string())),
            1 => Ok((found[0].0, found[0].1)),
            _ => {
                let tables: Vec<String> = found
                    .iter()
                    .map(|(t, _)| t.display_name().to_string())
                    .collect();
                Err(SemanticError::AmbiguousColumn(name.to_string(), tables))
            }
        }
    }

    /// Get the total number of scopes
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::DataType;

    fn create_mock_table() -> TableSymbol {
        TableSymbol::new("users").with_alias("u").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "users"),
            ColumnSymbol::new("name", DataType::Text, "users"),
            ColumnSymbol::new("email", DataType::Text, "users"),
        ])
    }

    fn create_mock_orders_table() -> TableSymbol {
        TableSymbol::new("orders").with_columns(vec![
            ColumnSymbol::new("id", DataType::Integer, "orders"),
            ColumnSymbol::new("user_id", DataType::Integer, "orders"),
            ColumnSymbol::new("id", DataType::Integer, "orders"),
        ])
    }

    #[test]
    fn test_scope_add_duplicate_alias_error() {
        let mut scope = Scope::new(0, ScopeType::Query);
        let table1 = create_mock_table();
        let table2 = TableSymbol::new("users")
            .with_alias("u")
            .with_columns(vec![ColumnSymbol::new("id", DataType::Integer, "users")]);

        scope.add_table(table1).unwrap();
        let result = scope.add_table(table2);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            SemanticError::DuplicateAlias(_)
        ));
    }

    #[test]
    fn test_scope_manager_new() {
        let manager = ScopeManager::new();
        assert_eq!(manager.scope_count(), 0);
    }

    #[test]
    fn test_scope_manager_create_scope() {
        let mut manager = ScopeManager::new();
        let id = manager.create_scope(ScopeType::Query, None);

        assert_eq!(id, 0);
        assert_eq!(manager.scope_count(), 1);
        assert!(manager.get_scope(id).is_some());
    }

    #[test]
    fn test_scope_manager_resolve_table_current_scope() {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        let table = create_mock_table();
        let _ = manager.get_scope_mut(scope_id).unwrap().add_table(table);

        let result = manager.resolve_table("users", scope_id);
        assert!(result.is_ok());

        let result = manager.resolve_table("u", scope_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_scope_manager_resolve_table_parent_scope() {
        let mut manager = ScopeManager::new();
        let parent_id = manager.create_scope(ScopeType::Query, None);
        let child_id = manager.create_scope(ScopeType::Subquery, Some(parent_id));

        let table = create_mock_table();
        let _ = manager.get_scope_mut(parent_id).unwrap().add_table(table);

        // Can resolve from child scope
        let result = manager.resolve_table("users", child_id);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().table_name, "users");
    }

    #[test]
    fn test_scope_manager_resolve_column_not_found() {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        let table = create_mock_table();
        let _ = manager.get_scope_mut(scope_id).unwrap().add_table(table);

        let result = manager.resolve_column("nonexistent", scope_id);
        assert!(matches!(
            result.unwrap_err(),
            SemanticError::ColumnNotFound(_)
        ));
    }

    #[test]
    fn test_scope_manager_resolve_column_ambiguous() {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        let table1 = create_mock_table();
        let table2 = create_mock_orders_table();

        let _ = manager.get_scope_mut(scope_id).unwrap().add_table(table1);
        let _ = manager.get_scope_mut(scope_id).unwrap().add_table(table2);

        // Both tables have "id" column
        let result = manager.resolve_column("id", scope_id);
        assert!(matches!(
            result.unwrap_err(),
            SemanticError::AmbiguousColumn(_, _)
        ));
    }

    #[test]
    fn test_scope_manager_resolve_column_qualified() {
        let mut manager = ScopeManager::new();
        let scope_id = manager.create_scope(ScopeType::Query, None);

        let table = create_mock_table();
        let _ = manager.get_scope_mut(scope_id).unwrap().add_table(table);

        let result = manager.resolve_column("name", scope_id);
        assert!(result.is_ok());
        let (table_ref, column_ref) = result.unwrap();
        assert_eq!(column_ref.name, "name");
        assert_eq!(table_ref.table_name, "users");
    }

    #[test]
    fn test_scope_hierarchy() {
        let mut manager = ScopeManager::new();

        // Create hierarchy: Query -> Subquery -> CTE
        let query_id = manager.create_scope(ScopeType::Query, None);
        let subquery_id = manager.create_scope(ScopeType::Subquery, Some(query_id));
        let cte_id = manager.create_scope(ScopeType::CTE, Some(subquery_id));

        // Check parent relationships
        let query = manager.get_scope(query_id).unwrap();
        let subquery = manager.get_scope(subquery_id).unwrap();
        let cte = manager.get_scope(cte_id).unwrap();

        assert!(query.parent_id.is_none());
        assert_eq!(subquery.parent_id, Some(query_id));
        assert_eq!(cte.parent_id, Some(subquery_id));
    }
}
