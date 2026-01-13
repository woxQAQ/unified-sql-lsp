// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

use crate::{Dialect, FunctionMetadata, builtin};
use std::collections::HashMap;

/// Function registry for builtin SQL functions
///
/// This struct stores and provides lookup for builtin functions
/// across different SQL dialects.
#[derive(Debug, Clone)]
pub struct FunctionRegistry {
    /// Functions organized by dialect
    functions: HashMap<Dialect, Vec<FunctionMetadata>>,
}

impl FunctionRegistry {
    /// Create a new function registry with all builtin functions loaded
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use unified_sql_lsp_function_registry::FunctionRegistry;
    ///
    /// let registry = FunctionRegistry::new();
    /// let mysql_funcs = registry.get_functions(Dialect::MySQL);
    /// ```
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
        };

        // Load builtin functions for each dialect
        registry
            .functions
            .insert(Dialect::MySQL, builtin::mysql::all_functions());
        registry
            .functions
            .insert(Dialect::PostgreSQL, builtin::postgresql::all_functions());

        registry
    }

    /// Get all functions for a specific dialect
    ///
    /// # Arguments
    ///
    /// * `dialect` - The SQL dialect to get functions for
    ///
    /// # Returns
    ///
    /// A vector of `FunctionMetadata` for the specified dialect.
    /// Returns an empty vector if the dialect is not supported.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let registry = FunctionRegistry::new();
    /// let mysql_funcs = registry.get_functions(Dialect::MySQL);
    /// assert!(!mysql_funcs.is_empty());
    /// ```
    pub fn get_functions(&self, dialect: Dialect) -> Vec<FunctionMetadata> {
        self.functions.get(&dialect).cloned().unwrap_or_default()
    }

    /// Lookup a single function by name and dialect
    ///
    /// # Arguments
    ///
    /// * `dialect` - The SQL dialect to search in
    /// * `name` - The function name to lookup (case-insensitive)
    ///
    /// # Returns
    ///
    /// `Some(&FunctionMetadata)` if found, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let registry = FunctionRegistry::new();
    /// let count_func = registry.get_function(Dialect::MySQL, "COUNT");
    /// assert!(count_func.is_some());
    /// assert_eq!(count_func.unwrap().name, "COUNT");
    /// ```
    pub fn get_function(&self, dialect: Dialect, name: &str) -> Option<&FunctionMetadata> {
        self.functions
            .get(&dialect)?
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Check if a function exists for a specific dialect
    ///
    /// # Arguments
    ///
    /// * `dialect` - The SQL dialect to check
    /// * `name` - The function name to check (case-insensitive)
    ///
    /// # Returns
    ///
    /// `true` if the function exists, `false` otherwise
    pub fn has_function(&self, dialect: Dialect, name: &str) -> bool {
        self.get_function(dialect, name).is_some()
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_registry() {
        let registry = FunctionRegistry::new();
        let mysql_funcs = registry.get_functions(Dialect::MySQL);
        let pg_funcs = registry.get_functions(Dialect::PostgreSQL);

        // Should have functions for both dialects
        assert!(!mysql_funcs.is_empty());
        assert!(!pg_funcs.is_empty());
    }

    #[test]
    fn test_get_function_case_insensitive() {
        let registry = FunctionRegistry::new();

        // All should return the same function
        let upper = registry.get_function(Dialect::MySQL, "COUNT");
        let lower = registry.get_function(Dialect::MySQL, "count");
        let mixed = registry.get_function(Dialect::MySQL, "Count");

        assert!(upper.is_some());
        assert!(lower.is_some());
        assert!(mixed.is_some());
    }

    #[test]
    fn test_has_function() {
        let registry = FunctionRegistry::new();

        assert!(registry.has_function(Dialect::MySQL, "COUNT"));
        assert!(!registry.has_function(Dialect::MySQL, "NONEXISTENT"));
    }
}
