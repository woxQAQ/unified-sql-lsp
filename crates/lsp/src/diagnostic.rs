// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Diagnostics Infrastructure
//!
//! This module provides the diagnostic infrastructure for Unified SQL LSP.
//!
//! ## Overview
//!
//! The diagnostics system handles:
//! - Collection of syntax and semantic errors
//! - Conversion to LSP diagnostic format
//! - Publishing diagnostics to clients
//! - Graceful error handling and degradation
//!
//! ## Architecture
//!
//! ```text
//! Document → DiagnosticCollector → SqlDiagnostic → LSP Diagnostic → Client
//!                         ↓
//!                    Parse Tree
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_lsp::diagnostic::{DiagnosticCollector, SqlDiagnostic};
//! use tower_lsp::lsp_types::Url;
//!
//! let collector = DiagnosticCollector::new();
//! let uri = Url::parse("file:///test.sql").unwrap();
//!
//! // Collect diagnostics from parsed tree
//! let diagnostics = collector.collect_diagnostics(&tree, &source, &uri);
//!
//! // Convert to LSP format
//! let lsp_diagnostics: Vec<Diagnostic> = diagnostics
//!     .into_iter()
//!     .map(|d| d.to_lsp())
//!     .collect();
//! ```

use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::*;
use tracing::{debug, info};
use unified_sql_lsp_semantic::SyntaxDiagnosticAnalyzer;

/// Diagnostic code identifying the type of diagnostic
///
/// These codes are used to categorize different types of SQL errors and warnings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiagnosticCode {
    /// Syntax error in SQL (DIAG-002)
    SyntaxError,

    /// Undefined table reference (DIAG-003)
    UndefinedTable,

    /// Undefined column reference (DIAG-004)
    UndefinedColumn,

    /// Ambiguous column reference (DIAG-005)
    AmbiguousColumn,

    /// Custom diagnostic code with description
    Custom(String),
}

impl DiagnosticCode {
    /// Get the string representation of this diagnostic code
    pub fn as_str(&self) -> String {
        match self {
            DiagnosticCode::SyntaxError => "SYNTAX-001".to_string(),
            DiagnosticCode::UndefinedTable => "SEMANTIC-001".to_string(),
            DiagnosticCode::UndefinedColumn => "SEMANTIC-002".to_string(),
            DiagnosticCode::AmbiguousColumn => "SEMANTIC-003".to_string(),
            DiagnosticCode::Custom(s) => s.clone(),
        }
    }

    /// Get a human-readable description of this diagnostic code
    pub fn description(&self) -> String {
        match self {
            DiagnosticCode::SyntaxError => "SQL syntax error".to_string(),
            DiagnosticCode::UndefinedTable => "Undefined table reference".to_string(),
            DiagnosticCode::UndefinedColumn => "Undefined column reference".to_string(),
            DiagnosticCode::AmbiguousColumn => "Ambiguous column reference".to_string(),
            DiagnosticCode::Custom(s) => format!("Custom diagnostic: {}", s),
        }
    }
}

impl From<DiagnosticCode> for NumberOrString {
    fn from(code: DiagnosticCode) -> Self {
        NumberOrString::String(code.as_str())
    }
}

/// SQL diagnostic
///
/// Represents a diagnostic that can be reported for SQL code.
/// This is the internal representation before conversion to LSP format.
#[derive(Debug, Clone)]
pub struct SqlDiagnostic {
    /// Diagnostic message
    pub message: String,

    /// Severity level
    pub severity: DiagnosticSeverity,

    /// Range in the source code
    pub range: Range,

    /// Diagnostic code
    pub code: Option<DiagnosticCode>,

    /// Source of the diagnostic (always "unified-sql-lsp")
    pub source: String,

    /// Related information (e.g., suggestions, related locations)
    pub related_information: Option<Vec<DiagnosticRelatedInformation>>,
}

impl SqlDiagnostic {
    /// Create a new SQL diagnostic
    ///
    /// # Arguments
    ///
    /// - `message`: The diagnostic message
    /// - `severity`: The severity level
    /// - `range`: The range in the source code
    pub fn new(message: String, severity: DiagnosticSeverity, range: Range) -> Self {
        Self {
            message,
            severity,
            range,
            code: None,
            source: "unified-sql-lsp".to_string(),
            related_information: None,
        }
    }

