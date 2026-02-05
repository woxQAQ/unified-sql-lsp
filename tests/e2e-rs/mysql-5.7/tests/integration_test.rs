// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! MySQL 5.7 E2E Tests
//!
//! Auto-generated from YAML test definitions

use unified_sql_lsp_e2e_core::generate_engine_tests;

generate_engine_tests!(
    engine: MySQL57,
    test_dir: "tests/mysql-5.7",
    test_types: [completion]
);
