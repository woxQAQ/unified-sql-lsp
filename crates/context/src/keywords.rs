// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # SQL Keywords for Completion
//!
//! This module defines SQL keywords organized by context and dialect.

use std::collections::HashSet;
use unified_sql_lsp_ir::Dialect;

/// SQL keyword with metadata
#[derive(Debug, Clone, PartialEq)]
pub struct SqlKeyword {
    /// The keyword text
    pub label: String,
    /// Optional description/documentation
    pub description: Option<String>,
    /// Sort order (lower = higher priority)
    pub sort_priority: i32,
}

impl SqlKeyword {
    /// Create a new SQL keyword
    pub fn new(label: &str, description: Option<&str>, sort_priority: i32) -> Self {
        Self {
            label: label.to_uppercase(),
            description: description.map(|d| d.to_string()),
            sort_priority,
        }
    }

    /// Create a simple keyword without description
    pub fn simple(label: &str, sort_priority: i32) -> Self {
        Self::new(label, None, sort_priority)
    }
}

/// Keyword set for a specific context
#[derive(Debug, Clone)]
pub struct KeywordSet {
    /// Keywords in this set
    pub keywords: Vec<SqlKeyword>,
}

impl KeywordSet {
    /// Create a new keyword set
    pub fn new(keywords: Vec<SqlKeyword>) -> Self {
        Self { keywords }
    }

    /// Get all keyword labels as a HashSet for filtering
    pub fn labels(&self) -> HashSet<String> {
        self.keywords.iter().map(|k| k.label.clone()).collect()
    }

    /// Filter keywords excluding the given set
    pub fn exclude(&self, exclude: &HashSet<String>) -> Vec<SqlKeyword> {
        self.keywords
            .iter()
            .filter(|k| !exclude.contains(&k.label))
            .cloned()
            .collect()
    }
}

/// Keyword provider for different SQL contexts
pub struct KeywordProvider {
    /// SQL dialect
    dialect: Dialect,
}

impl KeywordProvider {
    /// Create a new keyword provider
    pub fn new(dialect: Dialect) -> Self {
        Self { dialect }
    }

