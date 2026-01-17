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
use std::sync::Arc;
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

    /// Truncate all tables to ensure clean state at test start
    async fn truncate_tables(&self) -> Result<()>;
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
            container_name: "unified-sql-lsp-mysql-57".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        }
    }

    /// Create from default test configuration
    ///
    /// Matches docker-compose.yml configuration for MySQL 5.7
    pub fn from_default_config() -> Self {
        Self::new("mysql://test_user:test_password@127.0.0.1:3307/test_db".to_string())
    }

    /// Create for MySQL 5.7
    pub fn mysql_57() -> Self {
        Self {
            connection_string: "mysql://test_user:test_password@127.0.0.1:3307/test_db".to_string(),
            container_name: "unified-sql-lsp-mysql-57".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        }
    }

    /// Create for MySQL 8.0
    pub fn mysql_80() -> Self {
        Self {
            connection_string: "mysql://test_user:test_password@127.0.0.1:3308/test_db".to_string(),
            container_name: "unified-sql-lsp-mysql-80".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
            password: "test_password".to_string(),
        }
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
            "-i", // Keep STDIN open for passing SQL
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
            "-i", // Keep STDIN open for passing SQL
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
        tracing::info!("MySQL cleanup: dropping and recreating database");

        // Try to clean up, but don't fail if it doesn't work
        // The schema file has DROP TABLE IF EXISTS statements that handle partial cleanup

        // First, try to kill connections to the database
        let kill_all_sql = format!(
            "SELECT GROUP_CONCAT(CONCAT('KILL ', id, ';') SEPARATOR '') \
             FROM information_schema.PROCESSLIST \
             WHERE Db = '{}' AND Id != CONNECTION_ID();",
            self.database_name
        );

        let list_cmd = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-i",
                &self.container_name,
                "mysql",
                &format!("-u{}", &self.username),
                &format!("-p{}", &self.password),
                "-N",
                "-e",
                &kill_all_sql,
            ])
            .output()
            .await;

        if let Ok(output) = list_cmd {
            if output.status.success() {
                let kill_commands = String::from_utf8_lossy(&output.stdout);
                if !kill_commands.trim().is_empty() {
                    tracing::info!(
                        "Killing MySQL connections to database '{}'",
                        self.database_name
                    );
                    let exec_kill_cmd = tokio::process::Command::new("docker")
                        .args([
                            "exec",
                            "-i",
                            &self.container_name,
                            "mysql",
                            &format!("-u{}", &self.username),
                            &format!("-p{}", &self.password),
                            "-e",
                            &kill_commands,
                        ])
                        .output()
                        .await;

                    if let Ok(kill_result) = exec_kill_cmd {
                        if kill_result.status.success() {
                            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                        }
                    }
                }
            }
        }

        // Try to drop database
        let drop_sql = format!("DROP DATABASE IF EXISTS `{}`", self.database_name);
        let drop_cmd = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-i",
                &self.container_name,
                "mysql",
                &format!("-u{}", &self.username),
                &format!("-p{}", &self.password),
                "-e",
                &drop_sql,
            ])
            .output()
            .await;

        if drop_cmd.is_ok() {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // Try to create database (may fail if it already exists, which is OK)
        let create_sql = format!("CREATE DATABASE IF NOT EXISTS `{}`", self.database_name);
        let create_cmd = tokio::process::Command::new("docker")
            .args([
                "exec",
                "-i",
                &self.container_name,
                "mysql",
                &format!("-u{}", &self.username),
                &format!("-p{}", &self.password),
                "-e",
                &create_sql,
            ])
            .output()
            .await?;

        eprintln!(
            "!!! MySQL cleanup: DROP={:?}, CREATE={:?}",
            drop_cmd.as_ref().map(|c| c.status.code()).ok(),
            create_cmd.status.code()
        );

        if !create_cmd.status.success() {
            let stderr = String::from_utf8_lossy(&create_cmd.stderr);
            tracing::warn!("CREATE DATABASE failed (non-fatal): {}", stderr);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        tracing::info!("MySQL cleanup complete (may have partial state)");
        Ok(())
    }

    async fn truncate_tables(&self) -> Result<()> {
        tracing::info!("Truncating all tables in MySQL database...");

        let sql = "SET FOREIGN_KEY_CHECKS = 0; \
            DROP TABLE IF EXISTS order_items; \
            DROP TABLE IF EXISTS orders; \
            DROP TABLE IF EXISTS products; \
            DROP TABLE IF EXISTS post_tags; \
            DROP TABLE IF EXISTS posts; \
            DROP TABLE IF EXISTS tags; \
            DROP TABLE IF EXISTS employees; \
            DROP TABLE IF EXISTS logs; \
            DROP TABLE IF EXISTS users; \
            DROP TABLE IF EXISTS v_active_users; \
            DROP TABLE IF EXISTS v_order_summaries; \
            SET FOREIGN_KEY_CHECKS = 1;";

        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            "-i",
            &self.container_name,
            "mysql",
            &format!("-u{}", &self.username),
            &format!("-p{}", &self.password),
            "-e",
            &sql,
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
                "Failed to drop/truncate MySQL tables. Status: {:?}. Stderr: {}",
                output.status,
                stderr
            ));
        }

        tracing::info!("MySQL tables truncated successfully");
        Ok(())
    }
}

/// PostgreSQL database adapter
pub struct PostgreSQLAdapter {
    connection_string: String,
    container_name: String,
    database_name: String,
    username: String,
}

impl PostgreSQLAdapter {
    /// Create a new PostgreSQL adapter
    pub fn new(connection_string: String) -> Self {
        Self {
            connection_string,
            container_name: "unified-sql-lsp-postgresql-12".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
        }
    }

