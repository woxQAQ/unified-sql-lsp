// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Grammar API integration tests
//!
//! Comprehensive tests for the `language_for_dialect()` function and parsing functionality.

use tree_sitter::Parser;
use unified_sql_grammar::language_for_dialect;
use unified_sql_lsp_ir::Dialect;

#[test]
fn test_language_for_dialect_mysql_family() {
    // Test MySQL and MySQL-compatible dialects
    let dialects = vec![Dialect::MySQL, Dialect::TiDB, Dialect::MariaDB];

    for dialect in dialects {
        assert!(
            language_for_dialect(dialect).is_some(),
            "Expected language for {:?}",
            dialect
        );
    }
}

#[test]
fn test_language_for_dialect_postgresql_family() {
    // Test PostgreSQL and PostgreSQL-compatible dialects
    let dialects = vec![Dialect::PostgreSQL, Dialect::CockroachDB];

    for dialect in dialects {
        assert!(
            language_for_dialect(dialect).is_some(),
            "Expected language for {:?}",
            dialect
        );
    }
}

#[test]
fn test_language_for_dialect_coverage() {
    // Test that all currently implemented dialects have a grammar
    let supported = vec![
        Dialect::MySQL,
        Dialect::PostgreSQL,
        Dialect::TiDB,
        Dialect::MariaDB,
        Dialect::CockroachDB,
    ];

    for dialect in supported {
        assert!(
            language_for_dialect(dialect).is_some(),
            "Expected language for supported dialect {:?}",
            dialect
        );
    }
}

#[test]
fn test_parse_simple_query_mysql() {
    let language = language_for_dialect(Dialect::MySQL).expect("MySQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    let source = "SELECT * FROM users WHERE id = 1";
    let tree = parser.parse(source, None).expect("Failed to parse");

    // Verify parsing succeeded
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_mysql_specific_syntax() {
    let language = language_for_dialect(Dialect::MySQL).expect("MySQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Test MySQL-specific LIMIT syntax
    let source = "SELECT * FROM users LIMIT 10 OFFSET 20";
    let tree = parser.parse(source, None).expect("Failed to parse");

    assert!(!tree.root_node().has_error());
}

#[test]
#[ignore = "TODO: PostgreSQL DISTINCT ON syntax not yet supported in grammar"]
fn test_parse_postgresql_specific_syntax() {
    let language =
        language_for_dialect(Dialect::PostgreSQL).expect("PostgreSQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Test PostgreSQL-specific DISTINCT ON syntax
    let source = "SELECT DISTINCT ON (name) name, id FROM users";
    let tree = parser.parse(source, None).expect("Failed to parse");

    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_with_syntax_error() {
    let language = language_for_dialect(Dialect::MySQL).expect("MySQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Intentionally invalid SQL
    let source = "SELCT 1"; // Typo: SELCT
    let tree = parser.parse(source, None).expect("Failed to parse");

    // Should have an error node
    assert!(tree.root_node().has_error());

    // Find the ERROR node
    let mut has_error = false;
    let mut node = tree.root_node();
    find_error(&mut node, &mut has_error);
    assert!(has_error, "Expected ERROR node in tree");
}

fn find_error(node: &tree_sitter::Node, has_error: &mut bool) {
    if node.kind() == "ERROR" {
        *has_error = true;
        return;
    }
    for child in node.children(&mut node.walk()) {
        find_error(&child, has_error);
    }
}

#[test]
fn test_language_caching() {
    // Verify that language_for_dialect caches results
    let lang1 = language_for_dialect(Dialect::MySQL);
    let lang2 = language_for_dialect(Dialect::MySQL);

    // Should return the same pointer (cached)
    assert!(
        lang1.map(|l| l as *const _) == lang2.map(|l| l as *const _),
        "Expected cached language to be same instance"
    );
}

#[test]
fn test_parse_complex_query() {
    let language = language_for_dialect(Dialect::MySQL).expect("MySQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Complex query with JOINs, aggregates, and subquery
    let source = r#"
        SELECT u.id, u.name, COUNT(o.id) as order_count
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        WHERE u.created_at > '2024-01-01'
        GROUP BY u.id, u.name
        HAVING COUNT(o.id) > 5
        ORDER BY order_count DESC
        LIMIT 10
    "#;

    let tree = parser.parse(source, None).expect("Failed to parse");
    assert!(!tree.root_node().has_error());
}

#[test]
fn test_parse_multiple_statements() {
    let language =
        language_for_dialect(Dialect::PostgreSQL).expect("PostgreSQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Multiple statements separated by semicolons
    let source = "CREATE TABLE users (id INT); INSERT INTO users VALUES (1); SELECT * FROM users;";
    let tree = parser.parse(source, None).expect("Failed to parse");

    assert!(!tree.root_node().has_error());
}

#[test]
fn test_dialect_family_uses_same_grammar() {
    // MySQL family should all return the same language pointer (MySQL grammar)
    let mysql_lang = language_for_dialect(Dialect::MySQL);
    let tidb_lang = language_for_dialect(Dialect::TiDB);
    let mariadb_lang = language_for_dialect(Dialect::MariaDB);

    assert!(mysql_lang.is_some());
    assert!(tidb_lang.is_some());
    assert!(mariadb_lang.is_some());

    assert_eq!(
        mysql_lang.map(|l| l as *const _),
        tidb_lang.map(|l| l as *const _),
        "TiDB should use same grammar as MySQL"
    );
    assert_eq!(
        mysql_lang.map(|l| l as *const _),
        mariadb_lang.map(|l| l as *const _),
        "MariaDB should use same grammar as MySQL"
    );

    // PostgreSQL family should all return the same language pointer
    let pg_lang = language_for_dialect(Dialect::PostgreSQL);
    let crdb_lang = language_for_dialect(Dialect::CockroachDB);

    assert!(pg_lang.is_some());
    assert!(crdb_lang.is_some());

    assert_eq!(
        pg_lang.map(|l| l as *const _),
        crdb_lang.map(|l| l as *const _),
        "CockroachDB should use same grammar as PostgreSQL"
    );
}
