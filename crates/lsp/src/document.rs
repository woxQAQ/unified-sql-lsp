// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Document Management
//!
//! This module provides document management for the LSP server.
//!
//! ## Overview
//!
//! The document manager handles:
//! - Multiple open documents across different client connections
//! - Document synchronization (open, change, close)
//! - Text content management using Ropey for efficient edits
//! - Document metadata (language ID, version, URI)
//!
//! ## Architecture
//!
//! Documents are identified by their URI and support:
//! - Incremental text updates
//! - Multi-client scenarios (different connections to the same server)
//! - Thread-safe access
//!
//! ## Example
//!
//! ```rust,ignore
//! use unified_sql_lsp_lsp::{DocumentStore, Document};
//! use lsp_types::Url;
//!
//! let store = DocumentStore::new();
//! let uri = Url::parse("file:///test.sql").unwrap();
//!
//! // Open a document
//! store.open_document(
//!     uri.clone(),
//!     "SELECT * FROM users",
//!     1,
//!     "sql"
//! );
//!
//! // Get document
//! if let Some(doc) = store.get_document(&uri) {
//!     println!("Content: {}", doc.get_content());
//! }
//! ```

use ropey::Rope;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tower_lsp::lsp_types::{TextDocumentContentChangeEvent, Url, VersionedTextDocumentIdentifier};

/// Parse metadata
///
/// Contains information about a document parsing operation.
/// This type is defined in the document module and used by both
/// document and sync modules to track parse results.
#[derive(Debug, Clone)]
pub struct ParseMetadata {
    /// When the document was parsed
    pub parsed_at: std::time::SystemTime,

    /// Time taken to parse (milliseconds)
    pub parse_time_ms: u64,

    /// SQL dialect used for parsing
    pub dialect: unified_sql_lsp_ir::Dialect,

    /// Whether the parse had errors
    pub has_errors: bool,

    /// Number of parse errors
    pub error_count: usize,
}

/// Document metadata
///
/// Contains information about an open document.
#[derive(Debug, Clone)]
pub struct DocumentMetadata {
    /// Document URI
    pub uri: Url,

    /// Language identifier (e.g., "sql", "mysql", "postgresql")
    pub language_id: String,

    /// Document version
    /// Incremented on each change
    pub version: i32,

    /// Line count
    pub line_count: usize,
}

impl DocumentMetadata {
    /// Create new document metadata
    pub fn new(uri: Url, language_id: String, version: i32, line_count: usize) -> Self {
        Self {
            uri,
            language_id,
            version,
            line_count,
        }
    }
}

/// A document managed by the LSP server
///
/// Contains the document's content and metadata.
/// Uses Ropey for efficient text manipulation.
#[derive(Debug, Clone)]
pub struct Document {
    /// Document metadata
    metadata: DocumentMetadata,

    /// Document content as a rope for efficient editing
    content: Rope,

    /// Parsed syntax tree (if available)
    tree: Option<Arc<Mutex<tree_sitter::Tree>>>,

    /// Parse metadata (if parsing has occurred)
    parse_metadata: Option<Arc<ParseMetadata>>,

    /// Previous content (for computing incremental edits)
    previous_content: Option<Rope>,
}

impl Document {
    /// Create a new document
    pub fn new(uri: Url, content: String, version: i32, language_id: String) -> Self {
        let rope = Rope::from_str(&content);
        let line_count = rope.len_lines();

        let metadata = DocumentMetadata::new(uri, language_id, version, line_count);

        Self {
            metadata,
            content: rope,
            tree: None,
            parse_metadata: None,
            previous_content: None,
        }
    }

    /// Get the document URI
    pub fn uri(&self) -> &Url {
        &self.metadata.uri
    }

    /// Get the document language ID
    pub fn language_id(&self) -> &str {
        &self.metadata.language_id
    }

    /// Get the document version
    pub fn version(&self) -> i32 {
        self.metadata.version
    }

    /// Get the line count
    pub fn line_count(&self) -> usize {
        self.metadata.line_count
    }

    /// Get the full document content as a string
    pub fn get_content(&self) -> String {
        self.content.to_string()
    }

    /// Get a line of text
    ///
    /// # Arguments
    ///
    /// - `line`: The line number (0-indexed)
    ///
    /// # Returns
    ///
    /// The line content without the line ending
    pub fn get_line(&self, line: usize) -> Option<String> {
        if line >= self.line_count() {
            return None;
        }

        // ropey's line() includes the line ending, so we need to strip it
        let line_with_ending = self.content.line(line).to_string();
        Some(line_with_ending.trim_end_matches(['\r', '\n']).to_string())
    }

