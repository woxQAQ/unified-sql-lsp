// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! YAML test definition parser
//!
//! Parses test case definitions from YAML files into structured Rust types.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::Position;

/// Test suite definition from YAML
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestSuite {
    /// Test suite name
    pub name: String,

    /// Database configuration
    pub database: DatabaseConfig,

    /// Test cases
    pub tests: Vec<TestCase>,
}

/// Database configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    /// Dialect: "mysql" or "postgresql"
    pub dialect: String,

    /// Connection string (overrides default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_string: Option<String>,

    /// Schema setup files
    #[serde(default)]
    pub schemas: Vec<String>,

    /// Data setup files
    #[serde(default)]
    pub data: Vec<String>,
}

/// Individual test case
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TestCase {
    /// Test case name/ID
    pub name: String,

    /// Test description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// SQL snippet with cursor marker
    pub sql: String,

    /// Cursor position (line, character) - if not specified, uses | marker in SQL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<CursorPos>,

    /// Expected completion results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_completion: Option<CompletionExpectation>,

    /// Expected diagnostics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_diagnostics: Option<DiagnosticsExpectation>,

    /// Expected hover result
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expect_hover: Option<HoverExpectation>,
}

/// Cursor position
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CursorPos {
    /// Line number (0-based)
    pub line: u32,

    /// Character position (0-based)
    pub character: u32,
}

/// Completion expectations
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CompletionExpectation {
    /// Items that MUST be present
    #[serde(default)]
    pub contains: Vec<String>,

    /// Items that MUST NOT be present
    #[serde(default)]
    pub not_contains: Vec<String>,

    /// Expected total count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,

    /// Expected order (first N items)
    #[serde(default)]
    pub order: Vec<String>,

    /// Minimum count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_count: Option<usize>,
}

/// Diagnostics expectations
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DiagnosticsExpectation {
    /// Expected error count
    pub error_count: usize,

    /// Expected warning count
    #[serde(default)]
    pub warning_count: usize,

    /// Expected error messages (substrings)
    #[serde(default)]
    pub error_messages: Vec<String>,
}

/// Hover expectations
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct HoverExpectation {
    /// Expected hover content (substring match)
    pub contains: String,

    /// Hover should contain markdown
    #[serde(default)]
    pub is_markdown: bool,
}

impl TestSuite {
    /// Parse test suite from YAML string
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        let suite: TestSuite = serde_yaml::from_str(yaml)?;
        Ok(suite)
    }

    /// Parse test suite from YAML file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let yaml = std::fs::read_to_string(path)?;
        Self::from_yaml(&yaml)
    }

    /// Extract cursor position from SQL with | marker
    pub fn extract_cursor(&self, test: &TestCase) -> Result<Position> {
        // If explicit cursor position provided, use it
        if let Some(cursor) = &test.cursor {
            return Ok(Position::new(cursor.line, cursor.character));
        }

        // Otherwise, find | marker in SQL
        let sql = &test.sql;
        let cursor_char = '|';

        let mut line = 0u32;
        let mut character = 0u32;
        let mut found = false;

        for ch in sql.chars() {
            if ch == cursor_char {
                found = true;
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        if !found {
            return Err(anyhow::anyhow!(
                "No cursor marker '|' found in SQL and no explicit cursor position provided"
            ));
        }

        Ok(Position::new(line, character))
    }

    /// Strip cursor marker from SQL
    pub fn strip_cursor_marker(&self, test: &TestCase) -> String {
        test.sql.replace('|', "")
    }
}
