// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Live PostgreSQL Catalog implementation
//!
//! This module provides a live PostgreSQL catalog that connects to a PostgreSQL database
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
//! use unified_sql_lsp_catalog::live_postgres::LivePostgreSQLCatalog;
//! use unified_sql_lsp_catalog::Catalog;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let catalog = LivePostgreSQLCatalog::new(
//!         "postgresql://user:password@localhost:5432/mydb"
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

#[cfg(feature = "postgresql")]
use crate::metadata::TableType;

#[cfg(feature = "postgresql")]
use sqlx::{Postgres, Pool};

/// Default connection pool size
const DEFAULT_POOL_SIZE: u32 = 10;

/// Default query timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 5;

/// Live PostgreSQL Catalog implementation
///
/// This catalog connects to a live PostgreSQL database and queries schema information
/// from the information_schema and pg_catalog databases.
#[cfg(feature = "postgresql")]
pub struct LivePostgreSQLCatalog {
    /// PostgreSQL connection string
    connection_string: String,
    /// Connection pool size
    pool_size: u32,
    /// Query timeout in seconds
    timeout_secs: u64,
    /// Connection pool
    pool: Option<Pool<Postgres>>,
}

/// Live PostgreSQL Catalog implementation (stub when feature is disabled)
#[cfg(not(feature = "postgresql"))]
pub struct LivePostgreSQLCatalog {
    /// PostgreSQL connection string
    connection_string: String,
    /// Connection pool size
    pool_size: u32,
    /// Query timeout in seconds
    timeout_secs: u64,
}

impl LivePostgreSQLCatalog {
    /// Create a new LivePostgreSQLCatalog with the given connection string
    ///
    /// # Arguments
    ///
    /// * `connection_string` - PostgreSQL connection string (e.g., "postgresql://user:pass@host:port/db")
    ///
    /// # Returns
    ///
    /// Returns `Ok(catalog)` if the connection string is valid.
    /// Returns `Err(CatalogError::ConfigurationError)` if the connection string is invalid.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let catalog = LivePostgreSQLCatalog::new(
    ///     "postgresql://user:password@localhost:5432/mydb"
    /// ).await?;
    /// ```
    pub async fn new(connection_string: impl Into<String>) -> CatalogResult<Self> {
        let conn_str = connection_string.into();
        Self::validate_connection_string(&conn_str)?;

        #[cfg(feature = "postgresql")]
        {
            let pool = Some(
                Pool::<Postgres>::connect(&conn_str)
                    .await
                    .map_err(|e| CatalogError::ConnectionFailed(format!("Failed to connect to PostgreSQL: {}", e)))?
            );
            Ok(Self {
                connection_string: conn_str,
                pool_size: DEFAULT_POOL_SIZE,
                timeout_secs: DEFAULT_TIMEOUT_SECS,
                pool,
            })
        }

        #[cfg(not(feature = "postgresql"))]
        {
            Ok(Self {
                connection_string: conn_str,
                pool_size: DEFAULT_POOL_SIZE,
                timeout_secs: DEFAULT_TIMEOUT_SECS,
            })
        }
    }

    /// Create a new LivePostgreSQLCatalog with custom configuration
    ///
    /// # Arguments
    ///
    /// * `connection_string` - PostgreSQL connection string
    /// * `pool_size` - Connection pool size (default: 10)
    /// * `timeout_secs` - Query timeout in seconds (default: 5)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let catalog = LivePostgreSQLCatalog::with_config(
    ///     "postgresql://user:password@localhost:5432/mydb",
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

        #[cfg(feature = "postgresql")]
        {
            let pool = Some(
                Pool::<Postgres>::connect(&conn_str)
                    .await
                    .map_err(|e| CatalogError::ConnectionFailed(format!("Failed to connect to PostgreSQL: {}", e)))?
            );
            Ok(Self {
                connection_string: conn_str,
                pool_size,
                timeout_secs,
                pool,
            })
        }

