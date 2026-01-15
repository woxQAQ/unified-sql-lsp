//! Unified SQL Grammar
//!
//! This crate provides multi-dialect SQL grammar support using tree-sitter.
//!
//! ## Grammar Architecture
//!
//! The grammar is organized using a VERSIONED dialect strategy:
//!
//! - **`grammar.js`**: Main entry point that reads `DIALECT` environment variable
//! - **`dialect/mysql-5.7.js`**: BASE dialect for MySQL family (REPLACE, LIMIT, backticks)
//! - **`dialect/mysql-8.0.js`**: Extends MySQL 5.7 with window functions, recursive CTE, LATERAL
//! - **`dialect/postgresql-12.js`**: BASE dialect for PostgreSQL family (RETURNING, dollar-quotes)
//! - **`dialect/postgresql-14.js`**: Extends PostgreSQL 12 with JSON subscripting, SEARCH/CYCLE
//!
//! ## Version Hierarchy
//!
//! **MySQL Family**:
//! ```text
//! mysql-5.7 (base)
//!   └── mysql-8.0 (extends 5.7 with window functions, CTE, LATERAL)
//! ```
//!
//! **PostgreSQL Family**:
//! ```text
//! postgresql-12 (base)
//!   └── postgresql-14 (extends 12 with JSON subscripting, SEARCH/CYCLE)
//! ```
//!
//! ## Build Process
//!
//! The build script (`build.rs`) compiles separate parsers for each dialect:
//!
//! 1. Sets `DIALECT` environment variable (e.g., `DIALECT=mysql-5.7`)
//! 2. Runs `tree-sitter generate --no-bindings` to generate `parser.c`
//! 3. Compiles `parser.c` to dialect-specific object files
//! 4. Repeats for each dialect (base, mysql-5.7, mysql-8.0, postgresql-12, postgresql-14)
//!
//! ## Supported Dialects
//!
//! - **MySQL 5.7**: Base MySQL dialect with LIMIT syntax, REPLACE, backtick identifiers
//! - **MySQL 8.0+**: Adds window functions, recursive CTE, LATERAL derived tables
//! - **PostgreSQL 12**: Base PostgreSQL dialect with RETURNING, dollar-quoted strings
//! - **PostgreSQL 14+**: Adds JSON subscripting, SEARCH/CYCLE for CTEs
//! - **TiDB**: Uses MySQL 5.7 parser (MySQL-compatible)
//! - **MariaDB**: Uses MySQL 5.7 parser (MySQL-compatible)
//! - **CockroachDB**: Uses PostgreSQL 12 parser (PostgreSQL-compatible)
//!
//! ## Usage
//!
//! To get a parser for a specific dialect:
//!
//! ```text,ignore
//! use unified_sql_lsp_ir::Dialect;
//! use unified_sql_grammar::{language_for_dialect, language_for_dialect_with_version, DialectVersion};
//!
//! // Get base dialect parser (MySQL 5.7, PostgreSQL 12)
//! let mysql_57_lang = language_for_dialect(Dialect::MySQL).unwrap();
//! let pg_12_lang = language_for_dialect(Dialect::PostgreSQL).unwrap();
//!
//! // Get version-specific parser
//! let mysql_80_lang = language_for_dialect_with_version(
//!     Dialect::MySQL,
//!     Some(DialectVersion::MySQL80)
//! ).unwrap();
//!
//! let pg_14_lang = language_for_dialect_with_version(
//!     Dialect::PostgreSQL,
//!     Some(DialectVersion::PostgreSQL14)
//! ).unwrap();
//! ```

use std::sync::OnceLock;
use unified_sql_lsp_ir::Dialect;

