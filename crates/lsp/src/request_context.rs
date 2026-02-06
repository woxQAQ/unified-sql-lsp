// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Request-level context and service access for LSP handlers.

use std::sync::Arc;
use tokio::sync::RwLock;
use unified_sql_lsp_catalog::{Catalog, CatalogResult};

use crate::catalog_manager::CatalogManager;
use crate::config::EngineConfig;

/// Shared request context for resolving config and catalog services.
#[derive(Clone)]
pub struct RequestContext {
    config: Arc<RwLock<Option<EngineConfig>>>,
    catalog_manager: Arc<RwLock<CatalogManager>>,
}

impl RequestContext {
    pub fn new(
        config: Arc<RwLock<Option<EngineConfig>>>,
        catalog_manager: Arc<RwLock<CatalogManager>>,
    ) -> Self {
        Self {
            config,
            catalog_manager,
        }
    }

    /// Return current config or the runtime fallback config.
    pub async fn config_or_fallback(&self) -> EngineConfig {
        match self.config.read().await.clone() {
            Some(cfg) => cfg,
            None => EngineConfig::default_runtime_fallback(),
        }
    }

    /// Resolve a catalog for the given config.
    pub async fn catalog_for_config(
        &self,
        config: &EngineConfig,
    ) -> CatalogResult<Arc<dyn Catalog>> {
        self.catalog_manager.write().await.get_catalog(config).await
    }

    /// Resolve both the config and its catalog in one call.
    pub async fn config_and_catalog(&self) -> CatalogResult<(EngineConfig, Arc<dyn Catalog>)> {
        let config = self.config_or_fallback().await;
        let catalog = self.catalog_for_config(&config).await?;
        Ok((config, catalog))
    }
}
