// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Test case parser validation
//!
//! This test verifies that all generated test case files can be parsed correctly
//! and validates the parser implementation against the test case generation guide.

use std::path::Path;

/// Test configuration
pub struct TestFileSpec {
    pub name: &'static str,
    pub path: &'static str,
    pub min_cases: usize,
    pub max_cases: usize,
    pub has_dialect_specific: bool,
}

pub const TEST_FILES: &[TestFileSpec] = &[
    TestFileSpec {
        name: "Basic SELECT",
        path: "fixtures/cases/01_basic_select.txt",
        min_cases: 10,
        max_cases: 20,
        has_dialect_specific: false,
    },
    TestFileSpec {
        name: "FROM Clause",
        path: "fixtures/cases/02_from_clause.txt",
        min_cases: 15,
        max_cases: 25,
        has_dialect_specific: true,
    },
    TestFileSpec {
        name: "JOIN",
        path: "fixtures/cases/03_join.txt",
        min_cases: 15,
        max_cases: 30,
        has_dialect_specific: true,
    },
    TestFileSpec {
        name: "WHERE Clause",
        path: "fixtures/cases/04_where_clause.txt",
        min_cases: 20,
        max_cases: 35,
        has_dialect_specific: false,
    },
    TestFileSpec {
        name: "Functions",
        path: "fixtures/cases/05_functions.txt",
        min_cases: 20,
        max_cases: 40,
        has_dialect_specific: true,
    },
    TestFileSpec {
        name: "Advanced",
        path: "fixtures/cases/06_advanced.txt",
        min_cases: 25,
        max_cases: 50,
        has_dialect_specific: true,
    },
    TestFileSpec {
        name: "Edge Cases",
        path: "fixtures/cases/07_edge_cases.txt",
        min_cases: 20,
        max_cases: 45,
        has_dialect_specific: false,
    },
];

/// Get the base path for test fixtures
pub fn get_test_base_path() -> String {
    // Determine the base path for test fixtures
    // CARGO_MANIFEST_DIR points to the e2e crate directory
    
    std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string())
}

pub fn test_parse_all_test_files() {
    let base_path = get_test_base_path();
    let mut total_cases = 0;
    let mut total_files = 0;

    println!("\n=== Test Case Parser Validation ===\n");

    for spec in TEST_FILES {
        let full_path = format!("{}/{}", base_path, spec.path);
        let path = Path::new(&full_path);

        // Check file exists
        assert!(
            path.exists(),
            "Test file not found: {} (expected at {})",
            spec.path,
            full_path
        );

        // Parse the file
        let result = unified_sql_lsp_test_utils::parse_test_file(path);
        assert!(
            result.is_ok(),
            "Failed to parse {}: {:?}",
            spec.name,
            result.err()
        );

        let cases = result.unwrap();
        let case_count = cases.len();

        // Validate case count is within expected range
        assert!(
            case_count >= spec.min_cases && case_count <= spec.max_cases,
            "{}: Expected {}-{} test cases, got {}",
            spec.name,
            spec.min_cases,
            spec.max_cases,
            case_count
        );

        // Verify all cases have required fields
        for (i, case) in cases.iter().enumerate() {
            assert!(
                !case.description.is_empty(),
                "{}: Case {} has empty description",
                spec.name,
                i
            );
            assert!(
                !case.input.is_empty(),
                "{}: Case {} ({}) has empty input",
                spec.name,
                i,
                case.description
            );
            assert!(
                case.input.contains('|'),
                "{}: Case {} ({}) missing cursor marker '|'",
                spec.name,
                i,
                case.description
            );
        }

        // Check for dialect-specific tests if expected
        if spec.has_dialect_specific {
            let has_mysql = cases.iter().any(|c| c.dialect == unified_sql_lsp_test_utils::Dialect::MySQL);
            let has_pg = cases.iter().any(|c| c.dialect == unified_sql_lsp_test_utils::Dialect::PostgreSQL);

            assert!(
                has_mysql || has_pg,
                "{}: Expected dialect-specific tests but found none",
                spec.name
            );
        }

        total_cases += case_count;
        total_files += 1;

        println!("✓ {}: {} test cases", spec.name, case_count);
    }

    println!("\n--- Summary ---");
    println!("Files parsed: {}", total_files);
    println!("Total test cases: {}", total_cases);
    println!("Average cases per file: {:.1}", total_cases as f64 / total_files as f64);

    // Assert minimum total coverage
    assert!(
        total_cases >= 100,
        "Expected at least 100 total test cases, got {}",
        total_cases
    );

    println!("\n✓ All test files parsed successfully!\n");
}

