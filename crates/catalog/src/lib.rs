// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Unified SQL LSP - Catalog Layer
//!
//! This crate provides database schema abstraction for the Unified SQL LSP server.
//! It defines the `Catalog` trait and metadata types used for:
//!
//! - **Live Catalogs**: Direct database connections (MySQL, PostgreSQL, TiDB)
//! - **Static Catalogs**: Schema definitions from files (YAML/JSON)
//! - **Cached Catalogs**: Wrapper with LRU cache and TTL
//!
//! ## Architecture
//!
//! The catalog layer is responsible for:
//! - Providing schema information (tables, columns, functions)
//! - Abstracting different data sources
//! - Supporting multiple SQL dialects
//! - Enabling caching and performance optimizations
//!
//! ## Metadata Types
//!
//! - [`TableMetadata`]: Table information including columns, row count, type
//! - [`ColumnMetadata`]: Column details including type, nullability, keys
//! - [`FunctionMetadata`]: Function signatures and documentation
//! - [`DataType`]: Unified SQL data type representation
//!
//! ## Usage
//!
//! ```rust,ignore
//! // TODO: (CATALOG-001) Implement actual catalog to make this example testable
//! use unified_sql_lsp_catalog::{Catalog, CatalogError};
//!
//! async fn print_tables(catalog: &impl Catalog) -> Result<(), CatalogError> {
//!     let tables = catalog.list_tables().await?;
//!     for table in tables {
//!         println!("{}.{}", table.schema, table.name);
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Implementing the Catalog Trait
//!
//! To implement a custom catalog:
//!
//! ```rust,ignore
//! // TODO: (CATALOG-001) Implement actual catalog to make this example testable
//! use unified_sql_lsp_catalog::{Catalog, CatalogResult};
//! use async_trait::async_trait;
//!
//! struct MyCatalog;
//!
//! #[async_trait]
//! impl Catalog for MyCatalog {
//!     async fn list_tables(&self) -> CatalogResult<Vec<TableMetadata>> {
//!         // Your implementation here
//!     }
//!
//!     async fn get_columns(&self, table: &str) -> CatalogResult<Vec<ColumnMetadata>> {
//!         // Your implementation here
//!     }
//!
//!     async fn list_functions(&self) -> CatalogResult<Vec<FunctionMetadata>> {
//!         // Your implementation here
//!     }
//! }
//! ```

pub mod error;
pub mod live_mysql;
pub mod live_postgres;
pub mod metadata;
pub mod r#trait;

// Re-exports
pub use error::{CatalogError, CatalogResult};
pub use live_mysql::LiveMySQLCatalog;
pub use live_postgres::LivePostgreSQLCatalog;
pub use metadata::{
    ColumnMetadata, DataType, FunctionMetadata, FunctionParameter, FunctionType, TableMetadata,
    TableReference, TableType,
};
pub use r#trait::Catalog;
