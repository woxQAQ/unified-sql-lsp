// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Query Representation
//!
//! This module represents SQL queries in the IR.
//!
//! ## Design
//!
//! The query representation models the complete structure of SQL SELECT queries
//! and set operations (UNION, INTERSECT, EXCEPT). It serves as the unified
//! intermediate representation for all supported dialects.
//!
//! ## Query Structure
//!
//! A [`Query`] consists of:
//!
//! - **Body**: The main query (SELECT or set operation)
//! - **CTEs**: Common Table Expressions (WITH clauses)
//! - **ORDER BY**: Sorting specification
//! - **LIMIT**: Row count limit
//! - **OFFSET**: Row offset for pagination
//! - **Dialect**: Target SQL dialect for the query
//!
//! ## Set Operations
//!
//! [`SetOp`] represents set operations and query combinations:
//!
//! - `Select`: A single SELECT statement
//! - `Union`: UNION [ALL] of two queries
//! - `Intersect`: INTERSECT [DISTINCT] of two queries
//! - `Except`: EXCEPT [DISTINCT] of two queries
//!
//! Set operations form a tree structure, allowing complex nested combinations.
//!
//! ## SELECT Statement
//!
//! [`SelectStatement`] contains the core SELECT components:
//!
//! - **Projection**: SELECT clause with columns/expressions
//! - **FROM**: Table sources with joins
//! - **WHERE**: Row filtering condition
//! - **GROUP BY**: Aggregation groups
//! - **HAVING**: Group filtering condition
//! - **WINDOW**: Window definitions
//! - **Distinct**: DISTINCT modifier
//!
//! ## Table References and Joins
//!
//! [`TableRef`] represents a table in the FROM clause with:
//!
//! - **Name**: Table name (may include schema/database)
//! - **Alias**: Table alias for the query
//! - **Joins**: List of joined tables
//!
//! [`Join`] represents table joins with:
//!
//! - **Type**: INNER, LEFT, RIGHT, FULL, CROSS
//! - **Table**: The table being joined
//! - **Condition**: ON expression, USING columns, or NATURAL
//!
//! ### Join Examples
//!
//! ```sql
//! -- INNER JOIN
//! FROM users u INNER JOIN orders o ON u.id = o.user_id
//!
//! -- LEFT JOIN
//! FROM users u LEFT JOIN orders o ON u.id = o.user_id
//!
//! -- USING clause
//! FROM users u JOIN orders o USING (user_id)
//!
//! -- NATURAL JOIN
//! FROM users NATURAL JOIN profiles
//! ```
//!
//! ## Projection Items
//!
//! [`SelectItem`] represents items in the SELECT clause:
//!
//! - `UnnamedExpr`: Regular expression (e.g., `column`, `a + b`)
//! - `AliasedExpr`: Expression with alias (e.g., `col AS name`)
//! - `QualifiedWildcard`: Table-qualified wildcard (e.g., `table.*`)
//! - `Wildcard`: Unqualified wildcard (`*`)
//!
//! ### Projection Examples
//!
//! ```sql
//! -- Unnamed expressions
//! SELECT id, name, price * 1.1
//!
//! -- Aliased expressions
//! SELECT id AS user_id, COUNT(*) AS total
//!
//! -- Wildcards
//! SELECT *
//! SELECT users.*
//! ```
//!
//! ## Window Functions
//!
//! [`WindowDef`] represents window definitions for window functions:
//!
//! - **Name**: Optional window name
//! - **Partition By**: Grouping for window functions
//! - **Order By**: Ordering within window
//! - **Frame**: Window frame (ROWS, RANGE, GROUPS)
//!
//! ### Window Function Examples
//!
//! ```sql
//! -- Simple window function
//! SELECT ROW_NUMBER() OVER (ORDER BY created_at)
//!
//! -- With PARTITION BY
//! SELECT ROW_NUMBER() OVER (PARTITION BY department ORDER BY salary)
//!
//! -- Named window
//! SELECT ROW_NUMBER() OVER (w) AS rank
//! FROM employees
//! WINDOW w AS (PARTITION BY department ORDER BY salary)
//!
//! -- Window frame
//! SELECT SUM(amount) OVER (
//!   ORDER BY created_at
//!   ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW
//! )
//! ```
//!
//! ## Common Table Expressions (CTEs)
//!
//! [`CommonTableExpr`] represents CTEs (WITH clauses):
//!
//! - **Name**: CTE name
//! - **Columns**: Optional column list
//! - **Query**: The CTE query
//! - **Materialized**: Optional materialization hint (PostgreSQL)
//!
//! ### CTE Examples
//!
//! ```sql
//! -- Simple CTE
//! WITH active_users AS (
//!   SELECT * FROM users WHERE active = true
//! )
//! SELECT * FROM active_users
//!
//! -- CTE with column list
//! WITH user_counts (user_id, total) AS (
//!   SELECT user_id, COUNT(*) FROM orders GROUP BY user_id
//! )
//! SELECT * FROM user_counts
//!
//! -- Materialized CTE (PostgreSQL)
//! WITH active_users AS MATERIALIZED (
//!   SELECT * FROM users WHERE active = true
//! )
//! SELECT * FROM active_users
//!
//! -- Recursive CTE
//! WITH RECURSIVE hierarchy AS (
//!   SELECT id, parent_id, 1 AS level
//!   FROM categories WHERE parent_id IS NULL
//!   UNION ALL
//!   SELECT c.id, c.parent_id, h.level + 1
//!   FROM categories c
//!   JOIN hierarchy h ON c.parent_id = h.id
//! )
//! SELECT * FROM hierarchy
//! ```
//!
//! ## Builder Pattern
//!
//! All complex types support builder pattern methods:
//!
//! ```rust,ignore
//! use unified_sql_lsp_ir::{Query, SelectStatement, TableRef};
//! use unified_sql_lsp_ir::Dialect;
//!
//! let query = Query::new(Dialect::MySQL)
//!     .with_limit(Expr::Literal(10))
//!     .with_offset(Expr::Literal(5));
//! ```
//!
//! ## Examples
//!
//! ### Simple SELECT
//! ```sql
//! SELECT id, name FROM users WHERE active = true
//! ```
//!
//! ### SELECT with Joins
//! ```sql
//! SELECT u.id, u.name, o.total
//! FROM users u
//! LEFT JOIN orders o ON u.id = o.user_id
//! WHERE u.active = true
//! ```
//!
//! ### CTE with Window Function
//! ```sql
//! WITH ranked_users AS (
//!   SELECT id, name,
//!     ROW_NUMBER() OVER (PARTITION BY department ORDER BY salary) as rank
//!   FROM employees
//! )
//! SELECT * FROM ranked_users WHERE rank <= 10
//! ```
//!
//! ### Set Operations
//! ```sql
//! -- UNION ALL
//! SELECT id FROM users_a
//! UNION ALL
//! SELECT id FROM users_b
//!
//! -- INTERSECT
//! SELECT id FROM premium_users
//! INTERSECT
//! SELECT id FROM active_users
//!
//! -- EXCEPT
//! SELECT id FROM all_users
//! EXCEPT
//! SELECT id FROM deleted_users
//! ```
//!
//! ### Aggregate Queries
//! ```sql
//! SELECT department, COUNT(*) AS emp_count, AVG(salary) AS avg_salary
//! FROM employees
//! WHERE hire_date >= '2024-01-01'
//! GROUP BY department
//! HAVING COUNT(*) > 5
//! ORDER BY avg_salary DESC
//! LIMIT 10
//! ```
//!
//! ## Dialect Handling
//!
//! The [`Query`] struct includes a [`Dialect`] field to track which SQL dialect
//! the query is targeting. This enables:
//!
//! - Dialect-specific validation (e.g., MySQL's `LIMIT offset, count` vs PostgreSQL's `LIMIT count OFFSET offset`)
//! - Feature checking via [`Dialect::supports()`]
//! - Version-specific syntax handling

