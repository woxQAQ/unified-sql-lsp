//! # LSP Operation Profiling Module
//!
//! This module provides simplified LSP operations for performance profiling.
//!
//! ## Design Assumptions
//!
//! - Operations are **not** full LSP implementations
//! - Focus is on measuring **core logic** not protocol overhead
//! - Context detection is measured but result discarded (acceptable for timing)
//! - MockCatalog used instead of real catalog for reproducibility
//!
//! ## Usage
//!
//! These functions are designed for Criterion benchmarks in `lsp_operations.rs`.

use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_context::{detect_completion_context, Position};
use unified_sql_lsp_semantic::SemanticAnalyzer;
use unified_sql_lsp_test_utils::MockCatalog;
use std::sync::Arc;

pub struct OperationResult {
    pub duration_ns: u128,
    pub output_size: usize,
}

/// Simple document wrapper for profiling
pub struct Document {
    content: String,
    uri: lsp_types::Url,
}

impl Document {
    pub fn new(content: String, uri: lsp_types::Url) -> Self {
        Self { content, uri }
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn uri(&self) -> &lsp_types::Url {
        &self.uri
    }
}

/// Execute completion at the given position
pub fn execute_completion(
    doc: &Document,
    position: Position,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Parse document to get tree
    let language = unified_sql_grammar::language_for_dialect(Dialect::MySQL)
        .ok_or_else(|| "Failed to get language for dialect".to_string())?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let tree = parser
        .parse(doc.content(), None)
        .ok_or_else(|| "Parse returned None".to_string())?;

    // Detect completion context
    let _context = detect_completion_context(
        &tree.root_node(),
        position,
        doc.content(),
    );

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: 0,
    })
}

/// Execute hover at the given position
pub fn execute_hover(
    doc: &Document,
    position: Position,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Parse document
    let language = unified_sql_grammar::language_for_dialect(Dialect::MySQL)
        .ok_or_else(|| "Failed to get language for dialect".to_string())?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let _tree = parser
        .parse(doc.content(), None)
        .ok_or_else(|| "Parse returned None".to_string())?;

    // Build semantic analysis (simplified for profiling)
    let catalog = Arc::new(MockCatalog::default());
    let _analyzer = SemanticAnalyzer::new(catalog, Dialect::MySQL);
    // Full analysis would require lowering CST to IR first

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: 0,
    })
}

/// Execute full diagnostics on document
pub fn execute_diagnostics(
    doc: &Document,
) -> Result<OperationResult, String> {
    let start = std::time::Instant::now();

    // Parse
    let language = unified_sql_grammar::language_for_dialect(Dialect::MySQL)
        .ok_or_else(|| "Failed to get language for dialect".to_string())?;

    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .map_err(|e| format!("Failed to set language: {}", e))?;

    let tree = parser
        .parse(doc.content(), None)
        .ok_or_else(|| "Parse returned None".to_string())?;

    // Count nodes as output size
    let node_count = tree.root_node().descendant_count();

    let duration = start.elapsed().as_nanos();

    Ok(OperationResult {
        duration_ns: duration,
        output_size: node_count as usize,
    })
}

/// Apply simulated document changes
pub fn apply_document_change(
    doc: &mut Document,
    changes: Vec<(usize, usize, String)>, // (start_byte, end_byte, new_text)
) -> Result<(), String> {
    for (start, end, new_text) in changes {
        if start > end || end > doc.content.len() {
            return Err(format!(
                "Invalid range: {}..{} for document of length {}",
                start, end, doc.content.len()
            ));
        }
        let mut content = doc.content.clone();
        content.replace_range(start..end, &new_text);
        doc.content = content;
    }
    Ok(())
}
