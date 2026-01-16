// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E tests for PostgreSQL 16 specific features

use serial_test::serial;
use unified_sql_lsp_e2e::{Engine, ensure_engine_ready, run_suite};

#[cfg(test)]
#[serial(postgresql_16)]
mod postgresql_16_tests {
    use super::*;

    #[tokio::test]
    #[ignore = "PostgreSQL 16 JSON functions not yet added to function registry - needs JSON_BUILD_ARRAY, JSON_QUERY, etc."]
    async fn test_postgresql_16_advanced_features() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::PostgreSQL16).await?;
        run_suite("tests/postgresql-16/completion/advanced_features.yaml").await
    }
}
