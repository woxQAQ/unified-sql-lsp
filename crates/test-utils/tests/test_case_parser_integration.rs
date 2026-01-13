// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for the test case parser
//!
//! This test module verifies that the parser correctly handles all test case files
//! generated according to the test case generation guide.

use std::path::Path;
use unified_sql_lsp_test_utils::{parse_test_file, Dialect};

#[test]
fn test_parse_01_basic_select() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/01_basic_select.txt"))
        .expect("Failed to parse 01_basic_select.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Verify first test case
    let first = &cases[0];
    assert_eq!(first.description, "Simple column name completion");
    assert_eq!(first.dialect, Dialect::All);
    assert!(first.input.contains("SELECT | FROM users"));
    assert!(!first.expected.is_empty(), "Should have expected items");

    // Check that options are parsed
    if let Some(options) = &first.options {
        assert_eq!(options.min_items, Some(12));
    }

    println!("✓ 01_basic_select.txt: {} test cases parsed", cases.len());

    // Verify each case has required fields
    for (i, case) in cases.iter().enumerate() {
        assert!(!case.description.is_empty(), "Case {} has empty description", i);
        assert!(!case.input.is_empty(), "Case {} has empty input", i);
        assert!(case.input.contains('|'), "Case {} missing cursor marker", i);
    }
}

