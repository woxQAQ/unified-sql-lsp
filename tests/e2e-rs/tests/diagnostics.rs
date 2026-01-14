// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! E2E diagnostics tests
//!
//! Tests SQL error detection through actual LSP protocol with live database.

use unified_sql_lsp_e2e::{init_database, run_suite};

#[tokio::test]
#[ignore = "DIAG-001 through DIAG-005 not implemented yet"]
async fn test_basic_diagnostics() -> anyhow::Result<()> {
    init_database().await?;
    run_suite("tests/diagnostics/basic_diagnostics.yaml").await
}
