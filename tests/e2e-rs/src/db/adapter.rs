// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Database adapter for E2E tests
//!
//! Manages database connections and schema/data setup.

use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Database adapter trait
#[async_trait]
pub trait DatabaseAdapter: Send + Sync {
    /// Get connection string
    fn connection_string(&self) -> &str;

    /// Load schema from SQL file
    async fn load_schema(&self, schema_path: &Path) -> Result<()>;

    /// Load data from SQL file
    async fn load_data(&self, data_path: &Path) -> Result<()>;

    /// Clean up test data
    async fn cleanup(&self) -> Result<()>;
}

/// MySQL database adapter
pub struct MySQLAdapter {
    connection_string: String,
    container_name: String,
    database_name: String,
    username: String,
    password: String,
}

impl MySQLAdapter {
    /// Create a new MySQL adapter
    pub fn new(connection_string: String) -> Self {
        // Parse connection string or use defaults
        Self {
            connection_string,
            container_name: "unified-sql-lsp-mysql".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        }
    }

    /// Create from default test configuration
    ///
    /// Matches docker-compose.yml configuration
    pub fn from_default_config() -> Self {
        Self::new("mysql://test_user:test_password@127.0.0.1:3307/test_db".to_string())
    }
}

#[async_trait]
impl DatabaseAdapter for MySQLAdapter {
    fn connection_string(&self) -> &str {
        &self.connection_string
    }

    async fn load_schema(&self, schema_path: &Path) -> Result<()> {
        let sql = std::fs::read_to_string(schema_path)?;

        tracing::info!("Loading schema from: {:?}", schema_path);

        // Execute SQL using docker exec
        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            &self.container_name,
            "mysql",
            &format!("-u{}", &self.username),
            &format!("-p{}", &self.password),
            &self.database_name,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        // Write SQL to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(sql.as_bytes()).await?;
            drop(stdin);
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to load schema from {:?}. Status: {:?}. Stderr: {}",
                schema_path,
                output.status,
                stderr
            ));
        }

        tracing::info!("Schema loaded successfully");
        Ok(())
    }

    async fn load_data(&self, data_path: &Path) -> Result<()> {
        let sql = std::fs::read_to_string(data_path)?;

        tracing::info!("Loading data from: {:?}", data_path);

        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            &self.container_name,
            "mysql",
            &format!("-u{}", &self.username),
            &format!("-p{}", &self.password),
            &self.database_name,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(sql.as_bytes()).await?;
            drop(stdin);
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to load data from {:?}. Status: {:?}. Stderr: {}",
                data_path,
                output.status,
                stderr
            ));
        }

        tracing::info!("Data loaded successfully");
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        // For now, just log cleanup
        // Future: truncate tables, reset sequences, etc.
        tracing::info!("MySQL cleanup complete (no-op for now)");
        Ok(())
    }
}

/// PostgreSQL database adapter (reserved for future use)
#[cfg(feature = "postgresql")]
pub struct PostgresAdapter {
    connection_string: String,
}

#[cfg(feature = "postgresql")]
impl PostgresAdapter {
    pub fn new(connection_string: String) -> Self {
        Self { connection_string }
    }

    pub fn from_default_config() -> Self {
        Self::new("postgresql://test_user:test_password@127.0.0.1:5433/test_db".to_string())
    }
}

#[cfg(feature = "postgresql")]
#[async_trait]
impl DatabaseAdapter for PostgresAdapter {
    fn connection_string(&self) -> &str {
        &self.connection_string
    }

    async fn load_schema(&self, _schema_path: &Path) -> Result<()> {
        Err(anyhow::anyhow!("PostgreSQL setup not implemented yet"))
    }

    async fn load_data(&self, _data_path: &Path) -> Result<()> {
        Err(anyhow::anyhow!("PostgreSQL setup not implemented yet"))
    }

    async fn cleanup(&self) -> Result<()> {
        Ok(())
    }
}
