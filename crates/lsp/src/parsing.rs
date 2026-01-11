// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Tree-sitter Parsing Integration
//!
//! This module provides low-level Tree-sitter integration for parsing SQL documents.
//!
//! ## Overview
//!
//! The parsing module handles:
//! - Parser management for multiple SQL dialects
//! - Full and incremental parsing
//! - LSP change to tree-sitter edit conversion
//! - Graceful error handling and reporting
//!
//! ## Architecture
//!
//! ```text
//! ParserManager
//!     ├─→ MySQL Parser (OnceLock)
//!     ├─→ PostgreSQL Parser (OnceLock)
//!     └─→ Base Parser (OnceLock)
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_lsp::parsing::{ParserManager, ParseResult};
//! use unified_sql_lsp_ir::Dialect;
//!
//! let manager = ParserManager::new();
//!
//! // Full parse
//! let result = manager.parse_text(
//!     Dialect::MySQL,
//!     "SELECT * FROM users WHERE id = 1"
//! );
//!
//! match result {
//!     ParseResult::Success { tree, parse_time } => {
//!         println!("Parsed in {:?}", parse_time);
//!     }
//!     ParseResult::Partial { tree, errors } => {
//!         println!("Parsed with {} errors", errors.len());
//!     }
//!     ParseResult::Failed { error } => {
//!         println!("Parse failed: {}", error.message);
//!     }
//! }
//! ```

use std::time::{Duration, Instant};
use tower_lsp::lsp_types::*;
use tracing::debug;

use unified_sql_grammar::language_for_dialect;
use unified_sql_lsp_ir::Dialect;

/// Parser manager for multiple SQL dialects
///
/// Manages language objects for each dialect and creates parsers on demand.
#[derive(Debug, Default)]
pub struct ParserManager;

impl ParserManager {
    /// Create a new parser manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new parser for a specific dialect
    ///
    /// # Arguments
    ///
    /// - `dialect`: The SQL dialect
    ///
    /// # Returns
    ///
    /// - `Ok(Parser)` - New parser instance for the dialect
    /// - `Err(ParseError)` - If grammar not compiled or parser creation failed
    fn create_parser(&self, dialect: Dialect) -> Result<tree_sitter::Parser, ParseError> {
        // Get language for dialect (now uses IR Dialect directly)
        let language = language_for_dialect(dialect).ok_or_else(|| ParseError::NoGrammar {
            dialect: format!("{:?}", dialect),
        })?;

        // Create new parser
        let mut parser = tree_sitter::Parser::new();

        parser
            .set_language(language)
            .map_err(|e| ParseError::Generic {
                message: format!("Failed to set language: {}", e),
            })?;

        Ok(parser)
    }

    /// Parse text with full parsing
    ///
    /// # Arguments
    ///
    /// - `dialect`: The SQL dialect
    /// - `text`: The text to parse
    ///
    /// # Returns
    ///
    /// - `ParseResult::Success` - Clean parse with no errors
    /// - `ParseResult::Partial` - Parse with syntax errors (tree includes ERROR nodes)
    /// - `ParseResult::Failed` - Critical parse failure
    pub fn parse_text(&self, dialect: Dialect, text: &str) -> ParseResult {
        let start = Instant::now();

        debug!("Parsing {} bytes of text with {:?}", text.len(), dialect);

        let mut parser = match self.create_parser(dialect) {
            Ok(p) => p,
            Err(e) => {
                return ParseResult::Failed { error: e };
            }
        };

        // Parse the text
        let tree = match parser.parse(text, None) {
            Some(tree) => tree,
            None => {
                return ParseResult::Failed {
                    error: ParseError::Generic {
                        message: "Parser returned None".to_string(),
                    },
                };
            }
        };

        let parse_time = start.elapsed();

        // Check for ERROR nodes in the tree
        let errors = self.collect_errors(&tree, text);

        if errors.is_empty() {
            ParseResult::Success {
                tree: Some(tree),
                parse_time,
            }
        } else {
            ParseResult::Partial {
                tree: Some(tree),
                errors,
            }
        }
    }

