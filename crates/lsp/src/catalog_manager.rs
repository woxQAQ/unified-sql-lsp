// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Catalog manager
//!
//! This module manages catalog instances for the LSP server.
//!
//! The catalog manager is responsible for:
//! - Creating catalog instances based on engine configuration
//! - Reusing catalog connections across multiple completion requests
//! - Managing catalog lifecycle

use std::collections::HashMap;
use std::sync::Arc;
use unified_sql_lsp_catalog::{
    Catalog, CatalogError, CatalogResult, LiveMySQLCatalog, LivePostgreSQLCatalog,
};

use crate::config::EngineConfig;

/// Catalog manager
///
/// Manages catalog instances for different database connections.
pub struct CatalogManager {
    /// MySQL catalog instances (keyed by connection string)
    mysql_catalogs: HashMap<String, Arc<LiveMySQLCatalog>>,

    /// PostgreSQL catalog instances (keyed by connection string)
    postgres_catalogs: HashMap<String, Arc<LivePostgreSQLCatalog>>,
}

impl CatalogManager {
    /// Create a new catalog manager
    pub fn new() -> Self {
        Self {
            mysql_catalogs: HashMap::new(),
            postgres_catalogs: HashMap::new(),
        }
    }

    /// Get or create a catalog for the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The engine configuration
    ///
    /// # Returns
    ///
    /// An Arc to the catalog instance
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let catalog = manager.get_catalog(&config).await?;
    /// let columns = catalog.get_columns("users").await?;
    /// ```
    pub async fn get_catalog(&mut self, config: &EngineConfig) -> CatalogResult<Arc<dyn Catalog>> {
        match config.dialect {
            unified_sql_lsp_ir::Dialect::MySQL => self
                .get_mysql_catalog(config)
                .await
                .map(|c| c as Arc<dyn Catalog>),
            unified_sql_lsp_ir::Dialect::PostgreSQL => self
                .get_postgres_catalog(config)
                .await
                .map(|c| c as Arc<dyn Catalog>),
            _ => Err(CatalogError::NotSupported(format!(
                "Dialect {:?} is not supported yet",
                config.dialect
            ))),
        }
    }

    /// Get or create a MySQL catalog
    async fn get_mysql_catalog(
        &mut self,
        config: &EngineConfig,
    ) -> CatalogResult<Arc<LiveMySQLCatalog>> {
        // Check if we already have a catalog for this connection
        if let Some(catalog) = self.mysql_catalogs.get(&config.connection_string) {
            return Ok(catalog.clone());
        }

        // Create new catalog
        let catalog = LiveMySQLCatalog::new(&config.connection_string)
            .await
            .map_err(|e| CatalogError::ConnectionFailed(e.to_string()))?;

        let catalog = Arc::new(catalog);
        self.mysql_catalogs
            .insert(config.connection_string.clone(), catalog.clone());

        Ok(catalog)
    }

    /// Get or create a PostgreSQL catalog
    async fn get_postgres_catalog(
        &mut self,
        config: &EngineConfig,
    ) -> CatalogResult<Arc<LivePostgreSQLCatalog>> {
        // Check if we already have a catalog for this connection
        if let Some(catalog) = self.postgres_catalogs.get(&config.connection_string) {
            return Ok(catalog.clone());
        }

        // Create new catalog
        let catalog = LivePostgreSQLCatalog::new(&config.connection_string)
            .await
            .map_err(|e| CatalogError::ConnectionFailed(e.to_string()))?;

        let catalog = Arc::new(catalog);
        self.postgres_catalogs
            .insert(config.connection_string.clone(), catalog.clone());

        Ok(catalog)
    }

    /// Close all catalog connections
    ///
    /// This should be called when shutting down the server.
    pub async fn close_all(&mut self) {
        self.mysql_catalogs.clear();
        self.postgres_catalogs.clear();
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_catalog::Catalog;
    use unified_sql_lsp_ir::Dialect;

    #[tokio::test]
    async fn test_catalog_manager_new() {
        let manager = CatalogManager::new();
        assert!(manager.mysql_catalogs.is_empty());
        assert!(manager.postgres_catalogs.is_empty());
    }

    #[tokio::test]
    async fn test_catalog_manager_default() {
        let manager = CatalogManager::default();
        assert!(manager.mysql_catalogs.is_empty());
        assert!(manager.postgres_catalogs.is_empty());
    }

    #[tokio::test]
    async fn test_catalog_manager_unsupported_dialect() {
        let mut manager = CatalogManager::new();

        let config = EngineConfig {
            dialect: Dialect::TiDB, // Not supported yet
            ..Default::default()
        };

        let result = manager.get_catalog(&config).await;
        assert!(matches!(result, Err(CatalogError::NotSupported(_))));
    }

    #[tokio::test]
    async fn test_catalog_manager_close_all() {
        let mut manager = CatalogManager::new();
        manager.close_all().await;
        assert!(manager.mysql_catalogs.is_empty());
        assert!(manager.postgres_catalogs.is_empty());
    }

    // Note: Tests with actual database connections require
    // integration test setup with running databases.
    // Those tests are in the integration test suite.
}