    /// Get statement keywords (for start of statement)
    pub fn statement_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("SELECT", Some("Retrieve data from tables"), 1),
            SqlKeyword::new("INSERT", Some("Insert new rows into a table"), 2),
            SqlKeyword::new("UPDATE", Some("Modify existing rows in a table"), 3),
            SqlKeyword::new("DELETE", Some("Delete rows from a table"), 4),
            SqlKeyword::new("CREATE", Some("Create database objects"), 5),
            SqlKeyword::new("ALTER", Some("Modify database objects"), 6),
            SqlKeyword::new("DROP", Some("Remove database objects"), 7),
            SqlKeyword::new("TRUNCATE", Some("Remove all rows from a table"), 8),
            SqlKeyword::new("WITH", Some("Common Table Expression (CTE)"), 9),
        ];

        KeywordSet::new(keywords)
    }

    /// Get SELECT clause keywords (for within SELECT statements)
    pub fn select_clause_keywords(&self) -> KeywordSet {
        let mut keywords = vec![
            SqlKeyword::new("CASE", Some("Conditional expression"), 1),
            SqlKeyword::new("FROM", Some("Specify tables to query"), 2),
            SqlKeyword::new("WHERE", Some("Filter rows"), 3),
            SqlKeyword::new("GROUP BY", Some("Group rows by values"), 4),
            SqlKeyword::new("HAVING", Some("Filter groups"), 5),
            SqlKeyword::new("ORDER BY", Some("Sort result rows"), 6),
            SqlKeyword::new("LIMIT", Some("Limit number of rows"), 7),
            SqlKeyword::new("OFFSET", Some("Skip rows before limiting"), 8),
            SqlKeyword::new("JOIN", Some("Join with another table"), 9),
            SqlKeyword::new("INNER JOIN", Some("Inner join with another table"), 10),
            SqlKeyword::new("LEFT JOIN", Some("Left outer join"), 11),
            SqlKeyword::new("RIGHT JOIN", Some("Right outer join"), 12),
            SqlKeyword::new("FULL JOIN", Some("Full outer join"), 13),
            SqlKeyword::new("CROSS JOIN", Some("Cross join"), 14),
            SqlKeyword::new("STRAIGHT_JOIN", Some("Straight join (MySQL)"), 15),
            SqlKeyword::new("UNION", Some("Combine result sets"), 16),
            SqlKeyword::new("UNION ALL", Some("Combine result sets with duplicates"), 17),
            SqlKeyword::new("INTERSECT", Some("Intersection of result sets"), 18),
            SqlKeyword::new("EXCEPT", Some("Difference of result sets"), 19),
            SqlKeyword::new("DISTINCT", Some("Remove duplicate rows"), 20),
            SqlKeyword::new("ALL", Some("Include all rows (default)"), 21),
            SqlKeyword::new("AS", Some("Alias for columns or tables"), 22),
            SqlKeyword::new("INTO", Some("Select into variables or table"), 23),
        ];

        // Add dialect-specific keywords
        if self.dialect == Dialect::PostgreSQL {
            keywords.push(SqlKeyword::new("FETCH", Some("Fetch specific rows"), 24));
            keywords.push(SqlKeyword::new(
                "FOR UPDATE",
                Some("Lock selected rows"),
                25,
            ));
        } else if self.dialect == Dialect::MySQL || self.dialect == Dialect::TiDB {
            keywords.push(SqlKeyword::new(
                "FOR UPDATE",
                Some("Lock selected rows"),
                24,
            ));
            keywords.push(SqlKeyword::new(
                "LOCK IN SHARE MODE",
                Some("Lock rows in share mode"),
                25,
            ));
        }

        KeywordSet::new(keywords)
    }

    /// Get JOIN type keywords
    pub fn join_type_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("INNER", Some("Inner join"), 1),
            SqlKeyword::new("LEFT", Some("Left outer join"), 2),
            SqlKeyword::new("RIGHT", Some("Right outer join"), 3),
            SqlKeyword::new("FULL", Some("Full outer join"), 4),
            SqlKeyword::new("CROSS", Some("Cross join"), 5),
            SqlKeyword::new("NATURAL", Some("Natural join"), 6),
            SqlKeyword::new("LATERAL", Some("Lateral join"), 7),
        ];

        KeywordSet::new(keywords)
    }

    /// Get expression/operator keywords
    pub fn expression_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("AND", Some("Logical AND"), 1),
            SqlKeyword::new("OR", Some("Logical OR"), 2),
            SqlKeyword::new("NOT", Some("Logical NOT"), 3),
            SqlKeyword::new("IN", Some("Value in list"), 4),
            SqlKeyword::new("EXISTS", Some("Subquery exists"), 5),
            SqlKeyword::new("BETWEEN", Some("Value between range"), 6),
            SqlKeyword::new("LIKE", Some("Pattern matching"), 7),
            SqlKeyword::new("IS", Some("IS NULL, IS TRUE, etc."), 8),
            SqlKeyword::new("IS NULL", Some("Check if value is NULL"), 9),
            SqlKeyword::new("IS NOT NULL", Some("Check if value is not NULL"), 10),
            SqlKeyword::new("IS TRUE", Some("Check if value is TRUE"), 11),
            SqlKeyword::new("IS FALSE", Some("Check if value is FALSE"), 12),
            SqlKeyword::new("CASE", Some("Conditional expression"), 13),
            SqlKeyword::new("WHEN", Some("CASE WHEN condition"), 14),
            SqlKeyword::new("THEN", Some("CASE THEN result"), 15),
            SqlKeyword::new("ELSE", Some("CASE ELSE default"), 16),
            SqlKeyword::new("END", Some("END CASE expression"), 17),
            SqlKeyword::new("NULL", Some("NULL value"), 18),
            SqlKeyword::new("TRUE", Some("Boolean TRUE"), 19),
            SqlKeyword::new("FALSE", Some("Boolean FALSE"), 20),
            SqlKeyword::new("CAST", Some("Cast to type"), 21),
            SqlKeyword::new("EXTRACT", Some("Extract date/time part"), 22),
            SqlKeyword::new("COALESCE", Some("First non-NULL value"), 23),
            SqlKeyword::new("NULLIF", Some("NULL if equal"), 24),
        ];

        KeywordSet::new(keywords)
    }

    /// Get CREATE statement keywords
    pub fn create_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("TABLE", Some("Create table"), 1),
            SqlKeyword::new("INDEX", Some("Create index"), 2),
            SqlKeyword::new("VIEW", Some("Create view"), 3),
            SqlKeyword::new("DATABASE", Some("Create database"), 4),
            SqlKeyword::new("SCHEMA", Some("Create schema"), 5),
            SqlKeyword::new("FUNCTION", Some("Create function"), 6),
            SqlKeyword::new("PROCEDURE", Some("Create procedure"), 7),
            SqlKeyword::new("TRIGGER", Some("Create trigger"), 8),
            SqlKeyword::new("TEMPORARY", Some("Temporary object"), 9),
            SqlKeyword::new("OR REPLACE", Some("Replace if exists"), 10),
        ];

        KeywordSet::new(keywords)
    }

    /// Get ALTER statement keywords
    pub fn alter_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("TABLE", Some("Alter table"), 1),
            SqlKeyword::new("VIEW", Some("Alter view"), 2),
            SqlKeyword::new("DATABASE", Some("Alter database"), 3),
            SqlKeyword::new("SCHEMA", Some("Alter schema"), 4),
            SqlKeyword::new("FUNCTION", Some("Alter function"), 5),
            SqlKeyword::new("PROCEDURE", Some("Alter procedure"), 6),
            SqlKeyword::new("TRIGGER", Some("Alter trigger"), 7),
            SqlKeyword::new("INDEX", Some("Alter index"), 8),
        ];

        KeywordSet::new(keywords)
    }

    /// Get DROP statement keywords
    pub fn drop_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("TABLE", Some("Drop table"), 1),
            SqlKeyword::new("INDEX", Some("Drop index"), 2),
            SqlKeyword::new("VIEW", Some("Drop view"), 3),
            SqlKeyword::new("DATABASE", Some("Drop database"), 4),
            SqlKeyword::new("SCHEMA", Some("Drop schema"), 5),
            SqlKeyword::new("FUNCTION", Some("Drop function"), 6),
            SqlKeyword::new("PROCEDURE", Some("Drop procedure"), 7),
            SqlKeyword::new("TRIGGER", Some("Drop trigger"), 8),
            SqlKeyword::new("TEMPORARY", Some("Temporary object"), 9),
            SqlKeyword::new("IF EXISTS", Some("Drop if exists"), 10),
        ];

        KeywordSet::new(keywords)
    }

    /// Get UNION statement keywords
    pub fn union_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("ALL", Some("Include duplicates"), 1),
            SqlKeyword::new("SELECT", Some("Select statement"), 2),
        ];

        KeywordSet::new(keywords)
    }

    /// Get INSERT statement keywords
    pub fn insert_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("INTO", Some("Insert into table"), 1),
            SqlKeyword::new("VALUES", Some("Insert values"), 2),
            SqlKeyword::new("SET", Some("Set column values (MySQL)"), 3),
            SqlKeyword::new("ON DUPLICATE KEY UPDATE", Some("MySQL upsert"), 4),
            SqlKeyword::new("RETURNING", Some("Return inserted rows (PostgreSQL)"), 5),
        ];

        KeywordSet::new(keywords)
    }

    /// Get UPDATE statement keywords
    pub fn update_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("SET", Some("Set column values"), 1),
            SqlKeyword::new("WHERE", Some("Filter rows to update"), 2),
            SqlKeyword::new("FROM", Some("Additional tables (PostgreSQL)"), 3),
            SqlKeyword::new("RETURNING", Some("Return updated rows (PostgreSQL)"), 4),
        ];

        KeywordSet::new(keywords)
    }

    /// Get DELETE statement keywords
    pub fn delete_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("FROM", Some("Delete from table"), 1),
            SqlKeyword::new("WHERE", Some("Filter rows to delete"), 2),
            SqlKeyword::new("RETURNING", Some("Return deleted rows (PostgreSQL)"), 3),
        ];

        KeywordSet::new(keywords)
    }

    /// Get keywords available after a specific clause
    pub fn keywords_after_clause(&self, clause: &str) -> Vec<SqlKeyword> {
        match clause {
            "SELECT" | "select_statement" => {
                // After SELECT, we can have: DISTINCT, FROM, WHERE, JOIN, etc.
                let all = self.select_clause_keywords();
                all.keywords
            }
            "FROM" | "from_clause" => {
                // After FROM, we can have: JOIN, WHERE, GROUP BY, etc.
                let all = self.select_clause_keywords();
                all.exclude(&HashSet::from(["FROM".to_string()]))
            }
            "WHERE" | "where_clause" => {
                // After WHERE, we can have: GROUP BY, ORDER BY, LIMIT, etc.
                vec![
                    SqlKeyword::new("GROUP BY", Some("Group rows by values"), 1),
                    SqlKeyword::new("ORDER BY", Some("Sort result rows"), 2),
                    SqlKeyword::new("LIMIT", Some("Limit number of rows"), 3),
                    SqlKeyword::new("OFFSET", Some("Skip rows"), 4),
                    SqlKeyword::new("HAVING", Some("Filter groups"), 5),
                ]
            }
            "JOIN" | "join_clause" => {
                // After JOIN, we can have: ON, USING
                vec![
                    SqlKeyword::new("ON", Some("Join condition"), 1),
                    SqlKeyword::new("USING", Some("Join using columns"), 2),
                ]
            }
            "GROUP BY" | "group_by_clause" => {
                // After GROUP BY, we can have: HAVING, ORDER BY, LIMIT
                vec![
                    SqlKeyword::new("HAVING", Some("Filter groups"), 1),
                    SqlKeyword::new("ORDER BY", Some("Sort result rows"), 2),
                    SqlKeyword::new("LIMIT", Some("Limit number of rows"), 3),
                ]
            }
            "ORDER BY" | "order_by_clause" => {
                // After ORDER BY, we can have: LIMIT, OFFSET
                vec![
                    SqlKeyword::new("LIMIT", Some("Limit number of rows"), 1),
                    SqlKeyword::new("OFFSET", Some("Skip rows"), 2),
                ]
            }
            _ => vec![],
        }
    }

    /// Get all clause keywords for filtering
    pub fn all_clause_keywords(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("SELECT".to_string());
        set.insert("FROM".to_string());
        set.insert("WHERE".to_string());
        set.insert("GROUP BY".to_string());
        set.insert("HAVING".to_string());
        set.insert("ORDER BY".to_string());
        set.insert("LIMIT".to_string());
        set.insert("OFFSET".to_string());
        set.insert("JOIN".to_string());
        set.insert("INNER JOIN".to_string());
        set.insert("LEFT JOIN".to_string());
        set.insert("RIGHT JOIN".to_string());
        set.insert("FULL JOIN".to_string());
        set.insert("CROSS JOIN".to_string());
        set.insert("STRAIGHT_JOIN".to_string());
        set.insert("UNION".to_string());
        set.insert("UNION ALL".to_string());
        set.insert("INTERSECT".to_string());
        set.insert("EXCEPT".to_string());
        set.insert("WITH".to_string());
        set.insert("INSERT".to_string());
        set.insert("UPDATE".to_string());
        set.insert("DELETE".to_string());
        set.insert("CREATE".to_string());
        set.insert("ALTER".to_string());
        set.insert("DROP".to_string());
        set
    }

    /// Get sort direction keywords (ASC, DESC)
    pub fn sort_direction_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("ASC", Some("Ascending order"), 1),
            SqlKeyword::new("DESC", Some("Descending order"), 2),
        ];

        KeywordSet::new(keywords)
    }

    /// Get HAVING keyword (for after GROUP BY)
    pub fn having_keywords(&self) -> KeywordSet {
        let keywords = vec![SqlKeyword::new("HAVING", Some("Filter groups"), 1)];

        KeywordSet::new(keywords)
    }

    /// Get LIMIT keywords (common LIMIT values and OFFSET)
    pub fn limit_keywords(&self) -> KeywordSet {
        let keywords = vec![
            SqlKeyword::new("1", Some("Limit to 1 row"), 1),
            SqlKeyword::new("10", Some("Limit to 10 rows"), 2),
            SqlKeyword::new("100", Some("Limit to 100 rows"), 3),
            SqlKeyword::new("1000", Some("Limit to 1000 rows"), 4),
            SqlKeyword::new("OFFSET", Some("Skip rows before limiting"), 5),
        ];

        KeywordSet::new(keywords)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_keyword_creation() {
        let kw = SqlKeyword::new("SELECT", Some("Test description"), 1);
        assert_eq!(kw.label, "SELECT");
        assert_eq!(kw.description, Some("Test description".to_string()));
        assert_eq!(kw.sort_priority, 1);
    }

    #[test]
    fn test_keyword_set_labels() {
        let keywords = vec![
            SqlKeyword::simple("SELECT", 1),
            SqlKeyword::simple("FROM", 2),
            SqlKeyword::simple("WHERE", 3),
        ];
        let set = KeywordSet::new(keywords);
        let labels = set.labels();
        assert!(labels.contains("SELECT"));
        assert!(labels.contains("FROM"));
        assert!(labels.contains("WHERE"));
        assert_eq!(labels.len(), 3);
    }

    #[test]
    fn test_keyword_set_exclude() {
        let keywords = vec![
            SqlKeyword::simple("SELECT", 1),
            SqlKeyword::simple("FROM", 2),
            SqlKeyword::simple("WHERE", 3),
        ];
        let set = KeywordSet::new(keywords);
        let exclude = HashSet::from(["FROM".to_string()]);
        let filtered = set.exclude(&exclude);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().any(|k| k.label == "SELECT"));
        assert!(filtered.iter().any(|k| k.label == "WHERE"));
        assert!(!filtered.iter().any(|k| k.label == "FROM"));
    }

    #[test]
    fn test_keyword_provider_mysql() {
        let provider = KeywordProvider::new(Dialect::MySQL);
        let stmt_keywords = provider.statement_keywords();
        assert!(!stmt_keywords.keywords.is_empty());
        assert!(stmt_keywords.keywords.iter().any(|k| k.label == "SELECT"));

        let select_keywords = provider.select_clause_keywords();
        assert!(select_keywords.keywords.iter().any(|k| k.label == "LIMIT"));
    }

    #[test]
    fn test_keyword_provider_postgresql() {
        let provider = KeywordProvider::new(Dialect::PostgreSQL);
        let select_keywords = provider.select_clause_keywords();
        // PostgreSQL uses FETCH instead of LIMIT
        assert!(select_keywords.keywords.iter().any(|k| k.label == "FETCH"));
    }

    #[test]
    fn test_keywords_after_clause() {
        let provider = KeywordProvider::new(Dialect::MySQL);
        let after_from = provider.keywords_after_clause("FROM");
        // After FROM we should not get FROM keyword again
        assert!(!after_from.iter().any(|k| k.label == "FROM"));
        // But we should get WHERE
        assert!(after_from.iter().any(|k| k.label == "WHERE"));
    }
}