    /// Get text in a range
    ///
    /// # Arguments
    ///
    /// - `start_line`: Start line (0-indexed)
    /// - `start_col`: Start column (0-indexed, UTF-8)
    /// - `end_line`: End line (0-indexed)
    /// - `end_col`: End column (0-indexed, UTF-8)
    ///
    /// # Returns
    ///
    /// The text in the specified range
    pub fn get_text(
        &self,
        start_line: usize,
        start_col: usize,
        end_line: usize,
        end_col: usize,
    ) -> Option<String> {
        if start_line > end_line || start_line >= self.line_count() || end_line >= self.line_count()
        {
            return None;
        }

        let start_char = self.content.line_to_char(start_line) + start_col;
        let end_char = self.content.line_to_char(end_line) + end_col;

        if start_char > end_char || end_char > self.content.len_chars() {
            return None;
        }

        Some(self.content.slice(start_char..end_char).to_string())
    }

    /// Get the character offset for a line and column
    ///
    /// # Arguments
    ///
    /// - `line`: Line number (0-indexed)
    /// - `col`: Column number (0-indexed, UTF-8)
    ///
    /// # Returns
    ///
    /// The character offset, or None if the position is invalid
    pub fn offset(&self, line: usize, col: usize) -> Option<usize> {
        if line >= self.line_count() {
            return None;
        }

        let line_start = self.content.line_to_char(line);
        let line_end = self.content.line_to_char(line + 1);

        let offset = line_start + col;
        if offset > line_end {
            return None;
        }

        Some(offset)
    }

    /// Apply content changes to the document
    ///
    /// # Arguments
    ///
    /// - `changes`: List of content changes
    /// - `new_version`: New document version
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err otherwise
    pub fn apply_changes(
        &mut self,
        changes: &[TextDocumentContentChangeEvent],
        new_version: i32,
    ) -> Result<(), DocumentError> {
        for change in changes {
            match (&change.range, &change.range_length) {
                (Some(range), Some(_range_length)) => {
                    // Incremental change
                    let start_line = range.start.line as usize;
                    let start_col = range.start.character as usize;
                    let end_line = range.end.line as usize;
                    let end_col = range.end.character as usize;

                    // Validate range
                    if start_line >= self.line_count() || end_line >= self.line_count() {
                        return Err(DocumentError::InvalidRange {
                            start: (start_line, start_col),
                            end: (end_line, end_col),
                        });
                    }

                    let start_char = self.content.line_to_char(start_line) + start_col;
                    let end_char = self.content.line_to_char(end_line) + end_col;

                    // Validate character offsets
                    if start_char > end_char || end_char > self.content.len_chars() {
                        return Err(DocumentError::InvalidRange {
                            start: (start_line, start_col),
                            end: (end_line, end_col),
                        });
                    }

                    // Apply the change
                    self.content.remove(start_char..end_char);
                    self.content.insert(start_char, &change.text);
                }
                (None, None) => {
                    // Full document change
                    self.content = Rope::from_str(&change.text);
                }
                _ => {
                    // Invalid combination (range without range_length or vice versa)
                    return Err(DocumentError::InvalidChange);
                }
            }
        }

        // Update metadata
        self.metadata.version = new_version;
        self.metadata.line_count = self.content.len_lines();

        Ok(())
    }

    /// Get document metadata
    pub fn metadata(&self) -> &DocumentMetadata {
        &self.metadata
    }

    /// Get the parsed tree (if available)
    pub fn tree(&self) -> Option<Arc<Mutex<tree_sitter::Tree>>> {
        self.tree.clone()
    }

    /// Update the parsed tree
    pub fn set_tree(&mut self, tree: tree_sitter::Tree, metadata: ParseMetadata) {
        self.tree = Some(Arc::new(Mutex::new(tree)));
        self.parse_metadata = Some(Arc::new(metadata));
    }

    /// Clear the parsed tree
    pub fn clear_tree(&mut self) {
        self.tree = None;
        self.parse_metadata = None;
    }

    /// Get parse metadata
    pub fn parse_metadata(&self) -> Option<&ParseMetadata> {
        self.parse_metadata.as_deref()
    }

    /// Get previous content (for incremental edits)
    pub fn previous_content(&self) -> Option<&Rope> {
        self.previous_content.as_ref()
    }

    /// Store current content as previous content
    pub fn store_previous_content(&mut self) {
        self.previous_content = Some(self.content.clone());
    }
}

/// Document store for managing multiple documents
///
/// Thread-safe store for all open documents across all client connections.
#[derive(Debug, Default)]
pub struct DocumentStore {
    /// Map of document URI to document
    documents: Arc<RwLock<HashMap<Url, Document>>>,
}

