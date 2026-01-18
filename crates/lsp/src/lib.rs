// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Unified SQL LSP - Language Server Protocol
//!
//! This crate provides the LSP server implementation for Unified SQL LSP.
//!
//! ## Overview
//!
//! The LSP server provides:
//! - Multi-dialect SQL support (MySQL, PostgreSQL, TiDB)
//! - Real-time completion and diagnostics
//! - Schema-aware intelligence
//! - Multi-client document management
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         Client (VS Code, etc.)          │
//! └──────────────┬──────────────────────────┘
//!                │ LSP Protocol
//!                ↓
//! ┌─────────────────────────────────────────┐
//! │         LSP Backend (tower-lsp)         │
//! ├─────────────────────────────────────────┤
//! │  • did_open / did_change / did_close   │
//! │  • completion / hover / diagnostics     │
//! └──────────────┬──────────────────────────┘
//!                │
//!         ┌──────┴──────┬────────────────┐
//!         ↓             ↓                ↓
//! ┌────────────┐ ┌──────────┐  ┌──────────────┐
//! │   Config   │ │ Document │  │  (Future)    │
//! │   Engine   │ │   Store  │  │  Completion  │
//! └────────────┘ └──────────┘  │  Diagnostics │
//!                              └──────────────┘
//! ```
//!
//! ## Features
//!
//! ### Implemented (LSP-001)
//! - ✅ LSP server backend with tower-lsp integration
//! - ✅ Document management (multi-document support)
//! - ✅ Engine configuration management
//! - ✅ Incremental document synchronization
//!
//! ### Planned (Future Features)
//! - 📋 Completion (LSP-003, COMPLETION-001 through COMPLETION-007)
//! - 🔍 Diagnostics (DIAG-001 through DIAG-005)
//! - 🖱️ Hover support (HOVER-001)
//! - 🎨 Formatting (FORMAT-001)
//! - ⚡ Performance optimization (PERF-001 through PERF-004)
//!
//! ## Usage
//!
//! ### Starting the Server
//!
//! Starting the LSP server requires setting up tower-lsp with stdio transport:
//!
//! ```rust,no_run
//! use unified_sql_lsp_lsp::LspBackend;
//! use tower_lsp::{LspService, Server};
//!
//! #[tokio::main]
//! async fn main() {
//!     let stdin = tokio::io::stdin();
//!     let stdout = tokio::io::stdout();
//!
//!     // Create the LSP service
//!     let (service, socket) = LspService::new(LspBackend::new);
//!
//!     // Run the server
//!     Server::new(stdin, stdout, socket).serve(service).await;
//! }
//! ```
//!
//! ### Configuration
//!
//! The server can be configured through:
//!
//! 1. **Client Settings** (recommended)
//! ```json
//! {
//!   "unifiedSqlLsp.dialect": "mysql",
//!   "unifiedSqlLsp.version": "8.0",
//!   "unifiedSqlLsp.connectionString": "mysql://localhost:3306/mydb"
//! }
//! ```
//!
//! 2. **Configuration File** (CONFIG-001)
//! ```yaml
//! dialect: mysql
//! version: "8.0"
//! connection_string: "mysql://localhost:3306/mydb"
//! schema_filter:
//!   allowed_schemas:
//!     - public
//!     - myapp
//! ```
//!
//! ### Document Lifecycle
//!
//! ```rust,ignore
//! // Documents are automatically managed through LSP notifications
//! // No manual interaction needed in normal operation
//!
//! // For testing or custom integrations:
//! use unified_sql_lsp_lsp::DocumentStore;
//! use lsp_types::Url;
//!
//! let store = DocumentStore::new();
//! let uri = Url::parse("file:///test.sql").unwrap();
//!
//! // Open document
//! store.open_document(uri, content, version, language_id).await;
//!
//! // Get document
//! let doc = store.get_document(&uri).await;
//! ```
//!
//! ## Supported SQL Dialects
//!
//! - **MySQL** (5.7, 8.0) - Full support planned
//! - **PostgreSQL** (12, 14, 16) - Full support planned
//! - **TiDB** (5.0-8.0) - Planned (DIALECT-TIDB-001)
//!
//! ## Modules
//!
//! - [`backend`]: Main LSP server implementation
//! - [`document`]: Document management and storage
//! - [`config`]: Engine configuration and validation
//!
//! ## Error Handling
//!
//! The LSP server uses graceful degradation:
//! - Missing configuration → Use defaults, log warning
//! - Invalid document → Skip, log error
//! - Parse errors → Continue with partial results (TODO: LOWERING-001)
//! - Catalog errors → Cache last successful state (TODO: CATALOG-004)
//!
//! ## Performance Considerations
//!
//! - Documents use Ropey for efficient incremental edits
//! - Catalog queries will be cached (TODO: CATALOG-004)
//! - Semantic analysis will run asynchronously (TODO: PERF-002)
//!
//! ## Testing
//!
//! ```bash
//! # Run unit tests
//! cargo test -p unified-sql-lsp-lsp
//!
//! # Run integration tests (TODO: TEST-002)
//! cargo test --test integration
//! ```

#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod backend;
#[cfg(all(feature = "catalog", not(target_arch = "wasm32")))]
pub mod catalog_manager;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod completion;
#[cfg(feature = "catalog")]
pub mod config;
pub mod core;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod definition;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod diagnostic;
#[cfg(feature = "ropey")]
pub mod document;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod hover;
#[cfg(feature = "parser")]
pub mod parsing;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod symbols;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub mod sync;

// profiling module removed in "drop bench" commit
// TODO: restore if benchmarking is re-added

#[cfg(target_arch = "wasm32")]
pub mod wasm;

// Re-exports for convenience
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use backend::{LspBackend, LspError};
#[cfg(all(feature = "catalog", not(target_arch = "wasm32")))]
pub use catalog_manager::CatalogManager;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use completion::CompletionEngine;
#[cfg(feature = "catalog")]
pub use config::{ConfigError, ConnectionPoolConfig, DialectVersion, EngineConfig, SchemaFilter};
pub use core::LspCore;
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use definition::{
    ColumnDefinition, Definition, DefinitionError, DefinitionFinder, TableDefinition,
};
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use diagnostic::{
    DiagnosticCode, DiagnosticCollector, SqlDiagnostic, node_to_range,
    publish_diagnostics_for_document,
};
#[cfg(feature = "ropey")]
pub use document::{Document, DocumentError, DocumentMetadata, DocumentStore, ParseMetadata};
#[cfg(feature = "parser")]
pub use parsing::{ParseError, ParseResult, ParserManager};
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use symbols::{SymbolBuilder, SymbolCatalogFetcher, SymbolError, SymbolRenderer};
#[cfg(all(feature = "lsp", not(target_arch = "wasm32")))]
pub use sync::DocumentSync;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Server name
pub const SERVER_NAME: &str = "unified-sql-lsp";
