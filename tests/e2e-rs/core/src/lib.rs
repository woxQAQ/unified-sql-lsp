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
pub mod docker;
pub mod engine_manager;
pub mod runner;
pub mod utils;
pub mod yaml_parser;

pub use engine_manager::{Engine, ensure_engine_ready};
pub use unified_sql_lsp_e2e_core_macros::{generate_engine_tests, TestMetadata};
pub use yaml_parser::{TestCase, TestSuite};

use anyhow::Result;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::RwLock;
use tracing::info;

use client::LspConnection;
use db::adapter_from_test_path;
use docker::DockerCompose;

/// Global Docker Compose manager (initialized once, thread-safe)
static DOCKER_COMPOSE: LazyLock<Arc<RwLock<Option<DockerCompose>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

/// Initialize test database
///
/// This function should be called once before running any tests.
/// It starts Docker Compose services (all database engines).
/// If already initialized, it returns early.
pub async fn init_database() -> Result<()> {
    // Check if already initialized
    {
        let guard = DOCKER_COMPOSE.read().await;
        if guard.is_some() {
            info!("Docker Compose already initialized");
            return Ok(());
        }
    }

    // Start ALL Docker Compose services
    info!("Starting Docker Compose services...");
    let mut docker_compose = DockerCompose::from_default_config()?;
    docker_compose.start().await?;

    {
        let mut compose_guard = DOCKER_COMPOSE.write().await;
        *compose_guard = Some(docker_compose);
    }

    info!("All Docker services started successfully");
    Ok(())
}

/// Cleanup database resources
///
/// Stops Docker Compose services. Should be called after all tests complete.
pub async fn cleanup_database() -> Result<()> {
    info!("Cleaning up database resources...");

    // Stop Docker Compose services
    {
        let mut compose_guard = DOCKER_COMPOSE.write().await;
        if let Some(mut docker_compose) = compose_guard.take() {
            docker_compose.stop().await?;
        }
    }

    info!("Database cleanup complete");
    Ok(())
}

/// Run a single test case
pub async fn run_test(
    suite: &TestSuite,
    test: &yaml_parser::TestCase,
    suite_path: &std::path::Path,
) -> Result<()> {
    info!("=== Running test: {} ===", test.name);

    let adapter = adapter_from_test_path(suite_path)?;
    info!("Database adapter determined from path: {:?}", suite_path);

    if let Err(e) = adapter.truncate_tables().await {
        tracing::warn!("Failed to truncate tables (non-fatal): {}", e);
    }

    let suite_dir = suite_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot get parent directory of suite file"))?;

    for schema_path in &suite.database.schemas {
        let full_path = suite_dir.join(schema_path);
        eprintln!("!!! Loading schema from: {:?}", full_path);
        if full_path.exists() {
            eprintln!("!!! Schema file exists, loading...");
            if let Err(e) = adapter.load_schema(&full_path).await {
                eprintln!("!!! Failed to load schema: {}", e);
            } else {
                eprintln!("!!! Schema loaded successfully");
            }
        } else {
            eprintln!("!!! WARNING: Schema file not found: {:?}", full_path);
        }
    }

    for data_path in &suite.database.data {
        let full_path = suite_dir.join(data_path);
        eprintln!("!!! Loading data from: {:?}", full_path);
        if full_path.exists() {
            eprintln!("!!! Data file exists, loading...");
            if let Err(e) = adapter.load_data(&full_path).await {
                eprintln!("!!! Failed to load data: {}", e);
            } else {
                eprintln!("!!! Data loaded successfully");
            }
        } else {
            eprintln!("!!! WARNING: Data file not found: {:?}", full_path);
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
    // Use connection string from adapter (determined by test path)
    let connection_string = adapter.connection_string().to_string();
    let dialect = suite.database.dialect.clone();
    info!(
        "Setting engine configuration: dialect={}, connection={}",
        dialect, connection_string
    );

    // Send the configuration notification
    conn.did_change_configuration(&dialect, &connection_string)
        .await?;
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
        let completion_items = conn
            .completion(uri.clone(), position)
            .await?
            .unwrap_or_default();

        if !completion_expect.contains.is_empty() {
            assertions::assert_completion_contains(&completion_items, &completion_expect.contains)?;
        }

        if !completion_expect.not_contains.is_empty() {
            assertions::assert_completion_not_contains(
                &completion_items,
                &completion_expect.not_contains,
            )?;
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

        let diagnostics = conn.get_diagnostics(&uri).await.unwrap_or_default();

        assertions::assert_diagnostics(
            &diagnostics,
            diag_expect.error_count,
            diag_expect.warning_count,
        )?;

        if !diag_expect.error_messages.is_empty() {
            for expected_msg in &diag_expect.error_messages {
                let found = diagnostics.iter().any(|d| d.message.contains(expected_msg));
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

    drop(conn);
    lsp_runner.kill().await?;

    info!("Test passed: {}", test.name);
    Ok(())
}

/// Run all tests in a suite
pub async fn run_suite(suite_path: impl AsRef<std::path::Path>) -> Result<()> {
    // Resolve the path - if relative, make it relative to workspace root
    let path = suite_path.as_ref();
    let resolved_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        // Find workspace root and resolve relative path
        let workspace_root = crate::runner::find_workspace_root(
            &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        )?;
        workspace_root.join(path)
    };

    eprintln!("!!! Loading test suite from: {:?}", resolved_path);
    let suite = TestSuite::from_file(&resolved_path)?;
    eprintln!(
        "!!! Test suite loaded: {} with {} tests",
        suite.name,
        suite.tests.len()
    );

    let mut failed_tests = Vec::new();

    for test in &suite.tests {
        eprintln!("!!! About to run test: {}", test.name);
        match run_test(&suite, test, &resolved_path).await {
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

/// Global cleanup function called when test process exits
///
/// This uses the `ctor` crate to register a destructor that runs when
/// the test binary exits, ensuring Docker Compose services are stopped.
#[ctor::dtor]
fn global_cleanup() {
    // Check if we started Docker Compose services
    let needs_cleanup = DOCKER_COMPOSE
        .try_read()
        .map(|guard| guard.is_some())
        .unwrap_or(false);

    if needs_cleanup {
        eprintln!("!!! Global cleanup: stopping Docker Compose services...");

        // Find the compose file path by searching upward
        let compose_file = crate::docker::find_docker_compose_file()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|e| {
                eprintln!("!!! Failed to find docker-compose.yml: {}", e);
                "tests/e2e-rs/docker-compose.yml".to_string()
            });

        // Use std::process::Command for synchronous execution
        let result = std::process::Command::new("docker")
            .args([
                "compose",
                "-f",
                &compose_file,
                "-p",
                "unified-sql-lsp-e2e",
                "down",
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    eprintln!("!!! Docker Compose services stopped successfully");
                } else {
                    eprintln!(
                        "!!! Failed to stop Docker Compose: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!("!!! Failed to execute docker compose down: {}", e);
            }
        }
    }
}
