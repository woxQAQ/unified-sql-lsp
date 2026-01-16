// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E tests for MySQL 8.0 specific features

use serial_test::serial;
use unified_sql_lsp_e2e::{Engine, ensure_engine_ready, run_suite};

#[cfg(test)]
#[serial(mysql_80)]
mod mysql_80_tests {
    use super::*;

    #[tokio::test]
    async fn test_mysql_80_window_functions() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL80).await?;
        run_suite("tests/mysql-8.0/completion/window_functions.yaml").await
    }

    #[tokio::test]
    async fn test_mysql_80_cte() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL80).await?;
        run_suite("tests/mysql-8.0/completion/cte.yaml").await
    }
}