#[test]
fn test_parse_02_from_clause() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/02_from_clause.txt"))
        .expect("Failed to parse 02_from_clause.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Find a MySQL-specific test
    let mysql_test = cases.iter()
        .find(|c| c.dialect == Dialect::MySQL);

    assert!(mysql_test.is_some(), "Should have at least one MySQL-specific test");

    // Find a PostgreSQL-specific test
    let pg_test = cases.iter()
        .find(|c| c.dialect == Dialect::PostgreSQL);

    assert!(pg_test.is_some(), "Should have at least one PostgreSQL-specific test");

    println!("✓ 02_from_clause.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_parse_03_join() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/03_join.txt"))
        .expect("Failed to parse 03_join.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Verify a test with context
    let context_test = cases.iter()
        .find(|c| c.context.is_some());

    assert!(context_test.is_some(), "Should have at least one test with context");

    if let Some(case) = context_test {
        assert!(case.context.as_ref().unwrap().contains("foreign key"), "Context should mention foreign key");
    }

    println!("✓ 03_join.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_parse_04_where_clause() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/04_where_clause.txt"))
        .expect("Failed to parse 04_where_clause.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Verify a complex WHERE test
    let between_test = cases.iter()
        .find(|c| c.input.contains("BETWEEN"));

    assert!(between_test.is_some(), "Should have a BETWEEN test");

    println!("✓ 04_where_clause.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_parse_05_functions() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/05_functions.txt"))
        .expect("Failed to parse 05_functions.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Find MySQL-specific function test
    let mysql_func = cases.iter()
        .find(|c| c.dialect == Dialect::MySQL && c.input.contains("DATE_FORMAT"));

    assert!(mysql_func.is_some(), "Should have MySQL DATE_FORMAT test");

    // Find PostgreSQL-specific function test
    let pg_func = cases.iter()
        .find(|c| c.dialect == Dialect::PostgreSQL && c.input.contains("TO_CHAR"));

    assert!(pg_func.is_some(), "Should have PostgreSQL TO_CHAR test");

    // Find GROUP_CONCAT test
    let group_concat = cases.iter()
        .find(|c| c.input.contains("GROUP_CONCAT"));

    assert!(group_concat.is_some(), "Should have GROUP_CONCAT test");

    println!("✓ 05_functions.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_parse_06_advanced() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/06_advanced.txt"))
        .expect("Failed to parse 06_advanced.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Verify CTE test
    let cte_test = cases.iter()
        .find(|c| c.input.contains("WITH"));

    assert!(cte_test.is_some(), "Should have CTE test");

    // Verify UNION test
    let union_test = cases.iter()
        .find(|c| c.input.contains("UNION"));

    assert!(union_test.is_some(), "Should have UNION test");

    // Verify GROUP BY test
    let group_by_test = cases.iter()
        .find(|c| c.input.contains("GROUP BY"));

    assert!(group_by_test.is_some(), "Should have GROUP BY test");

    // Verify ORDER BY test
    let order_by_test = cases.iter()
        .find(|c| c.input.contains("ORDER BY"));

    assert!(order_by_test.is_some(), "Should have ORDER BY test");

    println!("✓ 06_advanced.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_parse_07_edge_cases() {
    let cases = parse_test_file(Path::new("tests/e2e/fixtures/cases/07_edge_cases.txt"))
        .expect("Failed to parse 07_edge_cases.txt");

    assert!(!cases.is_empty(), "Should have parsed at least one test case");

    // Verify empty input test
    let empty_test = cases.iter()
        .find(|c| c.input.trim() == "|");

    assert!(empty_test.is_some(), "Should have empty input test");

    // Verify syntax error test
    let syntax_error_test = cases.iter()
        .find(|c| c.input.contains("SELET"));

    assert!(syntax_error_test.is_some(), "Should have syntax error test");

    // Verify deep nesting test
    let nested_test = cases.iter()
        .find(|c| c.input.matches("SELECT").count() > 3);

    assert!(nested_test.is_some(), "Should have deep nesting test");

    println!("✓ 07_edge_cases.txt: {} test cases parsed", cases.len());
}

#[test]
fn test_all_files_parse_successfully() {
    let files = vec![
        "tests/e2e/fixtures/cases/01_basic_select.txt",
        "tests/e2e/fixtures/cases/02_from_clause.txt",
        "tests/e2e/fixtures/cases/03_join.txt",
        "tests/e2e/fixtures/cases/04_where_clause.txt",
        "tests/e2e/fixtures/cases/05_functions.txt",
        "tests/e2e/fixtures/cases/06_advanced.txt",
        "tests/e2e/fixtures/cases/07_edge_cases.txt",
    ];

    let mut total_cases = 0;

    for file in files {
        let cases = parse_test_file(Path::new(file))
            .expect(&format!("Failed to parse {}", file));

        let count = cases.len();
        total_cases += count;

        println!("✓ {} parsed successfully ({} cases)", file, count);

        // Verify all cases have required fields
        for (i, case) in cases.iter().enumerate() {
            assert!(!case.description.is_empty(), "{}: Case {} has empty description", file, i);
            assert!(!case.input.is_empty(), "{}: Case {} has empty input", file, i);
            assert!(case.input.contains('|'), "{}: Case {} missing cursor marker |", file, i);
        }
    }

    println!("\n✅ Total: {} test cases parsed successfully across 7 files", total_cases);
    assert!(total_cases > 100, "Should have parsed at least 100 test cases total");
}

#[test]
fn test_parse_expected_item_formats() {
    // Test parsing of different expected item formats
    let content = r#"
---
description: Test various expected formats
dialect: all
input: |
  SELECT | FROM users
expected: |
  id [Field] users.id
  username [Field] users.username
  email
  COUNT [Function] COUNT(*) - count rows
options: |
  - min_items: 2
  - contains: id, username
"#;

    let cases = unified_sql_lsp_test_utils::parse_test_content(content)
        .expect("Failed to parse test content");

    assert_eq!(cases.len(), 1);

    let case = &cases[0];
    assert_eq!(case.description, "Test various expected formats");
    assert_eq!(case.expected.len(), 4);

    // Check full format item
    match &case.expected[0] {
        unified_sql_lsp_test_utils::ExpectedItem::Full { label, kind, detail } => {
            assert_eq!(label, "id");
            assert_eq!(kind, "Field");
            assert_eq!(detail, "users.id");
        }
        _ => panic!("Expected Full format for first item"),
    }

    // Check simple format item
    match &case.expected[2] {
        unified_sql_lsp_test_utils::ExpectedItem::Simple(name) => {
            assert_eq!(name, "email");
        }
        _ => panic!("Expected Simple format for third item"),
    }

    // Check options parsing
    let options = case.options.as_ref().expect("Should have options");
    assert_eq!(options.min_items, Some(2));
    assert_eq!(
        options.contains.as_ref().unwrap(),
        &vec!["id".to_string(), "username".to_string()]
    );
}

#[test]
fn test_dialect_parsing() {
    let content = r#"
---
description: MySQL test
dialect: mysql
input: |
  SELECT | FROM users

---
description: PostgreSQL test
dialect: postgresql
input: |
  SELECT | FROM users

---
description: All dialects test
dialect: all
input: |
  SELECT | FROM users
"#;

    let cases = unified_sql_lsp_test_utils::parse_test_content(content)
        .expect("Failed to parse test content");

    assert_eq!(cases.len(), 3);
    assert_eq!(cases[0].dialect, Dialect::MySQL);
    assert_eq!(cases[1].dialect, Dialect::PostgreSQL);
    assert_eq!(cases[2].dialect, Dialect::All);
}

#[test]
fn test_multiline_input_handling() {
    let content = r#"
---
description: Multiline input test
dialect: all
input: |
  WITH user_stats AS (
    SELECT | FROM users
  )
  SELECT * FROM user_stats
expected: |
  id [Field] users.id
  username [Field] users.username
"#;

    let cases = unified_sql_lsp_test_utils::parse_test_content(content)
        .expect("Failed to parse test content");

    assert_eq!(cases.len(), 1);
    assert!(cases[0].input.contains("WITH user_stats"));
    assert!(cases[0].input.contains("SELECT | FROM users"));
}

#[test]
fn test_empty_expected_section() {
    let content = r#"
---
description: Empty expected test
dialect: all
input: |
  SELECT * FROM users
expected: |
options: |
  - min_items: 0
"#;

    let cases = unified_sql_lsp_test_utils::parse_test_content(content)
        .expect("Failed to parse test content");

    assert_eq!(cases.len(), 1);
    assert_eq!(cases[0].expected.len(), 0);
    assert_eq!(cases[0].options.as_ref().unwrap().min_items, Some(0));
}
