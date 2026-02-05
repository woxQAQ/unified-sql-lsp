// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Engine-level database lifecycle management
//!
//! This module provides engine-scoped database lifecycle management for E2E tests.
//! Databases are created once per engine (when first test runs) and destroyed once
//! (when last test for that engine completes).
//!
//! # Architecture
//!
//! - **Engine enum**: Represents supported database engines (MySQL 5.7/8.0, PostgreSQL 12/16)
//! - **EngineState**: Tracks active test count and initialization state per engine
//! - **EngineGuard**: RAII guard that auto-creates/destroys databases on drop
//! - **ensure_engine_ready()**: Entry point for tests to request engine readiness
//!
//! # Thread Safety
//!
//! Uses `LazyLock` and `AtomicUsize` for thread-safe parallel test execution.
//! Tests for the same engine are serialized via `serial_test` crate with engine-specific keys.
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_e2e::engine_manager::{Engine, ensure_engine_ready};
//! use serial_test::serial;
//!
//! #[tokio::test]
//! #[serial(mysql_57)]  // Ensures tests for same engine run serially
//! async fn test_completion() -> anyhow::Result<()> {
//!     let _guard = ensure_engine_ready(&Engine::MySQL57).await?;  // Auto-cleanup on drop
//!     run_suite("tests/mysql-5.7/completion/test.yaml").await
//! }
//! ```
//!
//! # Execution Flow
//!
//! 1. First test for MySQL 5.7 → `ensure_engine_ready()` creates database
//! 2. Subsequent MySQL 5.7 tests → reuse same database (no recreate)
//! 3. PostgreSQL 12 tests → run in parallel (different serial key)
//! 4. Last MySQL 5.7 test completes → `EngineGuard::drop()` destroys database

use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, OnceLock};

use crate::db::adapter::adapter_from_test_path;
use crate::debug_log;
use crate::docker::DockerCompose;

/// Database engine enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    MySQL57,
    MySQL80,
    PostgreSQL12,
    PostgreSQL16,
}

impl Engine {
    /// Parse engine from test file path
    ///
    /// # Examples
    ///
    /// ```
    /// # use unified_sql_lsp_e2e_core::engine_manager::Engine;
    /// let path = std::path::Path::new("tests/mysql-5.7/completion/test.yaml");
    /// assert_eq!(Engine::from_path(path), Engine::MySQL57);
    /// ```
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy();

        if path_str.contains("/mysql-5.7/") || path_str.contains("\\mysql-5.7\\") {
            Engine::MySQL57
        } else if path_str.contains("/mysql-8.0/") || path_str.contains("\\mysql-8.0\\") {
            Engine::MySQL80
        } else if path_str.contains("/postgresql-12/") || path_str.contains("\\postgresql-12\\") {
            Engine::PostgreSQL12
        } else if path_str.contains("/postgresql-16/") || path_str.contains("\\postgresql-16\\") {
            Engine::PostgreSQL16
        } else {
            tracing::warn!(
                "Could not determine engine from path '{}', using MySQL 5.7 default",
                path_str
            );
            Engine::MySQL57
        }
    }

    /// Get serial test key for this engine
    ///
    /// Used with `#[serial(key)]` attribute to ensure tests for same engine run serially.
    pub fn serial_key(&self) -> &'static str {
        match self {
            Engine::MySQL57 => "mysql_57",
            Engine::MySQL80 => "mysql_80",
            Engine::PostgreSQL12 => "postgresql_12",
            Engine::PostgreSQL16 => "postgresql_16",
        }
    }

    /// Get database adapter for this engine
    async fn adapter(&self) -> Result<Arc<dyn crate::db::adapter::DatabaseAdapter>> {
        let test_path = match self {
            Engine::MySQL57 => Path::new("tests/mysql-5.7/test.yaml"),
            Engine::MySQL80 => Path::new("tests/mysql-8.0/test.yaml"),
            Engine::PostgreSQL12 => Path::new("tests/postgresql-12/test.yaml"),
            Engine::PostgreSQL16 => Path::new("tests/postgresql-16/test.yaml"),
        };

        adapter_from_test_path(test_path)
    }
}

/// Engine state tracking
///
/// Tracks active test count and initialization state for a single engine.
struct EngineState {
    /// Number of currently active tests for this engine
    active_test_count: AtomicUsize,
    /// Whether database has been initialized (OnceLock for thread-safe lazy init)
    database_initialized: OnceLock<bool>,
}

impl EngineState {
    fn new() -> Self {
        Self {
            active_test_count: AtomicUsize::new(0),
            database_initialized: OnceLock::new(),
        }
    }
}

/// Global engine state registry
///
/// Lazy-initialized map of Engine → EngineState.
static ENGINE_STATES: LazyLock<HashMap<Engine, Arc<EngineState>>> = LazyLock::new(|| {
    HashMap::from([
        (Engine::MySQL57, Arc::new(EngineState::new())),
        (Engine::MySQL80, Arc::new(EngineState::new())),
        (Engine::PostgreSQL12, Arc::new(EngineState::new())),
        (Engine::PostgreSQL16, Arc::new(EngineState::new())),
    ])
});

/// Global Docker Compose manager (shared across all engines)
static DOCKER_COMPOSE: LazyLock<Arc<tokio::sync::RwLock<Option<DockerCompose>>>> =
    LazyLock::new(|| Arc::new(tokio::sync::RwLock::new(None)));

fn should_cleanup_docker_on_process_exit() -> bool {
    match std::env::var("E2E_AUTO_DOCKER_CLEANUP") {
        Ok(value) => value != "0" && !value.eq_ignore_ascii_case("false"),
        Err(_) => true,
    }
}