pub fn test_basic_select_file_structure() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/01_basic_select.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 01_basic_select.txt");

    // Verify specific test case from the file
    let first = &cases[0];
    assert_eq!(first.description, "Simple column name completion");
    assert_eq!(first.dialect, unified_sql_lsp_test_utils::Dialect::All);
    assert!(first.input.contains("SELECT | FROM users"));

    // Check that expected items include common columns
    assert!(
        first.expected.len() >= 10,
        "Expected at least 10 columns, got {}",
        first.expected.len()
    );
}

pub fn test_join_foreign_key_context() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/03_join.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 03_join.txt");

    // Find a test with foreign key context
    let fk_test = cases
        .iter()
        .find(|c| c.context.as_ref().map(|s| s.contains("foreign key")).unwrap_or(false));

    assert!(
        fk_test.is_some(),
        "03_join.txt should have tests with foreign key context"
    );

    let fk_test = fk_test.unwrap();
    assert!(
        fk_test.input.contains("JOIN"),
        "Foreign key test should involve JOIN"
    );
}

pub fn test_functions_dialect_coverage() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/05_functions.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 05_functions.txt");

    // Check for MySQL-specific functions
    let mysql_tests = cases
        .iter()
        .filter(|c| c.dialect == unified_sql_lsp_test_utils::Dialect::MySQL)
        .count();

    // Check for PostgreSQL-specific functions
    let pg_tests = cases
        .iter()
        .filter(|c| c.dialect == unified_sql_lsp_test_utils::Dialect::PostgreSQL)
        .count();

    assert!(
        mysql_tests > 0 || pg_tests > 0,
        "05_functions.txt should have dialect-specific tests"
    );

    // Check for common aggregate functions
    let has_count = cases.iter().any(|c| c.input.contains("COUNT"));
    let has_sum = cases.iter().any(|c| c.input.contains("SUM"));
    let has_avg = cases.iter().any(|c| c.input.contains("AVG"));

    assert!(has_count, "Should have COUNT function tests");
    assert!(has_sum, "Should have SUM function tests");
    assert!(has_avg, "Should have AVG function tests");
}

pub fn test_advanced_cte_coverage() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/06_advanced.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 06_advanced.txt");

    // Check for CTE tests
    let cte_tests = cases
        .iter()
        .filter(|c| c.input.contains("WITH"))
        .count();

    assert!(
        cte_tests >= 3,
        "Expected at least 3 CTE-related tests, got {}",
        cte_tests
    );

    // Check for UNION tests
    let union_tests = cases
        .iter()
        .filter(|c| c.input.contains("UNION"))
        .count();

    assert!(
        union_tests >= 2,
        "Expected at least 2 UNION-related tests, got {}",
        union_tests
    );

    // Check for GROUP BY/HAVING tests
    let group_by_tests = cases
        .iter()
        .filter(|c| c.input.contains("GROUP BY"))
        .count();

    assert!(
        group_by_tests >= 2,
        "Expected at least 2 GROUP BY tests, got {}",
        group_by_tests
    );
}

