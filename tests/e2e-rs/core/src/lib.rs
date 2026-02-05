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
pub mod db_pool;
pub mod docker;
pub mod engine_manager;
pub mod execution;
pub mod lifecycle;
pub mod logging;
pub mod lsp_pool;
pub mod orchestrator;
pub mod runner;
pub mod utils;
pub mod yaml_parser;

pub use db_pool::{DatabaseConfig, DatabasePoolManager};
pub use engine_manager::{Engine as EngineManagerEngine, ensure_engine_ready};
pub use execution::TestExecutor;
pub use lifecycle::{
    LifecycleManager, TestContext, TestLifecycle, TestPhase, helpers as lifecycle_helpers,
};
pub use lsp_pool::{LspClient, LspClientConfig, LspClientManager};
pub use orchestrator::{Engine, OrchestratorConfig, TestHandle, TestOrchestrator};
pub use unified_sql_lsp_e2e_core_macros::{TestMetadata, generate_engine_tests};
pub use yaml_parser::{TestCase, TestSuite};

use anyhow::Result;
use std::sync::Arc;
use std::sync::LazyLock;
use tokio::sync::{Mutex, RwLock};
use tracing::info;

use client::LspConnection;
use db::adapter_from_test_path;
use docker::DockerCompose;

use logging::initialize;

/// Global Docker Compose manager (initialized once, thread-safe)
static DOCKER_COMPOSE: LazyLock<Arc<RwLock<Option<DockerCompose>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(None)));

struct SharedLsp {
    _runner: runner::LspRunner,
    conn: LspConnection,
}

/// Global shared LSP process/connection for this test process.
static SHARED_LSP: LazyLock<Arc<Mutex<Option<SharedLsp>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(None)));

fn should_cleanup_docker_on_process_exit() -> bool {
    match std::env::var("E2E_AUTO_DOCKER_CLEANUP") {
        Ok(value) => value != "0" && !value.eq_ignore_ascii_case("false"),
        Err(_) => true,
    }
}

