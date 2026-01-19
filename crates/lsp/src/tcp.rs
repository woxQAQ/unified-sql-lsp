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
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::protocol::Message;
use tracing::{debug, error, info, warn};

use unified_sql_lsp_catalog::Catalog;

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

/// TCP/WebSocket server for LSP
pub struct TcpServer {
    listener: TcpListener,
    port: u16,
    _catalog: Arc<dyn Catalog>,
}

impl TcpServer {
    /// Create a new TCP server listening on the specified port
    pub async fn new(port: u16, catalog: Arc<dyn Catalog>) -> std::io::Result<Self> {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
        info!("TCP LSP server listening on port {}", port);

        Ok(Self {
            listener,
            port,
            _catalog: catalog,
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

                    // Spawn a task to handle this connection
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream).await {
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
        JsonRpcRequestData::Request { method, params: _params } => {
            debug!("LSP request: {}", method);

            // Return mock responses for now
            // TODO: Integrate with actual backend methods in future phase
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
        JsonRpcRequestData::Notification { method, params: _params } => {
            debug!("LSP notification: {}", method);

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
