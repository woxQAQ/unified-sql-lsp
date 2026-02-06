// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # LSP Engine Configuration
//!
//! This module provides configuration management for the LSP engine.
//!
//! ## Configuration Structure
//!
//! The engine configuration includes:
//! - SQL dialect (MySQL, PostgreSQL, TiDB)
//! - Dialect version (e.g., MySQL 8.0, PostgreSQL 14)
//! - Database connection settings
//! - Schema filters
//! - Performance tuning parameters
//!
//! ## Example
//!
//! ```rust,ignore
//! use unified_sql_lsp_lsp::{EngineConfig, Dialect, DialectVersion};
//!
//! let config = EngineConfig {
//!     dialect: Dialect::MySQL,
//!     version: DialectVersion::MySQL80,
//!     connection_string: "mysql://localhost:3306/mydb".to_string(),
//!     ..Default::default()
//! };
//! ```

use serde_json::Value;
use std::collections::HashSet;
use unified_sql_lsp_catalog::CatalogError;
use unified_sql_lsp_ir::Dialect;

/// SQL dialect version enumeration
///
/// Represents specific versions of SQL dialects for feature compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialectVersion {
    /// MySQL 5.7
    MySQL57,
    /// MySQL 8.0+
    MySQL80,
    /// PostgreSQL 12
    PostgreSQL12,
    /// PostgreSQL 14
    PostgreSQL14,
    /// PostgreSQL 16
    PostgreSQL16,
    /// TiDB 5.0
    TiDB50,
    /// TiDB 6.0
    TiDB60,
    /// TiDB 7.0
    TiDB70,
    /// TiDB 8.0
    TiDB80,
}

impl DialectVersion {
    /// Get the dialect for this version
    pub fn dialect(&self) -> Dialect {
        match self {
            DialectVersion::MySQL57 | DialectVersion::MySQL80 => Dialect::MySQL,
            DialectVersion::PostgreSQL12
            | DialectVersion::PostgreSQL14
            | DialectVersion::PostgreSQL16 => Dialect::PostgreSQL,
            DialectVersion::TiDB50
            | DialectVersion::TiDB60
            | DialectVersion::TiDB70
            | DialectVersion::TiDB80 => Dialect::TiDB,
        }
    }
}

/// Schema filter configuration
///
/// Controls which tables and schemas are visible in completion and diagnostics.
#[derive(Debug, Clone, Default)]
pub struct SchemaFilter {
    /// Allowed schemas (e.g., "public", "my_schema")
    /// If empty, all schemas are allowed
    pub allowed_schemas: HashSet<String>,

    /// Allowed table name patterns (e.g., "users_*", "temp.*")
    /// Supports glob patterns
    pub allowed_tables: Vec<String>,

    /// Excluded table name patterns
    /// Tables matching these patterns will be hidden
    pub excluded_tables: Vec<String>,
}

impl SchemaFilter {
    /// Create a new empty schema filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an allowed schema
    pub fn allow_schema(mut self, schema: impl Into<String>) -> Self {
        self.allowed_schemas.insert(schema.into());
        self
    }

    /// Add an allowed table pattern
    pub fn allow_table(mut self, pattern: impl Into<String>) -> Self {
        self.allowed_tables.push(pattern.into());
        self
    }

    /// Add an excluded table pattern
    pub fn exclude_table(mut self, pattern: impl Into<String>) -> Self {
        self.excluded_tables.push(pattern.into());
        self
    }

    /// Check if a schema is allowed
    pub fn is_schema_allowed(&self, schema: &str) -> bool {
        self.allowed_schemas.is_empty() || self.allowed_schemas.contains(schema)
    }

    /// Check if a table is allowed based on patterns
    ///
    /// Note: This is a basic implementation. Pattern matching will be
    /// enhanced in CATALOG-005.
    pub fn is_table_allowed(&self, table: &str) -> bool {
        // Check excluded patterns first
        for pattern in &self.excluded_tables {
            if table.contains(pattern) {
                return false;
            }
        }

        // If no allowed patterns, all tables are allowed
        if self.allowed_tables.is_empty() {
            return true;
        }

        // Check allowed patterns
        for pattern in &self.allowed_tables {
            if table.contains(pattern) {
                return true;
            }
        }

        false
    }
}

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of connections in the pool
    pub max_connections: usize,

    /// Minimum number of connections to maintain
    pub min_connections: usize,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_connections: 10,
            min_connections: 1,
            connection_timeout_secs: 30,
            idle_timeout_secs: 600,
        }
    }
}

/// Main engine configuration
///
/// Contains all settings for the LSP engine including dialect,
/// database connection, and performance tuning.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// SQL dialect
    pub dialect: Dialect,

    /// Dialect version
    pub version: DialectVersion,

    /// Database connection string
    /// Format: dialect://user:password@host:port/database
    pub connection_string: String,

    /// Schema filter configuration
    pub schema_filter: SchemaFilter,

    /// Connection pool configuration
    pub pool_config: ConnectionPoolConfig,

    /// Enable query logging
    pub log_queries: bool,

    /// Maximum query execution time for catalog queries (seconds)
    pub query_timeout_secs: u64,

    /// Cache enabled (will be used in PERF-001)
    pub cache_enabled: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            dialect: Dialect::MySQL,
            version: DialectVersion::MySQL80,
            connection_string: String::new(),
            schema_filter: SchemaFilter::default(),
            pool_config: ConnectionPoolConfig::default(),
            log_queries: false,
            query_timeout_secs: 5,
            cache_enabled: true,
        }
    }
}