/// Determine the database engine from a test path
///
/// This function extracts the engine type from the test file path
/// based on the directory structure (e.g., mysql-5.7, postgresql-12).
pub fn engine_from_test_path(test_path: &std::path::Path) -> Result<Engine> {
    let path_str = test_path.to_string_lossy();

    // Check for MySQL engines
    if path_str.contains("mysql-5.7") || path_str.contains("mysql_5_7") {
        return Ok(Engine::MySQL57);
    }
    if path_str.contains("mysql-8.0")
        || path_str.contains("mysql_8_0")
        || path_str.contains("mysql80")
    {
        return Ok(Engine::MySQL80);
    }

    // Check for PostgreSQL engines
    if path_str.contains("postgresql-12")
        || path_str.contains("postgresql_12")
        || path_str.contains("postgres12")
    {
        return Ok(Engine::PostgreSQL12);
    }
    if path_str.contains("postgresql-16")
        || path_str.contains("postgresql_16")
        || path_str.contains("postgres16")
    {
        return Ok(Engine::PostgreSQL16);
    }

    // Default to MySQL 5.7 for backwards compatibility
    tracing::warn!(
        "Could not determine engine from path: {:?}, defaulting to MySQL 5.7",
        test_path
    );
    Ok(Engine::MySQL57)
}

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
        debug_log!("!!! Loading schema from: {:?}", full_path);
        if full_path.exists() {
            debug_log!("!!! Schema file exists, loading...");
            if let Err(e) = adapter.load_schema(&full_path).await {
                debug_log!("!!! Failed to load schema: {}", e);
            } else {
                debug_log!("!!! Schema loaded successfully");
            }
        } else {
            debug_log!("!!! WARNING: Schema file not found: {:?}", full_path);
        }
    }

    for data_path in &suite.database.data {
        let full_path = suite_dir.join(data_path);
        debug_log!("!!! Loading data from: {:?}", full_path);
        if full_path.exists() {
            debug_log!("!!! Data file exists, loading...");
            if let Err(e) = adapter.load_data(&full_path).await {
                debug_log!("!!! Failed to load data: {}", e);
            } else {
                debug_log!("!!! Data loaded successfully");
            }
        } else {
            debug_log!("!!! WARNING: Data file not found: {:?}", full_path);
        }
    }

    // 2. Get or initialize shared LSP server
    let mut shared_lsp_guard = SHARED_LSP.lock().await;
    if shared_lsp_guard.is_none() {
        info!("Spawning shared LSP server...");
        let mut lsp_runner = runner::LspRunner::from_crate()?;
        lsp_runner.spawn().await?;

        let stdin = lsp_runner.stdin()?;
        let stdout = lsp_runner.stdout()?;
        let mut conn = LspConnection::new(stdin, stdout);

        info!("Initializing shared LSP server...");
        let _init_result = conn.initialize().await?;
        info!("Shared LSP server initialized");

        *shared_lsp_guard = Some(SharedLsp {
            _runner: lsp_runner,
            conn,
        });
    }

    let shared_lsp = shared_lsp_guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("Shared LSP not initialized"))?;
    let conn = &mut shared_lsp.conn;

    // 4.5. Set engine configuration through did_change_configuration
    // Use connection string from adapter (determined by test path)
    let connection_string = adapter.connection_string().to_string();
    let dialect = suite.database.dialect.clone();
    info!(
        "Setting engine configuration: dialect={}, connection={}",
        dialect, connection_string
    );

    // Send the configuration notification.
    // We reuse one LSP process across tests, so every case must reconfigure it.
    conn.did_change_configuration(&dialect, &connection_string)
        .await?;
    info!("Engine configuration set");

    // Avoid stale notifications from previous tests in this shared process.
    conn.client().clear().await;

    // Give server time to process the configuration
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // 5. Get cursor position and strip marker
    let position = suite.extract_cursor(test)?;
    let sql = suite.strip_cursor_marker(test);
    info!("Cursor position: {:?}", position);

    // 6. Open document
    info!("Opening document...");
    // Generate a unique URI to avoid cross-test document state interference.
    let sanitized_name: String = test
        .name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let uri = tower_lsp::lsp_types::Url::parse(&format!(
        "file:///test_{}_{}.sql",
        sanitized_name, unique_id
    ))?;
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

    // Best-effort cleanup of opened document in shared LSP process.
    let _ = conn.did_close(uri).await;
    drop(shared_lsp_guard);

    info!("Test passed: {}", test.name);
    Ok(())
}

fn resolve_suite_path(suite_path: impl AsRef<std::path::Path>) -> Result<std::path::PathBuf> {
    let path = suite_path.as_ref();
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let workspace_root = crate::runner::find_workspace_root(&std::path::PathBuf::from(env!(
            "CARGO_MANIFEST_DIR"
        )))?;
        Ok(workspace_root.join(path))
    }
}

fn normalize_meta_case_name(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    let mut previous_was_dash = false;

    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            previous_was_dash = false;
            continue;
        }

        if !output.is_empty() && !previous_was_dash {
            output.push('-');
            previous_was_dash = true;
        }
    }

    while output.ends_with('-') {
        output.pop();
    }

    if output.is_empty() {
        "unnamed-case".to_string()
    } else {
        output
    }
}

fn category_from_suite_path(path: &std::path::Path) -> String {
    path.parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown-category".to_string())
}

fn test_label(path: &std::path::Path, test_name: &str) -> String {
    let engine = engine_from_test_path(path)
        .map(|e| e.to_string())
        .unwrap_or_else(|_| "unknown-engine".to_string());
    let category = category_from_suite_path(path);
    let meta_case = normalize_meta_case_name(test_name);
    format!("{engine}:{category}:{meta_case}")
}

