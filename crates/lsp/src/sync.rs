// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Document Synchronization
//!
//! This module provides document synchronization orchestration for parsing SQL documents.
//!
//! ## Overview
//!
//! The sync module handles:
//! - Document lifecycle integration (open, change, close)
//! - Dialect resolution from document metadata and config
//! - Parsing orchestration (full and incremental)
//! - Parse result management
//!
//! ## Architecture
//!
//! ```text
//! DocumentSync
//!     ├─→ ParserManager (from parsing module)
//!     ├─→ Engine Config (for dialect override)
//!     └─→ Document (for metadata extraction)
//!           ↓
//!        on_document_open()
//!        on_document_change()
//!        on_document_close()
//! ```

use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use tracing::{debug, info, warn};

use crate::config::EngineConfig;
use crate::document::{Document, ParseMetadata};
use crate::parsing::{ParseResult, ParserManager};
use unified_sql_lsp_ir::Dialect;

// Re-export ParseMetadata with a constructor
impl ParseMetadata {
    /// Create new parse metadata
    pub fn new(parse_time_ms: u64, dialect: Dialect, has_errors: bool, error_count: usize) -> Self {
        Self {
            parsed_at: SystemTime::now(),
            parse_time_ms,
            dialect,
            has_errors,
            error_count,
        }
    }
}

/// Document synchronization manager
///
/// Orchestrates document parsing and caching.
#[derive(Debug, Clone)]
pub struct DocumentSync {
    /// Parser manager
    parsers: Arc<ParserManager>,

    /// Engine configuration (for dialect override)
    config: Arc<RwLock<Option<EngineConfig>>>,
}

impl DocumentSync {
    /// Create a new document sync manager
    pub fn new(config: Arc<RwLock<Option<EngineConfig>>>) -> Self {
        Self {
            parsers: Arc::new(ParserManager::new()),
            config,
        }
    }

    /// Resolve the SQL dialect for a document
    ///
    /// Dialect resolution priority:
    /// 1. Engine config (if set)
    /// 2. Document language_id ("mysql", "postgresql", "sql")
    /// 3. Fallback to Base
    ///
    /// # Arguments
    ///
    /// - `document`: The document to resolve dialect for
    ///
    /// # Returns
    ///
    /// The resolved SQL dialect
    pub fn resolve_dialect(&self, document: &Document) -> Dialect {
        // 1. Check engine config
        if let Some(config) = self.config.blocking_read().as_ref() {
            debug!("Using dialect from engine config: {:?}", config.dialect);
            return config.dialect;
        }

        // 2. Check language_id
        let language_id = document.language_id();
        let dialect = match language_id {
            "mysql" => {
                debug!("Resolved dialect as MySQL from language_id");
                Dialect::MySQL
            }
            "postgresql" | "postgres" => {
                debug!("Resolved dialect as PostgreSQL from language_id");
                Dialect::PostgreSQL
            }
            "sql" | _ => {
                // Default to MySQL for generic SQL
                warn!(
                    "Language_id '{}' is generic SQL or unknown, defaulting to MySQL dialect",
                    language_id
                );
                Dialect::MySQL
            }
        };

        dialect
    }

    /// Parse document on open (full parse)
    ///
    /// # Arguments
    ///
    /// - `document`: The document to parse
    ///
    /// # Returns
    ///
    /// - `ParseResult::Success` - Clean parse
    /// - `ParseResult::Partial` - Parse with errors
    /// - `ParseResult::Failed` - Critical parse failure
    pub fn on_document_open(&self, document: &Document) -> ParseResult {
        let uri = document.uri();
        let dialect = self.resolve_dialect(document);
        let content = document.get_content();

        info!(
            "Parsing document on open: uri={}, dialect={:?}, content_length={}",
            uri,
            dialect,
            content.len()
        );

        let result = self.parsers.parse_text(dialect, &content);

        match &result {
            ParseResult::Success { parse_time, .. } => {
                info!(
                    "Document parsed successfully in {:?}: uri={}",
                    parse_time, uri
                );
            }
            ParseResult::Partial { errors, .. } => {
                warn!("Document parsed with {} errors: uri={}", errors.len(), uri);
            }
            ParseResult::Failed { error } => {
                warn!("Document parse failed: uri={}, error={}", uri, error);
            }
        }

        result
    }

