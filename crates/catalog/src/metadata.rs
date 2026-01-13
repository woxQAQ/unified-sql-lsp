// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Metadata types for database schema information
//!
//! This module re-exports metadata types from the `unified-sql-lsp-ir` crate.
//! These types define the schema information for tables, columns, and functions.

// Re-export all metadata types from the ir crate
pub use unified_sql_lsp_ir::{
    ColumnMetadata, DataType, FunctionMetadata, FunctionParameter, FunctionType, TableMetadata,
    TableReference, TableType,
};
