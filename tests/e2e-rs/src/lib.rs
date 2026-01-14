// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # E2E Test Library for Unified SQL LSP
//!
//! This library provides a framework for end-to-end testing of the Unified SQL LSP server
//! through actual LSP protocol messages with live database connections.
//!
//! ## Features
//!
//! - Full LSP protocol testing (not direct backend testing)
//! - Live database connections via Docker (MySQL/PostgreSQL)
//! - Declarative test definitions in YAML
//! - Comprehensive assertion helpers
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_e2e::{run_suite, init_database};
//!
//! #[tokio::test]
//! async fn test_completion() {
//!     init_database().await.unwrap();
//!     run_suite("tests/completion/from_clause.yaml").await.unwrap();
//! }
//! ```

pub mod assertions;
pub mod client;
pub mod db;
pub mod runner;
pub mod text_case_runner;
pub mod utils;
pub mod yaml_parser;

use anyhow::Result;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tracing::info;

use client::LspConnection;
use db::{DatabaseAdapter, MySQLAdapter};
use yaml_parser::TestSuite;

/// Global test database adapter (initialized once, thread-safe)
static DB_ADAPTER: LazyLock<Arc<RwLock<Option<Arc<dyn DatabaseAdapter>>>>> = LazyLock::new(
    || Arc::new(RwLock::new(None))
);

/// Initialize test database
///
/// This function should be called once before running any tests.
/// It sets up the global database adapter that will be used by all tests.
/// If the database is already initialized, it returns the existing adapter.
pub async fn init_database() -> Result<Arc<dyn DatabaseAdapter>> {
    // Check if already initialized
    {
        let adapter_guard = DB_ADAPTER.read().await;
        if let Some(adapter) = adapter_guard.as_ref() {
            info!("Database adapter already initialized, reusing existing adapter");
            return Ok(adapter.clone());
        }
    }

    let adapter = Arc::new(MySQLAdapter::from_default_config()) as Arc<dyn DatabaseAdapter>;

    info!("Initializing test database adapter...");

    {
        let mut adapter_guard = DB_ADAPTER.write().await;
        *adapter_guard = Some(adapter.clone());
    }

    info!("Database adapter initialized successfully");
    Ok(adapter)
}