    /// Create from default test configuration
    ///
    /// Matches docker-compose.yml configuration for PostgreSQL 12
    pub fn from_default_config() -> Self {
        Self::new("postgresql://test_user:test_password@127.0.0.1:5433/test_db".to_string())
    }

    /// Create for PostgreSQL 12
    pub fn postgresql_12() -> Self {
        Self {
            connection_string: "postgresql://test_user:test_password@127.0.0.1:5433/test_db"
                .to_string(),
            container_name: "unified-sql-lsp-postgresql-12".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
        }
    }

    /// Create for PostgreSQL 16
    pub fn postgresql_16() -> Self {
        Self {
            connection_string: "postgresql://test_user:test_password@127.0.0.1:5434/test_db"
                .to_string(),
            container_name: "unified-sql-lsp-postgresql-16".to_string(),
            database_name: "test_db".to_string(),
            username: "test_user".to_string(),
        }
    }
}

#[async_trait]
impl DatabaseAdapter for PostgreSQLAdapter {
    fn connection_string(&self) -> &str {
        &self.connection_string
    }

    async fn load_schema(&self, schema_path: &Path) -> Result<()> {
        let sql = std::fs::read_to_string(schema_path)?;

        tracing::info!("Loading PostgreSQL schema from: {:?}", schema_path);

        // Execute SQL using docker exec with psql
        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            "-i", // Keep STDIN open for passing SQL
            &self.container_name,
            "psql",
            &format!("-U{}", &self.username),
            &format!("-d{}", &self.database_name),
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
                "Failed to load PostgreSQL schema from {:?}. Status: {:?}. Stderr: {}",
                schema_path,
                output.status,
                stderr
            ));
        }

        tracing::info!("PostgreSQL schema loaded successfully");
        Ok(())
    }

    async fn load_data(&self, data_path: &Path) -> Result<()> {
        let sql = std::fs::read_to_string(data_path)?;

        tracing::info!("Loading PostgreSQL data from: {:?}", data_path);

        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            "-i", // Keep STDIN open for passing SQL
            &self.container_name,
            "psql",
            &format!("-U{}", &self.username),
            &format!("-d{}", &self.database_name),
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
                "Failed to load PostgreSQL data from {:?}. Status: {:?}. Stderr: {}",
                data_path,
                output.status,
                stderr
            ));
        }

        tracing::info!("PostgreSQL data loaded successfully");
        Ok(())
    }

    async fn cleanup(&self) -> Result<()> {
        tracing::info!("PostgreSQL cleanup: dropping and recreating database");

        // Drop and recreate database
        let sql = format!(
            "DROP DATABASE IF EXISTS \"{}\"; CREATE DATABASE \"{}\";",
            self.database_name, self.database_name
        );

        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            "-i",
            &self.container_name,
            "psql",
            &format!("-U{}", &self.username),
            "postgres", // Connect to default postgres database
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
                "Failed to cleanup PostgreSQL database. Status: {:?}. Stderr: {}",
                output.status,
                stderr
            ));
        }

        tracing::info!("PostgreSQL cleanup complete");
        Ok(())
    }

    async fn truncate_tables(&self) -> Result<()> {
        tracing::info!("Truncating all tables in PostgreSQL database...");

        let sql = "DO $$
            BEGIN;
            TRUNCATE TABLE users, products, orders, order_items, employees, posts, tags, post_tags, logs RESTART IDENTITY CASCADE;
            COMMIT;
            $$;";

        let mut cmd = tokio::process::Command::new("docker");
        cmd.args([
            "exec",
            "-i",
            &self.container_name,
            "psql",
            &format!("-U{}", &self.username),
            &format!("-d{}", &self.database_name),
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
                "Failed to truncate PostgreSQL tables. Status: {:?}. Stderr: {}",
                output.status,
                stderr
            ));
        }

        tracing::info!("PostgreSQL tables truncated successfully");
        Ok(())
    }
}

/// Create database adapter based on test path
///
/// Parses the test file path to determine which database engine/version to use.
///
/// # Examples
///
/// ```
/// # use unified_sql_lsp_e2e_core::db::adapter_from_test_path;
/// # use std::path::Path;
/// let adapter = adapter_from_test_path(Path::new("tests/mysql-5.7/completion/test.yaml")).unwrap();
/// // Returns MySQL 5.7 adapter
///
/// let adapter = adapter_from_test_path(Path::new("tests/postgresql-12/completion/test.yaml")).unwrap();
/// // Returns PostgreSQL 12 adapter
/// ```
pub fn adapter_from_test_path(test_path: &std::path::Path) -> Result<Arc<dyn DatabaseAdapter>> {
    let path_str = test_path.to_string_lossy();

    if path_str.contains("/mysql-5.7/") || path_str.contains("\\mysql-5.7\\") {
        Ok(Arc::new(MySQLAdapter::mysql_57()) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/mysql-8.0/") || path_str.contains("\\mysql-8.0\\") {
        Ok(Arc::new(MySQLAdapter::mysql_80()) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/postgresql-12/") || path_str.contains("\\postgresql-12\\") {
        Ok(Arc::new(PostgreSQLAdapter::postgresql_12()) as Arc<dyn DatabaseAdapter>)
    } else if path_str.contains("/postgresql-16/") || path_str.contains("\\postgresql-16\\") {
        Ok(Arc::new(PostgreSQLAdapter::postgresql_16()) as Arc<dyn DatabaseAdapter>)
    } else {
        // Fallback to default (MySQL 5.7 for backward compatibility)
        tracing::warn!(
            "Could not determine database from path '{}', using MySQL 5.7 default",
            path_str
        );
        Ok(Arc::new(MySQLAdapter::mysql_57()) as Arc<dyn DatabaseAdapter>)
    }
}
