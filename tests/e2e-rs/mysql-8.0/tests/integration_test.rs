// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! MySQL 8.0 E2E Tests
//!
//! Auto-generated from YAML test definitions

use unified_sql_lsp_e2e_core::generate_engine_tests;

generate_engine_tests!(
    engine: MySQL80,
    test_dir: "tests/mysql-8.0",
    test_types: [completion, hover, diagnostics]
);
