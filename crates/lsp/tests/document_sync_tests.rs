// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! DocumentSync integration tests
//!
//! Comprehensive tests for document synchronization and parsing orchestration.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;
use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::config::{
    ConnectionPoolConfig, DialectVersion, EngineConfig, SchemaFilter,
};
use unified_sql_lsp_lsp::document::Document;
use unified_sql_lsp_lsp::parsing::{ParseError, ParseResult};
use unified_sql_lsp_lsp::sync::DocumentSync;

fn create_test_document(content: &str, language_id: &str) -> Document {
    let uri = Url::parse("file:///test.sql").unwrap();
    Document::new(uri, content.to_string(), 1, language_id.to_string())
}

#[test]
fn test_document_sync_resolve_dialect_mysql() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let doc = create_test_document("SELECT 1", "mysql");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(dialect, Dialect::MySQL);
}

#[test]
fn test_document_sync_resolve_dialect_postgresql() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let doc = create_test_document("SELECT 1", "postgresql");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(dialect, Dialect::PostgreSQL);
}

#[test]
fn test_document_sync_resolve_dialect_from_postgres_alias() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let doc = create_test_document("SELECT 1", "postgres");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(dialect, Dialect::PostgreSQL);
}

#[test]
fn test_document_sync_resolve_dialect_defaults_to_mysql() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    // Generic "sql" language_id should default to MySQL
    let doc = create_test_document("SELECT 1", "sql");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(dialect, Dialect::MySQL);
}

#[test]
fn test_document_sync_resolve_dialect_from_config() {
    let engine_config = EngineConfig {
        dialect: Dialect::PostgreSQL,
        version: DialectVersion::PostgreSQL14,
        connection_string: String::new(),
        schema_filter: SchemaFilter::default(),
        pool_config: ConnectionPoolConfig::default(),
        log_queries: false,
        query_timeout_secs: 5,
        cache_enabled: false,
    };

    let config = Arc::new(RwLock::new(Some(engine_config)));
    let sync = DocumentSync::new(config);

    // Config should override language_id
    let doc = create_test_document("SELECT 1", "mysql");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(dialect, Dialect::PostgreSQL);
}

#[test]
fn test_document_sync_on_open() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let doc = create_test_document("SELECT * FROM users", "mysql");
    let result = sync.on_document_open(&doc);

    match result {
        ParseResult::Success { tree, .. } => {
            assert!(tree.is_some(), "Expected tree after successful parse");
        }
        ParseResult::Partial { .. } => {
            // Acceptable
        }
        ParseResult::Failed { error } => {
            panic!("Document open failed to parse: {}", error);
        }
    }
}

#[test]
fn test_document_sync_on_open_with_postgresql() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let doc = create_test_document("SELECT DISTINCT ON (name) name FROM users", "postgresql");
    let result = sync.on_document_open(&doc);

    assert!(!result.is_failed(), "PostgreSQL parse should succeed");
}

#[test]
fn test_document_sync_metadata_creation_success() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let success = ParseResult::Success {
        tree: None,
        parse_time: Duration::from_millis(50),
    };

    let metadata = sync.create_metadata(&success, Dialect::MySQL);
    assert_eq!(metadata.parse_time_ms, 50);
    assert!(!metadata.has_errors);
    assert_eq!(metadata.error_count, 0);
}

#[test]
fn test_document_sync_metadata_creation_partial() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let partial = ParseResult::Partial {
        tree: None,
        errors: vec![ParseError::Generic {
            message: "Test error".to_string(),
        }],
    };

    let metadata = sync.create_metadata(&partial, Dialect::PostgreSQL);
    assert!(metadata.has_errors);
    assert_eq!(metadata.error_count, 1);
}

#[test]
fn test_document_sync_metadata_creation_failed() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let failed = ParseResult::Failed {
        error: ParseError::Generic {
            message: "Failed".to_string(),
        },
    };

    let metadata = sync.create_metadata(&failed, Dialect::MySQL);
    assert!(metadata.has_errors);
    assert_eq!(metadata.error_count, 1);
}

#[test]
fn test_document_sync_on_change_full_replacement() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    let mut doc = create_test_document("SELECT 1", "mysql");

    // Initial parse
    let _ = sync.on_document_open(&doc);

    // Full document replacement
    let changes = vec![TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: "SELECT * FROM users WHERE id = 1".to_string(),
    }];

    let result = sync.on_document_change(&doc, None, &changes);

    match result {
        ParseResult::Success { .. } | ParseResult::Partial { .. } => {
            // OK
        }
        ParseResult::Failed { error } => {
            panic!("Document change failed to parse: {}", error);
        }
    }
}

#[test]
fn test_document_sync_dialect_mapping() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    // Test various language_id mappings
    let test_cases = vec![
        ("mysql", Dialect::MySQL),
        ("postgresql", Dialect::PostgreSQL),
        ("postgres", Dialect::PostgreSQL),
        ("sql", Dialect::MySQL),
        ("unknown", Dialect::MySQL), // Defaults to MySQL
    ];

    for (language_id, expected_dialect) in test_cases {
        let doc = create_test_document("SELECT 1", language_id);
        let dialect = sync.resolve_dialect(&doc);
        assert_eq!(
            dialect, expected_dialect,
            "language_id '{}' should resolve to {:?}",
            language_id, expected_dialect
        );
    }
}

#[test]
fn test_document_sync_config_overrides_language_id() {
    // Test that engine config always takes precedence
    let engine_config = EngineConfig {
        dialect: Dialect::MySQL,
        version: DialectVersion::MySQL80,
        connection_string: String::new(),
        schema_filter: SchemaFilter::default(),
        pool_config: ConnectionPoolConfig::default(),
        log_queries: false,
        query_timeout_secs: 30,
        cache_enabled: true,
    };

    let config = Arc::new(RwLock::new(Some(engine_config)));
    let sync = DocumentSync::new(config);

    // Even with "postgresql" language_id, config should force MySQL
    let doc = create_test_document("SELECT 1", "postgresql");
    let dialect = sync.resolve_dialect(&doc);

    assert_eq!(
        dialect,
        Dialect::MySQL,
        "Config should override language_id"
    );
}

#[test]
fn test_document_sync_handles_invalid_sql_on_open() {
    let config = Arc::new(RwLock::new(None));
    let sync = DocumentSync::new(config);

    // Totally invalid SQL
    let doc = create_test_document("TOTALLY INVALID SQL HERE!!!", "mysql");
    let result = sync.on_document_open(&doc);

    // Should not crash - should return error or partial
    match result {
        ParseResult::Failed { .. } => {
            // Expected
        }
        ParseResult::Partial { .. } => {
            // Also acceptable
        }
        ParseResult::Success { .. } => {
            // Grammar might be very lenient
        }
    }
}
