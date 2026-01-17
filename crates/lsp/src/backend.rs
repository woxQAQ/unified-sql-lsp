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
use crate::diagnostic::{DiagnosticCollector, publish_diagnostics_for_document};
use crate::document::{Document, DocumentError, DocumentStore, ParseMetadata};
use crate::symbols::{SymbolBuilder, SymbolCatalogFetcher, SymbolError, SymbolRenderer};
use crate::sync::DocumentSync;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tracing::{debug, error, info, warn};

/// LSP backend implementation
///
/// Main entry point for all LSP protocol operations.
pub struct LspBackend {
    client: Client,
    documents: Arc<DocumentStore>,
    config: Arc<RwLock<Option<EngineConfig>>>,
    doc_sync: Arc<DocumentSync>,
    catalog_manager: Arc<RwLock<CatalogManager>>,
    diagnostic_collector: DiagnosticCollector,
}

impl LspBackend {
    pub fn new(client: Client) -> Self {
        debug!("!!! LSP: LspBackend::new() called");
        let config = Arc::new(RwLock::new(None));
        let doc_sync = Arc::new(DocumentSync::new(config.clone()));

        debug!("!!! LSP: LspBackend created successfully");
        Self {
            client,
            documents: Arc::new(DocumentStore::new()),
            config,
            doc_sync,
            catalog_manager: Arc::new(RwLock::new(CatalogManager::new())),
            diagnostic_collector: DiagnosticCollector::new(),
        }
    }

    pub fn documents(&self) -> &DocumentStore {
        &self.documents
    }

    pub async fn get_config(&self) -> Option<EngineConfig> {
        self.config.read().await.clone()
    }

    pub async fn set_config(&self, config: EngineConfig) {
        info!("Engine configuration updated: dialect={:?}", config.dialect);
        *self.config.write().await = Some(config);
    }

    async fn log_message(&self, message: &str, message_type: MessageType) {
        self.client.log_message(message_type, message).await;
    }

    async fn show_message(&self, message: &str, message_type: MessageType) {
        self.client.show_message(message_type, message).await;
    }

    #[allow(dead_code)]
    async fn publish_diagnostics(&self, uri: Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Publish diagnostics for a document
    ///
    /// Shared helper for publishing diagnostics after parsing.
    async fn publish_document_diagnostics(&self, uri: &Url) {
        let updated_document = self.documents.get_document(uri).await;
        if let Some(doc) = updated_document {
            let source = doc.get_content();
            let tree_ref = doc.tree();
            publish_diagnostics_for_document(
                &self.diagnostic_collector,
                &self.client,
                uri.clone(),
                &tree_ref,
                &source,
            )
            .await;
        }
    }

    /// Parse document and update its tree in the store
    ///
    /// Shared helper for did_open and did_change handlers.
    async fn parse_and_update_tree(&self, uri: &Url, document: &Document) {
        let dialect = self.doc_sync.resolve_dialect(document);

        match self.doc_sync.on_document_open(document) {
            crate::parsing::ParseResult::Success { tree, parse_time } => {
                info!("Document parsed successfully in {:?}", parse_time);
                let metadata = ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0);
                if let Some(tree) = tree
                    && let Err(e) = self
                        .documents
                        .update_document_tree(uri, tree, metadata)
                        .await
                {
                    error!("Failed to update document tree: {}", e);
                }
                self.publish_document_diagnostics(uri).await;
            }
            crate::parsing::ParseResult::Partial { tree, errors } => {
                warn!("Document parsed with {} errors", errors.len());
                let metadata = ParseMetadata::new(0, dialect, true, errors.len());
                if let Some(tree) = tree
                    && let Err(e) = self
                        .documents
                        .update_document_tree(uri, tree, metadata)
                        .await
                {
                    error!("Failed to update document tree: {}", e);
                }
                self.publish_document_diagnostics(uri).await;
            }
            crate::parsing::ParseResult::Failed { error } => {
                error!("Failed to parse document: {}", error);
                // Clear diagnostics on parse failure
                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
                    .await;
            }
        }
    }

