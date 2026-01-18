// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Shared LSP Core Logic
//!
//! This module contains platform-agnostic business logic for the LSP server.
//! It can compile to both native and WebAssembly targets.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         LSP Core (This Module)          │
//! │  - Completion logic                     │
//! │  - Hover logic                          │
//! │  - Diagnostics logic                    │
//! └──────────────┬──────────────────────────┘
//!                │
//!         ┌──────┴──────┬────────────────┐
//!         ↓             ↓                ↓
//! ┌────────────┐ ┌──────────┐  ┌──────────────┐
//! │   Native   │ │   WASM   │  │    Tests     │
//! │  (tower-   │ │ (wasm-   │  │              │
//! │   lsp)     │ │  bindgen)│  │              │
//! └────────────┘ └──────────┘  └──────────────┘
//! ```
//!
//! ## Purpose
//!
//! The core module separates business logic from transport layer concerns:
//! - **Core**: Pure LSP logic (completion, hover, diagnostics)
//! - **Backend**: tower-lsp protocol handling (native only)
//! - **WASM**: JavaScript bindings (browser only)
//!
//! ## Implementation Status
//!
//! Currently a stub/placeholder. Real LSP logic integration will happen in later tasks:
//! - Task 6: Completion integration
//! - Task 7: Hover integration
//! - Task 8: Diagnostics integration

use serde::{Deserialize, Serialize};

/// Simple completion item for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<u32>,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

/// Simple hover response for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hover {
    pub contents: String,
    pub range: Option<Range>,
}

/// Simple diagnostic for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: Option<u32>,
    pub message: String,
}

/// Simple position/range for WASM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Shared LSP core logic
///
/// This struct contains the platform-agnostic business logic for the LSP server.
/// It can be used from both native (tower-lsp) and WASM (wasm-bindgen) contexts.
///
/// # Examples
///
/// ## Native Usage
///
/// ```rust,ignore
/// use unified_sql_lsp_lsp::core::LspCore;
///
/// let core = LspCore::new();
/// let items = core.completion("SELECT * FROM users", 0, 20);
/// ```
///
/// ## WASM Usage
///
/// ```javascript
/// const server = new LspServer("mysql");
/// const completions = server.completion("SELECT * FROM users", 0, 20);
/// ```
pub struct LspCore {
    // TODO: Add fields for catalog, semantic analyzer, etc.
    // Future fields will include:
    // - parser: ParserManager
    // - catalog: Catalog (or LiveCatalog)
    // - semantic: SemanticAnalyzer
    // - completion: CompletionEngine
}

impl LspCore {
    /// Create a new LSP core instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use unified_sql_lsp_lsp::core::LspCore;
    ///
    /// let core = LspCore::new();
    /// ```
    pub fn new() -> Self {
        Self {}
    }

    /// Get completion items for a given position
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    /// * `line` - The zero-based line number
    /// * `col` - The zero-based column number (UTF-16 code units)
    ///
    /// # Returns
    ///
    /// A vector of completion items
    ///
    /// # Examples
    ///
    /// ```rust
    /// use unified_sql_lsp_lsp::core::LspCore;
    ///
    /// let core = LspCore::new();
    /// let items = core.completion("SELECT * FROM users", 0, 20);
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Mock implementation for playground testing. Returns basic SQL keywords,
    /// table names, and column names with context-aware suggestions.
    pub fn completion(&self, text: &str, _line: u32, _col: u32) -> Vec<CompletionItem> {
        let mut items = vec![
            CompletionItem {
                label: "SELECT".into(),
                kind: Some(14), // Keyword
                detail: Some("Keyword".into()),
                documentation: None,
            },
            CompletionItem {
                label: "FROM".into(),
                kind: Some(14), // Keyword
                detail: Some("Keyword".into()),
                documentation: None,
            },
            CompletionItem {
                label: "WHERE".into(),
                kind: Some(14), // Keyword
                detail: Some("Keyword".into()),
                documentation: None,
            },
            CompletionItem {
                label: "users".into(),
                kind: Some(5), // Class
                detail: Some("Table".into()),
                documentation: None,
            },
            CompletionItem {
                label: "id".into(),
                kind: Some(5), // Field
                detail: Some("Column (INT)".into()),
                documentation: None,
            },
            CompletionItem {
                label: "name".into(),
                kind: Some(5), // Field
                detail: Some("Column (VARCHAR)".into()),
                documentation: None,
            },
        ];

        // Context-aware: suggest FROM after SELECT
        if text.contains("SELECT") && !text.contains("FROM") {
            items.push(CompletionItem {
                label: " FROM ".into(),
                kind: Some(14), // Keyword
                detail: Some("Keyword".into()),
                documentation: None,
            });
        }

        items
    }

    /// Get hover information for a given position
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    /// * `line` - The zero-based line number
    /// * `col` - The zero-based column number (UTF-16 code units)
    ///
    /// # Returns
    ///
    /// Hover information if available, None otherwise
    ///
    /// # Examples
    ///
    /// ```rust
    /// use unified_sql_lsp_lsp::core::LspCore;
    ///
    /// let core = LspCore::new();
    /// let hover = core.hover("SELECT * FROM users", 0, 10);
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Currently returns None. Real implementation in Task 7.
    pub fn hover(&self, _text: &str, _line: u32, _col: u32) -> Option<Hover> {
        // TODO: Implement hover logic (Task 7)
        // Future implementation:
        // 1. Parse text with tree-sitter
        // 2. Find node at position
        // 3. Determine symbol type (table, column, function, etc.)
        // 4. Query catalog for type information
        // 5. Build hover content
        None
    }

    /// Get diagnostics for a document
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    ///
    /// # Returns
    ///
    /// A vector of diagnostics
    ///
    /// # Examples
    ///
    /// ```rust
    /// use unified_sql_lsp_lsp::core::LspCore;
    ///
    /// let core = LspCore::new();
    /// let diags = core.diagnostics("SELECT * FROM nonexistent");
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Currently returns empty vector. Real implementation in Task 8.
    pub fn diagnostics(&self, _text: &str) -> Vec<Diagnostic> {
        // TODO: Implement diagnostic logic (Task 8)
        // Future implementation:
        // 1. Parse text with tree-sitter
        // 2. Check for syntax errors
        // 3. Run semantic analysis (type checking, etc.)
        // 4. Build diagnostics with severity and range
        vec![]
    }
}

impl Default for LspCore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_creation() {
        let core = LspCore::new();
        // Just test that it creates successfully
        assert!(true);
    }

    #[test]
    fn test_completion_mock() {
        let core = LspCore::new();
        let items = core.completion("SELECT * FROM users", 0, 20);
        // Should return mock completion items
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label == "SELECT"));
        assert!(items.iter().any(|i| i.label == "FROM"));
        assert!(items.iter().any(|i| i.label == "users"));
    }

    #[test]
    fn test_hover_stub() {
        let core = LspCore::new();
        let hover = core.hover("SELECT * FROM users", 0, 10);
        assert!(hover.is_none());
    }

    #[test]
    fn test_diagnostics_stub() {
        let core = LspCore::new();
        let diags = core.diagnostics("SELECT * FROM nonexistent");
        assert!(diags.is_empty());
    }
}
