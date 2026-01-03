// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! SQL-specific test helpers and custom assertions

use unified_sql_lsp_catalog::{ColumnMetadata, DataType};
use unified_sql_lsp_ir::Expr;

/// Custom assertion helpers for SQL testing
pub struct SqlAssertions;

impl SqlAssertions {
    /// Assert that an expression is a column reference with the given name
    pub fn assert_column_ref(expr: &Expr, name: &str) {
        match expr {
            Expr::Column(col) => {
                assert_eq!(col.column, name, "Expected column '{}', found '{}'", name, col.column);
            }
            _ => panic!("Expected Column expression, found {:?}", expr),
        }
    }

    /// Assert that an expression is a literal with the given value
    pub fn assert_literal_int(expr: &Expr, value: i64) {
        match expr {
            Expr::Literal(unified_sql_lsp_ir::Literal::Integer(v)) => {
                assert_eq!(*v, value, "Expected integer {}, found {}", value, v);
            }
            _ => panic!("Expected Integer literal, found {:?}", expr),
        }
    }

    /// Assert that an expression is a literal string
    pub fn assert_literal_string(expr: &Expr, value: &str) {
        match expr {
            Expr::Literal(unified_sql_lsp_ir::Literal::String(v)) => {
                assert_eq!(v, value, "Expected string '{}', found '{}'", value, v);
            }
            _ => panic!("Expected String literal, found {:?}", expr),
        }
    }

    /// Assert that a column has the given properties
    pub fn assert_column(
        column: &ColumnMetadata,
        name: &str,
        data_type: DataType,
        nullable: bool,
    ) {
        assert_eq!(column.name, name, "Column name mismatch");
        assert_eq!(column.data_type, data_type, "Column data type mismatch");
        assert_eq!(column.nullable, nullable, "Column nullable mismatch");
    }

    /// Assert that a column is a primary key
    pub fn assert_primary_key(column: &ColumnMetadata) {
        assert!(
            column.is_primary_key,
            "Column '{}' is not a primary key",
            column.name
        );
    }

    /// Assert that a column is a foreign key referencing the given table
    pub fn assert_foreign_key(column: &ColumnMetadata, table: &str, ref_column: &str) {
        assert!(
            column.is_foreign_key,
            "Column '{}' is not a foreign key",
            column.name
        );

        if let Some(refs) = &column.references {
            assert_eq!(refs.table, table, "Foreign key references wrong table");
            assert_eq!(refs.column, ref_column, "Foreign key references wrong column");
        } else {
            panic!("Column '{}' has no reference information", column.name);
        }
    }
}

/// Helper to check if a string contains SQL keywords
pub fn contains_sql_keywords(text: &str) -> bool {
    let keywords = [
        "SELECT", "FROM", "WHERE", "JOIN", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP",
        "ALTER", "INDEX",
    ];

    let upper = text.to_uppercase();
    keywords.iter().any(|kw| upper.contains(kw))
}

/// Helper to validate SQL identifier naming
pub fn is_valid_identifier(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    // Check if it starts with a letter or underscore
    if !name.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false) {
        return false;
    }

    // Check remaining characters
    name.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// Helper to extract table name from a SQL query
pub fn extract_table_names(query: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let upper = query.to_uppercase();

    // Simple FROM clause extraction (not exhaustive)
    if let Some(from_idx) = upper.find("FROM") {
        let after_from = &query[from_idx + 4..];
        let rest: String = after_from
            .chars()
            .take_while(|c| c.is_whitespace() || c.is_alphanumeric() || *c == '_' || *c == '.')
            .collect();
        let table_name = rest.trim().to_string();
        if !table_name.is_empty() {
            tables.push(table_name);
        }
    }

    tables
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_ir::ColumnRef;

    #[test]
    fn test_assert_column_ref() {
        let expr = Expr::Column(ColumnRef::new("user_id"));
        SqlAssertions::assert_column_ref(&expr, "user_id");
    }

    #[test]
    #[should_panic(expected = "Expected Column expression")]
    fn test_assert_column_ref_fails() {
        let expr = Expr::Literal(unified_sql_lsp_ir::Literal::Integer(42));
        SqlAssertions::assert_column_ref(&expr, "user_id");
    }

    #[test]
    fn test_assert_literal_int() {
        let expr = Expr::Literal(unified_sql_lsp_ir::Literal::Integer(42));
        SqlAssertions::assert_literal_int(&expr, 42);
    }

    #[test]
    fn test_contains_sql_keywords() {
        assert!(contains_sql_keywords("SELECT * FROM users"));
        assert!(contains_sql_keywords("select * from users"));
        assert!(!contains_sql_keywords("Hello world"));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("users"));
        assert!(is_valid_identifier("user_id"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("table123"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123users"));
        assert!(!is_valid_identifier("user-id"));
    }

    #[test]
    fn test_extract_table_names() {
        let query = "SELECT id, name FROM users WHERE id > 10";
        let tables = extract_table_names(query);
        assert_eq!(tables, vec!["users"]);
    }

    #[test]
    fn test_assert_column() {
        let col = ColumnMetadata::new("id", DataType::BigInt).with_nullable(false);
        SqlAssertions::assert_column(&col, "id", DataType::BigInt, false);
    }

    #[test]
    fn test_assert_primary_key() {
        let col = ColumnMetadata::new("id", DataType::BigInt)
            .with_nullable(false)
            .with_primary_key();
        SqlAssertions::assert_primary_key(&col);
    }

    #[test]
    fn test_assert_foreign_key() {
        let col = ColumnMetadata::new("user_id", DataType::BigInt)
            .with_nullable(false)
            .with_foreign_key("users", "id");
        SqlAssertions::assert_foreign_key(&col, "users", "id");
    }
}