/// Run a single test case
pub async fn run_test(
    suite: &TestSuite,
    test: &yaml_parser::TestCase,
) -> Result<()> {
    info!("=== Running test: {} ===", test.name);

    // 1. Setup database (load schema/data if needed)
    info!("Getting database adapter...");
    let adapter = {
        let adapter_guard = DB_ADAPTER.read().await;
        adapter_guard.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database not initialized. Call init_database() first."))?
            .clone()
    };
    info!("Database adapter obtained");

    for schema_path in &suite.database.schemas {
        let full_path = std::path::PathBuf::from(schema_path);
        info!("Loading schema from: {:?}", full_path);
        if full_path.exists() {
            adapter.load_schema(&full_path).await?;
        } else {
            tracing::warn!("Schema file not found: {:?}", full_path);
        }
    }

    for data_path in &suite.database.data {
        let full_path = std::path::PathBuf::from(data_path);
        info!("Loading data from: {:?}", full_path);
        if full_path.exists() {
            adapter.load_data(&full_path).await?;
        } else {
            tracing::warn!("Data file not found: {:?}", full_path);
        }
    }

    // 2. Spawn LSP server
    info!("Spawning LSP server...");
    let mut lsp_runner = runner::LspRunner::from_crate()?;
    lsp_runner.spawn().await?;
    info!("LSP server spawned");

    // 3. Establish LSP connection
    info!("Establishing LSP connection...");
    let stdin = lsp_runner.stdin()?;
    let stdout = lsp_runner.stdout()?;
    let mut conn = LspConnection::new(stdin, stdout);
    info!("LSP connection established");

    // 4. Initialize server
    info!("Initializing LSP server...");
    let _init_result = conn.initialize().await?;
    info!("LSP server initialized");

    // 4.5. Set engine configuration through did_change_configuration
    // Use connection string from YAML if provided, otherwise use default
    let connection_string = suite.database.connection_string.clone().unwrap_or_default();
    let dialect = suite.database.dialect.clone();
    info!("Setting engine configuration: dialect={}, connection={}", dialect, connection_string);

    // Send the configuration notification
    conn.did_change_configuration(&dialect, &connection_string).await?;
    info!("Engine configuration set");

    // Give server time to process the configuration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. Get cursor position and strip marker
    let position = suite.extract_cursor(test)?;
    let sql = suite.strip_cursor_marker(test);
    info!("Cursor position: {:?}", position);

    // 6. Open document
    info!("Opening document...");
    // Sanitize test name to create valid URI (replace spaces with underscores)
    let sanitized_name = test.name.replace(' ', "_");
    let uri = tower_lsp::lsp_types::Url::parse(&format!("file:///test_{}.sql", sanitized_name))?;
    let dialect = suite.database.dialect.clone();
    conn.did_open(uri.clone(), dialect, sql).await?;
    info!("Document opened");

    // Give server time to parse
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 7. Run assertions based on test expectations
    if let Some(completion_expect) = &test.expect_completion {
        let completion_items = conn.completion(uri.clone(), position).await?
            .unwrap_or_default();

        if !completion_expect.contains.is_empty() {
            assertions::assert_completion_contains(&completion_items, &completion_expect.contains)?;
        }

        if !completion_expect.not_contains.is_empty() {
            assertions::assert_completion_not_contains(&completion_items, &completion_expect.not_contains)?;
        }

        if let Some(count) = completion_expect.count {
            assertions::assert_completion_count(&completion_items, count)?;
        }

        if let Some(min_count) = completion_expect.min_count {
            assertions::assert_completion_min_count(&completion_items, min_count)?;
        }

        if !completion_expect.order.is_empty() {
            assertions::assert_completion_order(&completion_items, &completion_expect.order)?;
        }
    }

    if let Some(diag_expect) = &test.expect_diagnostics {
        // Read any pending notifications (like publish_diagnostics)
        conn.read_pending_notifications().await?;

        let diagnostics = conn.get_diagnostics(&uri).await
            .unwrap_or_default();

        assertions::assert_diagnostics(&diagnostics, diag_expect.error_count, diag_expect.warning_count)?;

        if !diag_expect.error_messages.is_empty() {
            for expected_msg in &diag_expect.error_messages {
                let found = diagnostics.iter().any(|d| {
                    d.message.contains(expected_msg)
                });
                if !found {
                    return Err(anyhow::anyhow!(
                        "Expected diagnostics to contain error message '{}', but it was not found. Diagnostics: {:?}",
                        expected_msg,
                        diagnostics
                    ));
                }
            }
        }
    }

    if let Some(hover_expect) = &test.expect_hover {
        let hover_result = conn.hover(uri.clone(), position).await?;
        assertions::assert_hover_contains(hover_result.as_ref(), &hover_expect.contains)?;
    }

    // 8. Cleanup
    drop(conn);
    lsp_runner.kill().await?;
    adapter.cleanup().await?;

    info!("Test passed: {}", test.name);
    Ok(())
}

/// Run all tests in a suite
pub async fn run_suite(suite_path: impl AsRef<std::path::Path>) -> Result<()> {
    eprintln!("!!! Loading test suite from: {:?}", suite_path.as_ref());
    let suite = TestSuite::from_file(suite_path)?;
    eprintln!("!!! Test suite loaded: {} with {} tests", suite.name, suite.tests.len());

    let mut failed_tests = Vec::new();

    for test in &suite.tests {
        eprintln!("!!! About to run test: {}", test.name);
        match run_test(&suite, test).await {
            Ok(_) => {
                eprintln!("!!! Test completed: {}", test.name);
            }
            Err(e) => {
                eprintln!("!!! Test FAILED: {} - {}", test.name, e);
                failed_tests.push((test.name.clone(), e));
            }
        }
    }

    // Report summary
    if !failed_tests.is_empty() {
        eprintln!("\n!!! {} test(s) failed:", failed_tests.len());
        for (name, error) in &failed_tests {
            eprintln!("!!!   - {}: {}", name, error);
        }
        Err(anyhow::anyhow!("{} test(s) failed", failed_tests.len()))
    } else {
        Ok(())
    }
}

// Re-export text-based test runner functions
pub use text_case_runner::{init_database as init_database_text, run_test_file, run_test_directory};
