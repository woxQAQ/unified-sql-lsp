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
//! LSP Backend (document store, completion engine, catalog)
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
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};

use crate::catalog_manager::CatalogManager;
use crate::completion::CompletionEngine;
use crate::config::EngineConfig;
use crate::document::{DocumentStore, ParseMetadata};
use crate::parsing::{ParseResult, ParserManager};
use unified_sql_lsp_catalog::Catalog;
use unified_sql_lsp_ir::Dialect;
use tower_lsp::lsp_types::*;
use tower_lsp::jsonrpc::Result as JsonRpcResult;

/// JSON-RPC request
#[derive(Debug, Clone, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    #[serde(flatten)]
    data: JsonRpcRequestData,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum JsonRpcRequestData {
    Request {
        id: JsonValue,
        method: String,
        params: Option<JsonValue>,
    },
    Notification {
        method: String,
        params: Option<JsonValue>,
    },
}

impl JsonRpcRequest {
    /// Get the request ID if this is a request (not a notification)
    fn id(&self) -> Option<JsonValue> {
        match &self.data {
            JsonRpcRequestData::Request { id, .. } => Some(id.clone()),
            JsonRpcRequestData::Notification { .. } => None,
        }
    }

    /// Get the method name
    fn method(&self) -> &str {
        match &self.data {
            JsonRpcRequestData::Request { method, .. } => method,
            JsonRpcRequestData::Notification { method, .. } => method,
        }
    }

    /// Get the parameters
    fn params(&self) -> Option<&JsonValue> {
        match &self.data {
            JsonRpcRequestData::Request { params, .. } => params.as_ref(),
            JsonRpcRequestData::Notification { params, .. } => params.as_ref(),
        }
    }

    /// Check if this is a notification
    fn is_notification(&self) -> bool {
        matches!(self.data, JsonRpcRequestData::Notification { .. })
    }
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
    documents: Arc<DocumentStore>,
    catalog_manager: Arc<tokio::sync::RwLock<CatalogManager>>,
    config: Arc<tokio::sync::RwLock<Option<EngineConfig>>>,
    catalog: Arc<dyn Catalog>,
}

impl ClientSession {
    fn new(catalog: Arc<dyn Catalog>) -> Self {
        Self {
            documents: Arc::new(DocumentStore::new()),
            catalog_manager: Arc::new(tokio::sync::RwLock::new(CatalogManager::new())),
            config: Arc::new(tokio::sync::RwLock::new(None)),
            catalog,
        }
    }

    async fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text;
        let language_id = params.text_document.language_id;

        debug!("did_open: uri={}, language={}", uri, language_id);

        // Open document in store
        if let Err(e) = self.documents.open_document(uri.clone(), text, 0, language_id).await {
            error!("Failed to open document: {}", e);
            return;
        }

        // Parse document
        if let Some(document) = self.documents.get_document(&uri).await {
            let source = document.get_content();
            let dialect = Dialect::MySQL; // Default for playground

            let parse_result = ParserManager::parse_text(&ParserManager::new(), dialect, &source);

            match parse_result {
                ParseResult::Success { tree, parse_time } => {
                    let metadata = ParseMetadata {
                        parsed_at: std::time::SystemTime::now(),
                        parse_time_ms: parse_time.as_millis() as u64,
                        dialect,
                        has_errors: false,
                        error_count: 0,
                    };
                    if let Some(tree) = tree {
                        if let Err(e) = self.documents.update_document_tree(&uri, tree, metadata).await {
                            error!("Failed to update document tree: {}", e);
                        }
                    }
                }
                ParseResult::Partial { tree, errors } => {
                    let metadata = ParseMetadata {
                        parsed_at: std::time::SystemTime::now(),
                        parse_time_ms: 0,
                        dialect,
                        has_errors: true,
                        error_count: errors.len(),
                    };
                    if let Some(tree) = tree {
                        if let Err(e) = self.documents.update_document_tree(&uri, tree, metadata).await {
                            error!("Failed to update document tree: {}", e);
                        }
                    }
                }
                ParseResult::Failed { .. } => {
                    warn!("Failed to parse document, but continuing");
                }
            }
        }
    }

    async fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;

        debug!("did_change: uri={}, changes={}", uri, params.content_changes.len());

        // Update document content
        let identifier = VersionedTextDocumentIdentifier { uri: uri.clone(), version };
        if let Err(e) = self.documents.update_document(&identifier, &params.content_changes).await {
            error!("Failed to update document: {}", e);
            return;
        }

        // Re-parse document
        if let Some(document) = self.documents.get_document(&uri).await {
            let source = document.get_content();
            let dialect = Dialect::MySQL;

            let parse_result = ParserManager::parse_text(&ParserManager::new(), dialect, &source);

            match parse_result {
                ParseResult::Success { tree, parse_time } => {
                    let metadata = ParseMetadata {
                        parsed_at: std::time::SystemTime::now(),
                        parse_time_ms: parse_time.as_millis() as u64,
                        dialect,
                        has_errors: false,
                        error_count: 0,
                    };
                    if let Some(tree) = tree {
                        if let Err(e) = self.documents.update_document_tree(&uri, tree, metadata).await {
                            error!("Failed to update document tree: {}", e);
                        }
                    }
                }
                ParseResult::Partial { tree, errors } => {
                    let metadata = ParseMetadata {
                        parsed_at: std::time::SystemTime::now(),
                        parse_time_ms: 0,
                        dialect,
                        has_errors: true,
                        error_count: errors.len(),
                    };
                    if let Some(tree) = tree {
                        if let Err(e) = self.documents.update_document_tree(&uri, tree, metadata).await {
                            error!("Failed to update document tree: {}", e);
                        }
                    }
                }
                ParseResult::Failed { .. } => {
                    warn!("Failed to re-parse document after change");
                }
            }
        }
    }

    async fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        debug!("did_close: uri={}", uri);

        // Close document
        self.documents.close_document(&uri).await;
    }

    async fn complete(&self, params: CompletionParams, catalog: Arc<dyn Catalog>) -> JsonRpcResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // Create completion engine
        let engine = CompletionEngine::new(catalog);

        // Execute completion
        match engine.complete(&document, position).await {
            Ok(Some(items)) => Ok(Some(CompletionResponse::Array(items))),
            Ok(None) => Ok(None),
            Err(e) => {
                error!("Completion error: {}", e);
                Ok(None)
            }
        }
    }

    async fn hover(&self, params: HoverParams, catalog: Arc<dyn Catalog>) -> JsonRpcResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => return Ok(None),
        };

        // For now, return no hover info
        // TODO: Implement actual hover
        let _ = (document, position, catalog);
        Ok(None)
    }
}

