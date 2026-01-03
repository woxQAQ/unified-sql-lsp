// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # LSP Backend Implementation
//!
//! This module provides the main LSP server backend using tower-lsp.
//!
//! ## Overview
//!
//! The backend handles:
//! - LSP protocol communication via tower-lsp
//! - Document lifecycle (open, change, close)
//! - Multi-client support
//! - Engine configuration management
//!
//! ## Architecture
//!
//! ```text
//! Client → LSP Backend → Document Store
//!                ↓
//!           Engine Config
//!                ↓
//!           (Future: Catalog, Semantic Analysis, Completion)
//! ```
//!
//! ## Supported LSP Features
//!
//! Currently implemented:
//! - textDocument/didOpen
//! - textDocument/didChange
//! - textDocument/didClose
//!
//! Planned (in future features):
//! - textDocument/completion (LSP-003)
//! - textDocument/hover (HOVER-001)
//! - textDocument/definition
//! - textDocument/diagnostic (DIAG-001)
//!
//! ## Example
//!
//! ```rust,ignore
//! use unified_sql_lsp_lsp::LspBackend;
//! use tower_lsp::Server;
//!
//! #[tokio::main]
//! async fn main() {
//!     let backend = LspBackend::new();
//!     let service = Server::new(backend).unwrap();
//!     // Run service...
//! }
//! ```

use crate::catalog_manager::CatalogManager;
use crate::completion::CompletionEngine;
use crate::config::EngineConfig;
use crate::document::{DocumentError, DocumentStore, ParseMetadata};
use crate::sync::DocumentSync;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{error, info, warn};

/// LSP backend implementation
///
/// Main entry point for all LSP protocol operations.
/// Uses tower-lsp framework for protocol handling.
pub struct LspBackend {
    /// LSP client for sending notifications and requests
    client: Client,

    /// Document store for managing open documents
    documents: Arc<DocumentStore>,

    /// Engine configuration
    config: Arc<RwLock<Option<EngineConfig>>>,

    /// Document synchronization and parsing manager
    doc_sync: Arc<DocumentSync>,

    /// Catalog manager for database connections
    catalog_manager: Arc<RwLock<CatalogManager>>,
}

impl LspBackend {
    /// Create a new LSP backend
    ///
    /// # Arguments
    ///
    /// - `client`: LSP client handle
    pub fn new(client: Client) -> Self {
        let config = Arc::new(RwLock::new(None));
        let doc_sync = Arc::new(DocumentSync::new(config.clone()));

        Self {
            client,
            documents: Arc::new(DocumentStore::new()),
            config,
            doc_sync,
            catalog_manager: Arc::new(RwLock::new(CatalogManager::new())),
        }
    }

    /// Get the document store
    pub fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    /// Get the engine configuration
    pub async fn get_config(&self) -> Option<EngineConfig> {
        self.config.read().await.clone()
    }

    /// Set the engine configuration
    ///
    /// This is called when the client sends workspace configuration
    /// or through the `initialize` response.
    pub async fn set_config(&self, config: EngineConfig) {
        info!("Engine configuration updated: dialect={:?}", config.dialect);
        *self.config.write().await = Some(config);
    }

    /// Log a message to the client
    async fn log_message(&self, message: &str, message_type: MessageType) {
        self.client.log_message(message_type, message).await;
    }

    /// Show a message to the user
    async fn show_message(&self, message: &str, message_type: MessageType) {
        self.client.show_message(message_type, message).await;
    }

    /// Publish diagnostics to the client
    ///
    /// This will be used extensively in DIAG-001 and subsequent features.
    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspBackend {
    /// Initialize the LSP server
    ///
    /// Called when the client starts the server.
    /// Returns server capabilities and configuration.
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Initializing LSP server");
        info!("Client info: {:?}", params.client_info);

        // Log client capabilities
        if let Some(capabilities) = params.capabilities.text_document {
            info!(
                "Text document capabilities: sync={:?}",
                capabilities.synchronization
            );
        }

        // Send initialization message
        self.log_message("Unified SQL LSP server initialized", MessageType::INFO)
            .await;

        // Return server capabilities
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                // Text synchronization
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),