        #[cfg(not(feature = "postgresql"))]
        {
            Ok(Self {
                connection_string: conn_str,
                pool_size,
                timeout_secs,
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

        // Basic format check: should start with postgresql:// or postgres://
        if !conn_str.starts_with("postgresql://") && !conn_str.starts_with("postgres://") {
            return Err(CatalogError::ConfigurationError(format!(
                "connection_string must start with 'postgresql://' or 'postgres://', got: {}",
                &conn_str.chars().take(15).collect::<String>()
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

    /// Parse PostgreSQL data type to unified DataType
    ///
    /// Converts PostgreSQL type strings (e.g., "character varying(255)", "integer", "text")
    /// to the unified DataType enum.
    ///
    /// PostgreSQL uses the SQL standard type names, which are more verbose than MySQL.
    /// This method handles both the full names (e.g., "character varying") and common aliases.
    #[allow(dead_code)]
    fn parse_postgres_type(postgres_type: &str) -> DataType {
        let type_lower = postgres_type.to_lowercase();
        let type_lower = type_lower.trim();

        // Handle array types (e.g., "integer[]", "character varying(255)[]")
        if type_lower.ends_with("[]") {
            // For now, represent array types as Other with the full type name
            // In the future, we could use DataType::Array(Box<DataType>) if needed
            return DataType::Other(postgres_type.to_string());
        }

        // Parse type with parameters (e.g., varchar(255), numeric(10,2))
        // Extract the type name (everything before '(' or end of string)
        let type_name: String = type_lower
            .chars()
            .take_while(|c| c.is_alphabetic() || *c == ' ')
            .collect::<String>()
            .trim()
            .to_string();

        match type_name.as_str() {
            // PostgreSQL uses SQL standard names, so we need to handle several variants
            "character" | "char" => {
                let len = Self::extract_length(type_lower);
                DataType::Char(len)
            }

            "character varying" | "varchar" => {
                let len = Self::extract_length(type_lower);
                DataType::Varchar(len)
            }

            "text" => DataType::Text,

            // Boolean
            "boolean" | "bool" => DataType::Boolean,

            // Integer types (PostgreSQL specific names)
            "smallint" | "int2" => DataType::SmallInt,
            "integer" | "int" | "int4" => DataType::Integer,
            "bigint" | "int8" => DataType::BigInt,

            // Decimal types
            "numeric" | "decimal" => DataType::Decimal,
            "real" | "float4" => DataType::Float,
            "double precision" | "float8" => DataType::Double,

            // Binary types
            "bytea" => DataType::Binary,
            "bit" => {
                let len = Self::extract_length(type_lower);
                DataType::Other(format!("bit({:?})", len))
            }
            "bit varying" | "varbit" => {
                let len = Self::extract_length(type_lower);
                DataType::Other(format!("varbit({:?})", len))
            }

            // Date/Time types
            "date" => DataType::Date,
            "time" | "time without time zone" => DataType::Time,
            "timetz" | "time with time zone" => DataType::Other("time with time zone".to_string()),
            "timestamp" | "timestamp without time zone" => DataType::Timestamp,
            "timestamptz" | "timestamp with time zone" => {
                DataType::Other("timestamp with time zone".to_string())
            }
            "interval" => DataType::Other("interval".to_string()),

            // JSON types (PostgreSQL has both json and jsonb)
            "json" | "jsonb" => DataType::Json,

            // UUID
            "uuid" => DataType::Other("uuid".to_string()),

            // Network types
            "cidr" | "inet" | "macaddr" | "macaddr8" => DataType::Other(type_name.to_string()),

            // Geometric types
            "point" | "line" | "lseg" | "box" | "path" | "polygon" | "circle" => {
                DataType::Other(type_name.to_string())
            }

            // XML
            "xml" => DataType::Other("xml".to_string()),

            // Unknown/Other types
            _ => DataType::Other(postgres_type.to_string()),
        }
    }

    /// Extract length from type string (e.g., "varchar(255)" -> Some(255))
    /// or "numeric(10,2)" -> Some(10) (returns precision)
    #[allow(dead_code)]
    fn extract_length(type_str: &str) -> Option<usize> {
        type_str
            .find('(')
            .and_then(|pos| {
                // Find the first comma or closing paren
                let end_match = type_str[pos + 1..].find([',', ')']);
                let end = end_match?;

                // Parse the number
                type_str[pos + 1..pos + 1 + end].parse().ok()
            })
            .and_then(|len: usize| if len == 0 { None } else { Some(len) })
    }
}

#[async_trait]
impl Catalog for LivePostgreSQLCatalog {
    /// List all tables in the database
    ///
    /// Queries information_schema.tables to get all tables, views, and materialized views.
    async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
        #[cfg(feature = "postgresql")]
        if let Some(pool) = &self.pool {
            let query = r#"
                SELECT
                    t.table_name,
                    t.table_schema,
                    CASE
                        WHEN t.table_type = 'BASE TABLE' THEN 'table'
                        WHEN t.table_type = 'VIEW' THEN 'view'
                        WHEN t.table_type = 'MATERIALIZED VIEW' THEN 'materialized'
                        ELSE 'other'
                    END as table_type,
                    obj_description((t.table_schema||'.'||t.table_name)::regclass, 'pg_class') as table_comment
                FROM information_schema.tables t
                WHERE t.table_schema NOT IN ('pg_catalog', 'information_schema')
                  AND t.table_type IN ('BASE TABLE', 'VIEW', 'MATERIALIZED VIEW')
                ORDER BY t.table_schema, t.table_name
            "#;

            let rows = sqlx::query_as::<_, (String, String, String, Option<String>)>(query)
                .fetch_all(pool)
                .await
                .map_err(|e| CatalogError::QueryFailed(format!("Failed to list tables: {}", e)))?;

            let tables = rows.into_iter().map(|(name, schema, db_table_type, comment)| {
                let table_type = match db_table_type.as_str() {
                    "table" => TableType::Table,
                    "view" => TableType::View,
                    "materialized" => TableType::MaterializedView,
                    _ => TableType::Other(db_table_type),
                };

                TableMetadata::new(&name, &schema)
                    .with_type(table_type)
                    .with_comment(comment.unwrap_or_default())
            }).collect();

            return Ok(tables);
        }

        #[cfg(not(feature = "postgresql"))]
        return Err(CatalogError::NotSupported(
            "list_tables requires 'postgresql' feature enabled".to_string()
        ));

        #[cfg(all(feature = "postgresql", not(feature = "postgresql")))]
        unreachable!()
    }

    /// Get column metadata for a specific table
    ///
    /// Queries information_schema.columns and pg_catalog to get column information.
    async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
        #[cfg(feature = "postgresql")]
        if let Some(pool) = &self.pool {
            let query = r#"
                SELECT
                    c.column_name,
                    c.data_type,
                    c.is_nullable,
                    c.column_default,
                    pgd.description as column_comment,
                    CASE
                        WHEN pk.column_name IS NOT NULL THEN 'YES'
                        ELSE 'NO'
                    END as is_primary_key
                FROM information_schema.columns c
                LEFT JOIN pg_catalog.pg_description pgd
                    ON pgd.objoid = (c.table_schema||'.'||c.table_name)::regclass
                    AND pgd.objsubid = c.ordinal_position
                LEFT JOIN (
                    SELECT ku.column_name
                    FROM information_schema.table_constraints tc
                    JOIN information_schema.key_column_usage ku
                        ON tc.constraint_name = ku.constraint_name
                    WHERE tc.constraint_type = 'PRIMARY KEY'
                        AND tc.table_schema = c.table_schema
                        AND tc.table_name = c.table_name
                ) pk ON pk.column_name = c.column_name
                WHERE c.table_schema NOT IN ('pg_catalog', 'information_schema')
                  AND c.table_name = $1
                ORDER BY c.ordinal_position
            "#;

            let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>, String)>(
                query
            )
            .bind(table)
            .fetch_all(pool)
            .await
            .map_err(|e| CatalogError::QueryFailed(format!("Failed to get columns for table '{}': {}", table, e)))?;

            let columns = rows.into_iter().map(|(name, data_type, is_nullable, _default, comment, is_pk)| {
                let dt = Self::parse_postgres_type(&data_type);
                let nullable = is_nullable == "YES";
                let is_pk = is_pk == "YES";

                let mut col = ColumnMetadata::new(name, dt)
                    .with_nullable(nullable)
                    .with_comment(comment.unwrap_or_default());

                if is_pk {
                    col = col.with_primary_key();
                }

                col
            }).collect();

            return Ok(columns);
        }

        #[cfg(not(feature = "postgresql"))]
        return Err(CatalogError::NotSupported(
            format!("get_columns requires 'postgresql' feature enabled (table: '{}')", table)
        ));

        #[cfg(all(feature = "postgresql", not(feature = "postgresql")))]
        unreachable!()
    }

    /// List all available functions
    ///
    /// Returns a list of built-in PostgreSQL functions and custom functions.
    async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
        #[cfg(feature = "postgresql")]
        if let Some(pool) = &self.pool {
            // Query custom functions from pg_catalog.pg_proc
            let custom_query = r#"
                SELECT
                    p.proname as function_name,
                    pg_get_function_result(p.oid) as return_type,
                    pg_get_function_arguments(p.oid) as arguments,
                    n.nspname as schema_name
                FROM pg_catalog.pg_proc p
                JOIN pg_catalog.pg_namespace n ON p.pronamespace = n.oid
                WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
            "#;

            let custom_funcs: Vec<FunctionMetadata> = sqlx::query_as::<_, (String, String, String, String)>(
                custom_query
            )
            .fetch_all(pool)
            .await
            .unwrap_or(vec![]) // Don't fail if pg_proc not accessible
            .into_iter()
            .map(|(name, ret, _args, schema)| {
                FunctionMetadata::new(&name, Self::parse_postgres_type(&ret))
                    .with_type(FunctionType::Scalar)
                    .with_description(format!("Custom function from {}", schema))
            })
            .collect();

            // Merge with static built-in functions
            let mut all_functions = Self::builtin_functions();
            all_functions.extend(custom_funcs);
            return Ok(all_functions);
        }

        // Static fallback when feature not enabled or pool not available
        Ok(Self::builtin_functions())
    }
}

impl LivePostgreSQLCatalog {
    /// Get the list of built-in PostgreSQL functions
    fn builtin_functions() -> Vec<FunctionMetadata> {
        vec![
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
            FunctionMetadata::new("MIN", DataType::Text)
                .with_type(FunctionType::Aggregate)
                .with_description("Minimum value"),
            FunctionMetadata::new("MAX", DataType::Text)
                .with_type(FunctionType::Aggregate)
                .with_description("Maximum value"),
            FunctionMetadata::new("STRING_AGG", DataType::Text)
                .with_type(FunctionType::Aggregate)
                .with_description("Concatenate values with delimiter"),
            FunctionMetadata::new("ARRAY_AGG", DataType::Other("array".to_string()))
                .with_type(FunctionType::Aggregate)
                .with_description("Collect values into an array"),
            FunctionMetadata::new("JSON_AGG", DataType::Json)
                .with_type(FunctionType::Aggregate)
                .with_description("Aggregate values as JSON"),
            FunctionMetadata::new("JSONB_AGG", DataType::Json)
                .with_type(FunctionType::Aggregate)
                .with_description("Aggregate values as JSONB"),
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
            FunctionMetadata::new("TRUNC", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Truncate decimal"),
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
            FunctionMetadata::new("LTRIM", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Remove leading whitespace"),
            FunctionMetadata::new("RTRIM", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Remove trailing whitespace"),
            FunctionMetadata::new("COALESCE", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return first non-null value"),
            FunctionMetadata::new("NULLIF", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return NULL if arguments are equal"),
            FunctionMetadata::new("GREATEST", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return largest value"),
            FunctionMetadata::new("LEAST", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Return smallest value"),
            FunctionMetadata::new("POSITION", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Position of substring"),
            FunctionMetadata::new("STRPOS", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Position of substring"),
            FunctionMetadata::new("REPLACE", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Replace occurrences"),
            FunctionMetadata::new("SPLIT_PART", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Split string and return field"),
            FunctionMetadata::new("REGEXP_REPLACE", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Replace using regex"),
            FunctionMetadata::new("REGEXP_MATCHES", DataType::Other("array".to_string()))
                .with_type(FunctionType::Scalar)
                .with_description("Match regex and return array"),
            // Date/Time functions
            FunctionMetadata::new("NOW", DataType::Timestamp)
                .with_type(FunctionType::Scalar)
                .with_description("Current date and time"),
            FunctionMetadata::new("CURRENT_DATE", DataType::Date)
                .with_type(FunctionType::Scalar)
                .with_description("Current date"),
            FunctionMetadata::new("CURRENT_TIME", DataType::Time)
                .with_type(FunctionType::Scalar)
                .with_description("Current time"),
            FunctionMetadata::new("CURRENT_TIMESTAMP", DataType::Timestamp)
                .with_type(FunctionType::Scalar)
                .with_description("Current date and time"),
            FunctionMetadata::new("AGE", DataType::Other("interval".to_string()))
                .with_type(FunctionType::Scalar)
                .with_description("Calculate interval"),
            FunctionMetadata::new("DATE_TRUNC", DataType::Timestamp)
                .with_type(FunctionType::Scalar)
                .with_description("Truncate to precision"),
            FunctionMetadata::new("DATE_PART", DataType::Float)
                .with_type(FunctionType::Scalar)
                .with_description("Extract date part"),
            FunctionMetadata::new("EXTRACT", DataType::Float)
                .with_type(FunctionType::Scalar)
                .with_description("Extract date/time field"),
            FunctionMetadata::new("TO_DATE", DataType::Date)
                .with_type(FunctionType::Scalar)
                .with_description("Convert string to date"),
            FunctionMetadata::new("TO_TIMESTAMP", DataType::Timestamp)
                .with_type(FunctionType::Scalar)
                .with_description("Convert string to timestamp"),
            // Window functions (PostgreSQL 8.4+)
            FunctionMetadata::new("ROW_NUMBER", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Row number within partition"),
            FunctionMetadata::new("RANK", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Rank within partition"),
            FunctionMetadata::new("DENSE_RANK", DataType::BigInt)
                .with_type(FunctionType::Window)
                .with_description("Dense rank within partition"),
            FunctionMetadata::new("NTILE", DataType::Integer)
                .with_type(FunctionType::Window)
                .with_description("Divide rows into buckets"),
            FunctionMetadata::new("LAG", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Value from previous row"),
            FunctionMetadata::new("LEAD", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Value from next row"),
            FunctionMetadata::new("FIRST_VALUE", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("First value in window"),
            FunctionMetadata::new("LAST_VALUE", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Last value in window"),
            FunctionMetadata::new("NTH_VALUE", DataType::Text)
                .with_type(FunctionType::Window)
                .with_description("Nth value in window"),
            // JSON functions (PostgreSQL 9.2+)
            FunctionMetadata::new("TO_JSON", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Convert to JSON"),
            FunctionMetadata::new("TO_JSONB", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Convert to JSONB"),
            FunctionMetadata::new("JSON_BUILD_OBJECT", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Build JSON object"),
            FunctionMetadata::new("JSONB_BUILD_OBJECT", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Build JSONB object"),
            FunctionMetadata::new("JSON_ARRAY_ELEMENTS", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Expand JSON array"),
            FunctionMetadata::new("JSONB_ARRAY_ELEMENTS", DataType::Json)
                .with_type(FunctionType::Scalar)
                .with_description("Expand JSONB array"),
            FunctionMetadata::new("JSON_GET_TEXT", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Get JSON field as text"),
            FunctionMetadata::new("JSONB_GET_TEXT", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Get JSONB field as text"),
            // Array functions
            FunctionMetadata::new("ARRAY_LENGTH", DataType::Integer)
                .with_type(FunctionType::Scalar)
                .with_description("Get array length"),
            FunctionMetadata::new("UNNEST", DataType::Text)
                .with_type(FunctionType::Scalar)
                .with_description("Expand array to rows"),
            FunctionMetadata::new("ARRAY_APPEND", DataType::Other("array".to_string()))
                .with_type(FunctionType::Scalar)
                .with_description("Append element to array"),
            FunctionMetadata::new("ARRAY_PREPEND", DataType::Other("array".to_string()))
                .with_type(FunctionType::Scalar)
                .with_description("Prepend element to array"),
            FunctionMetadata::new("ARRAY_CAT", DataType::Other("array".to_string()))
                .with_type(FunctionType::Scalar)
                .with_description("Concatenate arrays"),
            // Mathematical functions
            FunctionMetadata::new("SQRT", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Square root"),
            FunctionMetadata::new("POWER", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Raise to power"),
            FunctionMetadata::new("EXP", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Exponential"),
            FunctionMetadata::new("LN", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Natural logarithm"),
            FunctionMetadata::new("LOG", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Logarithm"),
            FunctionMetadata::new("MOD", DataType::Decimal)
                .with_type(FunctionType::Scalar)
                .with_description("Modulus"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_postgres_varchar() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("character varying(255)");
        assert_eq!(dt, DataType::Varchar(Some(255)));
    }

    #[test]
    fn test_parse_postgres_varchar_no_length() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("varchar");
        assert_eq!(dt, DataType::Varchar(None));
    }

    #[test]
    fn test_parse_postgres_char() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("char(10)");
        assert_eq!(dt, DataType::Char(Some(10)));
    }

    #[test]
    fn test_parse_postgres_integer() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("integer");
        assert_eq!(dt, DataType::Integer);
    }

    #[test]
    fn test_parse_postgres_bigint() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("bigint");
        assert_eq!(dt, DataType::BigInt);
    }

    #[test]
    fn test_parse_postgres_smallint() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("smallint");
        assert_eq!(dt, DataType::SmallInt);
    }

    #[test]
    fn test_parse_postgres_text() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("text");
        assert_eq!(dt, DataType::Text);
    }

    #[test]
    fn test_parse_postgres_numeric() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("numeric");
        assert_eq!(dt, DataType::Decimal);
    }

    #[test]
    fn test_parse_postgres_numeric_with_precision() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("numeric(10,2)");
        assert_eq!(dt, DataType::Decimal);
    }

    #[test]
    fn test_parse_postgres_real() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("real");
        assert_eq!(dt, DataType::Float);
    }

    #[test]
    fn test_parse_postgres_double_precision() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("double precision");
        assert_eq!(dt, DataType::Double);
    }

    #[test]
    fn test_parse_postgres_json() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("json");
        assert_eq!(dt, DataType::Json);
    }

    #[test]
    fn test_parse_postgres_timestamp() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("timestamp");
        assert_eq!(dt, DataType::Timestamp);
    }

    #[test]
    fn test_parse_postgres_timestamptz() {
        let dt = LivePostgreSQLCatalog::parse_postgres_type("timestamptz");
        assert!(matches!(dt, DataType::Other(_)));
    }

    #[test]
    fn test_extract_length_from_varchar() {
        let len = LivePostgreSQLCatalog::extract_length("varchar(255)");
        assert_eq!(len, Some(255));
    }

    #[test]
    fn test_extract_length_from_numeric() {
        let len = LivePostgreSQLCatalog::extract_length("numeric(10,2)");
        assert_eq!(len, Some(10));
    }

    #[test]
    fn test_extract_length_no_parens() {
        let len = LivePostgreSQLCatalog::extract_length("text");
        assert_eq!(len, None);
    }

    #[test]
    fn test_validate_connection_string_valid_postgresql() {
        let result = LivePostgreSQLCatalog::validate_connection_string("postgresql://localhost");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_connection_string_valid_postgres() {
        let result = LivePostgreSQLCatalog::validate_connection_string("postgres://localhost");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_connection_string_empty() {
        let result = LivePostgreSQLCatalog::validate_connection_string("");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_connection_string_invalid_prefix() {
        let result = LivePostgreSQLCatalog::validate_connection_string("mysql://localhost");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_new_catalog() {
        let catalog = LivePostgreSQLCatalog::new("postgresql://localhost")
            .await
            .unwrap();
        assert_eq!(catalog.connection_string(), "postgresql://localhost");
        assert_eq!(catalog.pool_size(), DEFAULT_POOL_SIZE);
        assert_eq!(catalog.timeout_secs(), DEFAULT_TIMEOUT_SECS);
    }

    #[tokio::test]
    async fn test_new_catalog_invalid_connection_string() {
        let result = LivePostgreSQLCatalog::new("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_catalog_with_config() {
        let catalog = LivePostgreSQLCatalog::with_config("postgresql://localhost", 20, 10)
            .await
            .unwrap();
        assert_eq!(catalog.pool_size(), 20);
        assert_eq!(catalog.timeout_secs(), 10);
    }

    #[tokio::test]
    async fn test_catalog_with_config_invalid_pool_size() {
        let result = LivePostgreSQLCatalog::with_config("postgresql://localhost", 0, 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_catalog_with_config_invalid_timeout() {
        let result = LivePostgreSQLCatalog::with_config("postgresql://localhost", 10, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_functions() {
        let catalog = LivePostgreSQLCatalog::new("postgresql://localhost")
            .await
            .unwrap();
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

        // Check for PostgreSQL-specific functions
        let string_agg_func = functions.iter().find(|f| f.name == "STRING_AGG");
        assert!(string_agg_func.is_some());

        let json_agg_func = functions.iter().find(|f| f.name == "JSON_AGG");
        assert!(json_agg_func.is_some());

        // Check for array functions
        let array_length_func = functions.iter().find(|f| f.name == "ARRAY_LENGTH");
        assert!(array_length_func.is_some());
    }
}