use serde::{Deserialize, Serialize};
use crate::expr::Expr;
use crate::dialect::Dialect;

/// A SQL query (SELECT statement or set operation)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Query {
    /// The main body of the query
    pub body: SetOp,

    /// Optional ORDER BY clause
    pub order_by: Option<Vec<OrderBy>>,

    /// Optional LIMIT clause
    pub limit: Option<Expr>,

    /// Optional OFFSET clause
    pub offset: Option<Expr>,

    /// Optional WITH clause (CTE)
    pub ctes: Vec<CommonTableExpr>,

    /// The dialect this query is written for
    pub dialect: Dialect,
}

impl Query {
    pub fn new(dialect: Dialect) -> Self {
        Self {
            body: SetOp::Select(Box::default()),
            order_by: None,
            limit: None,
            offset: None,
            ctes: Vec::new(),
            dialect,
        }
    }

    pub fn with_ctes(mut self, ctes: impl IntoIterator<Item = CommonTableExpr>) -> Self {
        self.ctes = ctes.into_iter().collect();
        self
    }

    pub fn with_limit(mut self, limit: Expr) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: Expr) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_order_by(mut self, order_by: Vec<OrderBy>) -> Self {
        self.order_by = Some(order_by);
        self
    }
}

impl Default for Query {
    fn default() -> Self {
        Self::new(Dialect::MySQL)
    }
}

