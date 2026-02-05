// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # LSP Client Pool Manager
//!
//! This module provides LSP client connection management for E2E tests,
//! enabling efficient LSP server process reuse and health monitoring.

use anyhow::Context;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::orchestrator::Engine;

/// LSP client configuration
#[derive(Debug, Clone)]
pub struct LspClientConfig {
    pub server_path: String,
    pub args: Vec<String>,
    pub env_vars: HashMap<String, String>,
    pub timeout_secs: u64,
}

impl Default for LspClientConfig {
    fn default() -> Self {
        Self {
            server_path: "unified-sql-lsp".to_string(),
            args: vec![],
            env_vars: HashMap::new(),
            timeout_secs: 30,
        }
    }
}

/// LSP client handle
#[derive(Debug)]
pub struct LspClient {
    pub id: Uuid,
    pub engine: Engine,
    pub process: Child,
    pub config: LspClientConfig,
    pub created_at: std::time::Instant,
    pub last_used: std::time::Instant,
}

/// LSP client pool manager
pub struct LspClientManager {
    clients: Arc<RwLock<HashMap<Uuid, Arc<Mutex<LspClient>>>>>,
    available: Arc<RwLock<Vec<Uuid>>>,
    max_clients: usize,
    config: LspClientConfig,
}

impl LspClientManager {
    /// Create new manager
    pub fn new(max_clients: usize, config: LspClientConfig) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            available: Arc::new(RwLock::new(Vec::new())),
            max_clients,
            config,
        }
    }

    /// Spawn new LSP client
    pub async fn spawn_client(&self, engine: Engine) -> anyhow::Result<Uuid> {
        let id = Uuid::new_v4();

        info!(%id, %engine, "Spawning LSP client");

        let mut command = Command::new(&self.config.server_path);
        command
            .args(&self.config.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Set environment variables
        for (key, value) in &self.config.env_vars {
            command.env(key, value);
        }

        let child = command
            .spawn()
            .with_context(|| format!("Failed to spawn LSP server: {}", self.config.server_path))?;

        let client = LspClient {
            id,
            engine,
            process: child,
            config: self.config.clone(),
            created_at: std::time::Instant::now(),
            last_used: std::time::Instant::now(),
        };

        let mut clients = self.clients.write().await;
        clients.insert(id, Arc::new(Mutex::new(client)));

        let mut available = self.available.write().await;
        available.push(id);

        info!(%id, "LSP client spawned successfully");
        Ok(id)
    }

    /// Acquire client from pool
    pub async fn acquire_client(&self, engine: Engine) -> anyhow::Result<Arc<Mutex<LspClient>>> {
        // First try to find available client for this engine
        let available_id = {
            let available = self.available.read().await;
            let clients = self.clients.read().await;

            available
                .iter()
                .find(|id| {
                    if let Some(client) = clients.get(*id) {
                        if let Ok(guard) = client.try_lock() {
                            return guard.engine == engine;
                        }
                    }
                    false
                })
                .copied()
        };

        if let Some(id) = available_id {
            let mut available = self.available.write().await;
            available.retain(|&x| x != id);

            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&id) {
                return Ok(client.clone());
            }
        }

        // Check if we can spawn new client
        let current_count = {
            let clients = self.clients.read().await;
            clients.len()
        };

        if current_count < self.max_clients {
            let id = self.spawn_client(engine).await?;
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&id) {
                return Ok(client.clone());
            }
        }

        anyhow::bail!("No available LSP clients for engine: {}", engine)
    }

    /// Release client back to pool
    pub async fn release_client(&self, id: Uuid) {
        let client_guard = {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&id) {
                client.clone()
            } else {
                return;
            }
        };

        if let Ok(mut client) = client_guard.try_lock() {
            client.last_used = std::time::Instant::now();
        }

        let mut available = self.available.write().await;
        if !available.contains(&id) {
            available.push(id);
        }
    }

    /// Health check all clients
    pub async fn health_check(&self) -> Vec<Uuid> {
        let mut unhealthy = Vec::new();
        let clients = self.clients.read().await;

        for (id, client) in clients.iter() {
            if let Ok(mut guard) = client.try_lock() {
                // Check if process is still running
                match guard.process.try_wait() {
                    Ok(None) => {
                        // Process still running, check if stale
                        let idle_duration = guard.last_used.elapsed();
                        if idle_duration > std::time::Duration::from_secs(600) {
                            warn!(%id, "Client idle for too long, marking as unhealthy");
                            unhealthy.push(*id);
                        }
                    }
                    Ok(Some(_)) => {
                        // Process exited
                        warn!(%id, "LSP process exited, marking as unhealthy");
                        unhealthy.push(*id);
                    }
                    Err(e) => {
                        error!(%id, "Failed to check process status: {}", e);
                        unhealthy.push(*id);
                    }
                }
            }
        }

        unhealthy
    }

    /// Remove and cleanup unhealthy clients
    pub async fn cleanup_unhealthy(&self, unhealthy: Vec<Uuid>) {
        for id in unhealthy {
            info!(%id, "Cleaning up unhealthy client");

            // Remove from available list
            let mut available = self.available.write().await;
            available.retain(|&x| x != id);
            drop(available);

            // Remove from clients map and kill process
            let mut clients = self.clients.write().await;
            if let Some(client) = clients.remove(&id) {
                if let Ok(mut guard) = client.try_lock() {
                    if let Err(e) = guard.process.kill().await {
                        warn!(%id, "Failed to kill LSP process: {}", e);
                    }
                }
            }
        }
    }

    /// Shutdown all clients
    pub async fn shutdown(&self) {
        info!("Shutting down LSP client manager");

        // Get all client IDs
        let ids: Vec<Uuid> = {
            let clients = self.clients.read().await;
            clients.keys().copied().collect()
        };

        // Kill all processes
        for id in ids {
            let clients = self.clients.read().await;
            if let Some(client) = clients.get(&id) {
                if let Ok(mut guard) = client.try_lock() {
                    if let Err(e) = guard.process.kill().await {
                        warn!(%id, "Failed to kill LSP process during shutdown: {}", e);
                    }
                }
            }
        }

        // Clear all data
        let mut clients = self.clients.write().await;
        let mut available = self.available.write().await;
        clients.clear();
        available.clear();

        info!("LSP client manager shutdown complete");
    }
}

impl Clone for LspClientManager {
    fn clone(&self) -> Self {
        Self {
            clients: self.clients.clone(),
            available: self.available.clone(),
            max_clients: self.max_clients,
            config: self.config.clone(),
        }
    }
}