/// Run a single test case by index in a suite
pub async fn run_case(suite_path: impl AsRef<std::path::Path>, case_index: usize) -> Result<()> {
    // Disable anyhow backtrace for cleaner error output
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "0");
    }

    initialize();

    let resolved_path = resolve_suite_path(suite_path)?;
    let suite = TestSuite::from_file(&resolved_path)?;

    let test = suite.tests.get(case_index).ok_or_else(|| {
        anyhow::anyhow!(
            "Case index {} out of range for suite {} ({} cases)",
            case_index,
            resolved_path.display(),
            suite.tests.len()
        )
    })?;

    let case_label = test_label(&resolved_path, &test.name);
    eprintln!("Running test: {}", case_label);

    match run_test(&suite, test, &resolved_path).await {
        Ok(_) => {
            eprintln!("Test completed: {}", case_label);
            Ok(())
        }
        Err(e) => {
            let _ = logging::flush_to_file();
            eprintln!("Test FAILED: {}", case_label);
            if let Some(log_path) = logging::log_path() {
                eprintln!("Full debug log: {}", log_path.display());
            }
            Err(anyhow::anyhow!("{}: {}", case_label, e))
        }
    }
}

/// Run all tests in a suite
pub async fn run_suite(suite_path: impl AsRef<std::path::Path>) -> Result<()> {
    // Disable anyhow backtrace for cleaner error output
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "0");
    }

    // Initialize logging system
    initialize();

    let resolved_path = resolve_suite_path(suite_path)?;

    debug_log!("!!! Loading test suite from: {:?}", resolved_path);
    let suite = TestSuite::from_file(&resolved_path)?;
    debug_log!(
        "!!! Test suite loaded: {} with {} tests",
        suite.name,
        suite.tests.len()
    );

    let mut failed_tests = Vec::new();

    for test in &suite.tests {
        let case_label = test_label(&resolved_path, &test.name);
        eprintln!("Running test: {}", case_label);
        match run_test(&suite, test, &resolved_path).await {
            Ok(_) => {
                eprintln!("Test completed: {}", case_label);
            }
            Err(e) => {
                // First failure: flush logs to file
                if failed_tests.is_empty() {
                    let _ = logging::flush_to_file();
                }
                eprintln!("Test FAILED: {}", case_label);
                debug_log!("!!! Test FAILED: {} - {}", case_label, e);
                failed_tests.push((case_label, e));
            }
        }
    }

    // Report summary
    if !failed_tests.is_empty() {
        eprintln!("\n{} test(s) failed", failed_tests.len());
        for (name, _) in &failed_tests {
            eprintln!("  - {}", name);
        }
        if let Some((_, first_error)) = failed_tests.first() {
            eprintln!("First failure reason: {}", first_error);
        }
        if let Some(log_path) = logging::log_path() {
            eprintln!("\nFull debug log: {}", log_path.display());
        }
        // Disable anyhow backtrace for cleaner error output
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "0");
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
    if let Ok(mut shared_guard) = SHARED_LSP.try_lock() {
        // Dropping SharedLsp will trigger LspRunner::Drop and kill the child process.
        let _ = shared_guard.take();
    }

    if !should_cleanup_docker_on_process_exit() {
        debug_log!(
            "!!! Skipping Docker Compose down on process exit (E2E_AUTO_DOCKER_CLEANUP disabled)"
        );
        return;
    }

    // Check if we started Docker Compose services
    let needs_cleanup = DOCKER_COMPOSE
        .try_read()
        .map(|guard| guard.is_some())
        .unwrap_or(false);

    if needs_cleanup {
        debug_log!("!!! Global cleanup: stopping Docker Compose services...");

        // Find the compose file path by searching upward
        let compose_file = crate::docker::find_docker_compose_file()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|e| {
                debug_log!("!!! Failed to find docker-compose.yml: {}", e);
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
                    debug_log!("!!! Docker Compose services stopped successfully");
                } else {
                    debug_log!(
                        "!!! Failed to stop Docker Compose: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                debug_log!("!!! Failed to execute docker compose down: {}", e);
            }
        }
    }
}