/// Global cleanup function called when test process exits
///
/// This uses the `ctor` crate to register a destructor that runs when
/// the test binary exits, ensuring Docker Compose services are stopped.
#[ctor::dtor]
fn global_cleanup() {
    use std::sync::atomic::{AtomicBool, Ordering};

    static CLEANED_UP: AtomicBool = AtomicBool::new(false);

    // Only cleanup once
    if !CLEANED_UP.swap(true, Ordering::SeqCst) {
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
}

/// Engine lifecycle guard
///
/// RAII guard that:
/// - Increments active test count on creation
/// - Initializes database if first test for engine
/// - Decrements counter and destroys database on drop if last test
///
/// # Example
///
/// ```rust,ignore
/// let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
/// // Test code here...
/// // When _guard goes out of scope, database is destroyed if this was last test
/// ```
pub struct EngineGuard {
    engine: Engine,
}

impl EngineGuard {
    /// Create a new engine guard
    ///
    /// This function:
    /// 1. Increments the active test count for the engine
    /// 2. Starts Docker Compose if this is the very first test across all engines
    /// 3. Initializes the database if this is the first test for this engine
    ///
    /// The returned guard will automatically destroy the database on drop
    /// if this was the last test for the engine.
    pub async fn new(engine: Engine) -> Result<Self> {
        let state = ENGINE_STATES
            .get(&engine)
            .ok_or_else(|| anyhow::anyhow!("Engine not found in state registry"))?;

        // Start Docker Compose services if not already started
        {
            let docker_guard = DOCKER_COMPOSE.read().await;
            if docker_guard.is_none() {
                drop(docker_guard);
                tracing::info!("Starting Docker Compose services...");
                let mut docker_compose = DockerCompose::from_default_config()?;
                docker_compose.start().await?;

                let mut compose_guard = DOCKER_COMPOSE.write().await;
                *compose_guard = Some(docker_compose);
                tracing::info!("Docker Compose services started");
            }
        }

        let test_index = state.active_test_count.fetch_add(1, Ordering::SeqCst);

        tracing::info!(
            "Engine {} test {} started (active: {})",
            engine.serial_key(),
            test_index,
            test_index + 1
        );

        state.database_initialized.get_or_init(|| {
            tracing::info!(
                "First test for engine {}, initializing database...",
                engine.serial_key()
            );
            true
        });

        if test_index == 0 {
            tracing::info!(
                "Initializing database for engine {}...",
                engine.serial_key()
            );
            let adapter = engine.adapter().await?;

            if let Err(e) = adapter.cleanup().await {
                tracing::warn!("Initial cleanup failed (non-fatal): {}", e);
            }

            tracing::info!("Database initialized for engine {}", engine.serial_key());
        }

        Ok(Self { engine })
    }
}

impl Drop for EngineGuard {
    fn drop(&mut self) {
        let state = ENGINE_STATES.get(&self.engine).unwrap();
        let count = state.active_test_count.fetch_sub(1, Ordering::SeqCst) - 1;

        tracing::info!(
            "Engine {} test ended (remaining: {})",
            self.engine.serial_key(),
            count
        );

        if count == 0 {
            tracing::info!(
                "Last test for engine {}, destroying database...",
                self.engine.serial_key()
            );
        }
    }
}

/// Ensure engine is ready for testing
///
/// Creates database if first test, returns guard for automatic cleanup.
///
/// # Arguments
///
/// * `engine` - The database engine to prepare
///
/// # Returns
///
/// A guard that will destroy the database when dropped (if last test)
///
/// # Example
///
/// ```rust,ignore
/// use unified_sql_lsp_e2e::engine_manager::{Engine, ensure_engine_ready};
///
/// #[tokio::test]
/// async fn my_test() -> anyhow::Result<()> {
///     let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
///     // Test code...
///     Ok(())
/// }
/// ```
pub async fn ensure_engine_ready(engine: &Engine) -> Result<EngineGuard> {
    EngineGuard::new(*engine).await
}

/// Cleanup database for a specific engine
///
/// Explicitly destroys the database for the given engine.
/// This is useful for manual cleanup or custom test scenarios.
///
/// # Arguments
///
/// * `engine` - The database engine to cleanup
pub async fn cleanup_engine(engine: &Engine) -> Result<()> {
    tracing::info!("Cleaning up database for engine {}...", engine.serial_key());

    let adapter = engine.adapter().await?;
    adapter.cleanup().await?;

    tracing::info!(
        "Database cleanup complete for engine {}",
        engine.serial_key()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_from_path() {
        assert_eq!(
            Engine::from_path(Path::new("tests/mysql-5.7/completion/test.yaml")),
            Engine::MySQL57
        );
        assert_eq!(
            Engine::from_path(Path::new("tests/mysql-8.0/completion/test.yaml")),
            Engine::MySQL80
        );
        assert_eq!(
            Engine::from_path(Path::new("tests/postgresql-12/completion/test.yaml")),
            Engine::PostgreSQL12
        );
        assert_eq!(
            Engine::from_path(Path::new("tests/postgresql-16/completion/test.yaml")),
            Engine::PostgreSQL16
        );
    }

    #[test]
    fn test_serial_key() {
        assert_eq!(Engine::MySQL57.serial_key(), "mysql_57");
        assert_eq!(Engine::MySQL80.serial_key(), "mysql_80");
        assert_eq!(Engine::PostgreSQL12.serial_key(), "postgresql_12");
        assert_eq!(Engine::PostgreSQL16.serial_key(), "postgresql_16");
    }
}