impl DocumentStore {
    /// Create a new document store
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a document
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    /// - `content`: Initial document content
    /// - `version`: Document version
    /// - `language_id`: Language identifier
    pub async fn open_document(
        &self,
        uri: Url,
        content: String,
        version: i32,
        language_id: String,
    ) -> Result<(), DocumentError> {
        let mut docs = self.documents.write().await;

        let document = Document::new(uri.clone(), content, version, language_id);

        docs.insert(uri, document);

        Ok(())
    }

    /// Close a document
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    ///
    /// # Returns
    ///
    /// true if the document was closed, false if it didn't exist
    pub async fn close_document(&self, uri: &Url) -> bool {
        let mut docs = self.documents.write().await;
        docs.remove(uri).is_some()
    }

    /// Update a document
    ///
    /// # Arguments
    ///
    /// - `identifier`: Document identifier with version
    /// - `changes`: Content changes
    pub async fn update_document(
        &self,
        identifier: &VersionedTextDocumentIdentifier,
        changes: &[TextDocumentContentChangeEvent],
    ) -> Result<(), DocumentError> {
        let mut docs = self.documents.write().await;

        let document = docs
            .get_mut(&identifier.uri)
            .ok_or_else(|| DocumentError::DocumentNotFound(identifier.uri.clone()))?;

        document.apply_changes(changes, identifier.version)?;

        Ok(())
    }

    /// Get a document by URI
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    ///
    /// # Returns
    ///
    /// The document if it exists, None otherwise
    pub async fn get_document(&self, uri: &Url) -> Option<Document> {
        let docs = self.documents.read().await;
        docs.get(uri).cloned()
    }

    /// Check if a document exists
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    ///
    /// # Returns
    ///
    /// true if the document exists, false otherwise
    pub async fn has_document(&self, uri: &Url) -> bool {
        let docs = self.documents.read().await;
        docs.contains_key(uri)
    }

    /// Get all document URIs
    ///
    /// # Returns
    ///
    /// List of all document URIs
    pub async fn list_uris(&self) -> Vec<Url> {
        let docs = self.documents.read().await;
        docs.keys().cloned().collect()
    }

    /// Get the number of open documents
    pub async fn document_count(&self) -> usize {
        let docs = self.documents.read().await;
        docs.len()
    }

    /// Update document's parsed tree
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    /// - `tree`: Parsed syntax tree
    /// - `metadata`: Parse metadata
    pub async fn update_document_tree(
        &self,
        uri: &Url,
        tree: tree_sitter::Tree,
        metadata: ParseMetadata,
    ) -> Result<(), DocumentError> {
        let mut docs = self.documents.write().await;
        let doc = docs
            .get_mut(uri)
            .ok_or_else(|| DocumentError::DocumentNotFound(uri.clone()))?;
        doc.set_tree(tree, metadata);
        Ok(())
    }

    /// Clear document's parsed tree
    ///
    /// # Arguments
    ///
    /// - `uri`: Document URI
    pub async fn clear_document_tree(&self, uri: &Url) -> Result<(), DocumentError> {
        let mut docs = self.documents.write().await;
        let doc = docs
            .get_mut(uri)
            .ok_or_else(|| DocumentError::DocumentNotFound(uri.clone()))?;
        doc.clear_tree();
        Ok(())
    }
}

/// Document-related errors
#[derive(Debug, thiserror::Error)]
pub enum DocumentError {
    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(Url),

    /// Invalid range for text operation
    #[error("Invalid range: start={start:?}, end={end:?}")]
    InvalidRange {
        start: (usize, usize),
        end: (usize, usize),
    },

    /// Invalid content change
    #[error("Invalid content change")]
    InvalidChange,

    /// Lock poisoned
    #[error("Document store lock poisoned")]
    LockPoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types;

    fn create_test_uri() -> Url {
        Url::parse("file:///test.sql").unwrap()
    }

    #[test]
    fn test_document_new() {
        let uri = create_test_uri();
        let doc = Document::new(
            uri.clone(),
            "SELECT * FROM users".to_string(),
            1,
            "sql".to_string(),
        );

        assert_eq!(doc.uri(), &uri);
        assert_eq!(doc.language_id(), "sql");
        assert_eq!(doc.version(), 1);
        assert_eq!(doc.get_content(), "SELECT * FROM users");
    }

    #[test]
    fn test_document_get_line() {
        let uri = create_test_uri();
        let content = "SELECT *\nFROM users\nWHERE id = 1";
        let doc = Document::new(uri, content.to_string(), 1, "sql".to_string());

        assert_eq!(doc.get_line(0), Some("SELECT *".to_string()));
        assert_eq!(doc.get_line(1), Some("FROM users".to_string()));
        assert_eq!(doc.get_line(2), Some("WHERE id = 1".to_string()));
        assert_eq!(doc.get_line(3), None);
    }