                // Completion (will be implemented in LSP-003)
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), " ".to_string()]),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(false),
                    },
                    all_commit_characters: None,
                    completion_item: None,
                }),

                // Hover (will be implemented in HOVER-001)
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                // Diagnostics (will be implemented in DIAG-001)
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        work_done_progress_options: WorkDoneProgressOptions {
                            work_done_progress: Some(false),
                        },
                        inter_file_dependencies: false,
                        workspace_diagnostics: false,
                        identifier: None,
                    },
                )),

                // Definition (future feature)
                definition_provider: Some(OneOf::Left(true)),

                // Document formatting (will be implemented in FORMAT-001)
                document_formatting_provider: Some(OneOf::Left(true)),

                // Document symbols (future feature)
                document_symbol_provider: Some(OneOf::Left(true)),

                // Other capabilities
                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(false),
                        change_notifications: Some(OneOf::Left(false)),
                    }),
                    ..Default::default()
                }),

                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "unified-sql-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    /// Initialized notification
    ///
    /// Called after `initialize` completes successfully.
    async fn initialized(&self, _params: InitializedParams) {
        info!("LSP server initialized successfully");

        // Send a welcome message
        self.show_message(
            "Unified SQL LSP server ready! Configure your database connection in settings.",
            MessageType::INFO,
        )
        .await;

        // TODO: (CONFIG-001) Load configuration from client settings
        // For now, we'll wait for the client to send configuration
        // through workspace/didChangeConfiguration
    }

    /// Shutdown the LSP server
    ///
    /// Called when the client is shutting down the server.
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down LSP server");

        // Clean up resources
        // (None to clean up yet, but this will be important for catalog connections)

        Ok(())
    }

    /// Document opened notification
    ///
    /// Called when the client opens a document.
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let uri = doc.uri.clone();
        let language_id = doc.language_id.clone();
        let version = doc.version;
        let content = doc.text;

        info!(
            "Document opened: uri={}, language={}, version={}",
            uri, language_id, version
        );

        // Add to document store
        match self
            .documents
            .open_document(uri.clone(), content, version, language_id)
            .await
        {
            Ok(()) => {
                self.log_message(&format!("Document opened: {}", uri), MessageType::INFO)
                    .await;

                // Trigger parsing
                if let Some(document) = self.documents.get_document(&uri).await {
                    match self.doc_sync.on_document_open(&document) {
                        crate::parsing::ParseResult::Success { tree, parse_time } => {
                            info!("Document parsed successfully in {:?}", parse_time);
                            let metadata = ParseMetadata::new(
                                parse_time.as_millis() as u64,
                                self.doc_sync.resolve_dialect(&document),
                                false,
                                0,
                            );
                            if let Err(e) = self
                                .documents
                                .update_document_tree(&uri, tree, metadata)
                                .await
                            {
                                error!("Failed to update document tree: {}", e);
                            }
                        }
                        crate::parsing::ParseResult::Partial { tree, errors } => {
                            warn!("Document parsed with {} errors", errors.len());
                            let metadata = ParseMetadata::new(
                                0, // parse_time not available in Partial
                                self.doc_sync.resolve_dialect(&document),
                                true,
                                errors.len(),
                            );
                            if let Err(e) = self
                                .documents
                                .update_document_tree(&uri, tree, metadata)
                                .await
                            {
                                error!("Failed to update document tree: {}", e);
                            }
                            // TODO: (DIAG-002) Publish diagnostics
                        }
                        crate::parsing::ParseResult::Failed { error } => {
                            error!("Failed to parse document: {}", error);
                            // Document remains usable, just without tree
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to open document: {}", e);
                self.show_message(
                    &format!("Failed to open document: {}", e),
                    MessageType::ERROR,
                )
                .await;
            }
        }
    }

    /// Document changed notification
    ///
    /// Called when the client modifies a document.
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let identifier = params.text_document;
        let uri = identifier.uri.clone();
        let changes = params.content_changes;

        info!(
            "Document changed: uri={}, version={}, changes={}",
            uri,
            identifier.version,
            changes.len()
        );

        // Get old tree before update
        let old_document = self.documents.get_document(&uri).await;
        let old_tree: Option<tree_sitter::Tree> = old_document.as_ref().and_then(|d| {
            d.tree().as_ref().and_then(|arc_mutex| {
                // Try to lock and clone the tree
                arc_mutex.try_lock().ok().map(|guard| (*guard).clone())
            })
        });

        // Update document in store
        match self.documents.update_document(&identifier, &changes).await {
            Ok(()) => {
                // Trigger re-parsing
                if let Some(document) = self.documents.get_document(&uri).await {
                    match self
                        .doc_sync
                        .on_document_change(&document, old_tree.as_ref(), &changes)
                    {
                        crate::parsing::ParseResult::Success { tree, parse_time } => {
                            info!("Document reparsed in {:?}", parse_time);
                            let metadata = ParseMetadata::new(
                                parse_time.as_millis() as u64,
                                self.doc_sync.resolve_dialect(&document),
                                false,
                                0,
                            );
                            if let Err(e) = self
                                .documents
                                .update_document_tree(&uri, tree, metadata)
                                .await
                            {
                                error!("Failed to update document tree: {}", e);
                            }
                        }
                        crate::parsing::ParseResult::Partial { tree, errors } => {
                            warn!("Document reparsed with {} errors", errors.len());
                            let metadata = ParseMetadata::new(
                                0,
                                self.doc_sync.resolve_dialect(&document),
                                true,
                                errors.len(),
                            );
                            if let Err(e) = self
                                .documents
                                .update_document_tree(&uri, tree, metadata)
                                .await
                            {
                                error!("Failed to update document tree: {}", e);
                            }
                            // TODO: (DIAG-002) Publish diagnostics
                        }
                        crate::parsing::ParseResult::Failed { error } => {
                            error!("Failed to reparse document: {}", error);
                            if let Err(e) = self.documents.clear_document_tree(&uri).await {
                                error!("Failed to clear document tree: {}", e);
                            }
                        }
                    }
                }
            }
            Err(DocumentError::DocumentNotFound(uri)) => {
                warn!("Document not found for change: {}", uri);
            }
            Err(e) => {
                error!("Failed to update document: {}", e);
                self.show_message(
                    &format!("Failed to update document: {}", e),
                    MessageType::ERROR,
                )
                .await;
            }
        }
    }

    /// Document closed notification
    ///
    /// Called when the client closes a document.
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        info!("Document closed: uri={}", uri);

        // Remove from document store
        if self.documents.close_document(&uri).await {
            // Clear parse data
            self.doc_sync.on_document_close(&uri);

            self.log_message(&format!("Document closed: {}", uri), MessageType::INFO)
                .await;
        } else {
            warn!("Document not found for close: {}", uri);
        }
    }

    /// Completion request
    ///
    /// Called when the user requests completion (e.g., Ctrl+Space).
    /// Implements COMPLETION-001: SELECT clause column completion.
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        info!(
            "Completion requested: uri={}, line={}, col={}",
            uri, position.line, position.character
        );

        // Get document
        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for completion: {}", uri);
                return Ok(None);
            }
        };

        // Get engine configuration
        let config = match self.get_config().await {
            Some(cfg) => cfg,
            None => {
                warn!("No engine configuration available for completion");
                self.log_message(
                    "Completion requires database connection configuration",
                    MessageType::WARNING,
                )
                .await;
                return Ok(None);
            }
        };

        // Get catalog
        let catalog = {
            let mut manager = self.catalog_manager.write().await;
            match manager.get_catalog(&config).await {
                Ok(catalog) => catalog,
                Err(e) => {
                    error!("Failed to get catalog: {}", e);
                    self.log_message(
                        &format!("Failed to connect to database: {}", e),
                        MessageType::ERROR,
                    )
                    .await;
                    return Ok(None);
                }
            }
        };

        // Create completion engine and perform completion
        let engine = CompletionEngine::new(catalog);
        match engine.complete(&document, position).await {
            Ok(Some(items)) => {
                info!("Completion returned {} items", items.len());
                Ok(Some(CompletionResponse::Array(items)))
            }
            Ok(None) => {
                // No completion available (wrong context)
                Ok(None)
            }
            Err(e) => {
                error!("Completion error: {}", e);
                if e.should_return_empty() {
                    Ok(None)
                } else {
                    // Show error to user
                    self.log_message(&format!("Completion error: {}", e), MessageType::ERROR)
                        .await;
                    Ok(None)
                }
            }
        }
    }

    /// Hover request
    ///
    /// Called when the user hovers over a symbol.
    /// This is a stub implementation - full implementation will be in HOVER-001.
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Hover requested: uri={}, line={}, col={}",
            uri, position.line, position.character
        );

        // TODO: (HOVER-001) Implement actual hover logic
        Ok(None)
    }

    /// Definition request
    ///
    /// Called when the user requests go-to-definition.
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;

        info!("Go to definition requested: uri={}", uri);

        // TODO: (LSP-004) Implement go-to-definition
        // This feature is not yet tracked in FEATURE_LIST.yaml
        // Would enable users to jump to table/column definitions
        Ok(None)
    }

    /// Document formatting request
    ///
    /// Called when the user formats a document.
    /// This is a stub implementation - full implementation will be in FORMAT-001.
    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;

        info!("Document formatting requested: uri={}", uri);

        // TODO: (FORMAT-001) Implement SQL formatting
        self.log_message(
            "Document formatting is not yet implemented (FORMAT-001)",
            MessageType::INFO,
        )
        .await;

        Ok(None)
    }

    /// Document symbols request
    ///
    /// Called when the user requests document symbols (e.g., for outline view).
    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;

        info!("Document symbols requested: uri={}", uri);

        // TODO: (LSP-005) Implement document symbols (tables, columns, etc.)
        // This feature is not yet tracked in FEATURE_LIST.yaml
        // Would enable outline view showing tables, columns, aliases in the query
        Ok(None)
    }

    /// Configuration change notification
    ///
    /// Called when the client's configuration changes.
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        info!("Configuration changed: {:?}", params.settings);

        // TODO: (CONFIG-001) Parse and apply configuration from client
        // For now, just log that configuration changed
        self.log_message(
            "Configuration changed - restart server to apply changes",
            MessageType::INFO,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_new() {
        // Create a mock client for testing
        // Note: tower_lsp::Client cannot be directly instantiated,
        // so in real tests we would use dependency injection
        // This is a placeholder test structure
    }

    // More comprehensive tests would require:
    // - Mock LSP client
    // - Mock executor
    // - Integration test framework
    // These will be added in TEST-002
}

/// LSP backend errors
///
/// Errors that can occur during LSP operations.
#[derive(Debug, thiserror::Error)]
pub enum LspError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] crate::config::ConfigError),

    /// Document error
    #[error("Document error: {0}")]
    Document(#[from] DocumentError),

    /// Catalog error (future use)
    #[error("Catalog error: {0}")]
    Catalog(#[from] unified_sql_lsp_catalog::CatalogError),

    /// Generic error
    #[error("LSP error: {0}")]
    Generic(String),
}

/// Module for testing utilities
#[cfg(test)]
pub mod test_utils {
    use super::*;

    /// Create a test engine configuration
    pub fn test_config() -> EngineConfig {
        EngineConfig {
            dialect: unified_sql_lsp_ir::Dialect::MySQL,
            version: crate::config::DialectVersion::MySQL80,
            connection_string: "mysql://localhost:3306/test".to_string(),
            ..Default::default()
        }
    }
}
