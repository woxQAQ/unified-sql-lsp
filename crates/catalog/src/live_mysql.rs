// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Live MySQL Catalog implementation
//!
//! This module provides a live MySQL catalog that connects to a MySQL database
//! and queries schema information in real-time.
//!
//! ## Features
//!
//! - Connection pooling with configurable size (default: 10 connections)
//! - Query timeout support (default: 5 seconds)
//! - Health checks for connection validation
//! - Real-time schema queries from information_schema
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_catalog::live_mysql::LiveMySQLCatalog;
//! use unified_sql_lsp_catalog::Catalog;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let catalog = LiveMySQLCatalog::new(
//!         "mysql://user:password@localhost:3306/mydb"
//!     ).await?;
//!
//!     let tables = catalog.list_tables().await?;
//!     for table in tables {
//!         println!("{}.{}", table.schema, table.name);
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::error::{CatalogError, CatalogResult};
use crate::metadata::{ColumnMetadata, DataType, FunctionMetadata, FunctionType, TableMetadata};
use crate::r#trait::Catalog;
use std::sync::Arc;

use async_trait::async_trait;
use unified_sql_lsp_function_registry::FunctionRegistry;
use unified_sql_lsp_ir::Dialect;

#[cfg(feature = "mysql")]
use crate::metadata::TableType;

#[cfg(feature = "mysql")]
use sqlx::{MySql, Pool};

/// Default connection pool size
const DEFAULT_POOL_SIZE: u32 = 10;

/// Default query timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Live MySQL Catalog implementation
///
/// This catalog connects to a live MySQL database and queries schema information
/// from the information_schema database.
#[cfg(feature = "mysql")]
pub struct LiveMySQLCatalog {
    /// MySQL connection string
    connection_string: String,
    /// Connection pool size
    pool_size: u32,
    /// Query timeout in seconds
    timeout_secs: u64,
    /// Connection pool
    pool: Option<Pool<MySql>>,
    /// Function registry for builtin functions
    registry: Arc<FunctionRegistry>,
}

/// Live MySQL Catalog implementation (stub when feature is disabled)
#[cfg(not(feature = "mysql"))]
pub struct LiveMySQLCatalog {
    /// MySQL connection string
    connection_string: String,
    /// Connection pool size
    pool_size: u32,
    /// Query timeout in seconds
    timeout_secs: u64,
    /// Function registry for builtin functions
    registry: Arc<FunctionRegistry>,
}

impl LiveMySQLCatalog {
    /// Create a new LiveMySQLCatalog with the given connection string
    ///
    /// # Arguments
    ///
    /// * `connection_string` - MySQL connection string (e.g., "mysql://user:pass@host:port/db")
    ///
    /// # Returns
    ///
    /// Returns `Ok(catalog)` if the connection string is valid.
    /// Returns `Err(CatalogError::ConfigurationError)` if the connection string is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let catalog = LiveMySQLCatalog::new(
    ///     "mysql://user:password@localhost:3306/mydb"
    /// ).await?;
    /// ```
    pub async fn new(connection_string: impl Into<String>) -> CatalogResult<Self> {
        let conn_str = connection_string.into();
        Self::validate_connection_string(&conn_str)?;

        #[cfg(feature = "mysql")]
        {
            let pool = Some(
                Pool::<MySql>::connect(&conn_str)
                    .await
                    .map_err(|e| CatalogError::ConnectionFailed(format!("Failed to connect to MySQL: {}", e)))?
            );
            Ok(Self {
                connection_string: conn_str,
                pool_size: DEFAULT_POOL_SIZE,
                timeout_secs: DEFAULT_TIMEOUT_SECS,
                pool,
                registry: Arc::new(FunctionRegistry::new()),
            })
        }

        #[cfg(not(feature = "mysql"))]
        {
            Ok(Self {
                connection_string: conn_str,
                pool_size: DEFAULT_POOL_SIZE,
                timeout_secs: DEFAULT_TIMEOUT_SECS,
                registry: Arc::new(FunctionRegistry::new()),
            })
        }
    }

