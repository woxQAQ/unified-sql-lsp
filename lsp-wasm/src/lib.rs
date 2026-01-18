//! WebAssembly bindings for Unified SQL LSP playground
//!
//! This module provides mock LSP functionality for the browser-based playground.

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

// Mock completion item
#[derive(Clone, Serialize, Deserialize)]
struct CompletionItem {
    label: String,
    kind: u32,
    detail: String,
    documentation: String,
    insert_text: String,
}

// Mock LSP server
#[wasm_bindgen]
pub struct LspServer {
    _dialect: String,
}

#[wasm_bindgen]
impl LspServer {
    #[wasm_bindgen(constructor)]
    pub fn new(dialect: &str) -> Self {
        console_error_panic_hook::set_once();
        Self {
            _dialect: dialect.to_string(),
        }
    }

    pub fn completion(&self, text: &str, _line: u32, _col: u32) -> JsValue {
        let mut items = vec![
            CompletionItem {
                label: "SELECT".to_string(),
                kind: 14, // Keyword
                detail: "Keyword".to_string(),
                documentation: "Retrieves data from one or more tables".to_string(),
                insert_text: "SELECT ".to_string(),
            },
            CompletionItem {
                label: "FROM".to_string(),
                kind: 14, // Keyword
                detail: "Keyword".to_string(),
                documentation: "Specifies the table to query from".to_string(),
                insert_text: "FROM ".to_string(),
            },
            CompletionItem {
                label: "WHERE".to_string(),
                kind: 14, // Keyword
                detail: "Keyword".to_string(),
                documentation: "Filters rows based on a condition".to_string(),
                insert_text: "WHERE ".to_string(),
            },
            CompletionItem {
                label: "users".to_string(),
                kind: 5, // Field
                detail: "Table".to_string(),
                documentation: "User accounts table".to_string(),
                insert_text: "users".to_string(),
            },
            CompletionItem {
                label: "orders".to_string(),
                kind: 5, // Field
                detail: "Table".to_string(),
                documentation: "Customer orders table".to_string(),
                insert_text: "orders".to_string(),
            },
            CompletionItem {
                label: "order_items".to_string(),
                kind: 5, // Field
                detail: "Table".to_string(),
                documentation: "Order line items table".to_string(),
                insert_text: "order_items".to_string(),
            },
            CompletionItem {
                label: "id".to_string(),
                kind: 5, // Field
                detail: "INT".to_string(),
                documentation: "Primary key column".to_string(),
                insert_text: "id".to_string(),
            },
            CompletionItem {
                label: "name".to_string(),
                kind: 5, // Field
                detail: "VARCHAR(100)".to_string(),
                documentation: "User name column".to_string(),
                insert_text: "name".to_string(),
            },
            CompletionItem {
                label: "email".to_string(),
                kind: 5, // Field
                detail: "VARCHAR(255)".to_string(),
                documentation: "User email column".to_string(),
                insert_text: "email".to_string(),
            },
            CompletionItem {
                label: "created_at".to_string(),
                kind: 5, // Field
                detail: "TIMESTAMP".to_string(),
                documentation: "Creation timestamp column".to_string(),
                insert_text: "created_at".to_string(),
            },
        ];

        // Context-aware: suggest FROM after SELECT
        if text.contains("SELECT") && !text.contains("FROM") {
            items.push(CompletionItem {
                label: "FROM".to_string(),
                kind: 14,
                detail: "Keyword".to_string(),
                documentation: "Specifies the table to query from".to_string(),
                insert_text: "\nFROM ".to_string(),
            });
        }

        // Convert to JSON for JavaScript
        serde_json::to_string(&items).unwrap().into()
    }

    pub fn hover(&self, _text: &str, _line: u32, _col: u32) -> JsValue {
        let hover_info = serde_json::json!({
            "contents": {
                "kind": "markdown",
                "value": "### SQL Element\n\nHover information for this SQL element."
            }
        });
        hover_info.to_string().into()
    }

    pub fn diagnostics(&self, text: &str) -> JsValue {
        let mut diagnostics = vec![];

        // Simple mock diagnostics
        if text.contains("SELEC") && !text.contains("SELECT") {
            diagnostics.push(serde_json::json!({
                "severity": 1,
                "message": "Did you mean 'SELECT'?",
                "range": {
                    "start": { "line": 0, "character": 0 },
                    "end": { "line": 0, "character": 5 }
                }
            }));
        }

        serde_json::to_string(&diagnostics).unwrap().into()
    }
}
