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
pub mod metadata;
pub mod query;

// Re-export commonly used types
pub use dialect::{Dialect, DialectExtensions};
pub use expr::{BinaryOp, ColumnRef, Expr, Literal, UnaryOp};
pub use expr::{WindowFrame, WindowFrameBound, WindowFrameUnits, WindowSpec};
pub use metadata::{
    ColumnMetadata, DataType, FunctionMetadata, FunctionParameter, FunctionType, TableMetadata,
    TableReference, TableType,
};
pub use query::{
    Assignment, CommonTableExpr, DeleteStatement, InsertSource, InsertStatement, Join,
    JoinCondition, JoinType, OnConflict, OrderBy, Query, SelectItem, SelectStatement, SetOp,
    SortDirection, TableRef, UpdateStatement, WindowDef,
};