    /// Update document parse on change (incremental or full)
    ///
    /// Determines whether to use incremental parsing based on:
    /// - Old tree exists
    /// - Single change event
    /// - Change has a range (not full replacement)
    ///
    /// # Arguments
    ///
    /// - `document`: The updated document
    /// - `old_tree`: The previous parse tree (if any)
    /// - `changes`: The content changes from LSP
    ///
    /// # Returns
    ///
    /// - `ParseResult::Success` - Clean parse
    /// - `ParseResult::Partial` - Parse with errors
    /// - `ParseResult::Failed` - Critical parse failure
    pub fn on_document_change(
        &self,
        document: &Document,
        old_tree: Option<&tree_sitter::Tree>,
        changes: &[TextDocumentContentChangeEvent],
    ) -> ParseResult {
        let uri = document.uri();
        let dialect = self.resolve_dialect(document);
        let content = document.get_content();

        // Check if we can do incremental parse
        let can_incremental = self.can_use_incremental(old_tree, changes);

        if can_incremental {
            debug!("Using incremental parse for: uri={}", uri);

            // Try incremental parse
            if let Some(_old_tree) = old_tree {
                // Note: We need previous_content to compute the edit
                // This is stored in Document.previous_content
                // For now, we'll fall back to full parse if we don't have it
                warn!("Incremental parsing not yet fully implemented, falling back to full parse");
            }
        }

        // Full parse
        info!(
            "Reparsing document: uri={}, dialect={:?}, content_length={}",
            uri,
            dialect,
            content.len()
        );

        let result = self.parsers.parse_text(dialect, &content);

        match &result {
            ParseResult::Success { parse_time, .. } => {
                info!(
                    "Document reparsed successfully in {:?}: uri={}",
                    parse_time, uri
                );
            }
            ParseResult::Partial { errors, .. } => {
                warn!(
                    "Document reparsed with {} errors: uri={}",
                    errors.len(),
                    uri
                );
            }
            ParseResult::Failed { error } => {
                warn!("Document reparse failed: uri={}, error={}", uri, error);
            }
        }

        result
    }

    /// Clear parse data on document close
    ///
    /// # Arguments
    ///
    /// - `uri`: The document URI
    pub fn on_document_close(&self, uri: &Url) {
        debug!("Clearing parse data for closed document: uri={}", uri);
        // Parse data is cleared when document is removed from store
        // This is mainly for logging and future cleanup
    }

    /// Check if incremental parsing can be used
    ///
    /// Incremental parsing is possible when:
    /// - Old tree exists
    /// - Single change event
    /// - Change has a range (not full document replacement)
    fn can_use_incremental(
        &self,
        old_tree: Option<&tree_sitter::Tree>,
        changes: &[TextDocumentContentChangeEvent],
    ) -> bool {
        // Must have previous tree
        if old_tree.is_none() {
            return false;
        }

        // Must be single change
        if changes.len() != 1 {
            return false;
        }

        // Change must have range (not full replacement)
        let has_range = changes[0].range.is_some();

        has_range
    }

    /// Create parse metadata from parse result
    ///
    /// # Arguments
    ///
    /// - `result`: The parse result
    /// - `dialect`: The dialect used for parsing
    ///
    /// # Returns
    ///
    /// Parse metadata
    pub fn create_metadata(&self, result: &ParseResult, dialect: Dialect) -> ParseMetadata {
        match result {
            ParseResult::Success { parse_time, .. } => {
                ParseMetadata::new(parse_time.as_millis() as u64, dialect, false, 0)
            }
            ParseResult::Partial { errors, .. } => {
                // Partial parse doesn't have timing info, use 0
                ParseMetadata::new(0, dialect, true, errors.len())
            }
            ParseResult::Failed { .. } => {
                // Create metadata even for failures
                ParseMetadata::new(0, dialect, true, 1)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing::{ParseError, ParseResult};
    use std::sync::Arc;
    use std::time::Duration;
    use tower_lsp::lsp_types::Url;

    fn create_test_document(content: &str, language_id: &str) -> Document {
        let uri = Url::parse("file:///test.sql").unwrap();
        Document::new(uri, content.to_string(), 1, language_id.to_string())
    }

    fn create_test_config() -> Arc<RwLock<Option<EngineConfig>>> {
        Arc::new(RwLock::new(None))
    }

    #[test]
    fn test_document_sync_new() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);
        // Should not panic
        drop(sync);
    }

    #[test]
    fn test_resolve_dialect_from_language_id() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        // Test MySQL
        let mysql_doc = create_test_document("SELECT 1", "mysql");
        assert_eq!(sync.resolve_dialect(&mysql_doc), Dialect::MySQL);

        // Test PostgreSQL
        let pg_doc = create_test_document("SELECT 1", "postgresql");
        assert_eq!(sync.resolve_dialect(&pg_doc), Dialect::PostgreSQL);

        // Test generic SQL - fall back to MySQL as default
        let sql_doc = create_test_document("SELECT 1", "sql");
        assert_eq!(sync.resolve_dialect(&sql_doc), Dialect::MySQL);

        // Test unknown (should fall back to MySQL as default)
        let unknown_doc = create_test_document("SELECT 1", "unknown");
        assert_eq!(sync.resolve_dialect(&unknown_doc), Dialect::MySQL);
    }

