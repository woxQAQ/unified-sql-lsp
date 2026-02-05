// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Database Pool Manager
//!
//! This module provides connection pool management for multiple database engines,
//! enabling efficient database connection reuse across E2E tests.

use anyhow::Context;
use sqlx::{Any, Pool, pool::PoolConnection};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::orchestrator::Engine;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub connection_string: String,
    pub pool_size: usize,
}

/// Database connection pool manager
#[derive(Clone)]
pub struct DatabasePoolManager {
    pools: Arc<RwLock<HashMap<Engine, Pool<Any>>>>,
}

impl DatabasePoolManager {
    /// Create new manager
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            pools: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create connection pool for specified engine
    pub async fn create_pool(&self, engine: Engine, config: &DatabaseConfig) -> anyhow::Result<()> {
        {
            let pools = self.pools.read().await;
            if pools.contains_key(&engine) {
                return Ok(());
            }
        }

        info!(%engine, "Creating database connection pool");

        let pool = sqlx::any::AnyPoolOptions::new()
            .max_connections(config.pool_size as u32)
            .min_connections(2)
            .idle_timeout(std::time::Duration::from_secs(300))
            .connect(&config.connection_string)
            .await
            .with_context(|| format!("Failed to create pool for {}", engine))?;

        // Test connection
        let _ = pool.acquire().await?;

        {
            let mut pools = self.pools.write().await;
            pools.insert(engine, pool);
        }
        info!(%engine, "Database connection pool ready");

        Ok(())
    }

    /// Get connection for specified engine
    pub async fn acquire(&self, engine: Engine) -> anyhow::Result<PoolConnection<Any>> {
        let pools = self.pools.read().await;
        let pool = pools
            .get(&engine)
            .ok_or_else(|| anyhow::anyhow!("No pool for engine: {}", engine))?;

        let conn = pool
            .acquire()
            .await
            .with_context(|| format!("Failed to acquire connection for {}", engine))?;

        Ok(conn)
    }

    /// Truncate all tables (clean data but keep structure)
    pub async fn truncate_tables(&self, engine: Engine) -> anyhow::Result<()> {
        let mut conn = self.acquire(engine).await?;

        let tables: Vec<String> = match engine {
            Engine::MySQL57 | Engine::MySQL80 => {
                sqlx::query_scalar(
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'",
                )
                .fetch_all(&mut *conn)
                .await?
            }
            Engine::PostgreSQL12 | Engine::PostgreSQL16 => {
                sqlx::query_scalar(
                    "SELECT tablename FROM pg_tables
                     WHERE schemaname = 'public'",
                )
                .fetch_all(&mut *conn)
                .await?
            }
        };

        for table in tables {
            let sql = format!("TRUNCATE TABLE `{}`", table);
            sqlx::query(&sql).execute(&mut *conn).await?;
        }

        Ok(())
    }

    /// Close connection pool for specified engine
    pub async fn close_pool(&self, engine: Engine) -> anyhow::Result<()> {
        let mut pools = self.pools.write().await;
        if let Some(pool) = pools.remove(&engine) {
            pool.close().await;
            info!(%engine, "Database connection pool closed");
        }
        Ok(())
    }
}
