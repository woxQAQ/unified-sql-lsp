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

pub mod query;
pub mod expr;
pub mod dialect;

// Re-export commonly used types
pub use query::{
    Query, SetOp, SelectStatement, TableRef, Join, JoinType, JoinCondition,
    SelectItem, OrderBy, CommonTableExpr, WindowDef,
};
pub use expr::{Expr, ColumnRef, Literal, BinaryOp, UnaryOp};
pub use dialect::{Dialect, DialectExtensions};