impl EngineConfig {
    /// Create a new engine configuration
    pub fn new(
        dialect: Dialect,
        version: DialectVersion,
        connection_string: impl Into<String>,
    ) -> Self {
        Self {
            dialect,
            version,
            connection_string: connection_string.into(),
            ..Default::default()
        }
    }

    /// Validate the configuration
    ///
    /// Checks that:
    /// - Dialect and version are compatible
    /// - Connection string is not empty
    /// - Pool settings are reasonable
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check dialect/version compatibility
        if self.version.dialect() != self.dialect {
            return Err(ConfigError::IncompatibleVersion {
                dialect: self.dialect,
                version: self.version,
            });
        }

        // Check connection string
        if self.connection_string.is_empty() {
            return Err(ConfigError::MissingConnectionString);
        }

        // Validate connection string format
        if !self.connection_string.contains("://") {
            return Err(ConfigError::InvalidConnectionString {
                reason: "Missing protocol (e.g., mysql://, postgres://)".to_string(),
            });
        }

        // Check pool settings
        if self.pool_config.max_connections == 0 {
            return Err(ConfigError::InvalidPoolConfig {
                reason: "max_connections must be > 0".to_string(),
            });
        }

        if self.pool_config.min_connections > self.pool_config.max_connections {
            return Err(ConfigError::InvalidPoolConfig {
                reason: "min_connections cannot exceed max_connections".to_string(),
            });
        }

        Ok(())
    }

    /// Create a MySQL configuration
    pub fn mysql(
        version: DialectVersion,
        connection_string: impl Into<String>,
    ) -> Result<Self, ConfigError> {
        if version.dialect() != Dialect::MySQL {
            return Err(ConfigError::IncompatibleVersion {
                dialect: Dialect::MySQL,
                version,
            });
        }

        Ok(Self::new(Dialect::MySQL, version, connection_string))
    }

    /// Create a PostgreSQL configuration
    pub fn postgresql(
        version: DialectVersion,
        connection_string: impl Into<String>,
    ) -> Result<Self, ConfigError> {
        if version.dialect() != Dialect::PostgreSQL {
            return Err(ConfigError::IncompatibleVersion {
                dialect: Dialect::PostgreSQL,
                version,
            });
        }

        Ok(Self::new(Dialect::PostgreSQL, version, connection_string))
    }

    /// Create a TiDB configuration
    pub fn tidb(
        version: DialectVersion,
        connection_string: impl Into<String>,
    ) -> Result<Self, ConfigError> {
        if version.dialect() != Dialect::TiDB {
            return Err(ConfigError::IncompatibleVersion {
                dialect: Dialect::TiDB,
                version,
            });
        }

        Ok(Self::new(Dialect::TiDB, version, connection_string))
    }

    /// Parse engine config from LSP client settings payload.
    ///
    /// Expected shape:
    /// {
    ///   "unifiedSqlLsp": {
    ///     "dialect": "mysql" | "postgresql",
    ///     "version": "...",
    ///     "connectionString": "..."
    ///   }
    /// }
    pub fn from_lsp_settings(settings: &Value) -> Option<Self> {
        let lsp_settings = settings.get("unifiedSqlLsp")?;

        let dialect_str = lsp_settings.get("dialect")?.as_str()?;
        let dialect = match dialect_str {
            "mysql" => Dialect::MySQL,
            "postgresql" => Dialect::PostgreSQL,
            _ => return None,
        };

        let version_str = lsp_settings
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("8.0");

        let version = match (dialect, version_str) {
            (Dialect::MySQL, "5.7") => DialectVersion::MySQL57,
            (Dialect::MySQL, _) => DialectVersion::MySQL80,
            (Dialect::PostgreSQL, "12") => DialectVersion::PostgreSQL12,
            (Dialect::PostgreSQL, "14") => DialectVersion::PostgreSQL14,
            (Dialect::PostgreSQL, _) => DialectVersion::PostgreSQL16,
            _ => return None,
        };

        let connection_string = lsp_settings.get("connectionString")?.as_str()?.to_string();
        Some(Self::new(dialect, version, connection_string))
    }

    /// Default config used when client settings have not arrived yet.
    pub fn default_runtime_fallback() -> Self {
        let default_connection = std::env::var("E2E_MYSQL_CONNECTION").unwrap_or_else(|_| {
            "mysql://test_user:test_password@127.0.0.1:3307/test_db".to_string()
        });

        Self::new(Dialect::MySQL, DialectVersion::MySQL57, default_connection)
    }
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Incompatible dialect and version combination
    #[error("Dialect {dialect:?} is not compatible with version {version:?}")]
    IncompatibleVersion {
        dialect: Dialect,
        version: DialectVersion,
    },

    /// Missing connection string
    #[error("Connection string is required")]
    MissingConnectionString,

    /// Invalid connection string format
    #[error("Invalid connection string: {reason}")]
    InvalidConnectionString { reason: String },

    /// Invalid pool configuration
    #[error("Invalid pool configuration: {reason}")]
    InvalidPoolConfig { reason: String },

    /// Catalog-related error
    #[error("Catalog error: {0}")]
    CatalogError(#[from] CatalogError),
}
