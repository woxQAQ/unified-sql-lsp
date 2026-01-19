// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # TCP/WebSocket Transport for LSP Server
//!
//! This module provides TCP transport with WebSocket support for the LSP server,
//! enabling browser-based clients like the playground to connect.
//!
//! ## Architecture
//!
//! ```text
//! Browser (Monaco Editor)
//!   ↓ WebSocket
//! TCP Server (tokio-tungstenite)
//!   ↓ JSON-RPC messages
//! LSP Backend (existing)
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use unified_sql_lsp_lsp::tcp::TcpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = TcpServer::new(4137).await.unwrap();
//!     server.serve().await.unwrap();
//! }
//! ```

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};

use crate::completion::CompletionEngine;
use crate::document::{Document, DocumentStore, DocumentMetadata};
use crate::config::EngineConfig;
use crate::diagnostic::{DiagnosticCollector, publish_diagnostics_for_document};
use crate::parsing::ParseResult;
use unified_sql_lsp_catalog::{Catalog, StaticCatalog};
use unified_sql_lsp_ir::Dialect;
use tower_lsp::lsp_types::{CompletionItem, CompletionParams, Diagnostic, DidOpenTextDocumentParams,
                                DidChangeTextDocumentParams, Position, TextDocumentContentChangeEvent,
                                TextDocumentIdentifier, Url};

/// JSON-RPC request
#[derive(Debug, Clone, Deserialize, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<JsonValue>,
    #[serde(flatten)]
    data: JsonRpcRequestData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum JsonRpcRequestData {
    Request { method: String, params: Option<JsonValue> },
    Notification { method: String, params: Option<JsonValue> },
}

/// JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<JsonValue>,
}

/// Per-client session state
struct ClientSession {
    documents: DocumentStore,
    completion_engine: Arc<CompletionEngine>,
    diagnostic_collector: DiagnosticCollector,
}

impl ClientSession {
    fn new(catalog: Arc<dyn Catalog>) -> Self {
        let completion_engine = Arc::new(CompletionEngine::new(catalog));
        let diagnostic_collector = DiagnosticCollector::new();

        Self {
            documents: DocumentStore::new(),
            completion_engine,
            diagnostic_collector,
        }
    }

    async fn handle_did_open(&mut self, params: DidOpenTextDocumentParams) {
        let uri = &params.text_document.uri;
        let text = &params.text_document.text;
        let language_id = &params.text_document.language_id;

        debug!("did_open: uri={}, language={}", uri, language_id);

        // Create document
        let document = Document::new(uri.clone(), text.clone(), language_id);

        // Parse document
        let source = document.get_content();
        let dialect = Dialect::MySQL; // Default for playground

        match crate::parsing::ParserManager::parse(&source, dialect) {
            ParseResult::Success { tree, .. } => {
                let metadata = DocumentMetadata::new(0, dialect, false, 0);
                if let Err(e) = self.documents.update_document_tree(uri, tree, metadata).await {
                    error!("Failed to update document tree: {}", e);
                }
            }
            ParseResult::Partial { tree, errors } => {
                let metadata = DocumentMetadata::new(0, dialect, true, errors.len());
                if let Err(e) = self.documents.update_document_tree(uri, tree, metadata).await {
                    error!("Failed to update document tree: {}", e);
                }
            }
            ParseResult::Failed { .. } => {
                // Document couldn't be parsed, but we still store it
                warn!("Failed to parse document, but continuing");
            }
        }
    }

    async fn handle_did_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = &params.text_document.uri;
        let changes = &params.content_changes;

        debug!("did_change: uri={}, changes={}", uri, changes.len());

        // Get current document
        let current_doc = match self.documents.get_document(uri).await {
            Some(doc) => doc,
            None => {
                error!("Document not found: {}", uri);
                return;
            }
        };

        // Apply changes
        let mut new_text = current_doc.get_content().to_string();
        for change in changes {
            let range = change.range.expect("full sync not supported");
            new_text.replace_range(range.start..range.end, &change.text);
        }

        // Re-parse document
        let source = new_text.as_str();
        let dialect = Dialect::MySQL;

        match crate::parsing::ParserManager::parse(&source, dialect) {
            ParseResult::Success { tree, .. } => {
                let metadata = DocumentMetadata::new(0, dialect, false, 0);
                if let Err(e) = self.documents.update_document_tree(uri, tree, metadata).await {
                    error!("Failed to update document tree: {}", e);
                }

                // Publish diagnostics
                if let Some(doc) = self.documents.get_document(uri).await {
                    if let Some(tree) = doc.tree() {
                        publish_diagnostics_for_document(
                            &self.diagnostic_collector,
                            &MockClient, // TODO: Don't publish diagnostics in TCP mode
                            uri.clone(),
                            &tree,
                            &source,
                        ).await;
                    }
                }
            }
            ParseResult::Partial { tree, errors } => {
                let metadata = DocumentMetadata::new(0, dialect, true, errors.len());
                if let Err(e) = self.documents.update_document_tree(uri, tree, metadata).await {
                    error!("Failed to update document tree: {}", e);
                }
            }
            ParseResult::Failed { .. } => {
                warn!("Failed to re-parse document after change");
            }
        }
    }

    async fn complete(&self, uri: &Url, position: Position) -> Option<Vec<CompletionItem>> {
        let document = self.documents.get_document(uri).await?;
        self.completion_engine.complete(document, position).await.ok()?
    }
}