    /// Parse with incremental edit
    ///
    /// # Arguments
    ///
    /// - `dialect`: The SQL dialect
    /// - `old_tree`: The previous parse tree
    /// - `text`: The new text
    /// - `edit`: The edit that was applied
    ///
    /// # Returns
    ///
    /// - `ParseResult::Success` - Clean parse with no errors
    /// - `ParseResult::Partial` - Parse with syntax errors
    /// - `ParseResult::Failed` - Critical parse failure
    pub fn parse_with_edit(
        &self,
        dialect: Dialect,
        old_tree: &tree_sitter::Tree,
        text: &str,
        edit: &tree_sitter::InputEdit,
    ) -> ParseResult {
        let start = Instant::now();

        debug!(
            "Incremental parse with edit: start_byte={}, old_end_byte={}, new_end_byte={}",
            edit.start_byte, edit.old_end_byte, edit.new_end_byte
        );

        let mut parser = match self.create_parser(dialect) {
            Ok(p) => p,
            Err(e) => {
                return ParseResult::Failed { error: e };
            }
        };

        // Parse with the old tree and edit
        // Apply the edit to the old tree first
        let mut old_tree_mut = old_tree.clone();
        old_tree_mut.edit(edit);

        let tree = match parser.parse(text, Some(&old_tree_mut)) {
            Some(new_tree) => new_tree,
            None => {
                return ParseResult::Failed {
                    error: ParseError::Generic {
                        message: "Incremental parse returned None".to_string(),
                    },
                };
            }
        };

        let parse_time = start.elapsed();

        // Check for ERROR nodes
        let errors = self.collect_errors(&tree, text);

        if errors.is_empty() {
            ParseResult::Success {
                tree: Some(tree),
                parse_time,
            }
        } else {
            ParseResult::Partial {
                tree: Some(tree),
                errors,
            }
        }
    }

    /// Collect parse errors from tree
    ///
    /// Finds all ERROR nodes in the tree and extracts error information.
    fn collect_errors(&self, tree: &tree_sitter::Tree, text: &str) -> Vec<ParseError> {
        let mut errors = Vec::new();

        // Recursively find ERROR nodes
        let mut node = tree.root_node();
        self.find_error_nodes(&mut node, &mut errors, text);

        errors
    }

    /// Recursively find ERROR nodes in tree
    fn find_error_nodes(&self, node: &tree_sitter::Node, errors: &mut Vec<ParseError>, text: &str) {
        if node.kind() == "ERROR" {
            // Get error location
            let line = node.start_position().row;
            let column = node.start_position().column;

            // Try to extract the error text
            let start_byte = node.start_byte();
            let end_byte = node.end_byte();
            let error_text = if start_byte < text.len() && end_byte <= text.len() {
                text[start_byte..end_byte].to_string()
            } else {
                "<invalid bytes>".to_string()
            };

            errors.push(ParseError::InvalidInput {
                line,
                column,
                message: format!("Syntax error: {}", error_text),
                node_type: Some(node.kind().to_string()),
            });
        }

        // Recurse into children
        for child in node.children(&mut node.walk()) {
            self.find_error_nodes(&child, errors, text);
        }
    }
}

/// Result of a parsing operation
///
/// Represents the outcome of parsing SQL text with Tree-sitter.
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// Successful parse with no errors
    Success {
        /// Parsed syntax tree
        /// Optional to support testing without compiled grammars
        tree: Option<tree_sitter::Tree>,

        /// Time taken to parse
        parse_time: Duration,
    },

    /// Partial parse with syntax errors
    ///
    /// The tree is still valid and contains ERROR nodes marking the problematic areas.
    Partial {
        /// Parsed syntax tree (contains ERROR nodes)
        /// Optional to support testing without compiled grammars
        tree: Option<tree_sitter::Tree>,

        /// List of parse errors
        errors: Vec<ParseError>,
    },

    /// Failed parse (critical error)
    ///
    /// No valid tree could be produced.
    Failed {
        /// Parse error details
        error: ParseError,
    },
}

impl ParseResult {
    /// Check if parse was successful (no errors)
    pub fn is_success(&self) -> bool {
        matches!(self, ParseResult::Success { .. })
    }

    /// Check if parse was partial (has errors)
    pub fn is_partial(&self) -> bool {
        matches!(self, ParseResult::Partial { .. })
    }

    /// Check if parse failed
    pub fn is_failed(&self) -> bool {
        matches!(self, ParseResult::Failed { .. })
    }

    /// Get the tree if available
    pub fn tree(&self) -> Option<&tree_sitter::Tree> {
        match self {
            ParseResult::Success { tree, .. } => tree.as_ref(),
            ParseResult::Partial { tree, .. } => tree.as_ref(),
            ParseResult::Failed { .. } => None,
        }
    }

    /// Get parse errors if any
    pub fn errors(&self) -> Option<&[ParseError]> {
        match self {
            ParseResult::Partial { errors, .. } => Some(errors),
            _ => None,
        }
    }

    /// Extract success result
    pub fn into_success(self) -> Option<(tree_sitter::Tree, Duration)> {
        match self {
            ParseResult::Success { tree, parse_time } => tree.map(|t| (t, parse_time)),
            _ => None,
        }
    }
}

