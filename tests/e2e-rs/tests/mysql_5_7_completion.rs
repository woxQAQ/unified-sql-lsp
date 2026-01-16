// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E completion tests
//!
//! Tests code completion through actual LSP protocol with live database.

use serial_test::serial;
use unified_sql_lsp_e2e::{Engine, ensure_engine_ready, run_suite};

// Simple smoke test to verify LSP server works
#[tokio::test]
async fn test_lsp_server_smoke() -> anyhow::Result<()> {
    eprintln!("!!! SMOKE TEST: Starting");
    use tower_lsp::lsp_types::Url;
    use unified_sql_lsp_e2e::client::LspConnection;
    use unified_sql_lsp_e2e::runner::LspRunner;

    eprintln!("!!! SMOKE TEST: Spawning LSP server");
    let mut lsp_runner = LspRunner::from_crate()?;
    lsp_runner.spawn().await?;
    eprintln!("!!! SMOKE TEST: LSP server spawned");

    let stdin = lsp_runner.stdin()?;
    let stdout = lsp_runner.stdout()?;
    let mut conn = LspConnection::new(stdin, stdout);
    eprintln!("!!! SMOKE TEST: Connection created");

    eprintln!("!!! SMOKE TEST: Initializing...");
    let _init_result = conn.initialize().await?;
    eprintln!("!!! SMOKE TEST: Initialized successfully");

    eprintln!("!!! SMOKE TEST: Opening document");
    let uri = Url::parse("file:///test.sql")?;
    conn.did_open(uri, "mysql".to_string(), "SELECT * FROM users".to_string())
        .await?;
    eprintln!("!!! SMOKE TEST: Document opened");

    lsp_runner.kill().await?;
    eprintln!("!!! SMOKE TEST: PASSED");
    Ok(())
}

#[cfg(test)]
#[serial(mysql_57)]
mod mysql_57_tests {
    use super::*;

    #[tokio::test]
    async fn test_select_clause_completion() -> anyhow::Result<()> {
        eprintln!("!!! Starting test_select_clause_completion");
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        eprintln!("!!! Database ready, running test suite");
        run_suite("tests/mysql-5.7/completion/select_clause.yaml").await
    }

    #[tokio::test]
    async fn test_from_clause_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/from_clause.yaml").await
    }

    #[tokio::test]
    async fn test_join_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/join_completion.yaml").await
    }

    #[tokio::test]
    async fn test_select_advanced_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/select_advanced.yaml").await
    }

    #[tokio::test]
    async fn test_from_advanced_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/from_advanced.yaml").await
    }

    #[tokio::test]
    async fn test_where_clause_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/where_clause.yaml").await
    }

    #[tokio::test]
    async fn test_join_advanced_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/join_advanced.yaml").await
    }

    #[tokio::test]
    async fn test_functions_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/functions.yaml").await
    }

    #[tokio::test]
    async fn test_keywords_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/keywords.yaml").await
    }

    #[tokio::test]
    async fn test_basic_select_completion() -> anyhow::Result<()> {
        let _guard = ensure_engine_ready(&Engine::MySQL57).await?;
        run_suite("tests/mysql-5.7/completion/basic_select.yaml").await
    }
}
