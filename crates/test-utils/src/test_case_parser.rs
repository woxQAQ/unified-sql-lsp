// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Text-based test case format parser
//!
//! This module provides a parser for simple text-based test case definitions.
//! Test cases are separated by `---` and use a YAML-like field syntax.

use std::fmt;
use std::path::Path;
use thiserror::Error;

/// A single test case definition
#[derive(Debug, Clone)]
pub struct TestCase {
    pub description: String,
    pub dialect: Dialect,
    pub context: Option<String>,
    pub input: String,
    pub expected: Vec<ExpectedItem>,
    pub options: Option<TestOptions>,
}

/// SQL dialect specification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dialect {
    MySQL,
    PostgreSQL,
    All,
}

/// Expected completion item
#[derive(Debug, Clone)]
pub enum ExpectedItem {
    /// Full format: label [kind] detail
    Full {
        label: String,
        kind: String,
        detail: String,
    },
    /// Simple format: just the name
    Simple(String),
}

/// Test validation options
#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    pub min_items: Option<usize>,
    pub contains: Option<Vec<String>>,
    pub exact_match: Option<bool>,
    pub require_schema: Option<bool>,
}

/// Parse errors
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Invalid dialect value: {0}")]
    InvalidDialect(String),

    #[error("Invalid syntax at line {line}: {message}")]
    InvalidSyntax { line: usize, message: String },

    #[error("Empty test case file")]
    EmptyFile,

    #[error("Invalid expected item format: {0}")]
    InvalidExpectedItem(String),
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dialect::MySQL => write!(f, "mysql"),
            Dialect::PostgreSQL => write!(f, "postgresql"),
            Dialect::All => write!(f, "all"),
        }
    }
}

impl std::str::FromStr for Dialect {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "mysql" => Ok(Dialect::MySQL),
            "postgresql" | "postgres" => Ok(Dialect::PostgreSQL),
            "all" => Ok(Dialect::All),
            _ => Err(ParseError::InvalidDialect(s.to_string())),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Dialect {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Parse a test case file
pub fn parse_test_file(path: &Path) -> Result<Vec<TestCase>, ParseError> {
    let content = std::fs::read_to_string(path)?;
    parse_test_content(&content)
}

/// Parse test case content from a string
pub fn parse_test_content(content: &str) -> Result<Vec<TestCase>, ParseError> {
    let mut cases = Vec::new();
    let mut current_case: TestCaseBuilder = TestCaseBuilder::default();
    let mut current_field: Option<String> = None;
    let mut current_value = Vec::new();
    let mut line_num = 0;

    for line in content.lines() {
        line_num += 1;
        let trimmed = line.trim();

        // Skip empty lines at the start
        if trimmed.is_empty() && current_field.is_none() {
            continue;
        }

        // Separator
        if trimmed == "---" {
            if current_case.has_fields() {
                cases.push(current_case.build(line_num)?);
                current_case = TestCaseBuilder::default();
            }
            current_field = None;
            current_value.clear();
            continue;
        }

        // Field declaration (e.g., "description:", "input: |")
        // Also handle "description: value" format (inline value)
        if let Some(colon_pos) = trimmed.find(':') {
            let key = trimmed[..colon_pos].trim();
            let value = trimmed[colon_pos + 1..].trim();

            // Save previous field
            if let Some(field) = &current_field {
                current_case.set_field(field, &current_value);
            }

            current_field = Some(key.to_string());
            current_value.clear();

            // If there's a value on the same line (and not just the multi-line marker "|")
            if !value.is_empty() && value != "|" {
                current_value.push(value.to_string());
            }
            continue;
        }

        // Multi-line value content
        if current_field.is_some() {
            current_value.push(line.to_string());
        }
    }

    // Save the last field value
    if let Some(field) = &current_field {
        current_case.set_field(field, &current_value);
    }

    // Last test case
    if current_case.has_fields() {
        cases.push(current_case.build(line_num)?);
    }

    if cases.is_empty() {
        return Err(ParseError::EmptyFile);
    }

    Ok(cases)
}

/// Builder for constructing test cases incrementally
#[derive(Default)]
struct TestCaseBuilder {
    description: Option<String>,
    dialect: Option<Dialect>,
    context: Option<String>,
    input: Option<String>,
    expected: Option<Vec<String>>,
    options: Option<Vec<String>>,
}

impl TestCaseBuilder {
    fn has_fields(&self) -> bool {
        self.description.is_some() || self.input.is_some()
    }

    fn set_field(&mut self, field: &str, value: &[String]) {
        match field {
            "description" => {
                self.description = Some(value.join("\n").trim().to_string());
            }
            "dialect" => {
                self.dialect = value.join("").trim().parse().ok();
            }
            "context" => {
                self.context = Some(value.join("\n").trim().to_string());
            }
            "input" => {
                self.input = Some(dedent(value));
            }
            "expected" => {
                let items: Vec<String> = value
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !items.is_empty() {
                    self.expected = Some(items);
                }
            }
            "options" => {
                let opts: Vec<String> = value
                    .iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !opts.is_empty() {
                    self.options = Some(opts);
                }
            }
            _ => {}
        }
    }

    fn build(self, line_num: usize) -> Result<TestCase, ParseError> {
        let description = self.description.ok_or(ParseError::InvalidSyntax {
            line: line_num,
            message: "missing description field".to_string(),
        })?;

        let dialect = self.dialect.unwrap_or(Dialect::All);

        let input = self.input.ok_or(ParseError::InvalidSyntax {
            line: line_num,
            message: "missing input field".to_string(),
        })?;

        let expected = self
            .expected
            .unwrap_or_default()
            .into_iter()
            .map(|s| parse_expected_item(&s))
            .collect::<Result<Vec<_>, _>>()?;

        let options = self.options.and_then(|opts| parse_options(&opts));

        Ok(TestCase {
            description,
            dialect,
            context: self.context,
            input,
            expected,
            options,
        })
    }
}

/// Remove common leading whitespace from multi-line text
fn dedent(lines: &[String]) -> String {
    if lines.is_empty() {
        return String::new();
    }

    // Find minimum indentation (ignoring empty lines)
    let min_indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);

    // Remove common indentation
    let result: Vec<String> = lines
        .iter()
        .map(|line| {
            if line.len() >= min_indent {
                &line[min_indent..]
            } else {
                line.as_str()
            }
            .to_string()
        })
        .collect();

    result.join("\n").trim().to_string()
}

/// Parse an expected item string
fn parse_expected_item(s: &str) -> Result<ExpectedItem, ParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseError::InvalidExpectedItem("empty item".to_string()));
    }

    // Check for full format: label [kind] detail
    if let Some(kind_start) = s.find('[')
        && let Some(kind_end) = s.find(']')
    {
        let label = s[..kind_start].trim().to_string();
        let kind = s[kind_start + 1..kind_end].to_string();
        let detail = s[kind_end + 1..].trim().to_string();

        if !label.is_empty() {
            return Ok(ExpectedItem::Full {
                label,
                kind,
                detail,
            });
        }
    }

    // Simple format: just the name
    Ok(ExpectedItem::Simple(s.to_string()))
}