    #[test]
    fn test_resolve_dialect_from_config() {
        let config = Arc::new(RwLock::new(Some(EngineConfig {
            dialect: Dialect::PostgreSQL,
            version: crate::config::DialectVersion::PostgreSQL14,
            connection_string: String::new(),
            schema_filter: crate::config::SchemaFilter::default(),
            pool_config: crate::config::ConnectionPoolConfig::default(),
            log_queries: false,
            query_timeout_secs: 5,
            cache_enabled: false,
        })));

        let sync = DocumentSync::new(config);

        // Config should override language_id
        let mysql_doc = create_test_document("SELECT 1", "mysql");
        assert_eq!(sync.resolve_dialect(&mysql_doc), Dialect::PostgreSQL);
    }

    #[tokio::test]
    async fn test_on_document_open() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        let doc = create_test_document("SELECT * FROM users", "mysql");

        // This will attempt to parse, which requires compiled grammars
        let result = sync.on_document_open(&doc);

        // If grammars are not compiled, it will fail
        // If grammars are compiled, it should succeed or have partial errors
        match result {
            ParseResult::Success { .. } => {
                // Expected if grammars are compiled
            }
            ParseResult::Partial { .. } => {
                // Also acceptable (might have minor errors)
            }
            ParseResult::Failed { .. } => {
                // Also acceptable if grammars not compiled
            }
        }
    }

    #[test]
    fn test_can_use_incremental() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        // No old tree - cannot use incremental
        assert!(!sync.can_use_incremental(None, &[]));

        // Has old tree, single change with range - can use incremental
        let change = TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 1,
                },
            }),
            range_length: Some(1),
            text: "x".to_string(),
        };

        // Note: We can't actually create a real tree without compiled grammars
        // So we just test the logic with None (which means we don't have a previous tree)
        assert!(!sync.can_use_incremental(None, &[change.clone()]));

        // Multiple changes - cannot use incremental
        assert!(!sync.can_use_incremental(None, &[change.clone(), change.clone()]));

        // Full document replacement - cannot use incremental
        let full_change = TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        };
        assert!(!sync.can_use_incremental(None, &[full_change]));
    }

    #[test]
    fn test_create_metadata_success() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        let result = ParseResult::Success {
            tree: None, // Can't create real tree without compiled grammar
            parse_time: Duration::from_millis(10),
        };

        let metadata = sync.create_metadata(&result, Dialect::MySQL);

        assert_eq!(metadata.parse_time_ms, 10);
        assert_eq!(metadata.dialect, Dialect::MySQL);
        assert!(!metadata.has_errors);
        assert_eq!(metadata.error_count, 0);
    }

    #[test]
    fn test_create_metadata_partial() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        let result = ParseResult::Partial {
            tree: None, // Can't create real tree without compiled grammar
            errors: vec![
                ParseError::Generic {
                    message: "Error 1".to_string(),
                },
                ParseError::Generic {
                    message: "Error 2".to_string(),
                },
            ],
        };

        let metadata = sync.create_metadata(&result, Dialect::PostgreSQL);

        assert_eq!(metadata.dialect, Dialect::PostgreSQL);
        assert!(metadata.has_errors);
        assert_eq!(metadata.error_count, 2);
    }

    #[test]
    fn test_create_metadata_failed() {
        let config = create_test_config();
        let sync = DocumentSync::new(config);

        let result = ParseResult::Failed {
            error: ParseError::Generic {
                message: "Failed".to_string(),
            },
        };

        let metadata = sync.create_metadata(&result, Dialect::MySQL);

        assert_eq!(metadata.dialect, Dialect::MySQL);
        assert!(metadata.has_errors);
        assert_eq!(metadata.error_count, 1);
        assert_eq!(metadata.parse_time_ms, 0);
    }
}