/// TCP/WebSocket server for LSP
pub struct TcpServer {
    listener: TcpListener,
    port: u16,
    catalog: Arc<dyn Catalog>,
    sessions: HashMap<String, ClientSession>,
}

impl TcpServer {
    /// Create a new TCP server listening on the specified port
    pub async fn new(port: u16, catalog: Arc<dyn Catalog>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("TCP LSP server listening on port {}", port);

        Ok(Self {
            listener,
            port,
            catalog,
            sessions: HashMap::new(),
        })
    }

    /// Get the actual port the server is listening on
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Accept and handle incoming connections
    pub async fn serve(&self) -> std::io::Result<()> {
        info!("TCP LSP server ready to accept connections");

        loop {
            match self.listener.accept().await {
                Ok((stream, addr)) => {
                    info!("New connection from {}", addr);

                    // Create a new session for this connection
                    let session = ClientSession::new(self.catalog.clone());

                    // Spawn a task to handle this connection
                    let handle = tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, session).await {
                            error!("Error handling connection: {}", e);
                        }
                    });

                    // Detach the task so it runs in the background
                    handle.abort();
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

/// Mock client for diagnostics (we don't publish in TCP mode)
struct MockClient;

impl MockClient {
    async fn log_message(&self, _message: &str) {
        // Diagnostics go to console via logging
    }
}

/// Get or create a session ID from connection info
fn get_session_id(_addr: std::net::SocketAddr) -> String {
    // For now, use a single global session
    // In production, would track per-client sessions
    "global".to_string()
}

/// Handle a single WebSocket connection
async fn handle_connection(stream: tokio::net::TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    // Perform WebSocket handshake
    let ws_stream = tokio_tungstenite::accept_async(stream).await?;
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    info!("WebSocket connection established");

    // Handle incoming messages
    while let Some(msg_result) = ws_receiver.next().await {
        match msg_result {
            Ok(msg) => {
                if msg.is_text() || msg.is_binary() {
                    let text = msg.to_text()?;

                    debug!("Received message: {}", text);

                    // Parse JSON-RPC request
                    let response = match handle_lsp_message(text).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            error!("Error handling LSP message: {}", e);
                            JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: JsonValue::Null,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32603,
                                    message: format!("Internal error: {}", e),
                                    data: None,
                                }),
                            }
                        }
                    };

                    // Send response (skip for notifications with null id)
                    if response.id != JsonValue::Null {
                        let response_text = serde_json::to_string(&response)?;
                        if let Err(e) = ws_sender.send(Message::Text(response_text)).await {
                            error!("Error sending response: {}", e);
                            break;
                        }
                    }
                } else if msg.is_close() {
                    info!("Client requested close");
                    break;
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                break;
            }
        }
    }

    info!("WebSocket connection closed");
    Ok(())
}

/// Handle a single LSP message
async fn handle_lsp_message(
    message: &str,
) -> Result<JsonRpcResponse, Box<dyn std::error::Error>> {
    let request: JsonRpcRequest = serde_json::from_str(message)?;

    let id = request.id.clone().unwrap_or(JsonValue::Null);

    match request.data {
        JsonRpcRequestData::Request { method, params } => {
            debug!("LSP request: {} with params: {:?}", method, params);

            // For now, return mock responses
            // TODO: Integrate with actual backend methods in Phase 3
            let result = match method.as_str() {
                "initialize" => {
                    serde_json::json!({
                        "capabilities": {
                            "textDocumentSync": 1,
                            "completionProvider": {
                                "triggerCharacters": [".", " "]
                            },
                            "hoverProvider": true,
                            "diagnosticProvider": true
                        },
                        "serverInfo": {
                            "name": "unified-sql-lsp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    })
                }
                "initialized" => {
                    // Client confirmation of initialization
                    serde_json::json!({})
                }
                "shutdown" => {
                    serde_json::json!({})
                }
                "textDocument/completion" => {
                    // TODO: Call actual backend completion
                    serde_json::json!([])
                }
                "textDocument/hover" => {
                    // TODO: Call actual backend hover
                    serde_json::json!(null)
                }
                "textDocument/diagnostic" => {
                    // TODO: Call actual backend diagnostics
                    serde_json::json!([])
                }
                _ => {
                    warn!("Unknown method: {}", method);
                    return Ok(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError {
                            code: -32601,
                            message: format!("Method not found: {}", method),
                            data: None,
                        }),
                    });
                }
            };

            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            })
        }
        JsonRpcRequestData::Notification { method, params } => {
            debug!("LSP notification: {} with params: {:?}", method, params);

            // Handle notifications (no response expected)
            match method.as_str() {
                "textDocument/didOpen" => {
                    // TODO: Call backend did_open
                    debug!("textDocument/didOpen notification received");
                }
                "textDocument/didChange" => {
                    // TODO: Call backend did_change
                    debug!("textDocument/didChange notification received");
                }
                "textDocument/didClose" => {
                    // TODO: Call backend did_close
                    debug!("textDocument/didClose notification received");
                }
                "exit" => {
                    debug!("Received exit notification");
                }
                _ => {
                    warn!("Unknown notification: {}", method);
                }
            }

            // Notifications don't get responses
            // But we return Ok anyway for error handling
            Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: JsonValue::Null,
                result: None,
                error: None,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id, Some(JsonValue::Number(1.into())));

        match request.data {
            JsonRpcRequestData::Request { method, .. } => {
                assert_eq!(method, "initialize");
            }
            _ => panic!("Expected request"),
        }
    }
}