/// TCP/WebSocket server for LSP
pub struct TcpServer {
    listener: TcpListener,
    port: u16,
    catalog: Arc<dyn Catalog>,
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
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, session).await {
                            error!("Error handling connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single WebSocket connection
async fn handle_connection(
    stream: tokio::net::TcpStream,
    session: ClientSession,
) -> Result<(), Box<dyn std::error::Error>> {
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
                    let response = match handle_lsp_message(text, &session).await {
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
    session: &ClientSession,
) -> Result<JsonRpcResponse, Box<dyn std::error::Error>> {
    let request: JsonRpcRequest = serde_json::from_str(message)?;

    let id = request.id().unwrap_or(JsonValue::Null);
    let method = request.method().to_string();
    let params = request.params().cloned();

    if request.is_notification() {
        debug!("LSP notification: {}", method);

        // Handle notifications (no response expected)
        match method.as_str() {
            "textDocument/didOpen" => {
                let params_value = params.unwrap_or(JsonValue::Null);
                let parsed: DidOpenTextDocumentParams = serde_json::from_value(params_value)?;
                session.handle_did_open(parsed).await;
            }
            "textDocument/didChange" => {
                let params_value = params.unwrap_or(JsonValue::Null);
                let parsed: DidChangeTextDocumentParams = serde_json::from_value(params_value)?;
                session.handle_did_change(parsed).await;
            }
            "textDocument/didClose" => {
                let params_value = params.unwrap_or(JsonValue::Null);
                let parsed: DidCloseTextDocumentParams = serde_json::from_value(params_value)?;
                session.handle_did_close(parsed).await;
            }
            "exit" => {
                debug!("Received exit notification");
            }
            _ => {
                warn!("Unknown notification: {}", method);
            }
        }

        // Notifications don't get responses
        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: JsonValue::Null,
            result: None,
            error: None,
        })
    } else {
        debug!("LSP request: {}", method);

        let catalog = session.catalog.clone();

        // Call actual backend methods
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
                serde_json::json!({})
            }
            "shutdown" => {
                serde_json::json!({})
            }
            "textDocument/completion" => {
                let params_value = params.unwrap_or(JsonValue::Null);
                let parsed: CompletionParams = serde_json::from_value(params_value)?;

                match session.complete(parsed, catalog).await {
                    Ok(Some(response)) => {
                        match response {
                            CompletionResponse::Array(items) => serde_json::json!(items),
                            CompletionResponse::List(list) => serde_json::json!(list),
                        }
                    }
                    Ok(None) => serde_json::json!(null),
                    Err(e) => {
                        error!("Completion error: {}", e);
                        serde_json::json!([])
                    }
                }
            }
            "textDocument/hover" => {
                let params_value = params.unwrap_or(JsonValue::Null);
                let parsed: HoverParams = serde_json::from_value(params_value)?;

                match session.hover(parsed, catalog).await {
                    Ok(Some(hover)) => serde_json::json!(hover),
                    Ok(None) => serde_json::json!(null),
                    Err(e) => {
                        error!("Hover error: {}", e);
                        serde_json::json!(null)
                    }
                }
            }
            "textDocument/diagnostic" => {
                // TODO: Implement diagnostics
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_parsing() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id(), Some(JsonValue::Number(1.into())));
        assert_eq!(request.method(), "initialize");
        assert!(!request.is_notification());
    }

    #[test]
    fn test_json_rpc_notification_parsing() {
        let json = r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{}}"#;
        let request: JsonRpcRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.id(), None);
        assert_eq!(request.method(), "textDocument/didOpen");
        assert!(request.is_notification());
    }
}
