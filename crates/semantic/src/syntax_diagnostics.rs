// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Syntax diagnostics analysis for tree-sitter ERROR nodes.

/// Syntax diagnostic range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxRange {
    pub start_line: u32,
    pub start_character: u32,
    pub end_line: u32,
    pub end_character: u32,
}

/// Syntax diagnostic result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    pub message: String,
    pub range: SyntaxRange,
}

/// Analyzer for syntax diagnostics based on tree-sitter ERROR nodes.
#[derive(Debug, Clone, Default)]
pub struct SyntaxDiagnosticAnalyzer;

impl SyntaxDiagnosticAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn collect_diagnostics(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
    ) -> Vec<SyntaxDiagnostic> {
        let mut diagnostics = Vec::new();
        let root = tree.root_node();
        let mut found_real_errors = false;

        if root.has_error() {
            self.collect_error_nodes_recursive(
                &root,
                source,
                &mut diagnostics,
                &mut found_real_errors,
                0,
            );
        }

        if diagnostics.is_empty() && root.has_error() && found_real_errors {
            diagnostics.push(SyntaxDiagnostic {
                message: "Syntax error in SQL statement".to_string(),
                range: SyntaxRange {
                    start_line: 0,
                    start_character: 0,
                    end_line: source.lines().count() as u32,
                    end_character: 0,
                },
            });
        }

        diagnostics
    }

    fn collect_error_nodes_recursive(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        diagnostics: &mut Vec<SyntaxDiagnostic>,
        found_real_errors: &mut bool,
        depth: usize,
    ) {
        if depth > 100 {
            return;
        }

        if node.kind() == "ERROR" {
            if self.should_ignore_error_node(node, source) {
                // Continue into children.
            } else {
                *found_real_errors = true;
                diagnostics.push(self.create_error_diagnostic(node, source));
                return;
            }
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.collect_error_nodes_recursive(
                    &cursor.node(),
                    source,
                    diagnostics,
                    found_real_errors,
                    depth + 1,
                );
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    pub fn should_ignore_error_node(&self, node: &tree_sitter::Node, source: &str) -> bool {
        let error_text = &source[node.byte_range()];
        let trimmed = error_text.trim();
        trimmed.is_empty() || trimmed.len() == 1
    }

    fn create_error_diagnostic(&self, node: &tree_sitter::Node, source: &str) -> SyntaxDiagnostic {
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        let error_text = &source[node.byte_range()];
        let message = self.enhance_error_message(node, source, error_text);

        SyntaxDiagnostic {
            message,
            range: SyntaxRange {
                start_line: start_pos.row as u32,
                start_character: start_pos.column as u32,
                end_line: end_pos.row as u32,
                end_character: end_pos.column as u32,
            },
        }
    }

    pub fn enhance_error_message(
        &self,
        error_node: &tree_sitter::Node,
        source: &str,
        error_text: &str,
    ) -> String {
        if let Some(suggestion) = self.analyze_common_patterns(error_node, source, error_text) {
            return suggestion;
        }

        if error_text.len() <= 50 {
            format!("Syntax error near '{}'", error_text)
        } else {
            "Syntax error in this region".to_string()
        }
    }

    pub fn analyze_common_patterns(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        error_text: &str,
    ) -> Option<String> {
        let trimmed = error_text.trim();

        if self.is_missing_comma_pattern(trimmed) {
            return Some(format!(
                "Syntax error: missing comma between identifiers. Suggestion: Add comma after '{}'",
                self.first_identifier(trimmed)
            ));
        }

        if self.is_missing_from_pattern(node, source) {
            return Some(
                "Syntax error: SELECT statement missing FROM clause. Expected: 'SELECT ... FROM table ...'"
                    .to_string(),
            );
        }

        if self.is_unmatched_paren_pattern(trimmed) {
            return Some(
                "Syntax error: unbalanced parentheses. Check opening/closing pairs".to_string(),
            );
        }

        None
    }

    pub fn is_missing_comma_pattern(&self, text: &str) -> bool {
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() < 2 {
            return false;
        }
        let has_identifiers = parts.iter().all(|p| self.is_identifier(p));
        let no_comma = !text.contains(',');
        has_identifiers && no_comma
    }

    pub fn is_missing_from_pattern(&self, node: &tree_sitter::Node, _source: &str) -> bool {
        let mut current = Some(*node);
        while let Some(n) = current {
            match n.kind() {
                "select_statement" | "select" => {
                    let mut cursor = n.walk();
                    if cursor.goto_first_child() {
                        loop {
                            if cursor.node().kind() == "from_clause" {
                                return false;
                            }
                            if !cursor.goto_next_sibling() {
                                break;
                            }
                        }
                    }
                    return true;
                }
                "from_clause" => return false,
                _ => {}
            }
            current = n.parent();
        }
        false
    }

    pub fn is_unmatched_paren_pattern(&self, text: &str) -> bool {
        let open_count = text.matches('(').count();
        let close_count = text.matches(')').count();
        open_count != close_count
    }

    pub fn is_identifier(&self, text: &str) -> bool {
        !text.is_empty()
            && (text
                .chars()
                .next()
                .expect("non-empty string guaranteed by guard")
                .is_alphabetic()
                || text.starts_with('_')
                || text.starts_with('\"')
                || text.starts_with('`'))
            && text
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '\"' || c == '`')
    }

    pub fn first_identifier(&self, text: &str) -> String {
        text.split_whitespace().next().unwrap_or("").to_string()
    }
}
