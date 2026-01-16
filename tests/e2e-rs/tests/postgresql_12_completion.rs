// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E tests for PostgreSQL 12 specific features

use serial_test::serial;
use unified_sql_lsp_e2e::{Engine, ensure_engine_ready, run_suite};

#[cfg(test)]
#[serial(postgresql_12)]
mod postgresql_12_tests {
    use super::*;

    #[tokio::test]
    async fn test_postgresql_12_functions() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::PostgreSQL12).await?;
        run_suite("tests/postgresql-12/completion/postgresql_functions.yaml").await
    }

    #[tokio::test]
    #[ignore = "LSP server doesn't support RETURNING clause completion yet - needs context detection"]
    async fn test_postgresql_12_returning_clause() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::PostgreSQL12).await?;
        run_suite("tests/postgresql-12/completion/returning_clause.yaml").await
    }

    #[tokio::test]
    async fn test_postgresql_12_basic_select() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::PostgreSQL12).await?;
        run_suite("tests/postgresql-12/completion/basic_select.yaml").await
    }
}