/// Get the tree-sitter Language for a specific SQL dialect
///
/// This function returns a compiled tree-sitter grammar for the given dialect.
/// The grammar parsers are compiled at build time by `build.rs` using the
/// `DIALECT` environment variable to select which dialect rules to include.
///
/// # Dialect Mapping
///
/// The IR dialects are mapped to compiled grammar implementations:
///
/// - **MySQL family** → MySQL 5.7 grammar (`parser-mysql-5.7.o`):
///   - `Dialect::MySQL` - Maps to MySQL 5.7 (baseline MySQL dialect)
///   - `Dialect::TiDB` - TiDB is MySQL-compatible
///   - `Dialect::MariaDB` - MariaDB is MySQL-compatible
///
/// - **PostgreSQL family** → PostgreSQL 12 grammar (`parser-postgresql-12.o`):
///   - `Dialect::PostgreSQL` - Maps to PostgreSQL 12 (baseline PostgreSQL dialect)
///   - `Dialect::CockroachDB` - CockroachDB is PostgreSQL-compatible
///
/// # Architecture Note
///
/// This function maps dialects to their BASE version parsers:
/// - MySQL → MySQL 5.7 (base for MySQL family)
/// - PostgreSQL → PostgreSQL 12 (base for PostgreSQL family)
///
/// For version-specific parsers (e.g., MySQL 8.0, PostgreSQL 14), use
/// `language_for_dialect_with_version()` instead.
///
/// # Returns
///
/// - `Some(Language)` - Compiled tree-sitter language object for the dialect
/// - `None` - Dialect not supported or grammar compilation failed
///
/// # Example
///
/// ```rust,ignore
/// use unified_sql_lsp_ir::Dialect;
/// use unified_sql_grammar::language_for_dialect;
///
/// // Get MySQL 5.7 grammar (also works for TiDB, MariaDB)
/// if let Some(lang) = language_for_dialect(Dialect::MySQL) {
///     let mut parser = tree_sitter::Parser::new();
///     parser.set_language(lang).unwrap();
///     let tree = parser.parse("SELECT * FROM users LIMIT 10", None);
/// }
/// ```
pub fn language_for_dialect(dialect: Dialect) -> Option<&'static tree_sitter::Language> {
    static MYSQL_57_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
    static POSTGRESQL_12_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();

    // Map IR dialects to BASE version grammar implementations
    // MySQL family → MySQL 5.7 (base dialect)
    // PostgreSQL family → PostgreSQL 12 (base dialect)
    match dialect {
        Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB => MYSQL_57_LANG
            .get_or_init(|| unsafe {
                // Safety: tree_sitter_unified_sql_mysql_5_7() returns a pointer to the
                // language object compiled by tree-sitter from src/grammar/dialect/mysql-5.7.js
                // MySQL 5.7 is the base dialect for the MySQL family
                Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                    tree_sitter_unified_sql_mysql_5_7(),
                ))
            })
            .as_ref(),
        Dialect::PostgreSQL | Dialect::CockroachDB => POSTGRESQL_12_LANG
            .get_or_init(|| unsafe {
                // Safety: tree_sitter_unified_sql_postgresql_12() returns a pointer to the
                // language object compiled by tree-sitter from src/grammar/dialect/postgresql-12.js
                // PostgreSQL 12 is the base dialect for the PostgreSQL family
                Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                    tree_sitter_unified_sql_postgresql_12(),
                ))
            })
            .as_ref(),
        _ => None, // Unsupported dialect
    }
}

/// SQL dialect version for version-specific grammar selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DialectVersion {
    /// MySQL 5.7
    MySQL57,
    /// MySQL 8.0+
    MySQL80,
    /// PostgreSQL 12
    PostgreSQL12,
    /// PostgreSQL 14+
    PostgreSQL14,
}

impl DialectVersion {
    /// Parse a version string like "5.7", "8.0", "12", "14"
    pub fn parse(version: &str) -> Option<Self> {
        match version {
            "5.7" => Some(DialectVersion::MySQL57),
            "8.0" | "8" => Some(DialectVersion::MySQL80),
            "12" => Some(DialectVersion::PostgreSQL12),
            "14" | "15" | "16" => Some(DialectVersion::PostgreSQL14),
            _ => None,
        }
    }
}

