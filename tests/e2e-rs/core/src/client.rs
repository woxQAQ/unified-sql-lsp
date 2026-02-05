// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Mock LSP client for testing
//!
//! Captures server notifications for assertions.

use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;

use crate::debug_log;

/// Mock LSP client that captures responses
#[derive(Clone)]
pub struct MockClient {
    /// Captured diagnostics by document URI
    diagnostics: Arc<RwLock<std::collections::HashMap<Url, Vec<Diagnostic>>>>,

    /// Captured log messages
    log_messages: Arc<RwLock<Vec<(MessageType, String)>>>,

    /// Captured show messages
    show_messages: Arc<RwLock<Vec<(MessageType, String)>>>,
}

impl MockClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            diagnostics: Arc::new(RwLock::new(std::collections::HashMap::new())),
            log_messages: Arc::new(RwLock::new(Vec::new())),
            show_messages: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get captured diagnostics for a document
    pub async fn get_diagnostics(&self, uri: &Url) -> Option<Vec<Diagnostic>> {
        let diagnostics = self.diagnostics.read().await;
        diagnostics.get(uri).cloned()
    }

    /// Get all captured log messages
    pub async fn get_log_messages(&self) -> Vec<(MessageType, String)> {
        let messages = self.log_messages.read().await;
        messages.clone()
    }

    /// Get all captured show messages
    pub async fn get_show_messages(&self) -> Vec<(MessageType, String)> {
        let messages = self.show_messages.read().await;
        messages.clone()
    }

    /// Clear all captured data
    pub async fn clear(&self) {
        self.diagnostics.write().await.clear();
        self.log_messages.write().await.clear();
        self.show_messages.write().await.clear();
    }

    /// Internal method to record diagnostics (called by notification handler)
    pub(crate) async fn record_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        let mut map = self.diagnostics.write().await;
        map.insert(uri, diagnostics);
    }
}

impl Default for MockClient {
    fn default() -> Self {
        Self::new()
    }
}

/// LSP connection wrapper
///
/// Manages JSON-RPC communication with LSP server process.
pub struct LspConnection {
    /// Server stdin (for sending requests)
    stdin: ChildStdin,

    /// Server stdout (for reading responses) - wrapped in BufReader for buffering
    stdout: BufReader<ChildStdout>,

    /// Request ID counter
    next_id: Arc<std::sync::atomic::AtomicU64>,

    /// Mock client for receiving server notifications
    client: MockClient,
}