    /// Parse document with incremental update support
    ///
    /// Used for did_change when we have an old tree for incremental parsing.
    async fn parse_and_update_tree_incremental(
        &self,
        uri: &Url,
        document: &Document,
        old_tree: Option<&tree_sitter::Tree>,
        changes: &[TextDocumentContentChangeEvent],
    ) {
        let dialect = self.doc_sync.resolve_dialect(document);

        match self
            .doc_sync
            .on_document_change(document, old_tree, changes)
        {
            crate::parsing::ParseResult::Success { tree, parse_time } => {
                info!("Document reparsed in {:?}", parse_time);
                let metadata = ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0);
                if let Some(tree) = tree
                    && let Err(e) = self
                        .documents
                        .update_document_tree(uri, tree, metadata)
                        .await
                {
                    error!("Failed to update document tree: {}", e);
                }
                self.publish_document_diagnostics(uri).await;
            }
            crate::parsing::ParseResult::Partial { tree, errors } => {
                warn!("Document reparsed with {} errors", errors.len());
                let metadata = ParseMetadata::new(0, dialect, true, errors.len());
                if let Some(tree) = tree
                    && let Err(e) = self
                        .documents
                        .update_document_tree(uri, tree, metadata)
                        .await
                {
                    error!("Failed to update document tree: {}", e);
                }
                self.publish_document_diagnostics(uri).await;
            }
            crate::parsing::ParseResult::Failed { error } => {
                error!("Failed to reparse document: {}", error);
                if let Err(e) = self.documents.clear_document_tree(uri).await {
                    error!("Failed to clear document tree: {}", e);
                }
                // Clear diagnostics on parse failure
                self.client
                    .publish_diagnostics(uri.clone(), Vec::new(), None)
                    .await;
            }
        }
    }

    /// Parse engine configuration from client settings
    fn parse_config_from_settings(&self, settings: &serde_json::Value) -> Option<EngineConfig> {
        info!("Parsing configuration from settings: {:?}", settings);

        // Extract unifiedSqlLsp section
        let lsp_settings = settings.get("unifiedSqlLsp")?;
        info!("Found unifiedSqlLsp settings: {:?}", lsp_settings);

        // Parse dialect
        let dialect_str = lsp_settings.get("dialect")?.as_str()?;
        info!("Parsed dialect: {}", dialect_str);
        let dialect = match dialect_str {
            "mysql" => unified_sql_lsp_ir::Dialect::MySQL,
            "postgresql" => unified_sql_lsp_ir::Dialect::PostgreSQL,
            _ => return None,
        };

        // Parse version with default
        let version_str = lsp_settings
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("8.0");

        let version = match (dialect, version_str) {
            (unified_sql_lsp_ir::Dialect::MySQL, "5.7") => crate::config::DialectVersion::MySQL57,
            (unified_sql_lsp_ir::Dialect::MySQL, _) => crate::config::DialectVersion::MySQL80,
            (unified_sql_lsp_ir::Dialect::PostgreSQL, "12") => {
                crate::config::DialectVersion::PostgreSQL12
            }
            (unified_sql_lsp_ir::Dialect::PostgreSQL, "14") => {
                crate::config::DialectVersion::PostgreSQL14
            }
            (unified_sql_lsp_ir::Dialect::PostgreSQL, _) => {
                crate::config::DialectVersion::PostgreSQL16
            }
            _ => return None,
        };

        // Parse connection string
        let connection_string = lsp_settings.get("connectionString")?.as_str()?.to_string();
        info!("Parsed connection string: {}", connection_string);

        let config = EngineConfig::new(dialect, version, connection_string);
        info!(
            "Successfully parsed engine config: dialect={:?}, version={:?}",
            dialect, version
        );
        Some(config)
    }

    /// Get or create default engine configuration
    ///
    /// Returns the existing config or creates a default one for testing.
    async fn get_config_or_default(&self) -> EngineConfig {
        match self.get_config().await {
            Some(cfg) => cfg,
            None => {
                // Use default configuration for testing
                let default_connection =
                    std::env::var("E2E_MYSQL_CONNECTION").unwrap_or_else(|_| {
                        "mysql://test_user:test_password@127.0.0.1:3307/test_db".to_string()
                    });

                EngineConfig::new(
                    unified_sql_lsp_ir::Dialect::MySQL,
                    crate::config::DialectVersion::MySQL57,
                    default_connection,
                )
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspBackend {
    /// Initialize the LSP server
    ///
    /// Called when the client starts the server.
    /// Returns server capabilities and configuration.
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        debug!("!!! LSP: initialize() called");
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

        debug!(
            "!!! LSP: did_open() called: uri={}, language={}",
            uri, language_id
        );

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

                // Trigger parsing using shared helper
                if let Some(document) = self.documents.get_document(&uri).await {
                    self.parse_and_update_tree(&uri, &document).await;
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

        // Get old tree before update for incremental parsing
        let old_document = self.documents.get_document(&uri).await;
        let old_tree: Option<tree_sitter::Tree> = old_document.as_ref().and_then(|d| {
            d.tree()
                .as_ref()
                .and_then(|arc_mutex| arc_mutex.try_lock().ok().map(|guard| (*guard).clone()))
        });

        // Update document in store
        match self.documents.update_document(&identifier, &changes).await {
            Ok(()) => {
                // Trigger re-parsing using shared helper
                if let Some(document) = self.documents.get_document(&uri).await {
                    self.parse_and_update_tree_incremental(
                        &uri,
                        &document,
                        old_tree.as_ref(),
                        &changes,
                    )
                    .await;
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

            // Clear diagnostics
            self.client
                .publish_diagnostics(uri.clone(), Vec::new(), None)
                .await;

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

        debug!(
            "!!! LSP: Completion requested: uri={}, line={}, col={}",
            uri, position.line, position.character
        );

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

        let config = self.get_config_or_default().await;
        debug!("!!! LSP: Config dialect={:?}", config.dialect);

        // Get catalog
        let catalog = {
            debug!("!!! LSP: Getting catalog for config");
            let mut manager = self.catalog_manager.write().await;
            match manager.get_catalog(&config).await {
                Ok(catalog) => {
                    debug!("!!! LSP: Got catalog successfully");
                    catalog
                }
                Err(e) => {
                    debug!("!!! LSP: Failed to get catalog: {}", e);
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
        debug!("!!! LSP: Creating completion engine");
        let engine = CompletionEngine::new(catalog);
        debug!("!!! LSP: Calling complete with position {:?}", position);
        match engine.complete(&document, position).await {
            Ok(Some(items)) => {
                debug!("!!! LSP: Completion returned {} items", items.len());
                for (i, item) in items.iter().take(5).enumerate() {
                    debug!(
                        "!!! LSP:   Item {}: label={}, kind={:?}",
                        i, item.label, item.kind
                    );
                }
                info!("Completion returned {} items", items.len());
                Ok(Some(CompletionResponse::Array(items)))
            }
            Ok(None) => {
                // No completion available (wrong context)
                debug!("!!! LSP: Completion returned None (wrong context)");
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
    /// Uses HoverEngine for CST-based hover information.
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        debug!(
            "!!! LSP: Hover requested: uri={}, line={}, col={}",
            uri, position.line, position.character
        );

        info!(
            "Hover requested: uri={}, line={}, col={}",
            uri, position.line, position.character
        );

        // Get document
        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found for hover: {}", uri);
                return Ok(None);
            }
        };

        let config = self.get_config_or_default().await;

        // Get catalog
        let catalog = {
            let mut manager = self.catalog_manager.write().await;
            match manager.get_catalog(&config).await {
                Ok(catalog) => catalog,
                Err(e) => {
                    debug!("!!! LSP: Failed to get catalog for hover: {}", e);
                    error!("Failed to get catalog for hover: {}", e);
                    return Ok(None);
                }
            }
        };

        // Use HoverEngine for CST-based hover
        use crate::hover::HoverEngine;
        let engine = HoverEngine::new(catalog, config.dialect);

        if let Some(text) = engine.get_hover(&document, position).await {
            debug!("!!! LSP: Returning hover info: {}", text);
            Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: text,
                }),
                range: None,
            }))
        } else {
            debug!("!!! LSP: No hover info found");
            Ok(None)
        }
    }

    /// Definition request
    ///
    /// Called when the user requests go-to-definition (F12 in most editors).
    /// This allows users to jump from symbol references (tables, columns) to their definitions.
    ///
    /// # Examples
    ///
    /// ```sql
    /// SELECT u.id, u.name FROM users u WHERE u.id = 1
    /// ```
    ///
    /// Invoking go-to-definition on `u.id` in the WHERE clause will jump to `u.id` in the SELECT clause.
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        use crate::Definition;
        use crate::definition::DefinitionFinder;

        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        info!(
            "Go to definition requested: uri={}, pos={:?}",
            uri, position
        );

        // 1. Get document from store
        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found: {}", uri);
                return Ok(None);
            }
        };

        // 2. Get parse tree
        let tree = match document.tree() {
            Some(t) => t,
            None => {
                info!("Document not parsed: {}", uri);
                return Ok(None); // Graceful degradation
            }
        };

        // 3. Find definition using DefinitionFinder
        let tree_lock = match tree.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                warn!("Failed to acquire tree lock for go-to-definition");
                return Ok(None);
            }
        };
        let root_node = tree_lock.root_node();
        let source = document.get_content();

        match DefinitionFinder::find_at_position(&root_node, source.as_str(), position, &uri) {
            Ok(Some(definition)) => {
                let location = match definition {
                    Definition::Table(def) => def.location,
                    Definition::Column(def) => def.location,
                };
                info!("Definition found: {:?}", location);
                Ok(Some(GotoDefinitionResponse::Scalar(location)))
            }
            Ok(None) => {
                info!("No definition found at position");
                Ok(None)
            }
            Err(e) => {
                warn!("Error finding definition: {:?}", e);
                Ok(None) // Graceful degradation
            }
        }
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

        // 1. Get document from store
        let document = match self.documents.get_document(&uri).await {
            Some(doc) => doc,
            None => {
                warn!("Document not found: {}", uri);
                return Ok(None);
            }
        };

        // 2. Get parse tree
        let tree = match document.tree() {
            Some(t) => t,
            None => {
                warn!("Document not parsed: {}", uri);
                return Ok(None); // Graceful degradation
            }
        };

        // 3. Get catalog (optional for graceful degradation)
        let catalog = match self.get_config().await {
            Some(config) => {
                match self
                    .catalog_manager
                    .write()
                    .await
                    .get_catalog(&config)
                    .await
                {
                    Ok(cat) => Some(cat),
                    Err(e) => {
                        warn!("Catalog unavailable for symbols: {}", e);
                        None
                    }
                }
            }
            None => None,
        };

        // 4. Lock parse tree and extract source
        let tree_lock = match tree.try_lock() {
            Ok(lock) => lock,
            Err(_) => {
                error!("Failed to acquire tree lock for document symbols");
                return Ok(None);
            }
        };

        let root_node = tree_lock.root_node();
        let source = document.get_content();

        // 5. Build symbols from CST
        let mut queries = match SymbolBuilder::build_from_cst(&root_node, source.as_str()) {
            Ok(queries) => queries,
            Err(SymbolError::InvalidSyntax(e)) => {
                warn!("Symbol extraction failed due to syntax error: {}", e);
                return Ok(None);
            }
            Err(SymbolError::NotParsed) => {
                warn!("Symbol extraction failed: document not parsed");
                return Ok(None);
            }
            Err(e) => {
                error!("Symbol extraction failed: {}", e);
                return Ok(None);
            }
        };

        // 6. Enrich with catalog metadata (if available)
        if let Some(cat) = catalog {
            for query in &mut queries {
                let fetcher = SymbolCatalogFetcher::new(cat.clone());
                if let Err(e) = fetcher.populate_columns(&mut query.tables).await {
                    // Log warning but continue with partial results
                    warn!("Failed to populate columns for some tables: {}", e);
                }
            }
        }

        // 7. Render to LSP format
        let document_symbols = SymbolRenderer::render_document(queries);

        info!(
            "Document symbols returned: {} symbols",
            document_symbols.len()
        );

        Ok(Some(DocumentSymbolResponse::Nested(document_symbols)))
    }

    /// Configuration change notification
    ///
    /// Called when the client's configuration changes.
    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        debug!("!!! LSP: did_change_configuration called");
        debug!(
            "!!! LSP: Settings type: {:?}",
            std::any::type_name::<serde_json::Value>()
        );
        debug!("!!! LSP: Settings value: {:?}", params.settings);

        // Parse configuration from client settings
        match self.parse_config_from_settings(&params.settings) {
            Some(config) => {
                debug!(
                    "!!! LSP: Successfully parsed config: dialect={:?}",
                    config.dialect
                );
                self.set_config(config).await;
                debug!("!!! LSP: Engine configuration updated from client settings");
            }
            None => {
                debug!("!!! LSP: Failed to parse configuration from client settings");
            }
        }
    }
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