/// Get the tree-sitter Language for a specific SQL dialect with version
///
/// This function returns a compiled tree-sitter grammar for the given dialect
/// and version. This allows parsing syntax that is specific to certain versions
/// of SQL databases (e.g., window functions in MySQL 8.0+ but not 5.7).
///
/// # Arguments
///
/// * `dialect` - The SQL dialect (MySQL, PostgreSQL, etc.)
/// * `version` - Optional version specification for version-specific grammar
///
/// # Returns
///
/// - `Some(Language)` - Compiled tree-sitter language object
/// - `None` - Dialect/version not supported or grammar compilation failed
///
/// # Example
///
/// ```rust,ignore
/// use unified_sql_lsp_ir::Dialect;
/// use unified_sql_grammar::{language_for_dialect_with_version, DialectVersion};
///
/// // Get MySQL 8.0 grammar (supports window functions)
/// if let Some(lang) = language_for_dialect_with_version(
///     Dialect::MySQL,
///     Some(DialectVersion::MySQL80)
/// ) {
///     let mut parser = tree_sitter::Parser::new();
///     parser.set_language(lang).unwrap();
///     let tree = parser.parse("SELECT ROW_NUMBER() OVER () FROM users", None);
/// }
///
/// // Get MySQL 5.7 grammar (no window functions)
/// if let Some(lang) = language_for_dialect_with_version(
///     Dialect::MySQL,
///     Some(DialectVersion::MySQL57)
/// ) {
///     let mut parser = tree_sitter::Parser::new();
///     parser.set_language(lang).unwrap();
///     // This will fail to parse correctly - window functions not supported in 5.7
/// }
/// ```
pub fn language_for_dialect_with_version(
    dialect: Dialect,
    version: Option<DialectVersion>,
) -> Option<&'static tree_sitter::Language> {
    // If no version specified, use the default (latest) grammar
    let version = version.unwrap_or(match dialect {
        Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB => DialectVersion::MySQL80,
        Dialect::PostgreSQL | Dialect::CockroachDB => DialectVersion::PostgreSQL14,
        _ => DialectVersion::MySQL80, // Fallback
    });

    // Map to the appropriate version-specific grammar
    match (dialect, version) {
        (Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB, DialectVersion::MySQL57) => {
            static MYSQL_57_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
            MYSQL_57_LANG
                .get_or_init(|| unsafe {
                    Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                        tree_sitter_unified_sql_mysql_5_7(),
                    ))
                })
                .as_ref()
        }
        (Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB, DialectVersion::MySQL80) => {
            static MYSQL_80_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
            MYSQL_80_LANG
                .get_or_init(|| unsafe {
                    Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                        tree_sitter_unified_sql_mysql_8_0(),
                    ))
                })
                .as_ref()
        }
        (Dialect::PostgreSQL | Dialect::CockroachDB, DialectVersion::PostgreSQL12) => {
            static POSTGRESQL_12_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
            POSTGRESQL_12_LANG
                .get_or_init(|| unsafe {
                    Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                        tree_sitter_unified_sql_postgresql_12(),
                    ))
                })
                .as_ref()
        }
        (Dialect::PostgreSQL | Dialect::CockroachDB, DialectVersion::PostgreSQL14) => {
            static POSTGRESQL_14_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
            POSTGRESQL_14_LANG
                .get_or_init(|| unsafe {
                    Some(std::mem::transmute::<*const (), tree_sitter::Language>(
                        tree_sitter_unified_sql_postgresql_14(),
                    ))
                })
                .as_ref()
        }
        _ => None,
    }
}

