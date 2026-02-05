// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Test Orchestrator
//!
//! This module provides the core orchestrator for managing E2E test resources,
//! including database engine lifecycle, connection pooling, and test registration.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio::sync::RwLock;
use tracing::{Span, info_span};
use uuid::Uuid;

/// Database engine types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Engine {
    MySQL57,
    MySQL80,
    PostgreSQL12,
    PostgreSQL16,
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engine::MySQL57 => write!(f, "mysql-5.7"),
            Engine::MySQL80 => write!(f, "mysql-8.0"),
            Engine::PostgreSQL12 => write!(f, "postgresql-12"),
            Engine::PostgreSQL16 => write!(f, "postgresql-16"),
        }
    }
}

/// Global test orchestrator
pub struct TestOrchestrator {
    test_registry: Arc<RwLock<TestRegistry>>,
    /// Database connection pool manager
    pub db_pool: Arc<crate::db_pool::DatabasePoolManager>,
    /// LSP client pool manager
    pub lsp_pool: Arc<crate::lsp_pool::LspClientManager>,
}

#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    pub max_concurrent_tests: usize,
    pub db_pool_size: usize,
    pub lsp_pool_size: usize,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tests: 10,
            db_pool_size: 10,
            lsp_pool_size: 5,
        }
    }
}

/// Test registry entry
#[derive(Debug)]
pub struct TestRegistryEntry {
    pub id: Uuid,
    pub name: String,
    pub engine: Engine,
    pub yaml_file: std::path::PathBuf,
    pub case_index: usize,
    pub started_at: std::time::Instant,
    pub span: Span,
}

/// Test registry
#[derive(Debug, Default)]
pub struct TestRegistry {
    tests: HashMap<Uuid, TestRegistryEntry>,
    by_engine: HashMap<Engine, Vec<Uuid>>,
}

/// Global test orchestrator instance
static GLOBAL_ORCHESTRATOR: OnceLock<Arc<TestOrchestrator>> = OnceLock::new();

impl TestOrchestrator {
    /// Create new orchestrator
    pub async fn new(config: OrchestratorConfig) -> anyhow::Result<Self> {
        let db_pool = Arc::new(crate::db_pool::DatabasePoolManager::new().await?);
        let lsp_config = crate::lsp_pool::LspClientConfig::default();
        let lsp_pool = Arc::new(crate::lsp_pool::LspClientManager::new(
            config.lsp_pool_size,
            lsp_config,
        ));

        Ok(Self {
            test_registry: Arc::new(RwLock::new(TestRegistry::default())),
            db_pool,
            lsp_pool,
        })
    }

    /// Initialize global orchestrator instance
    pub async fn initialize_global(config: OrchestratorConfig) -> anyhow::Result<Arc<Self>> {
        let orchestrator = Arc::new(Self::new(config).await?);

        GLOBAL_ORCHESTRATOR
            .set(orchestrator.clone())
            .map_err(|_| anyhow::anyhow!("Global orchestrator already initialized"))?;

        Ok(orchestrator)
    }

    /// Get global orchestrator
    ///
    /// Returns `None` if the global orchestrator has not been initialized.
    pub fn global() -> Option<Arc<Self>> {
        GLOBAL_ORCHESTRATOR.get().cloned()
    }

    /// Get or initialize global orchestrator
    ///
    /// If the global orchestrator has not been initialized, this method will
    /// return an error. Use `initialize_global` to initialize it first.
    pub fn try_global() -> anyhow::Result<Arc<Self>> {
        Self::global().ok_or_else(|| {
            anyhow::anyhow!("Global orchestrator not initialized. Call initialize_global() first.")
        })
    }

    /// Register a test
    pub async fn register_test(
        &self,
        name: String,
        engine: Engine,
        yaml_file: std::path::PathBuf,
        case_index: usize,
    ) -> TestHandle {
        let id = Uuid::new_v4();
        let span = info_span!(
            "test_case",
            test_name = %name,
            engine = %engine,
            yaml_file = %yaml_file.display(),
            case_index = case_index,
        );

        let entry = TestRegistryEntry {
            id,
            name: name.clone(),
            engine,
            yaml_file,
            case_index,
            started_at: std::time::Instant::now(),
            span,
        };

        let mut registry = self.test_registry.write().await;
        registry.tests.insert(id, entry);
        registry.by_engine.entry(engine).or_default().push(id);

        TestHandle
    }
}

/// Test handle for resource management
pub struct TestHandle;

impl TestOrchestrator {
    /// Get the database pool manager
    pub fn db_pool(&self) -> &Arc<crate::db_pool::DatabasePoolManager> {
        &self.db_pool
    }

    /// Get the LSP client pool manager
    pub fn lsp_pool(&self) -> &Arc<crate::lsp_pool::LspClientManager> {
        &self.lsp_pool
    }

    /// Initialize Docker services for testing
    ///
    /// This method starts Docker Compose services if they are not already running.
    /// It integrates with the existing docker module for service management.
    pub async fn init_docker(&self) -> anyhow::Result<()> {
        use tracing::info;

        info!("Initializing Docker Compose services...");

        // Check if services are already running
        let mut compose = crate::docker::DockerCompose::from_default_config()?;

        if compose.is_running().await? {
            info!("Docker Compose services already running");
            return Ok(());
        }

        // Start services
        compose.start().await?;
        info!("Docker Compose services started successfully");

        Ok(())
    }

    /// Shutdown Docker services
    ///
    /// This method stops Docker Compose services. Should be called after all tests complete.
    pub async fn shutdown_docker(&self) -> anyhow::Result<()> {
        use tracing::info;

        info!("Shutting down Docker Compose services...");

        let mut compose = crate::docker::DockerCompose::from_default_config()?;
        compose.stop().await?;

        info!("Docker Compose services stopped");

        Ok(())
    }
}
