// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E completion tests
//!
//! Tests code completion through actual LSP protocol with live database.

use unified_sql_lsp_e2e::{init_database, run_suite};

// Simple smoke test to verify LSP server works
#[tokio::test]
async fn test_lsp_server_smoke() -> anyhow::Result<()> {
    eprintln!("!!! SMOKE TEST: Starting");
    use unified_sql_lsp_e2e::runner::LspRunner;
    use unified_sql_lsp_e2e::client::LspConnection;
    use tower_lsp::lsp_types::Url;

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
    conn.did_open(uri, "mysql".to_string(), "SELECT * FROM users".to_string()).await?;
    eprintln!("!!! SMOKE TEST: Document opened");

    lsp_runner.kill().await?;
    eprintln!("!!! SMOKE TEST: PASSED");
    Ok(())
}

// Basic completion tests
#[tokio::test]
async fn test_select_clause_completion() -> anyhow::Result<()> {
    eprintln!("!!! Starting test_select_clause_completion");
    init_database().await?;
    eprintln!("!!! Database initialized, running test suite");
    run_suite("tests/completion/select_clause.yaml").await
}

#[tokio::test]
async fn test_from_clause_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/from_clause.yaml").await
}

#[tokio::test]
async fn test_join_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/join_completion.yaml").await
}

// Advanced completion tests
#[tokio::test]
async fn test_select_advanced_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/select_advanced.yaml").await
}

#[tokio::test]
async fn test_from_advanced_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/from_advanced.yaml").await
}

#[tokio::test]
async fn test_where_clause_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/where_clause.yaml").await
}

#[tokio::test]
async fn test_join_advanced_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/join_advanced.yaml").await
}

#[tokio::test]
async fn test_functions_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/functions.yaml").await
}

#[tokio::test]
async fn test_keywords_completion() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/completion/keywords.yaml").await
}