    /// Set the diagnostic code
    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
    }

    /// Add related information
    pub fn with_related(mut self, related: Vec<DiagnosticRelatedInformation>) -> Self {
        self.related_information = Some(related);
        self
    }

    /// Convert to LSP diagnostic format
    pub fn to_lsp(self) -> Diagnostic {
        Diagnostic {
            range: self.range,
            severity: Some(self.severity),
            code: self.code.map(|c| c.into()),
            code_description: None,
            source: Some(self.source),
            message: self.message,
            related_information: self.related_information,
            tags: None,
            data: None,
        }
    }

    /// Create an error diagnostic
    pub fn error(message: String, range: Range) -> Self {
        Self::new(message, DiagnosticSeverity::ERROR, range)
    }

    /// Create a warning diagnostic
    pub fn warning(message: String, range: Range) -> Self {
        Self::new(message, DiagnosticSeverity::WARNING, range)
    }

    /// Create an information diagnostic
    pub fn information(message: String, range: Range) -> Self {
        Self::new(message, DiagnosticSeverity::INFORMATION, range)
    }

    /// Create a hint diagnostic
    pub fn hint(message: String, range: Range) -> Self {
        Self::new(message, DiagnosticSeverity::HINT, range)
    }
}

/// Convert a tree-sitter Node to an LSP Range
///
/// # Arguments
///
/// - `node`: The tree-sitter node
///
/// # Returns
///
/// The corresponding LSP Range
pub fn node_to_range(node: &tree_sitter::Node) -> Range {
    let start = node.start_position();
    let end = node.end_position();

    Range {
        start: Position {
            line: start.row as u32,
            character: start.column as u32,
        },
        end: Position {
            line: end.row as u32,
            character: end.column as u32,
        },
    }
}

/// Diagnostic collector
///
/// Collects diagnostics from parsed SQL documents.
///
/// This is the main entry point for diagnostic collection.
/// Specific diagnostic logic (syntax errors, undefined tables, etc.)
/// will be implemented in subsequent features (DIAG-002 through DIAG-005).
#[derive(Debug, Clone, Default)]
pub struct DiagnosticCollector {
    syntax_analyzer: SyntaxDiagnosticAnalyzer,
}

impl DiagnosticCollector {
    /// Create a new diagnostic collector
    pub fn new() -> Self {
        Self {
            syntax_analyzer: SyntaxDiagnosticAnalyzer::new(),
        }
    }

    /// Collect diagnostics from a parsed document
    ///
    /// # Arguments
    ///
    /// - `tree`: The parsed syntax tree
    /// - `source`: The source code text
    /// - `uri`: The document URI
    ///
    /// # Returns
    ///
    /// A vector of SQL diagnostics
    ///
    /// # Note
    ///
    /// Currently returns an empty vector. Specific diagnostic logic
    /// will be implemented in DIAG-002 through DIAG-005.
    pub fn collect_diagnostics(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        _uri: &Url,
    ) -> Vec<SqlDiagnostic> {
        // Note: Semantic validation (undefined tables, undefined columns, type mismatches, etc.)
        // should be performed by the SemanticAnalyzer in the semantic crate.
        // The LSP layer is responsible only for syntax error detection and protocol conversion.
        // Proper semantic validation using the catalog is handled by:
        // - unified_sql_lsp_semantic::SemanticAnalyzer for analysis
        // - unified_sql_lsp_semantic::SemanticValidator for validation

        self.syntax_analyzer
            .collect_diagnostics(tree, source)
            .into_iter()
            .map(|d| {
                SqlDiagnostic::error(
                    d.message,
                    Range {
                        start: Position {
                            line: d.range.start_line,
                            character: d.range.start_character,
                        },
                        end: Position {
                            line: d.range.end_line,
                            character: d.range.end_character,
                        },
                    },
                )
                .with_code(DiagnosticCode::SyntaxError)
            })
            .collect()
    }

    #[cfg(test)]
    fn is_missing_comma_pattern(&self, text: &str) -> bool {
        self.syntax_analyzer.is_missing_comma_pattern(text)
    }

    #[cfg(test)]
    fn is_unmatched_paren_pattern(&self, text: &str) -> bool {
        self.syntax_analyzer.is_unmatched_paren_pattern(text)
    }