/// Set operation (UNION, INTERSECT, EXCEPT) or SELECT
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SetOp {
    /// SELECT statement
    Select(Box<SelectStatement>),

    /// UNION [ALL | DISTINCT]
    Union {
        left: Box<Query>,
        right: Box<Query>,
        all: bool,
    },

    /// INTERSECT [DISTINCT]
    Intersect {
        left: Box<Query>,
        right: Box<Query>,
        distinct: bool,
    },

    /// EXCEPT [DISTINCT]
    Except {
        left: Box<Query>,
        right: Box<Query>,
        distinct: bool,
    },
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStatement {
    /// SELECT DISTINCT or ALL
    pub distinct: bool,

    /// Projection list (columns to select)
    pub projection: Vec<SelectItem>,

    /// FROM clause
    pub from: Vec<TableRef>,

    /// WHERE clause
    pub where_clause: Option<Expr>,

    /// GROUP BY clause
    pub group_by: Vec<Expr>,

    /// HAVING clause
    pub having: Option<Expr>,

    /// WINDOW clause
    pub window: Vec<WindowDef>,
}

impl Default for SelectStatement {
    fn default() -> Self {
        Self {
            distinct: false,
            projection: Vec::new(),
            from: Vec::new(),
            where_clause: None,
            group_by: Vec::new(),
            having: None,
            window: Vec::new(),
        }
    }
}

/// Item in a SELECT projection list
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
    /// Unnamed expression (e.g., `column` or `a + b`)
    UnnamedExpr(Expr),

    /// Expression with alias (e.g., `col AS name`)
    AliasedExpr { expr: Expr, alias: String },

    /// Qualified wildcard (e.g., `table.*`)
    QualifiedWildcard(String),

    /// Unqualified wildcard (`*`)
    Wildcard,
}

/// Table reference in FROM clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRef {
    /// Table name (may include schema/database)
    pub name: String,

    /// Alias
    pub alias: Option<String>,

    /// Joins
    pub joins: Vec<Join>,
}

/// JOIN clause
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Join {
    /// Join type
    pub join_type: JoinType,

    /// Table to join
    pub table: TableRef,

    /// Join condition (ON or USING)
    pub condition: JoinCondition,
}

/// Join type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

/// Join condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JoinCondition {
    On(Expr),
    Using(Vec<String>),
    Natural,
}

/// ORDER BY item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBy {
    pub expr: Expr,
    pub direction: Option<SortDirection>,
}

/// Sort direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Common Table Expression (CTE)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonTableExpr {
    pub name: String,
    pub columns: Vec<String>,
    pub query: Box<Query>,
    pub materialized: Option<bool>,
}

/// Window definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowDef {
    pub name: Option<String>,
    pub partition_by: Vec<Expr>,
    pub order_by: Vec<OrderBy>,
    pub window_frame: Option<WindowFrame>,
}

/// Window frame
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowFrame {
    pub units: WindowFrameUnits,
    pub start_bound: WindowFrameBound,
    pub end_bound: Option<WindowFrameBound>,
}

/// Window frame units
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WindowFrameUnits {
    Rows,
    Range,
    Groups,
}

/// Window frame bound
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WindowFrameBound {
    CurrentRow,
    Preceding(Option<Expr>),
    Following(Option<Expr>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr::{ColumnRef, Literal};

    #[test]
    fn test_query_creation() {
        let query = Query::new(Dialect::MySQL);
        assert_eq!(query.dialect, Dialect::MySQL);
        assert!(query.ctes.is_empty());
    }

    #[test]
    fn test_select_default() {
        let select = SelectStatement::default();
        assert!(!select.distinct);
        assert!(select.projection.is_empty());
        assert!(select.from.is_empty());
    }

    #[test]
    fn test_order_by() {
        let ob = OrderBy {
            expr: Expr::Column(ColumnRef::new("id")),
            direction: Some(SortDirection::Asc),
        };
        assert_eq!(ob.direction, Some(SortDirection::Asc));
    }

    #[test]
    fn test_window_frame() {
        let frame = WindowFrame {
            units: WindowFrameUnits::Rows,
            start_bound: WindowFrameBound::CurrentRow,
            end_bound: None,
        };
        assert_eq!(frame.units, WindowFrameUnits::Rows);
    }
}