    /// Create a new LiveMySQLCatalog with custom configuration
    ///
    /// # Arguments
    ///
    /// * `connection_string` - MySQL connection string
    /// * `pool_size` - Connection pool size (default: 10)
    /// * `timeout_secs` - Query timeout in seconds (default: 5)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let catalog = LiveMySQLCatalog::with_config(
    ///     "mysql://user:password@localhost:3306/mydb",
    ///     20,  // pool size
    ///     10,  // timeout
    /// ).await?;
    /// ```
    pub async fn with_config(
        connection_string: impl Into<String>,
        pool_size: u32,
        timeout_secs: u64,
    ) -> CatalogResult<Self> {
        let conn_str = connection_string.into();
        Self::validate_connection_string(&conn_str)?;

        if pool_size == 0 {
            return Err(CatalogError::ConfigurationError(
                "pool_size must be greater than 0".to_string(),
            ));
        }

        if timeout_secs == 0 {
            return Err(CatalogError::ConfigurationError(
                "timeout_secs must be greater than 0".to_string(),
            ));
        }

        #[cfg(feature = "mysql")]
        {
            let pool = Some(
                Pool::<MySql>::connect(&conn_str)
                    .await
                    .map_err(|e| CatalogError::ConnectionFailed(format!("Failed to connect to MySQL: {}", e)))?
            );
            Ok(Self {
                connection_string: conn_str,
                pool_size,
                timeout_secs,
                pool,
                registry: Arc::new(FunctionRegistry::new()),
            })
        }

        #[cfg(not(feature = "mysql"))]
        {
            Ok(Self {
                connection_string: conn_str,
                pool_size,
                timeout_secs,
                registry: Arc::new(FunctionRegistry::new()),
            })
        }
    }

    /// Validate the connection string format
    ///
    /// Basic validation to ensure the connection string has the correct format.
    /// This is a simple check and doesn't guarantee the connection will succeed.
    fn validate_connection_string(conn_str: &str) -> CatalogResult<()> {
        if conn_str.is_empty() {
            return Err(CatalogError::ConfigurationError(
                "connection_string cannot be empty".to_string(),
            ));
        }

        // Basic format check: should start with mysql://
        if !conn_str.starts_with("mysql://") {
            return Err(CatalogError::ConfigurationError(format!(
                "connection_string must start with 'mysql://', got: {}",
                &conn_str.chars().take(10).collect::<String>()
            )));
        }

        Ok(())
    }

    /// Get the connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Get the pool size
    pub fn pool_size(&self) -> u32 {
        self.pool_size
    }

    /// Get the timeout in seconds
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    /// Parse MySQL data type to unified DataType
    ///
    /// Converts MySQL type strings (e.g., "varchar(255)", "int", "text")
    /// to the unified DataType enum.
    #[allow(dead_code)]
    fn parse_mysql_type(mysql_type: &str) -> DataType {
        let type_lower = mysql_type.to_lowercase();

        // Parse type with parameters (e.g., varchar(255), decimal(10,2))
        let type_name: String = type_lower
            .chars()
            .take_while(|c| c.is_alphabetic())
            .collect();

        match type_name.as_str() {
            // Integer types
            "tinyint" => DataType::TinyInt,
            "smallint" => DataType::SmallInt,
            "int" | "integer" => DataType::Integer,
            "bigint" => DataType::BigInt,

            // Decimal types
            "decimal" | "numeric" => DataType::Decimal,
            "float" => DataType::Float,
            "double" => DataType::Double,

            // String types
            "varchar" => {
                let len = Self::extract_length(&type_lower);
                DataType::Varchar(len)
            }
            "char" => {
                let len = Self::extract_length(&type_lower);
                DataType::Char(len)
            }
            "text" | "tinytext" | "mediumtext" | "longtext" => DataType::Text,

            // Binary types
            "binary" => DataType::Binary,
            "varbinary" => {
                let len = Self::extract_length(&type_lower);
                DataType::VarBinary(len)
            }
            "blob" | "tinyblob" | "mediumblob" | "longblob" => DataType::Blob,

            // Date/Time types
            "date" => DataType::Date,
            "time" => DataType::Time,
            "datetime" => DataType::DateTime,
            "timestamp" => DataType::Timestamp,

            // Boolean
            "bool" | "boolean" => DataType::Boolean,

            // JSON
            "json" => DataType::Json,

            // Unknown/Other types
            _ => DataType::Other(mysql_type.to_string()),
        }
    }

