// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Unified SQL LSP - Intermediate Representation
//!
//! This crate provides the Intermediate Representation (IR) for SQL queries.
//! The IR is designed to:
//! - Be dialect-agnostic (support MySQL, PostgreSQL, etc.)
//! - Preserve semantic information for analysis
//! - Enable lowering from CST (Concrete Syntax Tree)
//! - Support dialect-specific extensions

pub mod dialect;
pub mod expr;
pub mod query;

// Re-export commonly used types
pub use dialect::{Dialect, DialectExtensions};
pub use expr::{BinaryOp, ColumnRef, Expr, Literal, UnaryOp};
pub use query::{
    CommonTableExpr, Join, JoinCondition, JoinType, OrderBy, Query, SelectItem, SelectStatement,
    SetOp, TableRef, WindowDef,
};