/// Parse options from a list of strings
fn parse_options(opts: &[String]) -> Option<TestOptions> {
    let mut result = TestOptions::default();

    for opt in opts {
        let opt = opt.trim();
        if opt.is_empty() {
            continue;
        }

        // Parse key: value or key: value1, value2
        if let Some(colon_pos) = opt.find(':') {
            let key = &opt[..colon_pos];
            let value = opt[colon_pos + 1..].trim();

            match key.trim() {
                "min_items" => {
                    if let Ok(n) = value.parse() {
                        result.min_items = Some(n);
                    }
                }
                "contains" => {
                    result.contains = Some(
                        value
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect(),
                    );
                }
                "exact_match" => {
                    result.exact_match = Some(value.parse().unwrap_or(false));
                }
                "require_schema" => {
                    result.require_schema = Some(value.parse().unwrap_or(false));
                }
                _ => {}
            }
        }
    }

    if result.min_items.is_some()
        || result.contains.is_some()
        || result.exact_match.is_some()
        || result.require_schema.is_some()
    {
        Some(result)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_parse_simple_case() {
        let content = r#"
---
description: Test case
dialect: mysql
input: |
  SELECT | FROM users
expected: |
  id [Field] users.id
  name [Field] users.name
"#;

        let cases = parse_test_content(content).unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].description, "Test case");
        assert_eq!(cases[0].dialect, Dialect::MySQL);
        assert_eq!(cases[0].input, "SELECT | FROM users");
        assert_eq!(cases[0].expected.len(), 2);
    }

    #[test]
    fn test_parse_multiple_cases() {
        let content = r#"
---
description: First test
dialect: all
input: |
  SELECT | FROM users
expected: |
  id

---
description: Second test
dialect: postgresql
input: |
  SELECT * FROM |
expected: |
  users
  orders
"#;

        let cases = parse_test_content(content).unwrap();
        assert_eq!(cases.len(), 2);
        assert_eq!(cases[0].description, "First test");
        assert_eq!(cases[1].description, "Second test");
        assert_eq!(cases[1].dialect, Dialect::PostgreSQL);
    }

    #[test]
    fn test_parse_dialect() {
        assert_eq!(Dialect::from_str("mysql").unwrap(), Dialect::MySQL);
        assert_eq!(Dialect::from_str("MySQL").unwrap(), Dialect::MySQL);
        assert_eq!(
            Dialect::from_str("postgresql").unwrap(),
            Dialect::PostgreSQL
        );
        assert_eq!(Dialect::from_str("postgres").unwrap(), Dialect::PostgreSQL);
        assert_eq!(Dialect::from_str("all").unwrap(), Dialect::All);
        assert!(Dialect::from_str("invalid").is_err());
    }

    #[test]
    fn test_parse_expected_item() {
        let full = parse_expected_item("id [Field] users.id").unwrap();
        match full {
            ExpectedItem::Full {
                label,
                kind,
                detail,
            } => {
                assert_eq!(label, "id");
                assert_eq!(kind, "Field");
                assert_eq!(detail, "users.id");
            }
            _ => panic!("Expected Full item"),
        }

        let simple = parse_expected_item("users").unwrap();
        match simple {
            ExpectedItem::Simple(name) => {
                assert_eq!(name, "users");
            }
            _ => panic!("Expected Simple item"),
        }
    }

    #[test]
    fn test_dedent() {
        let lines = vec![
            "    SELECT".to_string(),
            "      id,".to_string(),
            "      name".to_string(),
        ];
        assert_eq!(dedent(&lines), "SELECT\n  id,\n  name");
    }

    #[test]
    fn test_parse_options() {
        let opts = vec![
            "min_items: 2".to_string(),
            "contains: id, name".to_string(),
            "exact_match: true".to_string(),
        ];
        let result = parse_options(&opts).unwrap();
        assert_eq!(result.min_items, Some(2));
        assert_eq!(
            result.contains,
            Some(vec!["id".to_string(), "name".to_string()])
        );
        assert_eq!(result.exact_match, Some(true));
    }

    #[test]
    fn test_empty_file() {
        let content = "";
        assert!(parse_test_content(content).is_err());
    }
}