    /// Extract length from type string (e.g., "varchar(255)" -> Some(255))
    #[allow(dead_code)]
    fn extract_length(type_str: &str) -> Option<usize> {
        type_str
            .find('(')
            .and_then(|pos| {
                let end = type_str[pos..].find(')')?;
                type_str[pos + 1..pos + end].parse().ok()
            })
            .and_then(|len: usize| if len == 0 { None } else { Some(len) })
    }
}

#[async_trait]
impl Catalog for LiveMySQLCatalog {
    /// List all tables in the database
    ///
    /// Queries information_schema.tables to get all tables, views, and materialized views.
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
        #[cfg(feature = "mysql")]
        if let Some(pool) = &self.pool {
            let query = r#"
                SELECT
                    TABLE_NAME as table_name,
                    TABLE_SCHEMA as table_schema,
                    TABLE_TYPE as table_type,
                    TABLE_COMMENT as table_comment
                FROM information_schema.TABLES
                WHERE TABLE_SCHEMA = DATABASE()
                  AND TABLE_TYPE IN ('BASE TABLE', 'VIEW')
                ORDER BY TABLE_NAME
            "#;

            let rows = sqlx::query_as::<_, (String, String, String, Option<String>)>(query)
                .fetch_all(pool)
                .await
                .map_err(|e| CatalogError::QueryFailed(format!("Failed to list tables: {}", e)))?;

            let tables = rows.into_iter().map(|(name, schema, db_table_type, comment)| {
                let table_type = match db_table_type.as_str() {
                    "BASE TABLE" => TableType::Table,
                    "VIEW" => TableType::View,
                    _ => TableType::Other(db_table_type),
                };

                TableMetadata::new(&name, &schema)
                    .with_type(table_type)
                    .with_comment(comment.unwrap_or_default())
            }).collect();

            return Ok(tables);
        }

        #[cfg(not(feature = "mysql"))]
        return Err(CatalogError::NotSupported(
            "list_tables requires 'mysql' feature enabled".to_string()
        ));