impl LspConnection {
    /// Create a new LSP connection
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            client: MockClient::new(),
        }
    }

    /// Get the mock client
    pub fn client(&self) -> MockClient {
        self.client.clone()
    }

    /// Send a notification (fire-and-forget)
    pub async fn notify<P>(&mut self, method: String, params: P) -> Result<()>
    where
        P: serde::Serialize,
    {
        let json_notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let notification_str = serde_json::to_string(&json_notification)?;
        self.send_message(&notification_str).await?;
        Ok(())
    }

    /// Send a message with LSP Content-Length header
    async fn send_message(&mut self, content: &str) -> Result<()> {
        let header = format!("Content-Length: {}\r\n\r\n", content.len());
        let full_message = format!("{}{}", header, content);

        self.stdin.write_all(full_message.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    /// Read a message with LSP Content-Length header
    /// Skips notifications (which have "method" but no "id") and returns only responses (which have "id")
    async fn read_message(&mut self) -> Result<String> {
        loop {
            // Read headers line by line until we find empty line (\r\n)
            let mut content_length: Option<usize> = None;
            let mut line = String::new();

            loop {
                line.clear();
                let n = self.stdout.read_line(&mut line).await?;
                if n == 0 {
                    return Err(anyhow::anyhow!("EOF: No data from server"));
                }

                let trimmed = line.trim();
                debug_log!("!!! CLIENT: Read header line: {:?}", trimmed);

                if trimmed.is_empty() {
                    // Empty line marks end of headers
                    break;
                }

                if trimmed.to_lowercase().starts_with("content-length:") {
                    content_length = trimmed
                        .split(':')
                        .nth(1)
                        .and_then(|s| s.trim().parse::<usize>().ok());
                }
            }

            let content_length =
                content_length.ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?;
            debug_log!("!!! CLIENT: Content length: {}", content_length);

            // Read the content
            let mut content_buf = vec![0u8; content_length];
            self.stdout.read_exact(&mut content_buf).await?;

            let content_str = String::from_utf8(content_buf.clone())?;

            // Check if this is a response (has "id") or a notification (has "method" but no "id")
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content_str) {
                // If it has an "id" field, it's a response - return it
                if json.get("id").is_some() {
                    return Ok(content_str);
                }

                // It's a notification - check if it's publish_diagnostics
                if let Some(method) = json.get("method").and_then(|m| m.as_str()) {
                    if method == "textDocument/publishDiagnostics" {
                        debug_log!("!!! CLIENT: Received publish_diagnostics notification");
                        if let Some(params) = json.get("params") {
                            if let Some(uri) = params.get("uri").and_then(|u| u.as_str()) {
                                if let Some(diags) =
                                    params.get("diagnostics").and_then(|d| d.as_array())
                                {
                                    let url = Url::parse(uri)?;
                                    let diagnostics: Vec<Diagnostic> = serde_json::from_value(
                                        serde_json::Value::Array(diags.to_vec()),
                                    )?;
                                    let diag_count = diagnostics.len();
                                    self.client.record_diagnostics(url, diagnostics).await;
                                    debug_log!(
                                        "!!! CLIENT: Recorded {} diagnostics for {}",
                                        diag_count,
                                        uri
                                    );
                                }
                            }
                        }
                    } else {
                        debug_log!("!!! CLIENT: Skipping notification: {}", method);
                    }
                } else {
                    debug_log!("!!! CLIENT: Skipping notification: {}", content_str);
                }
            } else {
                // Not valid JSON, just return it
                return Ok(content_str);
            }
        }
    }

    /// Initialize LSP server
    pub async fn initialize(&mut self) -> Result<InitializeResult> {
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(Url::parse("file:///test")?),
            ..Default::default()
        };

        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Build JSON-RPC request
        let json_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": params,
        });

        self.send_message(&serde_json::to_string(&json_request)?)
            .await?;

        // Read response
        let response_str = self.read_message().await?;
        let json_response: serde_json::Value = serde_json::from_str(&response_str)?;

        // Check if it's an error
        if let Some(_error) = json_response.get("error") {
            return Err(anyhow::anyhow!("LSP initialize error: {}", json_response));
        }

        // Parse result
        let result = json_response
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("No result in initialize response"))?;

        let init_result: InitializeResult = serde_json::from_value(result.clone())?;

        // Send initialized notification
        self.notify("initialized".to_string(), serde_json::json!({}))
            .await?;

        Ok(init_result)
    }

    /// Open a document
    pub async fn did_open(&mut self, uri: Url, language_id: String, content: String) -> Result<()> {
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id,
                version: 1,
                text: content,
            },
        };

        self.notify("textDocument/didOpen".to_string(), params)
            .await
    }

    /// Close a document
    pub async fn did_close(&mut self, uri: Url) -> Result<()> {
        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };

        self.notify("textDocument/didClose".to_string(), params)
            .await
    }

    /// Send workspace/didChangeConfiguration notification to set engine config
    pub async fn did_change_configuration(
        &mut self,
        dialect: &str,
        connection_string: &str,
    ) -> Result<()> {
        debug_log!(
            "!!! CLIENT: Sending did_change_configuration: dialect={}, connection={}",
            dialect,
            connection_string
        );

        let params = DidChangeConfigurationParams {
            settings: serde_json::json!({
                "unifiedSqlLsp": {
                    "dialect": dialect,
                    "connectionString": connection_string
                }
            }),
        };

        debug_log!("!!! CLIENT: Calling notify for workspace/didChangeConfiguration");
        let result = self
            .notify("workspace/didChangeConfiguration".to_string(), params)
            .await;
        debug_log!(
            "!!! CLIENT: did_change_configuration notification sent: {:?}",
            result
        );
        result
    }

    /// Request completion
    pub async fn completion(
        &mut self,
        uri: Url,
        position: Position,
    ) -> Result<Option<Vec<CompletionItem>>> {
        debug_log!(
            "!!! CLIENT: Requesting completion for uri={}, line={}, col={}",
            uri,
            position.line,
            position.character
        );

        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        };

        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let json_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": params,
        });

        debug_log!(
            "!!! CLIENT: Sending completion request: {}",
            serde_json::to_string(&json_request)?
        );

        self.send_message(&serde_json::to_string(&json_request)?)
            .await?;

        debug_log!("!!! CLIENT: Request sent, reading response...");
        // Read response
        let response_str = self.read_message().await?;
        debug_log!("!!! CLIENT: Received response: {}", response_str);
        let json_response: serde_json::Value = serde_json::from_str(&response_str)?;

        if let Some(error) = json_response.get("error") {
            // Completion error is not critical - return empty
            debug_log!("!!! CLIENT: Completion error: {}", error);
            return Ok(None);
        }

        let result = json_response.get("result");
        debug_log!("!!! CLIENT: result = {:?}", result);

        match result {
            Some(serde_json::Value::Null) => {
                debug_log!("!!! CLIENT: result is Null, returning Ok(None)");
                Ok(None)
            }
            Some(result) => {
                // Try to parse as CompletionResponse
                if let Ok(response) = serde_json::from_value::<CompletionResponse>(result.clone()) {
                    debug_log!("!!! CLIENT: Parsed as CompletionResponse");
                    let items = match response {
                        CompletionResponse::Array(items) => {
                            debug_log!("!!! CLIENT: Got Array with {} items", items.len());
                            items
                        }
                        CompletionResponse::List(list) => {
                            debug_log!("!!! CLIENT: Got List with {} items", list.items.len());
                            list.items
                        }
                    };
                    Ok(Some(items))
                } else {
                    debug_log!(
                        "!!! CLIENT: Failed to parse as CompletionResponse, trying direct array"
                    );
                    // Try direct array parse
                    let items: Vec<CompletionItem> =
                        serde_json::from_value(result.clone()).unwrap_or_default();
                    debug_log!("!!! CLIENT: Direct array parse got {} items", items.len());
                    Ok(Some(items))
                }
            }
            None => {
                debug_log!("!!! CLIENT: result is None, returning Ok(None)");
                Ok(None)
            }
        }
    }

    /// Request hover
    pub async fn hover(&mut self, uri: Url, position: Position) -> Result<Option<Hover>> {
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        };

        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let json_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": params,
        });

        self.send_message(&serde_json::to_string(&json_request)?)
            .await?;

        let response_str = self.read_message().await?;
        let json_response: serde_json::Value = serde_json::from_str(&response_str)?;

        if let Some(_error) = json_response.get("error") {
            return Ok(None);
        }

        let result = json_response.get("result");

        match result {
            Some(serde_json::Value::Null) => Ok(None),
            Some(result) => serde_json::from_value(result.clone())
                .map(Some)
                .map_err(|e| anyhow::anyhow!("Failed to parse hover response: {}", e)),
            None => Ok(None),
        }
    }

    /// Get diagnostics (captured from publish_diagnostics notification)
    pub async fn get_diagnostics(&self, uri: &Url) -> Option<Vec<Diagnostic>> {
        self.client.get_diagnostics(uri).await
    }

    /// Read all pending notifications (with timeout)
    pub async fn read_pending_notifications(&mut self) -> Result<()> {
        // Try to read with a small timeout to collect any pending notifications
        match tokio::time::timeout(std::time::Duration::from_millis(100), self.read_message()).await
        {
            Ok(_) => Ok(()),
            Err(_) => Ok(()), // Timeout is expected if no messages
        }
    }
}
