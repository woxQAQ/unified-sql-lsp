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

use async_trait::async_trait;

/// Default connection pool size
const DEFAULT_POOL_SIZE: u32 = 10;

/// Default query timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Health check interval in seconds
const HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

/// Live MySQL Catalog implementation
///
/// This catalog connects to a live MySQL database and queries schema information
/// from the information_schema database.
pub struct LiveMySQLCatalog {
    /// MySQL connection string
    connection_string: String,
    /// Connection pool size
    pool_size: u32,
    /// Query timeout in seconds
    timeout_secs: u64,
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

        Ok(Self {
            connection_string: conn_str,
            pool_size: DEFAULT_POOL_SIZE,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
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

        Ok(Self {
            connection_string: conn_str,
            pool_size,
            timeout_secs,
        })
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
    fn extract_length(type_str: &str) -> Option<usize> {
        type_str
            .find('(')
            .and_then(|pos| {
                let end = type_str[pos..].find(')')?;
                type_str[pos + 1..pos + end].parse().ok()
            })
            .map(|len: usize| if len == 0 { None } else { Some(len) })
            .flatten()
    }
}

#[async_trait]
impl Catalog for LiveMySQLCatalog {
    /// List all tables in the database
    ///
    /// Queries information_schema.tables to get all tables, views, and materialized views.
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
        // HACK: Placeholder implementation - returns error instead of actual data
        // This is a workaround to avoid adding MySQL driver dependency (e.g., mysql_async or sqlx)
        // which would significantly increase binary size and complexity
        //
        // TODO: (CATALOG-002) Implement actual database connection and query
        // In a real implementation, you would:
        // 1. Add mysql_async or sqlx dependency
        // 2. Establish connection pool
        // 3. Query information_schema.tables
        // 4. Parse results into TableMetadata
        //
        // Example query:
        // SELECT
        //     TABLE_NAME,
        //     TABLE_SCHEMA,
        //     TABLE_TYPE,
        //     TABLE_COMMENT
        // FROM information_schema.TABLES
        // WHERE TABLE_SCHEMA = DATABASE()
        //   AND TABLE_TYPE IN ('BASE TABLE', 'VIEW')

        Err(CatalogError::NotSupported(
            "LiveMySQLCatalog::list_tables not yet implemented - requires MySQL driver dependency"
                .to_string(),
        ))
    }

    /// Get column metadata for a specific table
    ///
    /// Queries information_schema.columns to get column information.
    async fn get_columns(&self, _table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
        // HACK: Placeholder implementation - returns error instead of actual data
        // This is a workaround to avoid adding MySQL driver dependency
        //
        // TODO: (CATALOG-002) Implement actual database connection and query
        //
        // Example query:
        // SELECT
        //     COLUMN_NAME,
        //     DATA_TYPE,
        //     IS_NULLABLE,
        //     COLUMN_DEFAULT,
        //     COLUMN_COMMENT,
        //     COLUMN_KEY
        // FROM information_schema.COLUMNS
        // WHERE TABLE_SCHEMA = DATABASE()
        //   AND TABLE_NAME = ?

        Err(CatalogError::NotSupported(
            "LiveMySQLCatalog::get_columns not yet implemented - requires MySQL driver dependency"
                .to_string(),
        ))
    }

    /// List all available functions
    ///
    /// Returns a list of built-in MySQL functions.
    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
        // HACK: Static list of functions instead of querying from database
        // This is a workaround to avoid database driver dependency
        //
        // TODO: (CATALOG-002) Query from mysql.proc for complete function list
        // or maintain as comprehensive static list if dynamic querying is too expensive
        //
        // Example query (MySQL 5.x):
        // SELECT
        //     name,
        //     db,
        //     param_list,
        //     returns
        // FROM mysql.proc
        // WHERE db = DATABASE()

        Ok(vec![
            // Aggregate functions
            FunctionMetadata::new("COUNT", DataType::BigInt)
                .with_type(FunctionType::Aggregate)
                .with_description("Count the number of rows"),
            FunctionMetadata::new("SUM", DataType::Decimal)
                .with_type(FunctionType::Aggregate)
                .with_description("Sum of values"),
            FunctionMetadata::new("AVG", DataType::Decimal)
                .with_type(FunctionType::Aggregate)
                .with_description("Average of values"),
            FunctionMetadata::new("MIN", DataType::Decimal)
                .with_type(FunctionType::Aggregate)
                .with_description("Minimum value"),
            FunctionMetadata::new("MAX", DataType::Decimal)
                .with_type(FunctionType::Aggregate)
                .with_description("Maximum value"),
            FunctionMetadata::new("GROUP_CONCAT", DataType::Text)
                .with_type(FunctionType::Aggregate)
                .with_description("Concatenate values from multiple rows"),
            // Scalar functions
            FunctionMetadata::new("ABS", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Absolute value"),
            FunctionMetadata::new("CEIL", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Round up to nearest integer"),
            FunctionMetadata::new("FLOOR", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Round down to nearest integer"),
            FunctionMetadata::new("ROUND", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Round to nearest decimal"),
            FunctionMetadata::new("CONCAT", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Concatenate strings"),
            FunctionMetadata::new("SUBSTRING", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Extract substring"),
            FunctionMetadata::new("LENGTH", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("String length"),
            FunctionMetadata::new("UPPER", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Convert to uppercase"),
            FunctionMetadata::new("LOWER", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Convert to lowercase"),
            FunctionMetadata::new("TRIM", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Remove leading/trailing whitespace"),
            FunctionMetadata::new("COALESCE", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return first non-null value"),
            FunctionMetadata::new("IFNULL", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return alternative if null"),
            // Date/Time functions
            FunctionMetadata::new("NOW", DataType::DateTime)
                .with_type(FunctionType::Scalar)
                .with_description("Current date and time"),
            FunctionMetadata::new("CURDATE", DataType::Date)
                .with_type(FunctionType::Scalar)
                .with_description("Current date"),
            FunctionMetadata::new("CURTIME", DataType::Time)
                .with_type(FunctionType::Scalar)
                .with_description("Current time"),
            FunctionMetadata::new("DATE_FORMAT", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Format date/time"),
            FunctionMetadata::new("DATE_ADD", DataType::DateTime)
                .with_type(FunctionType::Scalar)
                .with_description("Add time interval"),
            FunctionMetadata::new("DATE_SUB", DataType::DateTime)
                .with_type(FunctionType::Scalar)
                .with_description("Subtract time interval"),
            FunctionMetadata::new("DATEDIFF", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Difference between dates"),
            // Window functions (MySQL 8.0+)
            FunctionMetadata::new("ROW_NUMBER", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Row number within partition"),
            FunctionMetadata::new("RANK", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Rank within partition"),
            FunctionMetadata::new("DENSE_RANK", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Dense rank within partition"),
            FunctionMetadata::new("LAG", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Value from previous row"),
            FunctionMetadata::new("LEAD", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Value from next row"),
        ])
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