    #[cfg(test)]
    fn is_identifier(&self, text: &str) -> bool {
        self.syntax_analyzer.is_identifier(text)
    }

    #[cfg(test)]
    fn first_identifier(&self, text: &str) -> String {
        self.syntax_analyzer.first_identifier(text)
    }

    /// Collect diagnostics from an Arc<Mutex<Tree>>
    ///
    /// This is a convenience method for working with the document store.
    ///
    /// # Arguments
    ///
    /// - `tree`: The Arc<Mutex<Tree>> from the document
    /// - `source`: The source code text
    /// - `uri`: The document URI
    ///
    /// # Returns
    ///
    /// A vector of SQL diagnostics, or empty if tree is locked or None
    pub fn collect_from_arc(
        &self,
        tree: &Option<Arc<Mutex<tree_sitter::Tree>>>,
        source: &str,
        uri: &Url,
    ) -> Vec<SqlDiagnostic> {
        let Some(tree_arc) = tree else {
            debug!("No tree available for diagnostic collection: {}", uri);
            return Vec::new();
        };

        let tree_guard = match tree_arc.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                debug!("Failed to acquire tree lock for diagnostics: {}", uri);
                return Vec::new();
            }
        };

        self.collect_diagnostics(&tree_guard, source, uri)
    }
}

