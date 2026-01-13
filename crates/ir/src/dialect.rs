// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Dialect Support
//!
//! This module defines SQL dialects and their specific extensions.
//!
//! ## Design
//!
//! The dialect system is organized into two levels:
//!
//! 1. **Dialect Family**: High-level groupings (MySQL, PostgreSQL) that share common syntax
//! 2. **Specific Dialect**: Individual database implementations (MySQL, PostgreSQL, TiDB, MariaDB, CockroachDB)
//!
//! ## Dialect Families
//!
//! - **MySQL Family**: Includes MySQL, TiDB, and MariaDB
//!   - Shared syntax: `LIMIT offset, count`, backtick identifiers, `AUTO_INCREMENT`
//!   - TiDB extends MySQL with distributed SQL features
//!   - MariaDB adds its own extensions while maintaining MySQL compatibility
//!
//! - **PostgreSQL Family**: Includes PostgreSQL and CockroachDB
//!   - Shared syntax: `LIMIT count OFFSET offset`, dollar-quoted strings, `ARRAY` types
//!   - CockroachDB extends PostgreSQL with distributed SQL features
//!
//! ## Dialect Extensions
//!
//! Extensions represent syntax features that vary across dialects:
//!
//! - `LimitOffset`: LIMIT/OFFSET syntax (MySQL vs PostgreSQL style)
//! - `DistinctOn`: PostgreSQL's `DISTINCT ON` clause
//! - `LateralJoin`: PostgreSQL's `LATERAL` keyword for subqueries in FROM
//! - `WindowFunctions`: Window functions (`ROW_NUMBER()`, `RANK()`, etc.)
//! - `StraightJoin`: MySQL's `STRAIGHT_JOIN` hint for join order
//! - `MultiDelete`: MySQL's multi-table DELETE syntax
//! - `TiDBSnapshot`: TiDB's `TIDB_SNAPSHOT` for reading historical data
//! - `CTE`: Common Table Expressions (WITH clauses)
//! - `FullOuterJoin`: FULL OUTER JOIN support
//!
//! ## Version Support
//!
//! Each dialect supports multiple versions:
//! - **MySQL**: 5.7, 8.0+
//! - **PostgreSQL**: 12, 14, 15+
//! - **TiDB**: 5.0, 6.0, 7.0, 8.0
//! - **MariaDB**: 10.x, 11.x
//! - **CockroachDB**: 21.x, 22.x, 23.x
//!
//! Future implementations may add version-specific extension support using `semver`.

use serde::{Deserialize, Serialize};

/// Supported SQL dialects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Dialect {
    /// MySQL (5.7, 8.0)
    MySQL,
    /// PostgreSQL (12, 14, 15+)
    PostgreSQL,
    /// TiDB (5.0, 6.0, 7.0, 8.0)
    TiDB,
    /// MariaDB (10.x, 11.x)
    MariaDB,
    /// CockroachDB (21.x, 22.x, 23.x)
    CockroachDB,
}

impl Dialect {
    /// Returns the family this dialect belongs to
    pub fn family(&self) -> DialectFamily {
        match self {
            Dialect::MySQL | Dialect::TiDB | Dialect::MariaDB => DialectFamily::MySQL,
            Dialect::PostgreSQL | Dialect::CockroachDB => DialectFamily::PostgreSQL,
        }
    }

    /// Check if this dialect supports a specific extension
    pub fn supports(&self, ext: DialectExtensions) -> bool {
        // MySQL family extensions
        let mysql_family = matches!(
            ext,
            DialectExtensions::LimitOffset
                | DialectExtensions::MultiDelete
                | DialectExtensions::StraightJoin
        );

        // PostgreSQL family extensions
        let postgresql_family = matches!(
            ext,
            DialectExtensions::LimitOffset
                | DialectExtensions::DistinctOn
                | DialectExtensions::LateralJoin
                | DialectExtensions::WindowFunctions
        );

        match self {
            Dialect::MySQL => mysql_family,
            Dialect::PostgreSQL => postgresql_family,
            Dialect::TiDB => mysql_family || ext == DialectExtensions::TiDBSnapshot,
            Dialect::MariaDB => mysql_family,
            Dialect::CockroachDB => postgresql_family,
        }
    }
}

/// Dialect family groupings
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DialectFamily {
    MySQL,
    PostgreSQL,
}

/// Dialect-specific extensions and features
///
/// These represent syntax or features that are not part of the core SQL subset
/// and are specific to certain dialects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DialectExtensions {
    /// LIMIT ... OFFSET ... syntax (MySQL, PostgreSQL)
    LimitOffset,

    /// DISTINCT ON (PostgreSQL, CockroachDB)
    DistinctOn,

    /// LATERAL JOIN (PostgreSQL, CockroachDB)
    LateralJoin,

    /// Window functions (PostgreSQL, MySQL 8.0+)
    WindowFunctions,

    /// STRAIGHT_JOIN hint (MySQL family)
    StraightJoin,

    /// Multi-table DELETE (MySQL family)
    MultiDelete,

    /// TiDB-specific: TIDB_SNAPSHOT
    TiDBSnapshot,

    /// CTE (Common Table Expression) - WITH clauses
    CTE,

    /// FULL OUTER JOIN
    FullOuterJoin,
}

impl DialectExtensions {
    /// Check if this extension is part of the core SQL subset
    pub fn is_core(self) -> bool {
        matches!(
            self,
            DialectExtensions::CTE | DialectExtensions::WindowFunctions
        )
    }
}
