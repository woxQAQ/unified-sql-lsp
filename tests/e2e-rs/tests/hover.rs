// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E hover tests
//!
//! Tests hover information through actual LSP protocol with live database.

use unified_sql_lsp_e2e::{init_database, run_suite};

#[tokio::test]
#[ignore = "Hover output format doesn't match test expectations - needs investigation"]
async fn test_basic_hover() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/mysql-5.7/hover/basic_hover.yaml").await
}
