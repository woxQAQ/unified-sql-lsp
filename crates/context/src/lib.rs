// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Unified SQL LSP - Context Detection Layer
//!
//! This crate provides syntax-level context detection for SQL completion and analysis.
//!
//! ## Overview
//!
//! Context detection analyzes the tree-sitter CST to determine the syntactic context
//! at a given position in SQL code. This information is used by the semantic layer
//! to provide appropriate completion suggestions, hover information, and diagnostics.
//!
//! ## Core Concepts
//!
//! ### Completion Context
//!
//! The [`completion::CompletionContext`] enum represents different SQL contexts where
//! completion can be triggered (SELECT projection, FROM clause, WHERE clause, etc.).
//!
//! ### CST Utilities
//!
//! The [`cst_utils`] module provides utility functions for working with tree-sitter
//! CST nodes, including position conversion and node traversal.
//!
//! ### Keywords
//!
//! The [`keywords`] module provides SQL keyword definitions organized by context
//! and dialect.
//!
//! ## Examples
//!
//! ### Detecting Completion Context
//!
//! ```rust,ignore
//! use unified_sql_lsp_context::completion::detect_completion_context;
//! use tower_lsp::lsp_types::Position;
//!
//! let tree = parser.parse(source, None).unwrap();
//! let ctx = detect_completion_context(
//!     &tree.root_node(),
//!     Position::new(0, 10),
//!     source
//! );
//!
//! match ctx {
//!     CompletionContext::SelectProjection { tables, qualifier } => {
//!         // Provide column completion
//!     }
//!     CompletionContext::FromClause { exclude_tables } => {
//!         // Provide table completion
//!     }
//!     _ => {}
//! }
//! ```

pub mod completion;
pub mod cst_utils;
pub mod keywords;

// Re-export commonly used types
pub use completion::{CompletionContext, detect_completion_context};
pub use cst_utils::{
    Position, Range, byte_to_position, extract_identifier_name, extract_node_text,
    find_node_at_position, node_to_range, position_to_byte_offset,
};
pub use keywords::{KeywordProvider, KeywordSet, SqlKeyword};
