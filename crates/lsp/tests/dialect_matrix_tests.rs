// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Multi-dialect test matrix
//!
//! Tests across all supported dialects to ensure consistency.

use unified_sql_lsp_ir::Dialect;
use unified_sql_lsp_lsp::parsing::ParserManager;

// Test basic parsing across dialects
#[test]
fn test_parse_basic_select_mysql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT id FROM users");

    assert!(
        !result.is_failed(),
        "Failed to parse for dialect MySQL",
    );
}

#[test]
fn test_parse_basic_select_postgresql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::PostgreSQL, "SELECT id FROM users");

    assert!(
        !result.is_failed(),
        "Failed to parse for dialect PostgreSQL",
    );
}

// Test dialect-specific syntax
#[test]
fn test_mysql_limit_syntax() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT * FROM users LIMIT 10 OFFSET 20");

    assert!(!result.is_failed());
}

#[test]
fn test_postgresql_distinct_on_syntax() {
    let manager = ParserManager::new();
    let result = manager.parse_text(
        Dialect::PostgreSQL,
        "SELECT DISTINCT ON (name) name, id FROM users",
    );

    assert!(!result.is_failed());
}

// Test dialect family mapping
#[test]
fn test_dialect_family_mapping() {
    use unified_sql_grammar::language_for_dialect;

    // MySQL family should all return the same language (MySQL)
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

    // PostgreSQL family
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

// Test JOIN syntax across dialects
#[test]
fn test_parse_join_mysql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(
        Dialect::MySQL,
        "SELECT * FROM users u JOIN orders o ON u.id = o.user_id",
    );

    assert!(!result.is_failed());
}

#[test]
fn test_parse_join_postgresql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(
        Dialect::PostgreSQL,
        "SELECT * FROM users u JOIN orders o ON u.id = o.user_id",
    );

    assert!(!result.is_failed());
}

// Test aggregate functions across dialects
#[test]
fn test_parse_aggregates_mysql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::MySQL, "SELECT COUNT(*), AVG(amount) FROM orders");

    assert!(!result.is_failed());
}

#[test]
fn test_parse_aggregates_postgresql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(Dialect::PostgreSQL, "SELECT COUNT(*), AVG(amount) FROM orders");

    assert!(!result.is_failed());
}

// Test WHERE clause across dialects
#[test]
fn test_parse_where_mysql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(
        Dialect::MySQL,
        "SELECT * FROM users WHERE active = true AND created_at > '2024-01-01'",
    );

    assert!(!result.is_failed());
}

#[test]
fn test_parse_where_postgresql() {
    let manager = ParserManager::new();
    let result = manager.parse_text(
        Dialect::PostgreSQL,
        "SELECT * FROM users WHERE active = true AND created_at > '2024-01-01'",
    );

    assert!(!result.is_failed());
}
