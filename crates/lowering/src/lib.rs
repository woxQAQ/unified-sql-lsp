// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Unified SQL LSP - Lowering Layer
//!
//! This crate provides CST (Concrete Syntax Tree) to IR (Intermediate Representation)
//! conversion for SQL queries across multiple dialects.
//!
//! ## Overview
//!
//! The lowering layer is responsible for:
//! - Converting Tree-sitter CST nodes to unified IR
//! - Handling dialect-specific syntax variations
//! - Graceful error recovery (partial success mode)
//! - Source mapping for diagnostics and code actions
//!
//! ## Lowering Process
//!
//! The lowering process converts syntax trees into semantic IR:
//!
//! ```text
//! Tree-sitter CST → Lowering → IR Query → Semantic Analysis → Completion
//! ```
//!
//! ## Error Handling Strategy
//!
//! The lowering trait supports three outcomes:
//!
//! - **Success**: Complete conversion without errors
//! - **Partial**: Some parts converted, others failed with placeholders
//! - **Failed**: Complete conversion failure (critical error)
//!
//! ## Usage
//!
//! ```rust,ignore
//! // TODO: (LOWERING-002, LOWERING-003) Implement actual lowering examples when dialect implementations are ready
//! use unified_sql_lsp_lowering::{Lowering, LoweringContext, LoweringResult};
//! use unified_sql_lsp_ir::{Query, Dialect};
//!
//! // Create dialect-specific lowering implementation
//! let lowering = MySQLLowering::new();
//!
//! // Create context with source and dialect
//! let mut ctx = LoweringContext::new(Dialect::MySQL, "SELECT * FROM users");
//!
//! // Lower CST node to IR
//! let result: LoweringResult<Query> = lowering.lower_query(&mut ctx, cst_root);
//!
//! match result {
//!     Ok(query) => {
//!         if ctx.has_errors() {
//!             // Partial success - query has placeholders for failed parts
//!             println!("Converted with {} errors", ctx.errors().len());
//!         } else {
//!             // Complete success
//!             println!("Fully converted");
//!         }
//!     }
//!     Err(e) => {
//!         // Complete failure
//!         eprintln!("Failed to lower: {}", e);
//!     }
//! }
//! ```

pub mod context;
pub mod cst;
pub mod dialect;
pub mod error;

pub use context::{LoweringContext, SourceLocation};
pub use cst::CstNode;
pub use error::{ErrorSeverity, LoweringError, LoweringOutcome, LoweringResult};

use unified_sql_lsp_ir::{Dialect, Expr, Query};

/// Core trait for lowering CST to IR
///
/// This trait defines the interface for converting dialect-specific CST nodes
/// into the unified IR. Implementations are provided for each supported SQL dialect.
///
/// # Type Parameters
///
/// - `N`: The CST node type (e.g., `tree_sitter::Node` in the future)
///
/// # Error Handling
///
/// Implementations should follow the graceful degradation strategy:
///
/// - **For critical errors** (missing required structure): Return `Err(LoweringError)`
/// - **For recoverable errors** (unsupported syntax, invalid literals):
///   - Insert placeholder IR nodes
///   - Add errors to `LoweringContext`
///   - Return `Ok` with partial result
///
/// # Example
///
/// ```rust,ignore
/// // TODO: (LOWERING-002, LOWERING-003) Implement actual MySQL/PostgreSQL lowering implementations
/// use unified_sql_lsp_lowering::{Lowering, LoweringContext, LoweringResult};
/// use unified_sql_lsp_ir::{Query, Dialect};
///
/// struct MySQLLowering;
///
/// impl<N> Lowering<N> for MySQLLowering
/// where
///     N: CstNode
/// {
///     fn lower_query(
///         &self,
///         ctx: &mut LoweringContext,
///         node: &N,
///     ) -> LoweringResult<Query> {
///         // Implementation...
///     }
/// }
/// ```
pub trait Lowering<N>
where
    N: CstNode,
{
    /// Lower a CST node to a Query IR
    ///
    /// This is the main entry point for converting a complete SQL query
    /// from CST to IR. The query may contain:
    ///
    /// - SELECT statements
    /// - Set operations (UNION, INTERSECT, EXCEPT)
    /// - CTEs (WITH clauses)
    /// - Subqueries
    ///
    /// # Arguments
    ///
    /// - `ctx`: The lowering context (accumulates errors and state)
    /// - `node`: The root CST node representing the query
    ///
    /// # Returns
    ///
    /// - `Ok(Query)`: Successfully lowered query (may contain placeholders)
    /// - `Err(LoweringError)`: Critical failure (e.g., empty CST, invalid structure)
    ///
    /// # Context Management
    ///
    /// Implementations should:
    /// 1. Use `ctx.dialect()` to check dialect-specific features
    /// 2. Call `ctx.add_error()` for recoverable errors
    /// 3. Use `ctx.create_placeholder()` for unsupported syntax
    fn lower_query(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Query>;

    /// Lower a CST node to an Expression IR
    ///
    /// Handles expressions like:
    /// - Column references (`column`, `table.column`)
    /// - Literals (`123`, `'string'`, `TRUE`)
    /// - Binary operations (`a + b`, `x = 5`)
    /// - Function calls (`COUNT(*)`, `MAX(column)`)
    /// - CASE expressions
    ///
    /// # Arguments
    ///
    /// - `ctx`: The lowering context
    /// - `node`: The CST node representing an expression
    ///
    /// # Returns
    ///
    /// - `Ok(Expr)`: Successfully lowered expression
    /// - `Err(LoweringError)`: Critical failure (e.g., unrecognized node type)
    fn lower_expr(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>;

    /// Check if this lowering implementation supports a specific node kind
    ///
    /// This allows graceful degradation by checking support before attempting
    /// to lower a node.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if !lowering.supports_node(&node, "lateral_join") {
    ///     ctx.add_error(LoweringError::UnsupportedSyntax {
    ///         feature: "LATERAL JOIN",
    ///         suggestion: "MySQL does not support LATERAL JOIN",
    ///     });
    ///     return Ok(ctx.create_placeholder());
    /// }
    /// ```
    fn supports_node(&self, node: &N, kind: &str) -> bool;

    /// Get the dialect this lowering implementation targets
    fn dialect(&self) -> Dialect;
}
