// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details
//
//! # Unified SQL LSP - Semantic Analysis Layer
//!
//! This crate provides scope and symbol resolution for SQL queries.
//!
//! ## Overview
//!
//! Semantic analysis builds on the IR layer to provide:
//! - **Scope management**: Track tables across nested queries
//! - **Symbol resolution**: Find tables and columns by name
//! - **Type checking**: Validate column references and types
//!
//! ## Core Concepts
//!
//! ### Scopes
//!
//! A [`Scope`] represents a lexical context in a SQL query where tables are visible.
//! Scopes form a hierarchy, allowing subqueries to access tables from parent queries.
//!
//! ```rust
//! use unified_sql_lsp_semantic::{Scope, ScopeType};
//!
//! let scope = Scope::new(0, ScopeType::Query);
//! ```
//!
//! ### Symbols
//!
//! [`TableSymbol`] and [`ColumnSymbol`] represent tables and columns with their metadata.
//!
//! ```rust
//! use unified_sql_lsp_semantic::{TableSymbol, ColumnSymbol};
//! use unified_sql_lsp_catalog::DataType;
//!
//! let table = TableSymbol::new("users")
//!     .with_alias("u")
//!     .with_columns(vec![
//!         ColumnSymbol::new("id", DataType::Integer, "users"),
//!         ColumnSymbol::new("name", DataType::Text, "users"),
//!     ]);
//! ```
//!
//! ### ScopeManager
//!
//! The [`ScopeManager`] manages a collection of scopes and provides symbol resolution
//! across the scope hierarchy.
//!
//! ```rust
//! use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol};
//!
//! let mut manager = ScopeManager::new();
//! let scope_id = manager.create_scope(ScopeType::Query, None);
//!
//! let table = TableSymbol::new("users");
//! manager.get_scope_mut(scope_id).unwrap().add_table(table);
//!
//! // Resolve table from scope
//! let resolved = manager.resolve_table("users", scope_id).unwrap();
//! assert_eq!(resolved.table_name, "users");
//! ```
//!
//! ## Examples
//!
//! ### Nested Query Scopes
//!
//! ```rust
//! use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol, ColumnSymbol};
//! use unified_sql_lsp_catalog::DataType;
//!
//! let mut manager = ScopeManager::new();
//!
//! // Create parent query scope
//! let parent_id = manager.create_scope(ScopeType::Query, None);
//! let users = TableSymbol::new("users")
//!     .with_columns(vec![
//!         ColumnSymbol::new("id", DataType::Integer, "users"),
//!         ColumnSymbol::new("name", DataType::Text, "users"),
//!     ]);
//! manager.get_scope_mut(parent_id).unwrap().add_table(users);
//!
//! // Create subquery scope with parent
//! let child_id = manager.create_scope(ScopeType::Subquery, Some(parent_id));
//!
//! // Child can access parent's tables
//! let resolved = manager.resolve_table("users", child_id).unwrap();
//! assert_eq!(resolved.table_name, "users");
//! ```
//!
//! ### Column Resolution with Ambiguity Detection
//!
//! ```rust
//! use unified_sql_lsp_semantic::{ScopeManager, ScopeType, TableSymbol, ColumnSymbol};
//! use unified_sql_lsp_catalog::DataType;
//!
//! let mut manager = ScopeManager::new();
//! let scope_id = manager.create_scope(ScopeType::Query, None);
//!
//! // Add two tables with "id" columns
//! let users = TableSymbol::new("users")
//!     .with_columns(vec![ColumnSymbol::new("id", DataType::Integer, "users")]);
//! let orders = TableSymbol::new("orders")
//!     .with_columns(vec![ColumnSymbol::new("id", DataType::Integer, "orders")]);
//!
//! manager.get_scope_mut(scope_id).unwrap().add_table(users);
//! manager.get_scope_mut(scope_id).unwrap().add_table(orders);
//!
//! // Resolving "id" will fail due to ambiguity
//! let result = manager.resolve_column("id", scope_id);
//! assert!(result.is_err());
//! ```

pub mod alias_resolution;
pub mod analyzer;
pub mod error;
pub mod resolution;
pub mod scope;
pub mod symbol;

// Re-export commonly used types
pub use alias_resolution::{AliasResolutionError, AliasResolver, ResolutionResult, ResolutionStrategy};
pub use analyzer::SemanticAnalyzer;
pub use error::{SemanticError, SemanticResult};
pub use resolution::{
    ColumnCandidate, ColumnResolutionResult, ColumnResolver, MatchKind, ResolutionConfig,
};
pub use scope::{Scope, ScopeManager, ScopeType};
pub use symbol::{ColumnSymbol, TableSymbol};
