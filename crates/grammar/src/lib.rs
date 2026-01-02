//! Unified SQL Grammar
//!
//! This crate provides multi-dialect SQL grammar support using tree-sitter.
//!
//! ## Grammar Architecture
//!
//! The grammar is organized using a compile-time dialect selection strategy:
//!
//! - **`grammar.js`**: Main entry point that reads `DIALECT` environment variable
//! - **`dialect/base.js`**: Common SQL rules shared across all dialects
//! - **`dialect/mysql.js`**: MySQL-specific syntax (LIMIT, AUTO_INCREMENT, etc.)
//! - **`dialect/postgresql.js`**: PostgreSQL-specific syntax (DISTINCT ON, LATERAL, etc.)
//!
//! ## Build Process
//!
//! The build script (`build.rs`) compiles separate parsers for each dialect:
//!
//! 1. Sets `DIALECT` environment variable (e.g., `DIALECT=mysql`)
//! 2. Runs `tree-sitter generate --no-bindings` to generate `parser.c`
//! 3. Compiles `parser.c` to dialect-specific object files
//! 4. Repeats for each dialect (base, mysql, postgresql)
//!
//! Each dialect gets its own compiled parser, allowing tree-sitter to parse
//! SQL with dialect-specific syntax correctly.
//!
//! ## Supported Dialects
//!
//! - **MySQL**: MySQL 5.7, 8.0+
//! - **PostgreSQL**: PostgreSQL 12+
//! - **TiDB**: Inherits MySQL grammar (MySQL-compatible)
//! - **MariaDB**: Inherits MySQL grammar (MySQL-compatible)
//! - **CockroachDB**: Inherits PostgreSQL grammar (PostgreSQL-compatible)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use unified_sql_lsp_ir::Dialect;
//! use unified_sql_grammar::language_for_dialect;
//!
//! // Get the tree-sitter Language for a specific dialect
//! let mysql_lang = language_for_dialect(Dialect::MySQL).unwrap();
//! let tidb_lang = language_for_dialect(Dialect::TiDB).unwrap();
//! let postgresql_lang = language_for_dialect(Dialect::PostgreSQL).unwrap();
//!
//! // Use the language with a parser
//! let mut parser = tree_sitter::Parser::new();
//! parser.set_language(mysql_lang).unwrap();
//! let tree = parser.parse("SELECT * FROM users", None).unwrap();
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
/// - **MySQL family** → MySQL grammar (`parser-mysql.o`):
///   - `Dialect::MySQL` - Native MySQL support
///   - `Dialect::TiDB` - TiDB is MySQL-compatible
///   - `Dialect::MariaDB` - MariaDB is MySQL-compatible
///
/// - **PostgreSQL family** → PostgreSQL grammar (`parser-postgresql.o`):
///   - `Dialect::PostgreSQL` - Native PostgreSQL support
///   - `Dialect::CockroachDB` - CockroachDB is PostgreSQL-compatible
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
/// // Get MySQL grammar (also works for TiDB, MariaDB)
/// if let Some(lang) = language_for_dialect(Dialect::MySQL) {
///     let mut parser = tree_sitter::Parser::new();
///     parser.set_language(lang).unwrap();
///     let tree = parser.parse("SELECT * FROM users LIMIT 10", None);
/// }
/// ```
pub fn language_for_dialect(dialect: Dialect) -> Option<&'static tree_sitter::Language> {
    static MYSQL_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
    static POSTGRESQL_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();

    // Map IR dialects to compiled grammar implementations
    // Each grammar was compiled separately by build.rs with different DIALECT env vars
    match dialect {
        Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB => MYSQL_LANG
            .get_or_init(|| unsafe {
                // Safety: tree_sitter_unified_sql_mysql() returns a pointer to the
                // language object compiled by tree-sitter from src/grammar/dialect/mysql.js
                // The transmute converts the function pointer to a Language struct
                Some(std::mem::transmute::<_, tree_sitter::Language>(
                    tree_sitter_unified_sql_mysql()
                ))
            })
            .as_ref(),
        Dialect::PostgreSQL | Dialect::CockroachDB => POSTGRESQL_LANG
            .get_or_init(|| unsafe {
                // Safety: tree_sitter_unified_sql_postgresql() returns a pointer to the
                // language object compiled by tree-sitter from src/grammar/dialect/postgresql.js
                Some(std::mem::transmute::<_, tree_sitter::Language>(
                    tree_sitter_unified_sql_postgresql()
                ))
            })
            .as_ref(),
        _ => None, // Unsupported dialect
    }
}

// External functions from compiled grammars
// Rust 2024 edition requires extern blocks to be unsafe
unsafe extern "C" {
    fn tree_sitter_unified_sql_base() -> *const ();
    fn tree_sitter_unified_sql_mysql() -> *const ();
    fn tree_sitter_unified_sql_postgresql() -> *const ();
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
}
