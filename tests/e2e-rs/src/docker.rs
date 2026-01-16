// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Docker Compose management for E2E tests
//!
//! Provides automatic startup and teardown of Docker Compose services.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, warn};

/// Docker Compose manager
pub struct DockerCompose {
    /// Path to docker-compose.yml file
    compose_file: PathBuf,
    /// Project name (to avoid conflicts with other projects)
    project_name: String,
    /// Whether we started the services (used for cleanup)
    started: bool,
}

impl DockerCompose {
    /// Create a new Docker Compose manager
    ///
    /// # Arguments
    ///
    /// * `compose_file` - Path to docker-compose.yml file
    /// * `project_name` - Unique project name for Docker Compose
    pub fn new<P: AsRef<Path>>(compose_file: P, project_name: String) -> Self {
        Self {
            compose_file: compose_file.as_ref().to_path_buf(),
            project_name,
            started: false,
        }
    }

    /// Create from default E2E test configuration
    pub fn from_default_config() -> Result<Self> {
        let workspace_dir = std::env::var("CARGO_MANIFEST_DIR")
            .or_else(|_| std::env::current_dir().map(|p| p.to_string_lossy().to_string()))?;

        let compose_file = PathBuf::from(workspace_dir).join("docker-compose.yml");

        if !compose_file.exists() {
            return Err(anyhow::anyhow!(
                "docker-compose.yml not found at {:?}",
                compose_file
            ));
        }

        Ok(Self::new(compose_file, "unified-sql-lsp-e2e".to_string()))
    }

    /// Check if services are already running
    pub async fn is_running(&self) -> Result<bool> {
        info!("Checking if Docker Compose services are running...");

        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file.to_string_lossy(),
                "-p",
                &self.project_name,
                "ps",
                "-q",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // If we get container IDs, services are running
        let running = !output.stdout.is_empty();
        info!("Docker Compose services running: {}", running);
        Ok(running)
    }

    /// Start Docker Compose services
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Docker Compose services...");

        // Check if already running
        if self.is_running().await? {
            info!("Services already running, skipping startup");
            self.started = false;
            return Ok(());
        }

        info!("Starting docker-compose up...");
        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file.to_string_lossy(),
                "-p",
                &self.project_name,
                "up",
                "-d",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to start Docker Compose services: {}",
                stderr
            ));
        }

        self.started = true;
        info!("Docker Compose services started successfully");

        // Wait for MySQL to be ready
        self.wait_for_mysql().await?;

        Ok(())
    }

    /// Wait for MySQL to be ready
    async fn wait_for_mysql(&self) -> Result<()> {
        info!("Waiting for MySQL to be ready...");

        let mut retries = 60; // 60 seconds max
        let interval = tokio::time::Duration::from_secs(1);

        while retries > 0 {
            // Check if MySQL is accepting connections
            let output = Command::new("docker")
                .args([
                    "exec",
                    "unified-sql-lsp-mysql",
                    "mysqladmin",
                    "ping",
                    "-h",
                    "localhost",
                    "-uroot",
                    "-proot_password",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await;

            if let Ok(out) = output {
                if out.status.success() {
                    info!("MySQL is ready!");
                    return Ok(());
                }
            }

            warn!("MySQL not ready yet, retrying... ({})", retries);
            tokio::time::sleep(interval).await;
            retries -= 1;
        }

        Err(anyhow::anyhow!("MySQL did not become ready in time"))
    }

    /// Stop Docker Compose services
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping Docker Compose services...");

        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file.to_string_lossy(),
                "-p",
                &self.project_name,
                "down",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to stop Docker Compose services: {}", stderr);
            // Don't return error, just warn
        } else {
            info!("Docker Compose services stopped successfully");
        }

        self.started = false;
        Ok(())
    }

    /// Remove volumes (for cleanup)
    pub async fn remove_volumes(&mut self) -> Result<()> {
        info!("Removing Docker Compose volumes...");

        let output = Command::new("docker")
            .args([
                "compose",
                "-f",
                &self.compose_file.to_string_lossy(),
                "-p",
                &self.project_name,
                "down",
                "-v",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("Failed to remove volumes: {}", stderr);
        } else {
            info!("Volumes removed successfully");
        }

        self.started = false;
        Ok(())
    }
}

impl Drop for DockerCompose {
    fn drop(&mut self) {
        // Note: Drop is synchronous, so we can't await async operations here
        // Users should call stop() explicitly or use cleanup_database()
        if self.started {
            warn!("DockerCompose dropped without stopping. Consider calling stop() explicitly.");
        }
    }
}