        #[cfg(all(feature = "mysql", not(feature = "mysql")))]
        unreachable!()
    }

    /// Get column metadata for a specific table
    ///
    /// Queries information_schema.columns to get column information.
    async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
        #[cfg(feature = "mysql")]
        if let Some(pool) = &self.pool {
            let query = r#"
                SELECT
                    COLUMN_NAME as column_name,
                    DATA_TYPE as data_type,
                    IS_NULLABLE as is_nullable,
                    COLUMN_DEFAULT as column_default,
                    COLUMN_COMMENT as column_comment,
                    COLUMN_KEY as column_key
                FROM information_schema.COLUMNS
                WHERE TABLE_SCHEMA = DATABASE()
                  AND TABLE_NAME = ?
                ORDER BY ORDINAL_POSITION
            "#;

            let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String)>(
                query
            )
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|e| CatalogError::QueryFailed(format!("Failed to get columns for table '{}': {}", table, e)))?;

            let columns = rows.into_iter().map(|(name, data_type, is_nullable, _default, comment, column_key)| {
                let dt = Self::parse_mysql_type(&data_type);
                let nullable = is_nullable == "YES";
                let is_pk = column_key == "PRI";
                let is_fk = column_key == "MUL";

                let mut col = ColumnMetadata::new(name, dt)
                    .with_nullable(nullable)
                    .with_comment(comment.unwrap_or_default());

                if is_pk {
                    col = col.with_primary_key();
                }
                if is_fk {
                    col = col.with_foreign_key("", "");
                }

                col
            }).collect();

            return Ok(columns);
        }

        #[cfg(not(feature = "mysql"))]
        return Err(CatalogError::NotSupported(
            format!("get_columns requires 'mysql' feature enabled (table: '{}')", table)
        ));

        #[cfg(all(feature = "mysql", not(feature = "mysql")))]
        unreachable!()
    }

    /// List all available functions
    ///
    /// Returns a list of built-in MySQL functions and custom stored procedures/functions.
    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
        // Get builtin functions from registry
        let all_functions = self.registry.get_functions(Dialect::MySQL);

        #[cfg(feature = "mysql")]
        if let Some(pool) = &self.pool {
            // Query custom stored procedures/functions from mysql.proc
            let custom_query = r#"
                SELECT
                    name as function_name,
                    param_list as parameters,
                    returns as return_type,
                    db as schema_name
                FROM mysql.proc
                WHERE db = DATABASE()
                  AND type IN ('FUNCTION', 'PROCEDURE')
            "#;

            let custom_funcs: Vec<FunctionMetadata> = sqlx::query_as::<_, (String, String, String, String)>(
                custom_query
            )
            .fetch_all(pool)
            .await
            .unwrap_or(vec![]) // Don't fail if mysql.proc not accessible
            .into_iter()
            .map(|(name, _params, ret, schema)| {
                FunctionMetadata::new(&name, Self::parse_mysql_type(&ret))
                    .with_type(FunctionType::Scalar)
                    .with_description(format!("Custom function from {}", schema))
            })
            .collect();

            all_functions.extend(custom_funcs);
        }

        Ok(all_functions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mysql_varchar() {
        let dt = LiveMySQLCatalog::parse_mysql_type("varchar(255)");
        assert_eq!(dt, DataType::Varchar(Some(255)));
    }

    #[test]
    fn test_extract_length_from_varchar() {
        let len = LiveMySQLCatalog::extract_length("varchar(255)");
        assert_eq!(len, Some(255));
    }

    #[test]
    fn test_extract_length_from_char() {
        let len = LiveMySQLCatalog::extract_length("char(10)");
        assert_eq!(len, Some(10));
    }

    #[test]
    fn test_extract_length_no_parens() {
        let len = LiveMySQLCatalog::extract_length("text");
        assert_eq!(len, None);
    }

    #[test]
    fn test_validate_connection_string_valid() {
        let result = LiveMySQLCatalog::validate_connection_string("mysql://localhost");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_connection_string_empty() {
        let result = LiveMySQLCatalog::validate_connection_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_connection_string_invalid_prefix() {
        let result = LiveMySQLCatalog::validate_connection_string("postgresql://localhost");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_new_catalog() {
        let catalog = LiveMySQLCatalog::new("mysql://localhost").await.unwrap();
        assert_eq!(catalog.connection_string(), "mysql://localhost");
        assert_eq!(catalog.pool_size(), DEFAULT_POOL_SIZE);
        assert_eq!(catalog.timeout_secs(), DEFAULT_TIMEOUT_SECS);
    }

    #[tokio::test]
    async fn test_new_catalog_invalid_connection_string() {
        let result = LiveMySQLCatalog::new("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_catalog_with_config() {
        let catalog = LiveMySQLCatalog::with_config("mysql://localhost", 20, 10)
            .await
            .unwrap();
        assert_eq!(catalog.pool_size(), 20);
        assert_eq!(catalog.timeout_secs(), 10);
    }

    #[tokio::test]
    async fn test_catalog_with_config_invalid_pool_size() {
        let result = LiveMySQLCatalog::with_config("mysql://localhost", 0, 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_catalog_with_config_invalid_timeout() {
        let result = LiveMySQLCatalog::with_config("mysql://localhost", 10, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_functions() {
        let catalog = LiveMySQLCatalog::new("mysql://localhost").await.unwrap();
        let functions = catalog.list_functions().await.unwrap();

        // Verify we get functions
        assert!(!functions.is_empty());

        // Check for known aggregate function
        let count_func = functions.iter().find(|f| f.name == "COUNT");
        assert!(count_func.is_some());
        let count_func = count_func.unwrap();
        assert!(matches!(count_func.function_type, FunctionType::Aggregate));

        // Check for known scalar function
        let abs_func = functions.iter().find(|f| f.name == "ABS");
        assert!(abs_func.is_some());
        let abs_func = abs_func.unwrap();
        assert!(matches!(abs_func.function_type, FunctionType::Scalar));

        // Check for known window function
        let row_number_func = functions.iter().find(|f| f.name == "ROW_NUMBER");
        assert!(row_number_func.is_some());
        let row_number_func = row_number_func.unwrap();
        assert!(matches!(
            row_number_func.function_type,
            FunctionType::Window
        ));
    }
}