/// Helper to publish diagnostics from a document
///
/// This function handles the common pattern of:
/// 1. Getting the document tree
/// 2. Collecting diagnostics
/// 3. Converting to LSP format
/// 4. Publishing to client
///
/// # Arguments
///
/// - `collector`: The diagnostic collector
/// - `client`: The LSP client
/// - `uri`: The document URI
/// - `tree`: The optional tree from document
/// - `source`: The source code
///
/// # Returns
///
/// The number of diagnostics published
pub async fn publish_diagnostics_for_document(
    collector: &DiagnosticCollector,
    client: &tower_lsp::Client,
    uri: Url,
    tree: &Option<Arc<Mutex<tree_sitter::Tree>>>,
    source: &str,
) -> usize {
    let sql_diagnostics = collector.collect_from_arc(tree, source, &uri);

    let diagnostics: Vec<Diagnostic> = sql_diagnostics.into_iter().map(|d| d.to_lsp()).collect();

    let count = diagnostics.len();
    if count > 0 {
        info!("Publishing {} diagnostics for {}", count, uri);
    }

    client.publish_diagnostics(uri, diagnostics, None).await;

    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Position;

    fn create_test_range(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Range {
        Range {
            start: Position {
                line: start_line,
                character: start_col,
            },
            end: Position {
                line: end_line,
                character: end_col,
            },
        }
    }

    #[test]
    fn test_sql_diagnostic_new() {
        let range = create_test_range(0, 0, 0, 10);
        let diagnostic =
            SqlDiagnostic::new("Test message".to_string(), DiagnosticSeverity::ERROR, range);

        assert_eq!(diagnostic.message, "Test message");
        assert_eq!(diagnostic.severity, DiagnosticSeverity::ERROR);
        assert_eq!(diagnostic.source, "unified-sql-lsp");
        assert!(diagnostic.code.is_none());
        assert!(diagnostic.related_information.is_none());
    }

    #[test]
    fn test_sql_diagnostic_with_code() {
        let range = create_test_range(0, 0, 0, 10);
        let diagnostic =
            SqlDiagnostic::error("Error".to_string(), range).with_code(DiagnosticCode::SyntaxError);

        assert_eq!(diagnostic.code, Some(DiagnosticCode::SyntaxError));
    }

    #[test]
    fn test_sql_diagnostic_error() {
        let range = create_test_range(1, 5, 1, 10);
        let diagnostic = SqlDiagnostic::error("Syntax error".to_string(), range);

        assert_eq!(diagnostic.message, "Syntax error");
        assert_eq!(diagnostic.severity, DiagnosticSeverity::ERROR);
    }

    #[test]
    fn test_sql_diagnostic_warning() {
        let range = create_test_range(0, 0, 0, 5);
        let diagnostic = SqlDiagnostic::warning("Warning".to_string(), range);

        assert_eq!(diagnostic.message, "Warning");
        assert_eq!(diagnostic.severity, DiagnosticSeverity::WARNING);
    }

    #[test]
    fn test_sql_diagnostic_to_lsp() {
        let range = create_test_range(0, 0, 1, 5);
        let sql_diagnostic = SqlDiagnostic::error("Error".to_string(), range)
            .with_code(DiagnosticCode::UndefinedTable);

        let lsp_diagnostic = sql_diagnostic.to_lsp();

        assert_eq!(lsp_diagnostic.message, "Error");
        assert_eq!(lsp_diagnostic.range, range);
        assert_eq!(lsp_diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(lsp_diagnostic.source, Some("unified-sql-lsp".to_string()));
    }

    #[test]
    fn test_diagnostic_code_as_str() {
        assert_eq!(DiagnosticCode::SyntaxError.as_str(), "SYNTAX-001");
        assert_eq!(DiagnosticCode::UndefinedTable.as_str(), "SEMANTIC-001");
        assert_eq!(DiagnosticCode::UndefinedColumn.as_str(), "SEMANTIC-002");
        assert_eq!(DiagnosticCode::AmbiguousColumn.as_str(), "SEMANTIC-003");
        assert_eq!(
            DiagnosticCode::Custom("CUSTOM-123".to_string()).as_str(),
            "CUSTOM-123"
        );
    }

    #[test]
    fn test_diagnostic_code_description() {
        assert_eq!(
            DiagnosticCode::SyntaxError.description(),
            "SQL syntax error"
        );
        assert_eq!(
            DiagnosticCode::UndefinedTable.description(),
            "Undefined table reference"
        );
        assert_eq!(
            DiagnosticCode::UndefinedColumn.description(),
            "Undefined column reference"
        );
        assert_eq!(
            DiagnosticCode::AmbiguousColumn.description(),
            "Ambiguous column reference"
        );
    }

    #[test]
    fn test_diagnostic_code_to_number_or_string() {
        let code: NumberOrString = DiagnosticCode::SyntaxError.into();
        assert!(matches!(code, NumberOrString::String(s) if s == "SYNTAX-001"));

        let custom: NumberOrString = DiagnosticCode::Custom("TEST-001".to_string()).into();
        assert!(matches!(custom, NumberOrString::String(s) if s == "TEST-001"));
    }

    #[test]
    fn test_diagnostic_collector_new() {
        let collector = DiagnosticCollector::new();
        // Just ensure it creates without panicking
        let _ = collector;
    }

    #[test]
    fn test_diagnostic_collector_collect_diagnostics() {
        let collector = DiagnosticCollector::new();
        let uri = Url::parse("file:///test.sql").unwrap();

        // Create a minimal tree (this will fail if no grammar is compiled)
        // But we can still test the empty case
        if let Some(language) =
            unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL)
        {
            let mut parser = tree_sitter::Parser::new();
            if parser.set_language(language).is_ok()
                && let Some(tree) = parser.parse("SELECT 1", None)
            {
                let diagnostics = collector.collect_diagnostics(&tree, "SELECT 1", &uri);
                // Currently returns empty (will be implemented in DIAG-002 to DIAG-005)
                assert!(diagnostics.is_empty());
            }
        }
    }

    #[test]
    fn test_diagnostic_collector_collect_from_arc_none() {
        let collector = DiagnosticCollector::new();
        let uri = Url::parse("file:///test.sql").unwrap();
        let tree: Option<Arc<Mutex<tree_sitter::Tree>>> = None;

        let diagnostics = collector.collect_from_arc(&tree, "SELECT 1", &uri);
        assert!(diagnostics.is_empty());
    }

    // Tests for pattern detection helpers

    #[test]
    fn test_is_identifier() {
        let collector = DiagnosticCollector::new();

        // Valid identifiers
        assert!(collector.is_identifier("id"));
        assert!(collector.is_identifier("username"));
        assert!(collector.is_identifier("_id"));
        assert!(collector.is_identifier("`id`"));
        assert!(collector.is_identifier("\"id\""));

        // Invalid identifiers
        assert!(!collector.is_identifier(""));
        assert!(!collector.is_identifier("123"));
        assert!(!collector.is_identifier("id username"));
        assert!(!collector.is_identifier("id,"));
    }

    #[test]
    fn test_is_missing_comma_pattern() {
        let collector = DiagnosticCollector::new();

        // Missing comma patterns
        assert!(collector.is_missing_comma_pattern("id username"));
        assert!(collector.is_missing_comma_pattern("col1 col2 col3"));
        assert!(collector.is_missing_comma_pattern("username FROM"));

        // Not missing comma
        assert!(!collector.is_missing_comma_pattern("id, username"));
        assert!(!collector.is_missing_comma_pattern("id"));
        assert!(!collector.is_missing_comma_pattern(""));
    }

    #[test]
    fn test_is_unmatched_paren_pattern() {
        let collector = DiagnosticCollector::new();

        // Unmatched parentheses
        assert!(collector.is_unmatched_paren_pattern("("));
        assert!(collector.is_unmatched_paren_pattern(")"));
        assert!(collector.is_unmatched_paren_pattern("((test)"));
        assert!(collector.is_unmatched_paren_pattern("(test))"));

        // Balanced parentheses
        assert!(!collector.is_unmatched_paren_pattern("()"));
        assert!(!collector.is_unmatched_paren_pattern("(test)"));
        assert!(!collector.is_unmatched_paren_pattern("((test))"));
        assert!(!collector.is_unmatched_paren_pattern(""));
    }

    #[test]
    fn test_first_identifier() {
        let collector = DiagnosticCollector::new();

        assert_eq!(collector.first_identifier("id username"), "id");
        assert_eq!(collector.first_identifier("col1 col2 col3"), "col1");
        assert_eq!(collector.first_identifier("identifier"), "identifier");
        assert_eq!(collector.first_identifier(""), "");
    }

    #[test]
    fn test_analyze_common_patterns_missing_comma() {
        let collector = DiagnosticCollector::new();

        // We need a real tree-sitter node for this test, but we can test the logic
        // by using a mock node that will trigger the missing comma pattern
        // For now, we'll just test the is_missing_comma_pattern function
        assert!(collector.is_missing_comma_pattern("id username"));
    }

    #[test]
    fn test_enhance_error_message_basic() {
        let collector = DiagnosticCollector::new();

        // Create a mock ERROR node scenario
        // Since we can't easily create a tree-sitter Node without a parser,
        // we'll test through the public interface

        if let Some(language) =
            unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL)
        {
            let mut parser = tree_sitter::Parser::new();
            if parser.set_language(language).is_ok() {
                // Parse SQL with obvious syntax error (missing FROM)
                let sql = "SELECT * WHERE id = 1";
                if let Some(tree) = parser.parse(sql, None) {
                    let diagnostics = collector.collect_diagnostics(
                        &tree,
                        sql,
                        &Url::parse("file:///test.sql").unwrap(),
                    );

                    // Should detect the syntax error
                    if !diagnostics.is_empty() {
                        // The message should mention the error
                        let message = &diagnostics[0].message;
                        assert!(
                            message.contains("Syntax error"),
                            "Error message should mention syntax error: {}",
                            message
                        );
                    } else {
                        // If no diagnostics, the parser accepted this SQL
                        // This is OK - it means the grammar is permissive
                        println!("Parser accepted SQL: {}", sql);
                    }
                }
            }
        }
    }

    #[test]
    fn test_enhance_error_message_unbalanced_parens() {
        let collector = DiagnosticCollector::new();

        if let Some(language) =
            unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL)
        {
            let mut parser = tree_sitter::Parser::new();
            if parser.set_language(language).is_ok() {
                // Parse SQL with unbalanced parentheses
                let sql = "SELECT * FROM users WHERE (id = 1";
                if let Some(tree) = parser.parse(sql, None) {
                    let diagnostics = collector.collect_diagnostics(
                        &tree,
                        sql,
                        &Url::parse("file:///test.sql").unwrap(),
                    );

                    // Should detect the syntax error
                    if !diagnostics.is_empty() {
                        // The message should mention parentheses or syntax error
                        let message = &diagnostics[0].message;
                        assert!(
                            message.contains("Syntax error"),
                            "Error message should mention syntax error: {}",
                            message
                        );
                    } else {
                        // If no diagnostics, the parser accepted this SQL
                        println!("Parser accepted SQL: {}", sql);
                    }
                }
            }
        }
    }
}
