// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # WASM exports for SQL LSP
//!
//! This module exports the LSP functionality to JavaScript via wasm-bindgen.

use wasm_bindgen::prelude::*;

/// WebAssembly LSP Server
///
/// This struct provides a JavaScript-accessible interface to the SQL LSP server
/// running in WebAssembly.
#[wasm_bindgen]
pub struct LspServer {
    // TODO: Add backend field in next task
}

#[wasm_bindgen]
impl LspServer {
    /// Create a new LSP server instance
    ///
    /// # Arguments
    ///
    /// * `dialect` - The SQL dialect to use (e.g., "mysql", "postgresql")
    ///
    /// # Example
    ///
    /// ```javascript
    /// const server = new LspServer("mysql");
    /// ```
    #[wasm_bindgen(constructor)]
    pub fn new(_dialect: &str) -> Self {
        // TODO: Initialize backend in next task
        Self {}
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
    /// # Example
    ///
    /// ```javascript
    /// const completions = server.completion("SELECT * FROM ", 0, 20);
    /// const items = JSON.parse(completions);
    /// ```
    #[wasm_bindgen]
    pub fn completion(&self, _text: &str, _line: u32, _col: u32) -> JsValue {
        // TODO: Implement in next task
        JsValue::from_str("[]")
    }
}
