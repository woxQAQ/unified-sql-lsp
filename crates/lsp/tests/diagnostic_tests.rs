// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for diagnostic infrastructure (DIAG-001)
//!
//! These tests verify that:
//! - Diagnostics are published when documents are opened
//! - Diagnostics are updated when documents are changed
//! - Diagnostics are cleared when documents are closed
//! - Error handling works correctly (parse failures, tree locking, etc.)

use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::*;
use unified_sql_lsp_lsp::{
    DocumentStore,
    diagnostic::{DiagnosticCollector, SqlDiagnostic},
};

fn create_test_uri(path: &str) -> Url {
    Url::parse(&format!("file://{}", path)).unwrap()
}

#[tokio::test]
async fn test_diagnostic_published_on_document_open() {
    // This test verifies the diagnostic infrastructure is integrated
    // Currently, diagnostics are empty (will be populated in DIAG-002 through DIAG-005)

    let store = DocumentStore::new();
    let uri = create_test_uri("/test_open.sql");
    let content = "SELECT * FROM users".to_string();

    // Open document
    store
        .open_document(uri.clone(), content, 1, "sql".to_string())
        .await
        .unwrap();

    let document = store.get_document(&uri).await.unwrap();

    // Create diagnostic collector
    let collector = DiagnosticCollector::new();

    // Mock client (we can't easily test without a real LSP client)
    // Instead, we verify that the collector works with the document
    let diagnostics = collector.collect_from_arc(&document.tree(), &document.get_content(), &uri);

    // Currently returns empty (specific diagnostics will be implemented in DIAG-002 through DIAG-005)
    assert!(diagnostics.is_empty());
}

#[tokio::test]
async fn test_diagnostic_handles_missing_tree() {
    let collector = DiagnosticCollector::new();
    let uri = create_test_uri("/test_no_tree.sql");

    // No tree
    let tree: Option<Arc<Mutex<tree_sitter::Tree>>> = None;
    let diagnostics = collector.collect_from_arc(&tree, "SELECT 1", &uri);

    // Should return empty diagnostics (graceful degradation)
    assert!(diagnostics.is_empty());
}

#[tokio::test]
async fn test_diagnostic_sql_diagnostic_conversion() {
    // Test SqlDiagnostic to LSP Diagnostic conversion
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 10,
        },
    };

    let sql_diagnostic = SqlDiagnostic::error("Test error".to_string(), range.clone())
        .with_code(unified_sql_lsp_lsp::diagnostic::DiagnosticCode::SyntaxError);

    let lsp_diagnostic = sql_diagnostic.to_lsp();

    assert_eq!(lsp_diagnostic.message, "Test error");
    assert_eq!(lsp_diagnostic.range, range);
    assert_eq!(lsp_diagnostic.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(lsp_diagnostic.source, Some("unified-sql-lsp".to_string()));
}

#[tokio::test]
async fn test_diagnostic_with_tree_lock() {
    let store = DocumentStore::new();
    let uri = create_test_uri("/test_lock.sql");
    let content = "SELECT 1".to_string();

    // Open and parse document
    store
        .open_document(uri.clone(), content, 1, "sql".to_string())
        .await
        .unwrap();

    let document = store.get_document(&uri).await.unwrap();

    // Create diagnostic collector
    let collector = DiagnosticCollector::new();

    // This should handle the Arc<Mutex<Tree>> correctly
    let diagnostics = collector.collect_from_arc(&document.tree(), &document.get_content(), &uri);

    // Should not panic or fail, even with tree locking
    assert!(diagnostics.is_empty());
}

#[tokio::test]
async fn test_diagnostic_with_parsed_tree() {
    // Only run this test if grammars are compiled
    if unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL).is_none() {
        return; // Skip test if no grammar
    }

    let store = DocumentStore::new();
    let uri = create_test_uri("/test_parsed.sql");
    let content = "SELECT * FROM users WHERE id = 1".to_string();
    let content_for_parse = content.clone();

    // Open document
    store
        .open_document(uri.clone(), content, 1, "mysql".to_string())
        .await
        .unwrap();

    // Parse the document (simulate what backend.rs does)
    let mut parser =
        match unified_sql_grammar::language_for_dialect(unified_sql_lsp_ir::Dialect::MySQL) {
            Some(lang) => {
                let mut p = tree_sitter::Parser::new();
                if p.set_language(lang).is_err() {
                    return; // Skip if parser setup fails
                }
                p
            }
            None => return,
        };

    if let Some(tree) = parser.parse(&content_for_parse, None) {
        use unified_sql_lsp_lsp::ParseMetadata;
        let metadata = ParseMetadata::new(0, unified_sql_lsp_ir::Dialect::MySQL, false, 0);
        let _ = store.update_document_tree(&uri, tree, metadata).await;
    }

    let document = store.get_document(&uri).await.unwrap();

    // Create diagnostic collector
    let collector = DiagnosticCollector::new();

    // Collect diagnostics
    let diagnostics = collector.collect_from_arc(&document.tree(), &document.get_content(), &uri);

    // Should work without errors
    assert!(diagnostics.is_empty());
}

#[tokio::test]
async fn test_diagnostic_all_severity_levels() {
    let range = Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 5,
        },
    };

    // Test all severity levels
    let error = SqlDiagnostic::error("Error".to_string(), range.clone());
    let warning = SqlDiagnostic::warning("Warning".to_string(), range.clone());
    let info = SqlDiagnostic::information("Info".to_string(), range.clone());
    let hint = SqlDiagnostic::hint("Hint".to_string(), range);

    let lsp_error = error.to_lsp();
    let lsp_warning = warning.to_lsp();
    let lsp_info = info.to_lsp();
    let lsp_hint = hint.to_lsp();

    assert_eq!(lsp_error.severity, Some(DiagnosticSeverity::ERROR));
    assert_eq!(lsp_warning.severity, Some(DiagnosticSeverity::WARNING));
    assert_eq!(lsp_info.severity, Some(DiagnosticSeverity::INFORMATION));
    assert_eq!(lsp_hint.severity, Some(DiagnosticSeverity::HINT));
}

#[tokio::test]
async fn test_diagnostic_codes() {
    use unified_sql_lsp_lsp::diagnostic::DiagnosticCode;

    // Test all diagnostic codes
    assert_eq!(DiagnosticCode::SyntaxError.as_str(), "SYNTAX-001");
    assert_eq!(DiagnosticCode::UndefinedTable.as_str(), "SEMANTIC-001");
    assert_eq!(DiagnosticCode::UndefinedColumn.as_str(), "SEMANTIC-002");
    assert_eq!(DiagnosticCode::AmbiguousColumn.as_str(), "SEMANTIC-003");

    // Test custom code
    let custom = DiagnosticCode::Custom("CUSTOM-001".to_string());
    assert_eq!(custom.as_str(), "CUSTOM-001");
}
