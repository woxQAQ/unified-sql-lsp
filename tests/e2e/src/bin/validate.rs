// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Binary entry point for running test case parser validation

use unified_sql_lsp_e2e_tests::parser_validation;

fn main() {
    println!("Running test case parser validation...\n");

    parser_validation::test_parse_all_test_files();
    parser_validation::test_basic_select_file_structure();
    parser_validation::test_join_foreign_key_context();
    parser_validation::test_functions_dialect_coverage();
    parser_validation::test_advanced_cte_coverage();
    parser_validation::test_edge_cases_boundary_conditions();
    parser_validation::test_options_validation();
    parser_validation::test_expected_item_formats();
    parser_validation::test_dialect_distribution();

    println!("\n✓ All parser validation tests passed!");
}
