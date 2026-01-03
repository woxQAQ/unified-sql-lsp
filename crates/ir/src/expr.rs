// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Expressions
//!
//! This module represents SQL expressions in the IR.
//!
//! ## Design
//!
//! Expressions are the building blocks of SQL queries and can represent:
//!
//! - **Column references**: `table.column` or unqualified `column`
//! - **Literal values**: Numbers, strings, booleans, NULL
//! - **Binary operations**: Arithmetic, comparison, logical, string operations
//! - **Unary operations**: Negation, NOT, EXISTS
//! - **Function calls**: Built-in and user-defined functions
//! - **Case expressions**: Conditional logic (CASE WHEN...THEN...ELSE)
//! - **Cast expressions**: Type conversions
//! - **Lists**: Value lists for IN clauses
//!
//! ## Expression Hierarchy
//!
//! Expressions form a tree structure where complex expressions contain
//! sub-expressions. For example:
//!
//! ```text
//! BinaryOp {
//!   left: Box<Expr::Column("price")>,
//!   op: Mul,
//!   right: Box<Expr::BinaryOp {
//!     left: Box<Expr::Literal(1.1)>>,
//!     op: Add,
//!     right: Box<Expr::Literal(0.5)>
//!   }>
//! }
//! ```
//!
//! Represents: `price * (1.1 + 0.5)`
//!
//! ## Column References
//!
//! [`ColumnRef`] represents column references with optional table qualification:
//!
//! ```sql
//! -- Unqualified column
//! user_id
//!
//! -- Qualified column
//! users.user_id
//! ```
//!
//! ## Literal Values
//!
//! [`Literal`] supports common SQL literal types:
//!
//! - `Null`: NULL value
//! - `Boolean`: true/false
//! - `Integer`: 64-bit integers
//! - `Float`: 64-bit floating-point
//! - `String`: Text/varchar values
//!
//! ## Operators
//!
//! ### Binary Operators
//!
//! - **Arithmetic**: Add, Sub, Mul, Div, Mod
//! - **Comparison**: Eq, NotEq, Lt, LtEq, Gt, GtEq
//! - **Logical**: And, Or
//! - **String**: Like, NotLike, ILike, NotILike
//! - **Other**: In, NotIn, Is, IsNot
//!
//! ### Unary Operators
//!
//! - **Neg**: Numeric negation (-x)
//! - **Not**: Logical NOT
//! - **Exists**: EXISTS subquery
//!
//! ## Function Calls
//!
//! Function expressions support:
//!
//! - Aggregate functions: `COUNT(*)`, `SUM(column)`, `MAX(expr)`
//! - Scalar functions: `ABS(x)`, `UPPER(str)`
//! - DISTINCT modifier: `COUNT(DISTINCT column)`
//!
//! ## Examples
//!
//! ### Column reference in WHERE clause
//! ```sql
//! WHERE user_id = 123
//! ```
//!
//! ### Arithmetic expression
//! ```sql
//! SELECT price * 1.1 AS price_with_tax
//! FROM products
//! ```
//!
//! ### Complex expression with parentheses
//! ```sql
//! WHERE price * (1.1 + 0.5) > 100
//! ```
//!
//! ### Function calls
//! ```sql
//! -- Aggregate function
//! SELECT COUNT(*), SUM(amount) FROM orders
//!
//! -- Scalar function
//! SELECT UPPER(name), ABS(balance)
//! FROM accounts
//!
//! -- DISTINCT in aggregate
//! SELECT COUNT(DISTINCT category)
//! FROM products
//! ```
//!
//! ### Comparison and logical operators
//! ```sql
//! WHERE age >= 18
//!   AND status = 'active'
//!   OR (is_admin = true)
//! ```

use serde::{Deserialize, Serialize};

/// A SQL expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Expr {
    /// Column reference (e.g., `table.column` or just `column`)
    Column(ColumnRef),

    /// Literal value
    Literal(Literal),

    /// Binary operation (e.g., `a + b`, `x = 5`)
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },

    /// Unary operation (e.g., `-x`, `NOT a`)
    UnaryOp { op: UnaryOp, expr: Box<Expr> },

    /// Function call (e.g., `COUNT(*)`, `MAX(column)`)
    Function {
        name: String,
        args: Vec<Expr>,
        distinct: bool,
    },

    /// CASE expression
    Case {
        conditions: Vec<Expr>,
        results: Vec<Expr>,
        else_result: Option<Box<Expr>>,
    },

    /// CAST expression
    Cast { expr: Box<Expr>, type_name: String },

    /// Parenthesized expression
    Paren(Box<Expr>),

    /// List of expressions (e.g., for IN clause)
    List(Vec<Expr>),
}

/// Column reference
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnRef {
    /// Optional table/alias name
    pub table: Option<String>,
    /// Column name
    pub column: String,
}

impl ColumnRef {
    pub fn new(column: impl Into<String>) -> Self {
        Self {
            table: None,
            column: column.into(),
        }
    }

    pub fn with_table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn qualified(&self) -> String {
        match &self.table {
            Some(table) => format!("{}.{}", table, self.column),
            None => self.column.clone(),
        }
    }
}

/// Literal value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Logical
    And,
    Or,

    // String
    Like,
    NotLike,
    ILike,
    NotILike,

    // Other
    In,
    NotIn,
    Is,
    IsNot,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UnaryOp {
    Neg,
    Not,
    Exists,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_ref() {
        let col = ColumnRef::new("id");
        assert_eq!(col.qualified(), "id");
        assert!(col.table.is_none());

        let qualified = col.with_table("users");
        assert_eq!(qualified.qualified(), "users.id");
        assert_eq!(qualified.table.as_deref(), Some("users"));
    }
}
