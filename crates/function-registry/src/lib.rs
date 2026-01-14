// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # SQL Function Registry
//!
//! This crate provides a centralized registry for builtin SQL functions
//! across multiple dialects and versions.
//!
//! ## Features
//!
//! - Centralized function definitions for MySQL and PostgreSQL
//! - Type-safe function lookup by dialect
//! - Re-exports metadata types from the ir crate
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_function_registry::{FunctionRegistry, Dialect};
//!
//! let registry = FunctionRegistry::new();
//! let mysql_funcs = registry.get_functions(Dialect::MySQL);
//! let pg_funcs = registry.get_functions(Dialect::PostgreSQL);
//! ```

pub mod builtin;
pub mod hover;
pub mod registry;

// Re-exports from ir for convenience
pub use unified_sql_lsp_ir::{
    DataType, Dialect, FunctionMetadata, FunctionParameter, FunctionType,
};

pub use hover::HoverInfoProvider;
pub use registry::FunctionRegistry;
