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
//! JavaScript → LspServer (WASM exports) → LspCore (shared logic)
//! ```
//!
//! The WASM exports are thin wrappers around the shared `LspCore` logic.

use crate::core::LspCore;
use wasm_bindgen::prelude::*;

/// WebAssembly LSP Server
///
/// This struct provides a JavaScript-accessible interface to the SQL LSP server
/// running in WebAssembly. It delegates all business logic to the shared `LspCore`.
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
    core: LspCore,
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
    pub fn new(_dialect: &str) -> Self {
        Self {
            core: LspCore::new(),
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
    pub fn completion(&self, text: &str, line: u32, col: u32) -> JsValue {
        let items = self.core.completion(text, line, col);
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
    pub fn hover(&self, text: &str, line: u32, col: u32) -> JsValue {
        let hover = self.core.hover(text, line, col);
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
    pub fn diagnostics(&self, text: &str) -> JsValue {
        let diags = self.core.diagnostics(text);
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
        assert!(true);
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
