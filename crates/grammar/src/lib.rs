//! Unified SQL Grammar
//!
//! This crate provides multi-dialect SQL grammar support using tree-sitter.
//!
//! ## Supported Dialects
//!
//! - **base**: Core SQL grammar (common subset)
//! - **mysql**: MySQL-specific extensions
//! - **postgresql**: PostgreSQL-specific extensions
//!
//! ## Usage
//!
//! ```rust
//! use unified_sql_grammar::Dialect;
//!
//! // Get the tree-sitter language for a specific dialect
//! let mysql_lang = Dialect::MySQL.language();
//! let postgresql_lang = Dialect::PostgreSQL.language();
//! ```

use std::sync::OnceLock;

/// SQL dialect identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    /// Base SQL (common subset)
    Base,
    /// MySQL dialect
    MySQL,
    /// PostgreSQL dialect
    PostgreSQL,
}

impl Dialect {
    /// Get all supported dialects
    pub fn all() -> &'static [Dialect] {
        &[Dialect::Base, Dialect::MySQL, Dialect::PostgreSQL]
    }

    /// Get dialect name as string
    pub fn name(&self) -> &'static str {
        match self {
            Dialect::Base => "base",
            Dialect::MySQL => "mysql",
            Dialect::PostgreSQL => "postgresql",
        }
    }

    /// Parse dialect from string
    pub fn from_str(s: &str) -> Option<Dialect> {
        match s.to_lowercase().as_str() {
            "base" => Some(Dialect::Base),
            "mysql" => Some(Dialect::MySQL),
            "postgresql" | "postgres" => Some(Dialect::PostgreSQL),
            _ => None,
        }
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Get the tree-sitter Language for a specific dialect
///
/// Note: This requires the tree-sitter feature and compiled grammars.
pub fn language_for_dialect(dialect: Dialect) -> Option<&'static tree_sitter::Language> {
    static BASE_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
    static MYSQL_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();
    static POSTGRESQL_LANG: OnceLock<Option<tree_sitter::Language>> = OnceLock::new();

    match dialect {
        Dialect::Base => BASE_LANG
            .get_or_init(|| unsafe {
                tree_sitter::Language::from_raw(tree_sitter_unified_sql_base())
            })
            .as_ref(),
        Dialect::MySQL => MYSQL_LANG
            .get_or_init(|| unsafe {
                tree_sitter::Language::from_raw(tree_sitter_unified_sql_mysql())
            })
            .as_ref(),
        Dialect::PostgreSQL => POSTGRESQL_LANG
            .get_or_init(|| unsafe {
                tree_sitter::Language::from_raw(tree_sitter_unified_sql_postgresql())
            })
            .as_ref(),
    }
}

// External functions from compiled grammars
extern "C" {
    fn tree_sitter_unified_sql_base() -> *const ();
    fn tree_sitter_unified_sql_mysql() -> *const ();
    fn tree_sitter_unified_sql_postgresql() -> *const ();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialect_from_str() {
        assert_eq!(Dialect::from_str("mysql"), Some(Dialect::MySQL));
        assert_eq!(Dialect::from_str("MySQL"), Some(Dialect::MySQL));
        assert_eq!(Dialect::from_str("postgresql"), Some(Dialect::PostgreSQL));
        assert_eq!(Dialect::from_str("postgres"), Some(Dialect::PostgreSQL));
        assert_eq!(Dialect::from_str("base"), Some(Dialect::Base));
        assert_eq!(Dialect::from_str("invalid"), None);
    }

    #[test]
    fn test_dialect_display() {
        assert_eq!(Dialect::MySQL.to_string(), "mysql");
        assert_eq!(Dialect::PostgreSQL.to_string(), "postgresql");
        assert_eq!(Dialect::Base.to_string(), "base");
    }
}