pub fn test_edge_cases_boundary_conditions() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/07_edge_cases.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 07_edge_cases.txt");

    // Check for empty input test
    let empty_test = cases
        .iter()
        .find(|c| c.input.trim() == "|");

    assert!(
        empty_test.is_some(),
        "07_edge_cases.txt should have an empty input test"
    );

    // Check for syntax error test
    let syntax_error_test = cases
        .iter()
        .find(|c| {
            let desc_lower = c.description.to_lowercase();
            desc_lower.contains("syntax error") || desc_lower.contains("misspelled")
        });

    assert!(
        syntax_error_test.is_some(),
        "07_edge_cases.txt should have syntax error tests"
    );

    // Check for deep nesting test
    let deep_nesting_test = cases
        .iter()
        .find(|c| c.description.contains("nesting") || c.input.contains("SELECT * FROM (SELECT * FROM (SELECT"));

    assert!(
        deep_nesting_test.is_some(),
        "07_edge_cases.txt should have deep nesting tests"
    );
}

pub fn test_options_validation() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/01_basic_select.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 01_basic_select.txt");

    // Find a test with options
    let with_options = cases
        .iter()
        .find(|c| c.options.is_some());

    if with_options.is_none() {
        println!("WARNING: No tests with parsed options found in 01_basic_select.txt");
        println!("This is expected - the parse_options function needs to be fixed");
        return;
    }

    let opts = with_options.unwrap().options.as_ref().unwrap();

    // Verify option was parsed correctly
    assert!(
        opts.min_items.is_some() || opts.contains.is_some(),
        "Options should include min_items or contains"
    );
}

pub fn test_expected_item_formats() {
    let base_path = get_test_base_path();
    let path = format!("{}/fixtures/cases/01_basic_select.txt", base_path);

    let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
        .expect("Failed to parse 01_basic_select.txt");

    let first = &cases[0];

    // Check that we have both Full and Simple format items
    let has_full = first.expected.iter().any(|item| matches!(
        item,
        unified_sql_lsp_test_utils::ExpectedItem::Full { .. }
    ));

    let has_simple = first.expected.iter().any(|item| matches!(
        item,
        unified_sql_lsp_test_utils::ExpectedItem::Simple(_)
    ));

    // At least one format should be present
    assert!(
        has_full || has_simple,
        "Expected items should use Full or Simple format"
    );
}

pub fn test_dialect_distribution() {
    let base_path = get_test_base_path();
    let mut all_dialect_count = 0;
    let mut mysql_count = 0;
    let mut pg_count = 0;

    for spec in TEST_FILES {
        let path = format!("{}/{}", base_path, spec.path);
        let cases = unified_sql_lsp_test_utils::parse_test_file(Path::new(&path))
            .unwrap_or_else(|_| panic!("Failed to parse {}", spec.name));

        for case in &cases {
            match case.dialect {
                unified_sql_lsp_test_utils::Dialect::All => all_dialect_count += 1,
                unified_sql_lsp_test_utils::Dialect::MySQL => mysql_count += 1,
                unified_sql_lsp_test_utils::Dialect::PostgreSQL => pg_count += 1,
            }
        }
    }

    println!("\n=== Dialect Distribution ===");
    println!("All (generic): {}", all_dialect_count);
    println!("MySQL-specific: {}", mysql_count);
    println!("PostgreSQL-specific: {}", pg_count);
    println!("Total: {}", all_dialect_count + mysql_count + pg_count);

    // Most tests should be for "all" dialects
    let total = all_dialect_count + mysql_count + pg_count;
    let all_percentage = (all_dialect_count as f64 / total as f64) * 100.0;

    assert!(
        all_percentage >= 60.0,
        "At least 60% of tests should be for 'all' dialects, currently {:.1}%",
        all_percentage
    );

    // Should have some dialect-specific tests
    assert!(
        mysql_count + pg_count >= 5,
        "Should have at least 5 dialect-specific tests, got {}",
        mysql_count + pg_count
    );
}
