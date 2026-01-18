// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # WASM exports for SQL LSP
//!
//! This module exports the LSP functionality to JavaScript via wasm-bindgen.
//!
//! ## Architecture
//!
//! ```text
//! JavaScript → LspServer (WASM exports) → Core logic
//! ```
//!
//! The WASM exports provide a JavaScript-accessible interface to the SQL LSP.

use wasm_bindgen::prelude::*;

/// Placeholder completion item
#[derive(serde::Serialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: u32,
}

/// Placeholder hover info
#[derive(serde::Serialize)]
pub struct HoverInfo {
    pub contents: String,
}

/// Placeholder diagnostic
#[derive(serde::Serialize)]
pub struct Diagnostic {
    pub message: String,
    pub line: u32,
    pub col: u32,
}

/// WebAssembly LSP Server
///
/// This struct provides a JavaScript-accessible interface to the SQL LSP server
/// running in WebAssembly.
///
/// # Examples
///
/// ```javascript
/// // Create a new LSP server instance
/// const server = new LspServer("mysql");
///
/// // Get completions
/// const completions = server.completion("SELECT * FROM users", 0, 20);
/// const items = JSON.parse(completions);
///
/// // Get hover information
/// const hover = server.hover("SELECT * FROM users", 0, 10);
/// const hoverInfo = JSON.parse(hover);
///
/// // Get diagnostics
/// const diagnostics = server.diagnostics("SELECT * FROM nonexistent");
/// const diags = JSON.parse(diagnostics);
/// ```
#[wasm_bindgen]
pub struct LspServer {
    _dialect: String,
}

#[wasm_bindgen]
impl LspServer {
    /// Create a new LSP server instance
    ///
    /// # Arguments
    ///
    /// * `dialect` - The SQL dialect to use (e.g., "mysql", "postgresql")
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const server = new LspServer("mysql");
    /// ```
    ///
    /// # Note
    ///
    /// The dialect parameter is currently unused but will be used in future tasks
    /// to configure the parser and catalog appropriately.
    #[wasm_bindgen(constructor)]
    pub fn new(dialect: &str) -> Self {
        Self {
            _dialect: dialect.to_string(),
        }
    }

    /// Get completion items for a given position in the SQL text
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    /// * `line` - The zero-based line number
    /// * `col` - The zero-based column number
    ///
    /// # Returns
    ///
    /// A JSON string representing an array of completion items
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const completions = server.completion("SELECT * FROM users", 0, 20);
    /// const items = JSON.parse(completions);
    /// console.log(items); // Array of CompletionItem objects
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Currently returns empty array. Real implementation in Task 6.
    #[wasm_bindgen]
    pub fn completion(&self, _text: &str, _line: u32, _col: u32) -> JsValue {
        let items: Vec<CompletionItem> = vec![];
        serde_json::to_string(&items)
            .expect("Failed to serialize completion items")
            .into()
    }

    /// Get hover information for a given position in the SQL text
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    /// * `line` - The zero-based line number
    /// * `col` - The zero-based column number
    ///
    /// # Returns
    ///
    /// A JSON string representing hover information, or "null" if no hover available
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const hover = server.hover("SELECT * FROM users WHERE id = 1", 0, 25);
    /// const hoverInfo = JSON.parse(hover);
    /// if (hoverInfo) {
    ///     console.log(hoverInfo.contents); // Hover content
    /// }
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Currently returns null. Real implementation in Task 7.
    #[wasm_bindgen]
    pub fn hover(&self, _text: &str, _line: u32, _col: u32) -> JsValue {
        let hover: Option<HoverInfo> = None;
        serde_json::to_string(&hover)
            .expect("Failed to serialize hover info")
            .into()
    }

    /// Get diagnostics for a SQL document
    ///
    /// # Arguments
    ///
    /// * `text` - The full SQL text
    ///
    /// # Returns
    ///
    /// A JSON string representing an array of diagnostics
    ///
    /// # Examples
    ///
    /// ```javascript
    /// const diagnostics = server.diagnostics("SELECT * FROM nonexistent");
    /// const diags = JSON.parse(diagnostics);
    /// console.log(diags); // Array of Diagnostic objects
    /// ```
    ///
    /// # Implementation Status
    ///
    /// Currently returns empty array. Real implementation in Task 8.
    #[wasm_bindgen]
    pub fn diagnostics(&self, _text: &str) -> JsValue {
        let diags: Vec<Diagnostic> = vec![];
        serde_json::to_string(&diags)
            .expect("Failed to serialize diagnostics")
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_creation() {
        let server = LspServer::new("mysql");
        // Just test that it creates successfully
        assert_eq!(server._dialect, "mysql");
    }

    #[test]
    fn test_completion_returns_json() {
        let server = LspServer::new("mysql");
        let result = server.completion("SELECT * FROM users", 0, 20);
        let result_str = result.as_string().unwrap();
        assert_eq!(result_str, "[]");
    }

    #[test]
    fn test_hover_returns_json() {
        let server = LspServer::new("mysql");
        let result = server.hover("SELECT * FROM users", 0, 10);
        let result_str = result.as_string().unwrap();
        assert_eq!(result_str, "null");
    }

    #[test]
    fn test_diagnostics_returns_json() {
        let server = LspServer::new("mysql");
        let result = server.diagnostics("SELECT * FROM nonexistent");
        let result_str = result.as_string().unwrap();
        assert_eq!(result_str, "[]");
    }
}