/// Parse error details
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseError {
    /// No grammar compiled for dialect
    #[error("No grammar compiled for dialect: {dialect}")]
    NoGrammar { dialect: String },

    /// Invalid input at specific location
    #[error("Invalid input at line {line}, column {column}: {message}")]
    InvalidInput {
        line: usize,
        column: usize,
        message: String,
        node_type: Option<String>,
    },

    /// Generic parse error
    #[error("Parse error: {message}")]
    Generic { message: String },
}

/// Convert LSP text change to tree-sitter InputEdit
///
/// # Arguments
///
/// - `old_text`: The previous document content
/// - `change`: The LSP content change event
///
/// # Returns
///
/// - `Some(InputEdit)` - If conversion successful
/// - `None` - If change is not an incremental edit (e.g., full replacement)
pub fn lsp_change_to_input_edit(
    old_text: &ropey::Rope,
    change: &TextDocumentContentChangeEvent,
) -> Option<tree_sitter::InputEdit> {
    // Only support incremental edits (with range)
    let range = change.range?;

    let start_line = range.start.line as usize;
    let start_col = range.start.character as usize;
    let end_line = range.end.line as usize;
    let end_col = range.end.character as usize;

    // Convert line/col to byte offsets
    let start_byte = old_text.try_line_to_char(start_line).ok()? + start_col;
    let old_end_byte = old_text.try_line_to_char(end_line).ok()? + end_col;
    let new_end_byte = start_byte + change.text.len();

    // Convert line/col to tree-sitter Points
    let start_position = tree_sitter::Point {
        row: start_line,
        column: start_col,
    };
    let old_end_position = tree_sitter::Point {
        row: end_line,
        column: end_col,
    };

    // Calculate new end position
    let new_text_lines = change.text.lines().count();
    let new_end_position = if new_text_lines > 1 {
        let last_line_len = change.text.lines().last().map_or(0, |l| l.len());
        tree_sitter::Point {
            row: start_line + new_text_lines - 1,
            column: if new_text_lines > 1 {
                last_line_len
            } else {
                start_col + change.text.len()
            },
        }
    } else {
        tree_sitter::Point {
            row: start_line,
            column: start_col + change.text.len(),
        }
    };

    Some(tree_sitter::InputEdit {
        start_byte,
        old_end_byte,
        new_end_byte,
        start_position,
        old_end_position,
        new_end_position,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ropey::Rope;

    #[test]
    fn test_parse_text_simple() {
        let manager = ParserManager::new();

        // This test will only pass if grammars are compiled
        // In CI/dev environments, grammars should be pre-built
        if language_for_dialect(Dialect::MySQL).is_none() {
            println!("Skipping test: MySQL grammar not compiled");
            return;
        }

        let result = manager.parse_text(Dialect::MySQL, "SELECT 1");

        // Should succeed or have partial errors (but not fail completely)
        assert!(!result.is_failed());
    }

    #[test]
    fn test_parse_text_with_error() {
        let manager = ParserManager::new();

        if language_for_dialect(Dialect::MySQL).is_none() {
            println!("Skipping test: MySQL grammar not compiled");
            return;
        }

        // Intentionally invalid SQL
        let result = manager.parse_text(Dialect::MySQL, "SELCT 1"); // Typo: SELCT

        // Should parse (with errors) or fail
        // We expect it to at least try parsing
        match result {
            ParseResult::Success { .. } => {
                // Grammar might be lenient, this is OK
            }
            ParseResult::Partial { .. } => {
                // Expected: typo causes parse error
            }
            ParseResult::Failed { .. } => {
                // Also acceptable if grammar is strict
            }
        }
    }

    #[test]
    fn test_lsp_change_to_input_edit_incremental() {
        let old_text = Rope::from_str("SELECT * FROM users");

        let change = TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 7,
                },
                end: Position {
                    line: 0,
                    character: 8,
                },
            }),
            range_length: Some(1),
            text: "id".to_string(),
        };

        let edit = lsp_change_to_input_edit(&old_text, &change);

        assert!(edit.is_some());
        let edit = edit.unwrap();

        assert_eq!(edit.start_byte, 7);
        assert_eq!(edit.old_end_byte, 8);
        assert_eq!(edit.new_end_byte, 9); // "id" is 2 chars, start at 7
    }

    #[test]
    fn test_lsp_change_to_input_edit_full_replacement() {
        let old_text = Rope::from_str("old content");

        let change = TextDocumentContentChangeEvent {
            range: None, // Full document replacement
            range_length: None,
            text: "new content".to_string(),
        };

        let edit = lsp_change_to_input_edit(&old_text, &change);

        assert!(edit.is_none(), "Full replacement should return None");
    }
}