    #[test]
    fn test_document_get_text() {
        let uri = create_test_uri();
        let content = "SELECT *\nFROM users";
        let doc = Document::new(uri, content.to_string(), 1, "sql".to_string());

        assert_eq!(doc.get_text(0, 7, 0, 8), Some("*".to_string()));
        assert_eq!(doc.get_text(1, 0, 1, 4), Some("FROM".to_string()));
    }

    #[test]
    fn test_document_offset() {
        let uri = create_test_uri();
        let content = "SELECT *\nFROM users";
        let doc = Document::new(uri, content.to_string(), 1, "sql".to_string());

        // "SELECT *" = 8 chars
        assert_eq!(doc.offset(0, 0), Some(0));
        assert_eq!(doc.offset(0, 7), Some(7));
        assert_eq!(doc.offset(1, 0), Some(9)); // After newline
        assert_eq!(doc.offset(1, 4), Some(13));
        assert_eq!(doc.offset(2, 0), None); // Past end
    }

    #[test]
    fn test_document_apply_changes_full() {
        let uri = create_test_uri();
        let mut doc = Document::new(uri, "old content".to_string(), 1, "sql".to_string());

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        }];

        doc.apply_changes(&changes, 2).unwrap();

        assert_eq!(doc.get_content(), "new content");
        assert_eq!(doc.version(), 2);
    }

    #[test]
    fn test_document_apply_changes_incremental() {
        let uri = create_test_uri();
        let mut doc = Document::new(uri, "SELECT * FROM users".to_string(), 1, "sql".to_string());

        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 7,
                },
                end: lsp_types::Position {
                    line: 0,
                    character: 8,
                },
            }),
            range_length: Some(1),
            text: "id".to_string(),
        }];

        doc.apply_changes(&changes, 2).unwrap();

        assert_eq!(doc.get_content(), "SELECT id FROM users");
        assert_eq!(doc.version(), 2);
    }

    #[test]
    fn test_document_apply_changes_invalid_range() {
        let uri = create_test_uri();
        let mut doc = Document::new(uri, "SELECT *".to_string(), 1, "sql".to_string());

        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(lsp_types::Range {
                start: lsp_types::Position {
                    line: 0,
                    character: 0,
                },
                end: lsp_types::Position {
                    line: 10, // Past end
                    character: 0,
                },
            }),
            range_length: Some(1),
            text: "x".to_string(),
        }];

        let result = doc.apply_changes(&changes, 2);
        assert!(matches!(result, Err(DocumentError::InvalidRange { .. })));
    }

    #[tokio::test]
    async fn test_document_store_open() {
        let store = DocumentStore::new();
        let uri = create_test_uri();

        store
            .open_document(uri.clone(), "SELECT *".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        assert!(store.has_document(&uri).await);
        assert_eq!(store.document_count().await, 1);
    }

    #[tokio::test]
    async fn test_document_store_close() {
        let store = DocumentStore::new();
        let uri = create_test_uri();

        store
            .open_document(uri.clone(), "SELECT *".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        assert!(store.close_document(&uri).await);
        assert!(!store.has_document(&uri).await);
        assert_eq!(store.document_count().await, 0);
    }

    #[tokio::test]
    async fn test_document_store_get() {
        let store = DocumentStore::new();
        let uri = create_test_uri();

        store
            .open_document(uri.clone(), "SELECT *".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        let doc = store.get_document(&uri).await;
        assert!(doc.is_some());

        let doc = doc.unwrap();
        assert_eq!(doc.get_content(), "SELECT *");
        assert_eq!(doc.language_id(), "sql");
    }

    #[tokio::test]
    async fn test_document_store_update() {
        let store = DocumentStore::new();
        let uri = create_test_uri();

        store
            .open_document(uri.clone(), "old".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        let identifier = VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        };

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new".to_string(),
        }];

        store.update_document(&identifier, &changes).await.unwrap();

        let doc = store.get_document(&uri).await.unwrap();
        assert_eq!(doc.get_content(), "new");
        assert_eq!(doc.version(), 2);
    }

    #[tokio::test]
    async fn test_document_store_list_uris() {
        let store = DocumentStore::new();
        let uri1 = Url::parse("file:///test1.sql").unwrap();
        let uri2 = Url::parse("file:///test2.sql").unwrap();

        store
            .open_document(uri1.clone(), "SELECT 1".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        store
            .open_document(uri2.clone(), "SELECT 2".to_string(), 1, "sql".to_string())
            .await
            .unwrap();

        let uris = store.list_uris().await;
        assert_eq!(uris.len(), 2);
        assert!(uris.contains(&uri1));
        assert!(uris.contains(&uri2));
    }
}