// External functions from compiled grammars
// Rust 2024 edition requires extern blocks to be unsafe
//
// Versioned dialect architecture:
// - MySQL 5.7 is the BASE MySQL dialect (no generic "mysql" parser)
// - PostgreSQL 12 is the BASE PostgreSQL dialect (no generic "postgresql" parser)
unsafe extern "C" {
    #[allow(dead_code)]
    fn tree_sitter_unified_sql_base() -> *const ();
    fn tree_sitter_unified_sql_mysql_5_7() -> *const ();
    fn tree_sitter_unified_sql_mysql_8_0() -> *const ();
    fn tree_sitter_unified_sql_postgresql_12() -> *const ();
    fn tree_sitter_unified_sql_postgresql_14() -> *const ();
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_ir::Dialect;

    #[test]
    fn test_language_for_dialect() {
        // Test MySQL-family dialects
        assert!(language_for_dialect(Dialect::MySQL).is_some());
        assert!(language_for_dialect(Dialect::TiDB).is_some());
        assert!(language_for_dialect(Dialect::MariaDB).is_some());

        // Test PostgreSQL-family dialects
        assert!(language_for_dialect(Dialect::PostgreSQL).is_some());
        assert!(language_for_dialect(Dialect::CockroachDB).is_some());
    }

    #[test]
    fn test_language_for_dialect_with_version() {
        // Test MySQL 5.7
        assert!(
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL57))
                .is_some()
        );

        // Test MySQL 8.0
        assert!(
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
                .is_some()
        );

        // Test PostgreSQL 12
        assert!(
            language_for_dialect_with_version(
                Dialect::PostgreSQL,
                Some(DialectVersion::PostgreSQL12)
            )
            .is_some()
        );

        // Test PostgreSQL 14
        assert!(
            language_for_dialect_with_version(
                Dialect::PostgreSQL,
                Some(DialectVersion::PostgreSQL14)
            )
            .is_some()
        );
    }

    #[test]
    fn test_dialect_version_parsing() {
        // Test version string parsing
        assert_eq!(DialectVersion::parse("5.7"), Some(DialectVersion::MySQL57));
        assert_eq!(DialectVersion::parse("8.0"), Some(DialectVersion::MySQL80));
        assert_eq!(DialectVersion::parse("8"), Some(DialectVersion::MySQL80));
        assert_eq!(
            DialectVersion::parse("12"),
            Some(DialectVersion::PostgreSQL12)
        );
        assert_eq!(
            DialectVersion::parse("14"),
            Some(DialectVersion::PostgreSQL14)
        );
        assert_eq!(
            DialectVersion::parse("15"),
            Some(DialectVersion::PostgreSQL14)
        );
        assert_eq!(
            DialectVersion::parse("16"),
            Some(DialectVersion::PostgreSQL14)
        );
        assert_eq!(DialectVersion::parse("unknown"), None);
    }

    #[test]
    fn test_basic_parsing_with_versioned_grammar() {
        use tree_sitter::Parser;

        // Test MySQL 5.7 parsing
        if let Some(lang) =
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL57))
        {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let sql = "SELECT * FROM users LIMIT 10";
            let tree = parser.parse(sql, None);
            assert!(tree.is_some(), "Failed to parse basic SQL with MySQL 5.7");
        }

        // Test MySQL 8.0 parsing
        if let Some(lang) =
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
        {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let sql = "SELECT * FROM users LIMIT 10";
            let tree = parser.parse(sql, None);
            assert!(tree.is_some(), "Failed to parse basic SQL with MySQL 8.0");
        }

        // Test PostgreSQL 12 parsing
        if let Some(lang) = language_for_dialect_with_version(
            Dialect::PostgreSQL,
            Some(DialectVersion::PostgreSQL12),
        ) {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let sql = "SELECT * FROM users LIMIT 10";
            let tree = parser.parse(sql, None);
            assert!(
                tree.is_some(),
                "Failed to parse basic SQL with PostgreSQL 12"
            );
        }

        // Test PostgreSQL 14 parsing
        if let Some(lang) = language_for_dialect_with_version(
            Dialect::PostgreSQL,
            Some(DialectVersion::PostgreSQL14),
        ) {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let sql = "SELECT * FROM users LIMIT 10";
            let tree = parser.parse(sql, None);
            assert!(
                tree.is_some(),
                "Failed to parse basic SQL with PostgreSQL 14"
            );
        }
    }

    #[test]
    fn test_window_function_version_differentiation() {
        use tree_sitter::Parser;

        // SQL with window function (MySQL 8.0+ feature)
        let window_function_sql = "SELECT ROW_NUMBER() OVER (PARTITION BY id) FROM users";

        // MySQL 5.7 should NOT support window functions (no OVER clause in grammar)
        if let Some(lang) =
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL57))
        {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let tree = parser.parse(window_function_sql, None);
            if let Some(tree) = tree {
                // MySQL 5.7: The parser exists but should have errors parsing window functions
                // since OVER clause is not in the grammar
                assert!(
                    tree.root_node().has_error(),
                    "MySQL 5.7 should have errors parsing window functions"
                );
            }
        }

        // MySQL 8.0 should support window functions (OVER clause added)
        if let Some(lang) =
            language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
        {
            let mut parser = Parser::new();
            parser.set_language(lang).unwrap();
            let tree = parser.parse(window_function_sql, None);
            assert!(tree.is_some(), "MySQL 8.0 should parse window functions");
            if let Some(tree) = tree {
                // Should parse without errors for basic OVER clause
                assert!(
                    !tree.root_node().has_error(),
                    "MySQL 8.0 should parse window functions without errors"
                );
            }
        }
    }
}
