// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Test Lifecycle Management
//!
//! This module provides test lifecycle hooks for E2E tests,
//! enabling proper resource initialization, cleanup, and state management.

use anyhow::{Context, Result};
use chrono;
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::orchestrator::Engine;

/// Test execution phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPhase {
    /// Before test execution starts
    Setup,
    /// During test execution
    Execution,
    /// After test execution completes
    Teardown,
}

/// Test context passed to lifecycle hooks
#[derive(Debug, Clone)]
pub struct TestContext {
    /// Test name
    pub name: String,
    /// Test phase
    pub phase: TestPhase,
    /// Database engine
    pub engine: Engine,
    /// YAML file path
    pub yaml_file: std::path::PathBuf,
    /// Case index in the YAML file
    pub case_index: usize,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl TestContext {
    /// Create new test context
    pub fn new(
        name: String,
        engine: Engine,
        yaml_file: std::path::PathBuf,
        case_index: usize,
    ) -> Self {
        Self {
            name,
            phase: TestPhase::Setup,
            engine,
            yaml_file,
            case_index,
            metadata: HashMap::new(),
        }
    }

    /// Set test phase
    pub fn set_phase(&mut self, phase: TestPhase) {
        self.phase = phase;
    }

    /// Add metadata
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
}

/// Test lifecycle trait
///
/// Implement this trait to provide custom lifecycle hooks for E2E tests.
#[async_trait::async_trait]
pub trait TestLifecycle: Send + Sync {
    /// Called before test execution starts
    ///
    /// Use this hook to:
    /// - Initialize database connections
    /// - Load test data
    /// - Start LSP clients
    /// - Prepare test environment
    async fn before_test(&self, ctx: &mut TestContext) -> Result<()>;

    /// Called after test execution completes
    ///
    /// Use this hook to:
    /// - Clean up test data
    /// - Release database connections
    /// - Stop LSP clients
    /// - Collect test artifacts
    async fn after_test(&self, ctx: &TestContext) -> Result<()>;

    /// Called when test execution fails
    ///
    /// Use this hook to:
    /// - Collect diagnostic information
    /// - Save error logs
    /// - Perform emergency cleanup
    async fn on_test_failure(&self, ctx: &TestContext, error: &anyhow::Error) -> Result<()> {
        warn!("Test {} failed: {}", ctx.name, error);
        Ok(())
    }
}

/// Default test lifecycle implementation
pub struct DefaultTestLifecycle;

#[async_trait::async_trait]
impl TestLifecycle for DefaultTestLifecycle {
    async fn before_test(&self, ctx: &mut TestContext) -> Result<()> {
        info!(
            test_name = %ctx.name,
            engine = %ctx.engine,
            "Setting up test environment"
        );

        // Add setup metadata
        ctx.add_metadata("setup_time", chrono::Utc::now().to_rfc3339());

        Ok(())
    }

    async fn after_test(&self, ctx: &TestContext) -> Result<()> {
        info!(
            test_name = %ctx.name,
            engine = %ctx.engine,
            "Cleaning up test environment"
        );

        Ok(())
    }
}

/// Test lifecycle manager
///
/// Manages the execution of test lifecycle hooks.
pub struct LifecycleManager {
    lifecycle: Box<dyn TestLifecycle>,
}

impl LifecycleManager {
    /// Create new lifecycle manager with default lifecycle
    pub fn new() -> Self {
        Self {
            lifecycle: Box::new(DefaultTestLifecycle),
        }
    }

    /// Create new lifecycle manager with custom lifecycle
    pub fn with_lifecycle<L: TestLifecycle + 'static>(lifecycle: L) -> Self {
        Self {
            lifecycle: Box::new(lifecycle),
        }
    }

    /// Execute test with lifecycle hooks
    pub async fn execute_test<F, Fut>(&self, mut ctx: TestContext, test_fn: F) -> anyhow::Result<()>
    where
        F: FnOnce(TestContext) -> Fut,
        Fut: std::future::Future<Output = anyhow::Result<()>>,
    {
        // Setup phase
        ctx.set_phase(TestPhase::Setup);
        if let Err(e) = self.lifecycle.before_test(&mut ctx).await {
            error!("Test setup failed: {}", e);
            return Err(e);
        }

        // Execution phase
        ctx.set_phase(TestPhase::Execution);
        let result = test_fn(ctx.clone()).await;

        // Handle test result
        if let Err(ref e) = result {
            if let Err(cleanup_err) = self.lifecycle.on_test_failure(&ctx, e).await {
                warn!("Test failure cleanup error: {}", cleanup_err);
            }
        }

        // Teardown phase
        ctx.set_phase(TestPhase::Teardown);
        if let Err(e) = self.lifecycle.after_test(&ctx).await {
            warn!("Test cleanup error: {}", e);
        }

        result
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for common lifecycle operations
pub mod helpers {
    use super::*;
    use crate::db_pool::DatabasePoolManager;
    use crate::lsp_pool::LspClientManager;
    use std::sync::Arc;

    /// Initialize database for test
    pub async fn init_database(
        pool: &Arc<DatabasePoolManager>,
        engine: Engine,
        connection_string: &str,
    ) -> anyhow::Result<()> {
        let config = crate::db_pool::DatabaseConfig {
            connection_string: connection_string.to_string(),
            pool_size: 5,
        };

        pool.create_pool(engine, &config)
            .await
            .with_context(|| format!("Failed to initialize database pool for {}", engine))?;

        info!(%engine, "Database initialized for test");
        Ok(())
    }

    /// Initialize LSP client for test
    pub async fn init_lsp_client(
        pool: &Arc<LspClientManager>,
        engine: Engine,
    ) -> anyhow::Result<uuid::Uuid> {
        let client_id = pool
            .spawn_client(engine)
            .await
            .with_context(|| format!("Failed to spawn LSP client for {}", engine))?;

        info!(%client_id, %engine, "LSP client initialized for test");
        Ok(client_id)
    }

    /// Cleanup database resources
    pub async fn cleanup_database(
        pool: &Arc<DatabasePoolManager>,
        engine: Engine,
    ) -> anyhow::Result<()> {
        pool.close_pool(engine)
            .await
            .with_context(|| format!("Failed to close database pool for {}", engine))?;

        info!(%engine, "Database resources cleaned up");
        Ok(())
    }

    /// Cleanup LSP client
    pub async fn cleanup_lsp_client(
        _pool: &Arc<LspClientManager>,
        client_id: uuid::Uuid,
    ) -> anyhow::Result<()> {
        // Note: In a full implementation, we would gracefully shutdown the client
        // For now, we rely on the health check cleanup
        info!(%client_id, "LSP client cleanup requested");
        Ok(())
    }
}
