// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! # Completion context detection
//!
//! This module provides context detection for SQL completion.
//! It analyzes the tree-sitter CST to determine what kind of
//! completion should be provided based on cursor position.

use tracing::debug;

use crate::cst_utils::{
    Position, extract_identifier_name, find_node_at_position, position_to_byte_offset,
};
use tree_sitter::Node;

/// Parts of a window function specification (OVER clause)
#[derive(Debug, Clone, PartialEq)]
pub enum WindowFunctionPart {
    /// Inside OVER ( at the beginning
    OverStart,
    /// Inside PARTITION BY
    PartitionBy,
    /// Inside ORDER BY (within OVER clause)
    OrderBy,
    /// Inside window frame specification
    WindowFrame,
}

/// Completion context types
///
/// Represents different SQL contexts where completion can be triggered.
#[derive(Debug, Clone, PartialEq)]
pub enum CompletionContext {
    /// SELECT clause projection
    ///
    /// User is typing in the SELECT projection list, e.g., `SELECT id, | FROM users`
    SelectProjection {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users." if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// FROM clause
    ///
    /// User is typing in the FROM clause, e.g., `SELECT * FROM |`
    FromClause {
        /// Tables to exclude from completion (already in FROM clause)
        exclude_tables: Vec<String>,
    },

    /// WHERE clause
    ///
    /// User is typing in the WHERE clause, e.g., `SELECT * FROM users WHERE |`
    WhereClause {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users" if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// JOIN ON condition
    ///
    /// User is typing in the JOIN ON condition, e.g., `SELECT * FROM users JOIN orders ON |`
    JoinCondition {
        /// Left table in the join
        left_table: Option<String>,
        /// Right table in the join
        right_table: Option<String>,
        /// Qualifier (table alias) if user typed one, e.g., "o" in "ON o.|"
        qualifier: Option<String>,
    },

    /// ORDER BY clause
    ///
    /// User is typing in the ORDER BY clause, e.g., `SELECT * FROM users ORDER BY |`
    OrderByClause {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users." if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// GROUP BY clause
    ///
    /// User is typing in the GROUP BY clause, e.g., `SELECT * FROM users GROUP BY |`
    GroupByClause {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users." if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// LIMIT clause
    ///
    /// User is typing in the LIMIT clause, e.g., `SELECT * FROM users LIMIT |`
    LimitClause,

    /// HAVING clause
    ///
    /// User is typing in the HAVING clause, e.g., `SELECT * FROM users GROUP BY col HAVING |`
    HavingClause {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Optional table qualifier (e.g., "users." if cursor is after "users.")
        qualifier: Option<String>,
    },

    /// CTE (Common Table Expression) definition
    ///
    /// User is typing in a WITH clause, defining a CTE
    CteDefinition {
        /// Tables that can be used to create CTEs from
        available_tables: Vec<String>,
        /// CTEs that have already been defined in this WITH clause
        defined_ctes: Vec<String>,
    },

    /// Window function specification (OVER clause)
    ///
    /// User is typing in an OVER clause, e.g., `SELECT ROW_NUMBER() OVER (|`
    WindowFunctionClause {
        /// Tables visible in this scope
        tables: Vec<String>,
        /// Which part of the OVER clause we're in
        window_part: WindowFunctionPart,
    },

    /// Keyword completion
    ///
    /// User is typing at a position where SQL keywords are appropriate
    Keywords {
        /// The type of statement being typed
        statement_type: Option<String>,
        /// Existing clauses that should not be suggested again
        existing_clauses: Vec<String>,
    },

    /// Unknown context
    ///
    /// Cursor is in a position that doesn't match known completion contexts
    Unknown,
}

impl CompletionContext {
    /// Check if this is a SELECT projection context
    pub fn is_select_projection(&self) -> bool {
        matches!(self, CompletionContext::SelectProjection { .. })
    }

    /// Check if this is a FROM clause context
    pub fn is_from_clause(&self) -> bool {
        matches!(self, CompletionContext::FromClause { .. })
    }

    /// Check if this is a WHERE clause context
    pub fn is_where_clause(&self) -> bool {
        matches!(self, CompletionContext::WhereClause { .. })
    }

    /// Check if this is a JOIN ON condition context
    pub fn is_join_condition(&self) -> bool {
        matches!(self, CompletionContext::JoinCondition { .. })
    }

    /// Check if this is a keyword completion context
    pub fn is_keywords(&self) -> bool {
        matches!(self, CompletionContext::Keywords { .. })
    }
}

/// Detect the completion context based on cursor position
///
/// # Arguments
///
/// * `root` - Root node of the parsed tree
/// * `position` - Cursor position (line, character)
/// * `source` - Source code text
///
/// # Returns
///
/// The detected completion context
///
/// # Examples
///
/// ```no_run
/// use unified_sql_lsp_context::{Position, detect_completion_context};
/// use tree_sitter::Parser;
/// use unified_sql_grammar::language_for_dialect;
/// use unified_sql_lsp_ir::Dialect;
///
/// let source = "SELECT id FROM users";
/// let mut parser = Parser::new();
/// let lang = language_for_dialect(Dialect::MySQL).unwrap();
/// parser.set_language(&lang).unwrap();
/// let tree = parser.parse(source, None).unwrap();
/// let ctx = detect_completion_context(
///     &tree.root_node(),
///     Position::new(0, 10),
///     source
/// );
/// # let _ = ctx;
/// ```
pub fn detect_completion_context(
    root: &Node,
    position: Position,
    source: &str,
) -> CompletionContext {
    // Find the node at the cursor position
    let node = match find_node_at_position(root, position, source) {
        Some(n) => n,
        None => {
            eprintln!("!!! No node found at position, using text-based detection");
            return detect_context_from_text(source, position);
        }
    };

    eprintln!(
        "!!! Found node '{}' at position {:?}",
        node.kind(),
        position
    );
    eprintln!("!!! Node byte range: {:?}", node.byte_range());

    // Walk up the parent chain to find the context
    let mut current = Some(node);
    let mut qualifier = None;
    let mut depth = 0;

    while let Some(n) = current {
        eprintln!("!!! Parent[{}]: '{}'", depth, n.kind());
        depth += 1;
        match n.kind() {
            // Check if we're after a table qualifier (e.g., "users.")
            "table_reference" => {
                // Check if cursor is after a dot in a qualified reference
                if let Some(q) = extract_qualifier(&n, source, position) {
                    qualifier = Some(q);
                }
            }

            // SELECT clause
            "select_statement" => {
                eprintln!("!!! Found select_statement node");
                // Check if we're in the projection list
                if is_in_projection(&n, position) {
                    eprintln!("!!! is_in_projection returned TRUE");
                    // Extract table names from FROM clause
                    let mut tables = extract_tables_from_from_clause(&n, source);

                    // If CST extraction failed (incomplete SQL), use text-based fallback
                    if tables.is_empty() {
                        debug!(
                            "!!! LSP: CST extraction returned empty tables, using text-based extraction"
                        );
                        tables = extract_tables_from_source(source);
                    }

                    // Filter out SQL keywords that might have been incorrectly extracted
                    tables.retain(|t| {
                        !matches!(
                            t.as_str(),
                            "SELECT" | "FROM" | "WHERE" | "JOIN" | "ON" | "AND" | "OR" | "*"
                        )
                    });

                    // Check if we're inside an OVER clause (window function)
                    // This should be checked before returning SelectProjection
                    let byte_offset = position_to_byte_offset(source, position);
                    let text_before = &source[..byte_offset.min(source.len())];
                    let text_upper = text_before.to_uppercase();

                    // Check for OVER clause patterns
                    let over_pos = text_upper
                        .rfind("OVER (")
                        .or_else(|| text_upper.rfind("OVER("));
                    if let Some(over_pos) = over_pos {
                        // We're inside an OVER clause
                        let after_over = &text_upper[over_pos..];

                        // Count parentheses to check if we're inside the OVER clause
                        let open_parens = after_over.matches('(').count();
                        let close_parens = after_over.matches(')').count();

                        if open_parens > close_parens {
                            // We're inside the OVER clause
                            // Check which part we're in
                            if after_over.to_uppercase().contains("PARTITION BY") {
                                // Check if cursor is after ORDER BY
                                if let Some(order_pos) = after_over.to_uppercase().rfind("ORDER BY")
                                {
                                    let after_order = &after_over[order_pos + 8..];
                                    if !after_order.contains(')') {
                                        return CompletionContext::WindowFunctionClause {
                                            tables,
                                            window_part: WindowFunctionPart::OrderBy,
                                        };
                                    }
                                }
                                // We're in PARTITION BY
                                return CompletionContext::WindowFunctionClause {
                                    tables,
                                    window_part: WindowFunctionPart::PartitionBy,
                                };
                            } else if after_over.to_uppercase().contains("ORDER BY") {
                                return CompletionContext::WindowFunctionClause {
                                    tables,
                                    window_part: WindowFunctionPart::OrderBy,
                                };
                            }

                            // Just after OVER (
                            return CompletionContext::WindowFunctionClause {
                                tables,
                                window_part: WindowFunctionPart::OverStart,
                            };
                        }
                    }

                    // If qualifier is still None, try text-based extraction
                    let qualifier = if qualifier.is_none() {
                        extract_table_qualifier_from_position(source, position)
                    } else {
                        qualifier
                    };

                    return CompletionContext::SelectProjection { tables, qualifier };
                }

                // Additional check: if we're right after SELECT keyword (even with no projection node),
                // treat it as projection context
                // This handles cases like "SELECT | FROM table" where the projection list is empty
                eprintln!("!!! select_statement: checking is_right_after_select_keyword");
                if is_right_after_select_keyword(&n, source, position) {
                    eprintln!("!!! select_statement: is_right_after_select_keyword returned TRUE");
                    // For subqueries, use text-based extraction to get the table name
                    // because CST parsing might be incomplete for nested queries
                    let text_before =
                        if let Ok(offset) = position_to_byte_offset_checked(source, position) {
                            &source[..offset.min(source.len())]
                        } else {
                            ""
                        };

                    // Check if we're in a subquery (parentheses)
                    let is_subquery =
                        text_before.matches('(').count() > text_before.matches(')').count();

                    let tables = if is_subquery {
                        // Extract table from text: look for the last FROM in the subquery
                        extract_tables_from_subquery_text(source, position)
                    } else {
                        // For normal queries, use CST extraction
                        let mut tables = extract_tables_from_from_clause(&n, source);

                        // If CST extraction failed (incomplete SQL), use text-based fallback
                        if tables.is_empty() {
                            tables = extract_tables_from_source(source);
                        }

                        // Filter out SQL keywords that might have been incorrectly extracted
                        tables.retain(|t| {
                            !matches!(
                                t.as_str(),
                                "SELECT" | "FROM" | "WHERE" | "JOIN" | "ON" | "AND" | "OR" | "*"
                            )
                        });

                        tables
                    };

                    // If qualifier is still None, try text-based extraction
                    let qualifier = if qualifier.is_none() {
                        extract_table_qualifier_from_position(source, position)
                    } else {
                        qualifier
                    };

                    return CompletionContext::SelectProjection { tables, qualifier };
                }
            }

            // FROM clause
            "from_clause" => {
                // Check if we're actually inside an OVER clause (window function)
                // tree-sitter may incorrectly parse incomplete OVER clauses as from_clause
                let byte_offset = position_to_byte_offset(source, position);
                let text_before = &source[..byte_offset.min(source.len())];
                let text_upper = text_before.to_uppercase();

                // Check for OVER clause patterns before returning FromClause
                let over_pos = text_upper
                    .rfind("OVER (")
                    .or_else(|| text_upper.rfind("OVER("));
                if let Some(over_pos) = over_pos {
                    // We're inside an OVER clause, which takes precedence
                    let after_over = &text_upper[over_pos..];

                    // Count parentheses to check if we're inside the OVER clause
                    let open_parens = after_over.matches('(').count();
                    let close_parens = after_over.matches(')').count();

                    if open_parens > close_parens {
                        // We're inside the OVER clause
                        // Extract tables from source
                        let tables = extract_tables_from_source(source);

                        // Check which part we're in
                        if after_over.to_uppercase().contains("PARTITION BY") {
                            // Check if cursor is after ORDER BY
                            if let Some(order_pos) = after_over.to_uppercase().rfind("ORDER BY") {
                                let after_order = &after_over[order_pos + 8..];
                                if !after_order.contains(')') {
                                    return CompletionContext::WindowFunctionClause {
                                        tables,
                                        window_part: WindowFunctionPart::OrderBy,
                                    };
                                }
                            }
                            // We're in PARTITION BY
                            return CompletionContext::WindowFunctionClause {
                                tables,
                                window_part: WindowFunctionPart::PartitionBy,
                            };
                        } else if after_over.to_uppercase().contains("ORDER BY") {
                            return CompletionContext::WindowFunctionClause {
                                tables,
                                window_part: WindowFunctionPart::OrderBy,
                            };
                        }

                        // Just after OVER (
                        return CompletionContext::WindowFunctionClause {
                            tables,
                            window_part: WindowFunctionPart::OverStart,
                        };
                    }
                }

                // Check if the cursor is actually within this from_clause's range
                // tree-sitter may incorrectly mark subquery content as from_clause
                // when parsing incomplete SQL
                let node_start = n.start_position();
                let _node_end = n.end_position();

                // Check if cursor is actually within this from_clause node
                // If cursor is before the node starts, it's in a subquery projection (e.g., "(SELECT | FROM ...)")
                let cursor_line = position.line as usize;
                let cursor_char = position.character as usize;
                let node_line = node_start.row;
                let node_char = node_start.column;

                if cursor_line < node_line || (cursor_line == node_line && cursor_char < node_char)
                {
                    // Cursor is before from_clause, likely in subquery projection
                    return detect_context_from_text(source, position);
                }

                // Extract tables from FROM clause for exclusion in JOIN contexts
                let exclude_tables = extract_tables_from_from_clause_node(&n, source);
                return CompletionContext::FromClause { exclude_tables };
            }

            // WHERE clause
            "where_clause" => {
                let tables = extract_tables_from_source(source);
                return CompletionContext::WhereClause { tables, qualifier };
            }

            // CTE (Common Table Expression)
            "common_table_expression" | "cte" => {
                // Check if cursor is in CTE name position (after WITH, before AS)
                let byte_offset = position_to_byte_offset(source, position);
                let text_before = &source[..byte_offset.min(source.len())];
                let text_upper = text_before.to_uppercase();

                // Pattern: "WITH cte_name | AS" or "WITH | AS"
                if text_upper.ends_with("WITH ")
                    || text_upper.ends_with("WITH,")
                    || text_upper.ends_with("WITH\t")
                {
                    // At CTE name position, suggest available tables
                    let available_tables = extract_tables_from_source(source);
                    return CompletionContext::CteDefinition {
                        available_tables,
                        defined_ctes: vec![],
                    };
                }

                // Pattern: "WITH cte_name AS (SELECT |" or inside CTE subquery
                if text_upper.contains(" AS (") {
                    // Check if cursor is after SELECT
                    if let Some(select_pos) = text_upper.rfind("SELECT ") {
                        let after_select = &text_upper[select_pos + 7..];
                        // Count parentheses to check if we're inside CTE definition
                        let open_parens = text_before.matches('(').count();
                        let close_parens = text_before.matches(')').count();

                        if open_parens > close_parens && !after_select.contains("FROM ") {
                            // We're inside CTE subquery projection
                            let tables = extract_tables_from_source(source);
                            return CompletionContext::SelectProjection { tables, qualifier };
                        }
                    }
                }

                // Fall back to text-based detection
                return detect_context_from_text(source, position);
            }

            // Window function (OVER clause)
            "window_specification" | "window_definition" => {
                debug!("!!! LSP CST: Found window_specification/window_definition node");
                let tables = extract_tables_from_source(source);

                // Check which part of the OVER clause we're in
                let byte_offset = position_to_byte_offset(source, position);
                let text_before = &source[..byte_offset.min(source.len())];
                let text_upper = text_before.to_uppercase();

                debug!("!!! LSP CST: text_before='{}'", text_before);
                debug!("!!! LSP CST: tables={:?}", tables);

                if text_upper.ends_with("OVER (") || text_upper.ends_with("OVER( ") {
                    debug!("!!! LSP CST: Detected OVER clause start");
                    return CompletionContext::WindowFunctionClause {
                        tables,
                        window_part: WindowFunctionPart::OverStart,
                    };
                }

                if text_upper.ends_with("PARTITION BY ") || text_upper.ends_with("PARTITION BY") {
                    debug!("!!! LSP CST: Detected PARTITION BY");
                    return CompletionContext::WindowFunctionClause {
                        tables,
                        window_part: WindowFunctionPart::PartitionBy,
                    };
                }

                if text_upper.ends_with("ORDER BY ") || text_upper.ends_with("ORDER BY") {
                    debug!("!!! LSP CST: Detected ORDER BY in window function");
                    return CompletionContext::WindowFunctionClause {
                        tables,
                        window_part: WindowFunctionPart::OrderBy,
                    };
                }

                debug!(
                    "!!! LSP CST: No specific pattern matched, falling back to text-based detection"
                );
                // Default to text-based detection for more complex cases
                return detect_context_from_text(source, position);
            }

            // JOIN clause
            "join_clause" => {
                // Check if cursor is inside a subquery in the JOIN
                // For example: "JOIN (SELECT | FROM orders)" should detect the SELECT projection
                let byte_offset = position_to_byte_offset(source, position);
                let text_before_cursor = &source[..byte_offset.min(source.len())];
                let open_parens = text_before_cursor.matches('(').count();
                let close_parens = text_before_cursor.matches(')').count();

                if open_parens > close_parens {
                    // Check if we're right after SELECT in the subquery
                    let text_before_upper = text_before_cursor.to_uppercase();

                    if text_before_upper.ends_with("SELECT ")
                        || text_before_upper.ends_with("SELECT\t")
                        || text_before_upper.ends_with("SELECT(")
                        || text_before_upper.trim_end().ends_with("(SELECT")
                    {
                        // Extract table from the subquery's FROM clause
                        // Find the last SELECT, then find FROM after it
                        let source_upper = source.to_uppercase();
                        let tables = if let Some(last_select_pos) = source_upper.rfind("SELECT") {
                            let after_select = &source_upper[last_select_pos + 6..]; // +6 for "SELECT"
                            if let Some(from_pos) = after_select.find("FROM") {
                                let from_absolute = last_select_pos + 6 + from_pos;
                                let after_from = &source[from_absolute + 4..];
                                let after_from_trimmed = after_from.trim_start();
                                if let Some(table_end) = after_from_trimmed.find([' ', ')', ';']) {
                                    let table_name =
                                        after_from_trimmed[..table_end].trim().to_string();
                                    vec![table_name]
                                } else {
                                    vec![]
                                }
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        };

                        return CompletionContext::SelectProjection {
                            tables,
                            qualifier: None,
                        };
                    }

                    // Not right after SELECT, fall back to text-based detection
                    return detect_context_from_text(source, position);
                }

                // Extract left and right table names from the join
                let (left_table, right_table) = extract_join_tables(&n, source);

                // If we don't have the right table yet, user is typing table name after JOIN
                // Return FromClause context to provide table completion
                if right_table.is_none() {
                    // Extract tables from FROM clause for exclusion
                    let exclude_tables = extract_tables_from_join_parent(&n, source);
                    return CompletionContext::FromClause { exclude_tables };
                }

                // Otherwise, user is in the ON clause - return JoinCondition for column completion
                return CompletionContext::JoinCondition {
                    left_table,
                    right_table,
                    qualifier: None,
                };
            }

            _ => {}
        }

        current = n.parent();
    }

    // Check if we only got to source_file level
    // This might happen when CST parsing is incomplete (e.g., in subqueries)
    // In such cases, fall back to text-based detection
    if node.kind() == "source_file" {
        debug!(
            "!!! LSP: Only found source_file node, CST may be incomplete, falling back to text-based detection"
        );
        return detect_context_from_text(source, position);
    }

    // If no specific completion context was found from CST, use text-based fallback
    // This handles incomplete SQL where tree-sitter doesn't create proper nodes
    detect_context_from_text(source, position)
}

/// Detect completion context from text when CST parsing fails
///
/// This is a fallback for incomplete SQL where tree-sitter doesn't create
/// the expected CST nodes. It analyzes the text before the cursor to determine
/// what kind of completion would be appropriate.
fn detect_context_from_text(source: &str, position: Position) -> CompletionContext {
    debug!("!!! LSP: >>>>> detect_context_from_text called");
    debug!("!!! LSP: position = {:?}", position);
    debug!("!!! LSP: source.len() = {}", source.len());

    // Get the byte position of the cursor
    let byte_offset = position_to_byte_offset(source, position);
    debug!("!!! LSP: byte_offset = {}", byte_offset);

    // Get text before cursor (handle out of bounds)
    let text_before = if byte_offset <= source.len() {
        &source[..byte_offset]
    } else {
        source
    };

    debug!("!!! LSP: Text before cursor: '{}'", text_before);
    debug!("!!! LSP: Text before cursor length: {}", text_before.len());
    debug!("!!! LSP: XXXXX About to call detect_from_or_join_context");

    // Check for specific patterns
    // Note: We pass both full source and text_before because different
    // detection functions need different views

    // Pattern 0.1: UPDATE/INSERT/DELETE keyword completion
    // Must check before FROM/JOIN patterns to avoid wrong detection
    let trimmed = text_before.trim_end();
    if trimmed.ends_with("UPDATE ") || trimmed == "UPDATE" {
        debug!("!!! LSP: Detected UPDATE - expecting SET keyword");
        // For UPDATE, suggest SET keyword, not table names
        return CompletionContext::Keywords {
            statement_type: Some("UPDATE".to_string()),
            existing_clauses: vec![],
        };
    }

    if trimmed.ends_with("INSERT ") || trimmed == "INSERT" {
        debug!("!!! LSP: Detected INSERT - expecting INTO keyword");
        return CompletionContext::Keywords {
            statement_type: Some("INSERT".to_string()),
            existing_clauses: vec![],
        };
    }

    if trimmed.ends_with("DELETE ") || trimmed == "DELETE" {
        debug!("!!! LSP: Detected DELETE - expecting FROM keyword");
        return CompletionContext::Keywords {
            statement_type: Some("DELETE".to_string()),
            existing_clauses: vec![],
        };
    }

    // Pattern 0.5: UNION set operations (check early before FROM/JOIN patterns)
    let trimmed = text_before.trim_end();
    debug!(
        "!!! LSP: Pattern 0.5: Checking for UNION, trimmed='{}'",
        trimmed
    );
    if trimmed.ends_with("UNION ") || trimmed == "UNION" || trimmed.ends_with("UNION") {
        debug!("!!! LSP: Detected UNION - expecting ALL or SELECT");
        return CompletionContext::Keywords {
            statement_type: Some("UNION".to_string()),
            existing_clauses: vec![],
        };
    }

    // Pattern 0.6: CTE (WITH clause) - check before other patterns
    debug!("!!! LSP: Pattern 0.6: Checking for CTE/WITH clause");
    if let Some(ctx) = detect_cte_context(source, text_before) {
        debug!("!!! LSP: detect_cte_context returned Some(ctx)");
        return ctx;
    }

    // Pattern 0.7: Window function (OVER clause) - check before other patterns
    debug!("!!! LSP: Pattern 0.7: Checking for window function OVER clause");
    if let Some(ctx) = detect_window_function_context(source, text_before) {
        debug!("!!! LSP: detect_window_function_context returned Some(ctx)");
        return ctx;
    }

    // Pattern 1: "SELECT ... FROM |" or "SELECT ... FROM ... JOIN |"
    // Suggest table names
    if let Some(ctx) = detect_from_or_join_context(source, text_before) {
        debug!("!!! LSP: detect_from_or_join_context returned Some(ctx)");
        return ctx;
    }

    debug!("!!! LSP: detect_from_or_join_context returned None");

    // Pattern 2: "SELECT |" or "SELECT id, |"
    // Suggest columns (projection)
    if let Some(ctx) = detect_projection_context(source, text_before) {
        return ctx;
    }

    // Pattern 3: "SELECT ... WHERE |"
    // Suggest columns/expressions
    if let Some(ctx) = detect_where_context(source, text_before) {
        return ctx;
    }

    // Pattern 4: "SELECT ... FROM ... JOIN ... ON |" or "JOIN ... USING (|"
    // Suggest columns for JOIN condition
    if let Some(ctx) = detect_join_using_context(source, text_before) {
        return ctx;
    }
    if let Some(ctx) = detect_join_on_context(source, text_before) {
        return ctx;
    }

    // Pattern 5: "SELECT ... ORDER BY |"
    // Suggest columns and sort directions
    if let Some(ctx) = detect_order_by_context(source, text_before) {
        return ctx;
    }

    // Pattern 6: "SELECT ... GROUP BY |"
    // Suggest columns and HAVING
    if let Some(ctx) = detect_group_by_context(source, text_before) {
        return ctx;
    }

    // Pattern 7: "SELECT ... LIMIT |"
    // Suggest numbers and OFFSET
    if let Some(ctx) = detect_limit_context(text_before) {
        return ctx;
    }

    // Pattern 8: "SELECT ... GROUP BY ... HAVING |"
    // Suggest columns and aggregations
    if let Some(ctx) = detect_having_context(source, text_before) {
        return ctx;
    }

    // Pattern 9: DDL statements (CREATE, ALTER, DROP)
    debug!("!!! LSP: About to check DDL context");
    if let Some(ctx) = detect_ddl_context(text_before) {
        return ctx;
    }

    // Default: if we couldn't detect a specific context and text is empty or very short,
    // assume keyword completion
    debug!(
        "!!! LSP: Checking if keyword context: text_before.len()={}",
        text_before.len()
    );
    if text_before.trim().is_empty() || text_before.len() < 3 {
        // At the beginning of a statement - suggest statement keywords
        debug!("!!! LSP: Detected keyword context (beginning of statement)");
        CompletionContext::Keywords {
            statement_type: None,
            existing_clauses: vec![],
        }
    } else {
        debug!("!!! LSP: No specific context detected, returning Unknown");
        CompletionContext::Unknown
    }
}

/// Extract real table names (not aliases) from source SQL
/// This is used for exclusion lists where we need the actual table names
fn extract_real_table_names_from_source(source: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let source_upper = source.to_uppercase();

    // Find FROM clause
    if let Some(from_pos) = source_upper.find("FROM") {
        // Get text after FROM
        let after_from = &source[from_pos + 4..]; // +4 to skip "FROM"

        // Split by WHERE, GROUP BY, ORDER BY, LIMIT, ON to get the FROM clause part
        let from_clause_end_keywords = [" WHERE ", " GROUP BY ", " ORDER BY ", " LIMIT ", " ON "];
        let from_part = after_from;
        let mut end_pos = from_part.len();

        for keyword in &from_clause_end_keywords {
            if let Some(pos) = from_part.to_uppercase().find(keyword)
                && pos < end_pos
            {
                end_pos = pos;
            }
        }

        let from_clause = &from_part[..end_pos];
        debug!(
            "!!! LSP: extract_real_table_names_from_source: from_clause='{}'",
            from_clause
        );

        // Strip trailing semicolons and other statement terminators
        let from_clause = from_clause
            .trim()
            .trim_end_matches(';')
            .trim_end_matches(';')
            .trim();

        debug!(
            "!!! LSP: extract_real_table_names_from_source: from_clause after stripping='{}'",
            from_clause
        );

        // Check if this is a comma-style join (FROM table1, table2, ...)
        let has_commas = from_clause.contains(',');

        // Extract real table names (not aliases)
        let words: Vec<&str> = from_clause
            .split([' ', '\n', '\t', ','])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        debug!(
            "!!! LSP: extract_real_table_names_from_source: words={:?}, has_commas={}",
            words, has_commas
        );

        let mut i = 0;
        while i < words.len() {
            let word = words[i].to_uppercase();
            debug!(
                "!!! LSP: extract_real_table_names_from_source: i={}, word='{}'",
                i, word
            );

            // Handle JOIN keyword - skip it and process the next table
            if word == "JOIN"
                || matches!(
                    word.as_str(),
                    "INNER" | "LEFT" | "RIGHT" | "FULL" | "CROSS" | "STRAIGHT" | "STRAIGHT_JOIN"
                )
            {
                i += 1;
                continue;
            }

            // Skip AS keyword
            if word == "AS" {
                i += 2; // Skip AS and the alias
                continue;
            }

            // Skip other keywords
            if matches!(
                word.as_str(),
                "ON" | "AND" | "OR" | "WHERE" | "GROUP" | "ORDER" | "LIMIT"
            ) {
                i += 1;
                continue;
            }

            // This is likely a table name - add it
            tables.push(words[i].to_string());
            i += 1;

            // Check if next word is an alias - if so, skip it
            // But not if it's a comma-style join (has_commas = true)
            if !has_commas && i < words.len() {
                let next_word = words[i].to_uppercase();
                if !matches!(
                    next_word.as_str(),
                    "JOIN"
                        | "INNER"
                        | "LEFT"
                        | "RIGHT"
                        | "FULL"
                        | "CROSS"
                        | "STRAIGHT"
                        | "ON"
                        | "WHERE"
                        | "GROUP"
                        | "ORDER"
                        | "LIMIT"
                        | "AS"
                        | "AND"
                        | "OR"
                ) {
                    // Next word is an alias, skip it
                    debug!(
                        "!!! LSP: extract_real_table_names_from_source: skipping alias '{}'",
                        words[i]
                    );
                    i += 1;
                }
            }
        }

        debug!(
            "!!! LSP: extract_real_table_names_from_source: extracted tables={:?}",
            tables
        );
    }

    tables
}

/// Detect if cursor is in FROM or JOIN clause (for table completion)
fn detect_from_or_join_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_before_upper = text_before.to_uppercase();

    // Check if cursor is inside a subquery (parentheses)
    // If we're in a subquery, let other context detectors handle the inner context
    let open_parens = text_before.matches('(').count();
    let close_parens = text_before.matches(')').count();
    if open_parens > close_parens {
        return None;
    }

    // Additional check: if we're right after FROM keyword in a subquery like "(SELECT | FROM ...)",
    // the paren counting above should have caught it, but double-check the position
    // If the text ends with "SELECT " (just typed SELECT), we should skip FROM detection
    if text_before_upper.trim_end().ends_with("SELECT ")
        || text_before_upper.trim_end().ends_with("SELECT\t")
        || text_before_upper.trim_end() == "SELECT"
    {
        return None;
    }

    // Additional check for patterns like "JOIN (SELECT |" where we're in a subquery projection
    let trimmed = text_before.trim_end();

    // More robust check: if we have "(SELECT" and very little content after it, we're in subquery projection
    let in_subquery_after_select = trimmed.contains("(SELECT")
        && (trimmed.ends_with("SELECT ")
            || trimmed.ends_with("SELECT\t")
            || trimmed.ends_with("SELECT(")
            || trimmed.ends_with("(SELECT")
            || (trimmed.contains("(SELECT ") && {
                let after_select = trimmed.split("(SELECT ").last().unwrap_or("");
                after_select.split_whitespace().count() <= 2
            }));

    if in_subquery_after_select {
        return None;
    }

    // Check for JOIN pattern (e.g., "...JOIN |")
    // Look for the last JOIN keyword - if it's at the end, we're likely typing table name
    if let Some(join_pos) = text_before_upper.rfind("JOIN") {
        // Get text after the last JOIN
        let after_join = &text_before_upper[join_pos + 4..]; // +4 for "JOIN"

        debug!(
            "!!! LSP: Found JOIN at pos {}, after='{}' (len={})",
            join_pos,
            after_join,
            after_join.len()
        );

        // Check if what follows looks like we're still typing the table name
        // Split by whitespace and check
        let words: Vec<&str> = after_join.split_whitespace().collect();

        debug!(
            "!!! LSP: words after JOIN: {:?}, count={}",
            words,
            words.len()
        );
        debug!("!!! LSP: ends_with(' '): {}", after_join.ends_with(' '));
        debug!(
            "!!! LSP: trim_end().ends_with(' '): {}",
            after_join.trim_end().ends_with(' ')
        );

        debug!(
            "!!! LSP: Checking condition: words.is_empty()={}, words.len()={}, trim_end().ends_with(' ')={}",
            words.is_empty(),
            words.len(),
            after_join.trim_end().ends_with(' ')
        );
        if words.is_empty() || (words.len() <= 3 && after_join.trim_end().ends_with(' ')) {
            // We're likely right after JOIN, or after "JOIN <partial_word>"
            debug!("!!! LSP: Detected FROM/JOIN context (after JOIN)");
            // Extract tables from source for exclusion (use real table names, not aliases)
            let exclude_tables = extract_real_table_names_from_source(source);

            // Check if we should allow self-join by examining aliases
            // Pattern: "employees e1 JOIN" - alias with number suffix suggests self-join
            // Pattern: "users u JOIN" - simple alias, likely joining to different table
            debug!(
                "!!! LSP: Checking for self-join hint, exclude_tables={:?}",
                exclude_tables
            );
            let allow_self_join = if exclude_tables.len() == 1 {
                // Check if the table alias has a number suffix (e1, e2, t1, t2, etc.)
                if let Some(table) = exclude_tables.first() {
                    let source_upper = source.to_uppercase();
                    // Look for pattern "<TABLE> <ALIAS>" where alias ends with digit
                    let pattern = format!("{} ", table).to_uppercase();
                    debug!("!!! LSP: Looking for pattern '{}' in source", pattern);
                    if let Some(pos) = source_upper.find(&pattern) {
                        let after_table = &source[pos + pattern.len()..];
                        // Get the next word (alias)
                        let alias = after_table.split_whitespace().next().unwrap_or("");
                        debug!("!!! LSP: Found alias '{}' after table '{}'", alias, table);
                        // Check if alias ends with a digit (e1, e2, t1, t2, etc.)
                        let ends_with_digit = alias.chars().last().is_some_and(|c| c.is_numeric());
                        debug!(
                            "!!! LSP: Alias '{}' ends with digit: {}",
                            alias, ends_with_digit
                        );
                        ends_with_digit
                    } else {
                        debug!("!!! LSP: Pattern '{}' not found in source", pattern);
                        false
                    }
                } else {
                    debug!("!!! LSP: No tables in exclude list");
                    false
                }
            } else {
                debug!("!!! LSP: More than 1 table, not checking for self-join");
                false
            };

            let final_exclude = if allow_self_join {
                debug!("!!! LSP: Detected numbered alias (likely self-join), not excluding table");
                vec![]
            } else {
                debug!(
                    "!!! LSP: Excluding {} already-used tables",
                    exclude_tables.len()
                );
                exclude_tables
            };

            return Some(CompletionContext::FromClause {
                exclude_tables: final_exclude,
            });
        }
    }

    // Check for comma-style join pattern: "FROM table1, table2, |"
    // This needs to be checked before the regular FROM pattern
    if text_before_upper.contains("FROM") && text_before.trim_end().ends_with(',') {
        debug!("!!! LSP: Detected comma-style join pattern");
        // Extract tables from source for exclusion
        let exclude_tables = extract_real_table_names_from_source(source);
        return Some(CompletionContext::FromClause { exclude_tables });
    }

    // Check for FROM pattern (e.g., "...FROM |")
    if is_after_keyword(&text_before_upper, "FROM") {
        let after_from = extract_after_keyword(&text_before_upper, "FROM");
        debug!(
            "!!! LSP: after_from='{}', trimmed.len()={}",
            after_from,
            after_from.trim().len()
        );
        if after_from.trim().len() < 10 {
            // Check if we've already typed a table name (FROM <table> <space>)
            // Pattern: "FROM table " should suggest clauses, not tables
            // EXCEPT: "FROM table, " should suggest tables (comma-style join)
            let trimmed = after_from.trim();
            let words: Vec<&str> = trimmed.split_whitespace().collect();
            debug!("!!! LSP: words={:?}, words.len()={}", words, words.len());

            if !words.is_empty() {
                // We have at least one word after FROM
                // Check if cursor is after a space or at the end (meaning table name is complete)
                // Handle test patterns that end with "|" as cursor marker
                let ends_with_space = after_from.ends_with(' ') || after_from.ends_with('\t');
                let ends_with_cursor =
                    trimmed.ends_with('|') || words.last().is_some_and(|w| *w == "|");

                debug!(
                    "!!! LSP: ends_with_space={}, ends_with_cursor={}",
                    ends_with_space, ends_with_cursor
                );

                if ends_with_space || ends_with_cursor {
                    // Check if this is a comma-style join: "FROM table1, table2, |"
                    // If the trimmed text ends with comma, it's a comma join
                    if trimmed.ends_with(',') {
                        debug!("!!! LSP: Detected comma-style join, suggesting tables");
                        // Extract tables from source for exclusion
                        let exclude_tables = extract_real_table_names_from_source(source);
                        return Some(CompletionContext::FromClause { exclude_tables });
                    }

                    debug!("!!! LSP: Detected clause context after table name");
                    // Return Keywords context with existing clauses
                    return Some(CompletionContext::Keywords {
                        statement_type: Some("SELECT".to_string()),
                        existing_clauses: vec!["SELECT".to_string(), "FROM".to_string()],
                    });
                }
            }

            debug!("!!! LSP: Detected FROM/JOIN context (after FROM)");
            return Some(CompletionContext::FromClause {
                exclude_tables: vec![],
            });
        }
    }

    debug!("!!! LSP: detect_from_or_join_context returning None");
    None
}

/// Detect if cursor is in SELECT projection (e.g., "SELECT |" or "SELECT id, |")
fn detect_projection_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let source_upper = source.to_uppercase();
    let text_before_upper = text_before.to_uppercase();

    // Check if cursor is inside a subquery: "JOIN (SELECT | FROM ...)"
    // Count opening and closing parens to determine if we're in a subquery
    let open_parens = text_before.matches('(').count();
    let close_parens = text_before.matches(')').count();

    // If we have more opens than closes, we're inside parentheses
    let in_subquery = open_parens > close_parens;

    debug!(
        "!!! LSP: detect_projection_context: in_subquery={}, open_parens={}, close_parens={}",
        in_subquery, open_parens, close_parens
    );

    // Pattern: "SELECT" at start followed by incomplete projection
    // OR: Inside a subquery that starts with SELECT
    if source_upper.starts_with("SELECT") || in_subquery {
        // Check if we have FROM keyword in the full source
        if source_upper.contains("FROM") {
            // We have "SELECT ... FROM ..."
            // If cursor is before FROM, it's in projection
            // Check if the text before cursor contains FROM
            // BUT: If we're in a subquery, check for FROM within the subquery
            if in_subquery {
                // In subquery: find the last SELECT to check if cursor is before its FROM
                // For "JOIN (SELECT | FROM orders)", check if we just typed SELECT or if cursor is after SELECT in parens
                let just_typed_select = text_before_upper.ends_with("SELECT ")
                    || text_before_upper.ends_with("SELECT\t")
                    || text_before_upper.ends_with("SELECT(")
                    || text_before_upper.trim_end().ends_with("(SELECT");

                if just_typed_select {
                    debug!(
                        "!!! LSP: Detected SELECT projection context (in subquery, just typed SELECT)"
                    );
                    debug!("!!! LSP: source='{}'", source);

                    // Try to extract table from the subquery's FROM clause
                    // For "JOIN (SELECT | FROM orders)", we need "orders" table
                    // We need to find the FROM that's inside the subquery (after the last SELECT)
                    let tables = if let Some(last_select_pos) = source_upper.rfind("SELECT") {
                        debug!("!!! LSP: last_select_pos={}", last_select_pos);
                        // Look for FROM after this SELECT
                        let after_select = &source_upper[last_select_pos..];
                        debug!("!!! LSP: after_select='{}'", after_select);
                        if let Some(from_pos_relative) = after_select.find("FROM") {
                            let from_pos_absolute = last_select_pos + from_pos_relative;
                            debug!("!!! LSP: from_pos_absolute={}", from_pos_absolute);
                            // Get text after this FROM
                            let after_from = &source[from_pos_absolute + 4..];
                            debug!("!!! LSP: after_from='{}'", after_from);
                            // Trim leading whitespace first
                            let after_from_trimmed = after_from.trim_start();
                            debug!("!!! LSP: after_from_trimmed='{}'", after_from_trimmed);
                            // Extract first word as table name
                            if let Some(table_end) = after_from_trimmed.find([' ', ')', ';']) {
                                let table_name = after_from_trimmed[..table_end].trim().to_string();
                                debug!(
                                    "!!! LSP: Extracted table from subquery FROM: '{}'",
                                    table_name
                                );
                                vec![table_name]
                            } else {
                                debug!("!!! LSP: No table end found in after_from");
                                vec![]
                            }
                        } else {
                            debug!("!!! LSP: No FROM found after last SELECT");
                            vec![]
                        }
                    } else {
                        debug!("!!! LSP: No SELECT found in source");
                        vec![]
                    };

                    return Some(CompletionContext::SelectProjection {
                        tables,
                        qualifier: None,
                    });
                }
            } else if !text_before_upper.contains("FROM") {
                debug!("!!! LSP: Detected SELECT projection context");

                // Extract table names from FROM clause
                // Simple extraction: find FROM keyword, then get the next word(s)
                let tables = extract_tables_from_source(source);
                debug!("!!! LSP: Extracted tables from source: {:?}", tables);

                // Extract table qualifier (e.g., "users." from "SELECT users.|")
                let qualifier = extract_table_qualifier(text_before);
                debug!(
                    "!!! LSP: Extracted qualifier from text_before: {:?}",
                    qualifier
                );

                return Some(CompletionContext::SelectProjection { tables, qualifier });
            }
        } else {
            // No FROM yet, definitely in projection
            debug!("!!! LSP: Detected SELECT projection context (no FROM yet)");

            // Extract table qualifier (e.g., "users." from "SELECT users.|")
            let qualifier = extract_table_qualifier(text_before);
            debug!(
                "!!! LSP: Extracted qualifier from text_before: {:?}",
                qualifier
            );

            return Some(CompletionContext::SelectProjection {
                tables: vec![],
                qualifier,
            });
        }
    }

    None
}

/// Extract table names from source SQL
/// This is a simple fallback extraction for incomplete SQL
/// Extract table names and aliases from source SQL
/// This is a simple fallback extraction for incomplete SQL
/// Returns aliases when present (e.g., "u" from "users u"), otherwise returns table names
fn extract_tables_from_source(source: &str) -> Vec<String> {
    let mut tables = Vec::new();
    let source_upper = source.to_uppercase();

    // Find FROM clause
    if let Some(from_pos) = source_upper.find("FROM") {
        // Get text after FROM
        let after_from = &source[from_pos + 4..]; // +4 to skip "FROM"

        // Split by WHERE, GROUP BY, ORDER BY, LIMIT, ON to get the FROM clause part
        // Note: We stop at ON to avoid including JOIN conditions
        let from_clause_end_keywords = [" WHERE ", " GROUP BY ", " ORDER BY ", " LIMIT ", " ON "];
        let from_part = after_from;
        let mut end_pos = from_part.len();

        for keyword in &from_clause_end_keywords {
            if let Some(pos) = from_part.to_uppercase().find(keyword)
                && pos < end_pos
            {
                end_pos = pos;
            }
        }

        let from_clause = &from_part[..end_pos];
        debug!(
            "!!! LSP: extract_tables_from_source: from_clause='{}'",
            from_clause
        );

        // Strip trailing semicolons and other statement terminators
        let from_clause = from_clause
            .trim()
            .trim_end_matches(';')
            .trim_end_matches(';')
            .trim();

        debug!(
            "!!! LSP: extract_tables_from_source: from_clause after stripping='{}'",
            from_clause
        );

        // Extract table aliases using a regex-like pattern
        // Patterns: "table_name", "table_name alias", "table_name AS alias"
        // And handle JOINs: "JOIN table alias", "JOIN table AS alias"

        let words: Vec<&str> = from_clause
            .split([' ', '\n', '\t', ','])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        let mut i = 0;
        while i < words.len() {
            let word = words[i].to_uppercase();

            // Handle JOIN keyword - skip it and process the next table
            if word == "JOIN"
                || matches!(
                    word.as_str(),
                    "INNER" | "LEFT" | "RIGHT" | "FULL" | "CROSS" | "STRAIGHT" | "STRAIGHT_JOIN"
                )
            {
                i += 1;
                continue;
            }

            // Skip AS keyword - the next word is the alias
            if word == "AS" {
                i += 1;
                if i < words.len() {
                    // Clean up the alias by removing trailing special characters
                    let alias =
                        words[i].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    tables.push(alias.to_string());
                }
                i += 1;
                continue;
            }

            // Skip other SQL keywords that shouldn't be treated as table names
            if matches!(
                word.as_str(),
                "ON" | "AND"
                    | "OR"
                    | "WHERE"
                    | "GROUP"
                    | "ORDER"
                    | "LIMIT"
                    | "SELECT"
                    | "FROM"
                    | "*"
                    | "INSERT"
                    | "UPDATE"
                    | "DELETE"
                    | "CREATE"
                    | "WITH"
                    | "SET"
                    | "VALUES"
                    | "INTO"
            ) {
                i += 1;
                continue;
            }

            // Skip words that are purely special characters or numbers
            if words[i].chars().all(|c| !c.is_alphabetic()) {
                i += 1;
                continue;
            }

            // This is likely a table name - check if next word is an alias
            let has_alias = if i + 1 < words.len() {
                let next_word = words[i + 1].to_uppercase();
                !matches!(
                    next_word.as_str(),
                    "JOIN"
                        | "INNER"
                        | "LEFT"
                        | "RIGHT"
                        | "FULL"
                        | "CROSS"
                        | "STRAIGHT"
                        | "ON"
                        | "WHERE"
                        | "GROUP"
                        | "ORDER"
                        | "LIMIT"
                        | "AS"
                        | "AND"
                        | "OR"
                )
            } else {
                false
            };

            if has_alias {
                // Use the alias instead of the table name
                // Clean up the alias by removing trailing special characters
                let alias =
                    words[i + 1].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
                tables.push(alias.to_string());
                i += 2;
            } else {
                // No alias, use the table name
                // Clean up the table name by removing trailing special characters
                let table_name =
                    words[i].trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
                tables.push(table_name.to_string());
                i += 1;
            }
        }

        debug!(
            "!!! LSP: extract_tables_from_source: extracted tables={:?}",
            tables
        );
    }

    tables
}

/// Detect if cursor is in WHERE clause
fn detect_where_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Check if we're after WHERE keyword
    // We should detect WHERE regardless of what comes after (even complex expressions)
    if text_upper.contains("WHERE") {
        debug!("!!! LSP: Detected WHERE context");
        // Extract tables from source
        let tables = extract_tables_from_source(source);
        // Check for table qualifier (e.g., "u.")
        let qualifier = extract_table_qualifier(text_before);
        return Some(CompletionContext::WhereClause { tables, qualifier });
    }

    None
}

/// Detect if cursor is in JOIN USING clause
fn detect_join_using_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    debug!(
        "!!! LSP: detect_join_using_context: text_before='{}'",
        text_before
    );
    debug!(
        "!!! LSP: detect_join_using_context: contains USING={}",
        text_upper.contains("USING")
    );
    debug!(
        "!!! LSP: detect_join_using_context: ends_with '({}'",
        text_before.trim_end().ends_with('(')
    );

    // Pattern: "...JOIN ... USING (|"
    // Check if we're after USING keyword and have an opening paren
    if text_upper.contains("USING") && text_before.trim_end().ends_with('(') {
        // Also verify we're in a JOIN statement
        if !text_upper.contains("JOIN") {
            return None;
        }

        debug!("!!! LSP: Detected JOIN USING context");

        // For USING, we need to extract table names manually
        // Pattern: "FROM table1 JOIN table2 USING ("
        // We need to find the table names before USING
        let source_upper = source.to_uppercase();

        // Find the last FROM keyword
        if let Some(from_pos) = source_upper.find("FROM") {
            let after_from = &source[from_pos + 4..]; // Skip "FROM"

            // Extract tables by looking for JOIN keywords
            // Pattern: "table1 JOIN table2 USING"
            let mut tables = Vec::new();

            // Split by JOIN to get individual table references
            let parts: Vec<&str> = after_from.split("JOIN").collect();

            for part in parts {
                // Extract the table name (first word before any alias or USING)
                let trimmed = part.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("USING") {
                    // Get the first word (table name or alias)
                    if let Some(first_word) = trimmed.split_whitespace().next() {
                        tables.push(first_word.to_string());
                    }
                }
            }

            debug!("!!! LSP: JOIN USING: extracted tables={:?}", tables);

            if tables.len() >= 2 {
                // Get the last two tables (the ones being joined)
                let left_table = Some(tables[tables.len() - 2].clone());
                let right_table = Some(tables[tables.len() - 1].clone());

                debug!(
                    "!!! LSP: JOIN USING tables - left: {:?}, right: {:?}",
                    left_table, right_table
                );

                return Some(CompletionContext::JoinCondition {
                    left_table,
                    right_table,
                    qualifier: None,
                });
            }
        }
    }

    None
}

/// Detect if cursor is in JOIN ON clause
fn detect_join_on_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Pattern: "...JOIN ... ON |" or "...JOIN ... ON ... AND |"
    // We need to check if cursor is after ON keyword, possibly with AND/OR in between
    // Check if ON exists in the text
    if text_upper.contains("ON") {
        // Also verify we're in a JOIN statement
        if !text_upper.contains("JOIN") {
            return None;
        }
        // Check for table qualifier (e.g., "u.")
        let qualifier = extract_table_qualifier(text_before);
        debug!(
            "!!! LSP: Detected JOIN ON context, qualifier: {:?}",
            qualifier
        );

        // Extract table aliases from source
        // For "SELECT * FROM users u JOIN orders o ON u.|", we need ["u", "o"]
        let table_aliases = extract_tables_from_source(source);

        // For JOIN ON, we need to determine which tables are available
        // If qualifier is provided, show columns from that table
        // Otherwise, we might want to show columns from both tables
        let (left_table, right_table) = if table_aliases.len() >= 2 {
            // Assume first two aliases are the left and right tables
            (
                Some(table_aliases[0].clone()),
                Some(table_aliases[1].clone()),
            )
        } else {
            (None, None)
        };

        // If qualifier is provided, only show columns from that table
        // This matches the test expectation: "u." should only show u.* columns, not o.*
        let (left_table, right_table) = match &qualifier {
            Some(q) => {
                // Find which table the qualifier refers to
                if left_table.as_ref().map(|t| t == q).unwrap_or(false) {
                    // Qualifier matches left table, only show left table columns
                    (left_table, None)
                } else if right_table.as_ref().map(|t| t == q).unwrap_or(false) {
                    // Qualifier matches right table, only show right table columns
                    (None, right_table)
                } else {
                    // Unknown qualifier, show both tables
                    (left_table, right_table)
                }
            }
            None => (left_table, right_table),
        };

        debug!(
            "!!! LSP: JOIN ON tables - left: {:?}, right: {:?}, qualifier: {:?}",
            left_table, right_table, qualifier
        );

        // Note: We preserve the original qualifier here so that the completion engine
        // can decide whether to add the qualifier prefix based on user input
        return Some(CompletionContext::JoinCondition {
            left_table,
            right_table,
            qualifier,
        });
    }

    None
}

/// Detect if cursor is in ORDER BY clause
fn detect_order_by_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Check if we're after ORDER BY keyword
    if is_after_keyword(&text_upper, "ORDER BY") {
        debug!("!!! LSP: Detected ORDER BY context");
        // Extract tables from source
        let tables = extract_tables_from_source(source);
        // Check for table qualifier
        let qualifier = extract_table_qualifier(text_before);
        return Some(CompletionContext::OrderByClause { tables, qualifier });
    }

    None
}

/// Detect if cursor is in GROUP BY clause
fn detect_group_by_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Check if we're after GROUP BY keyword
    if is_after_keyword(&text_upper, "GROUP BY") {
        debug!("!!! LSP: Detected GROUP BY context");
        // Extract tables from source
        let tables = extract_tables_from_source(source);
        // Check for table qualifier
        let qualifier = extract_table_qualifier(text_before);
        return Some(CompletionContext::GroupByClause { tables, qualifier });
    }

    None
}

/// Detect if cursor is in LIMIT clause
fn detect_limit_context(text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Check if we're after LIMIT keyword
    if is_after_keyword(&text_upper, "LIMIT") {
        debug!("!!! LSP: Detected LIMIT context");
        return Some(CompletionContext::LimitClause);
    }

    None
}

/// Detect if cursor is in HAVING clause
fn detect_having_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let text_upper = text_before.to_uppercase();

    // Check if we're after HAVING keyword
    if is_after_keyword(&text_upper, "HAVING") {
        debug!("!!! LSP: Detected HAVING context");
        // Extract tables from source
        let tables = extract_tables_from_source(source);
        // Check for table qualifier
        let qualifier = extract_table_qualifier(text_before);
        return Some(CompletionContext::HavingClause { tables, qualifier });
    }

    None
}

/// Detect DDL/DML statement context (CREATE, ALTER, DROP, INSERT, UPDATE, DELETE)
fn detect_ddl_context(text_before: &str) -> Option<CompletionContext> {
    let trimmed = text_before.trim_end();

    // Check for CREATE pattern
    if trimmed.ends_with("CREATE ") || trimmed == "CREATE" {
        debug!("!!! LSP: Detected CREATE context");
        return Some(CompletionContext::Keywords {
            statement_type: Some("CREATE".to_string()),
            existing_clauses: vec![],
        });
    }

    // Check for ALTER pattern
    if trimmed.ends_with("ALTER ") || trimmed == "ALTER" {
        debug!("!!! LSP: Detected ALTER context");
        return Some(CompletionContext::Keywords {
            statement_type: Some("ALTER".to_string()),
            existing_clauses: vec![],
        });
    }

    // Check for DROP pattern
    if trimmed.ends_with("DROP ") || trimmed == "DROP" {
        debug!("!!! LSP: Detected DROP context");
        return Some(CompletionContext::Keywords {
            statement_type: Some("DROP".to_string()),
            existing_clauses: vec![],
        });
    }

    // NOTE: INSERT, UPDATE, DELETE after the keyword require table names,
    // so we don't detect them here - they fall through to table completion

    None
}

/// Extract table qualifier from text (e.g., "u." -> "u")
fn extract_table_qualifier(text: &str) -> Option<String> {
    // Look for pattern like "table_name." at the end of text
    let trimmed = text.trim();
    if let Some(dot_pos) = trimmed.rfind('.') {
        // Check if the dot is followed by whitespace or is at the end
        // Also handle test patterns that end with "|" as cursor marker
        let after_dot = &trimmed[dot_pos + 1..];
        let is_at_end = after_dot.is_empty()
            || after_dot.starts_with(' ')
            || after_dot.starts_with('\t')
            || after_dot == "|";

        if is_at_end {
            // Get the identifier before the dot
            let before_dot = &trimmed[..dot_pos];
            if let Some(ident_end) = before_dot.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                Some(trimmed[ident_end + 1..dot_pos].to_string())
            } else {
                Some(before_dot.to_string())
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract table qualifier based on cursor position
/// This is a more precise version that uses the cursor position
fn extract_table_qualifier_from_position(source: &str, position: Position) -> Option<String> {
    // Get the byte position of the cursor
    let byte_offset = position_to_byte_offset(source, position);

    // Check if offset is valid
    if byte_offset >= source.len() {
        return None;
    }

    // Get text before cursor
    let text_before = &source[..byte_offset];

    // Use extract_table_qualifier on the text before cursor
    extract_table_qualifier(text_before)
}

/// Check if cursor is after a specific keyword (case-insensitive)
fn is_after_keyword(text: &str, keyword: &str) -> bool {
    let text_upper = text.to_uppercase();

    // Find the keyword
    if let Some(pos) = text_upper.find(keyword) {
        // Check if there's nothing significant between keyword and cursor position
        let after_keyword = &text_upper[pos + keyword.len()..];
        // Only whitespace and maybe partial identifier should be after keyword
        let trimmed = after_keyword.trim_start();
        // If it's very short or just identifiers/whitespace, we're likely right after the keyword
        trimmed.len() < 20 || trimmed.split_whitespace().count() <= 2
    } else {
        false
    }
}

/// Extract text after a specific keyword
fn extract_after_keyword(text: &str, keyword: &str) -> String {
    let text_upper = text.to_uppercase();

    if let Some(pos) = text_upper.find(keyword) {
        text[pos + keyword.len()..].to_string()
    } else {
        String::new()
    }
}

/// Check if the position is within the SELECT projection list
fn is_in_projection(select_node: &Node, position: Position) -> bool {
    // The projection is typically the first child after "SELECT" keyword
    for child in select_node.children(&mut select_node.walk()) {
        if child.kind() == "projection" {
            let start = child.start_position();
            let end = child.end_position();

            return position.line as usize >= start.row
                && position.line as usize <= end.row
                && position.character as usize >= start.column;
        }
    }

    false
}

/// Check if the position is right after the SELECT keyword
/// This handles cases where the projection list is empty (e.g., "SELECT | FROM table")
fn is_right_after_select_keyword(select_node: &Node, source: &str, position: Position) -> bool {
    // Get the byte offset of the cursor position
    let byte_offset = position_to_byte_offset(source, position);

    // Get the text before the cursor
    let text_before = if byte_offset <= source.len() {
        &source[..byte_offset]
    } else {
        source
    };

    // Check if the text ends with "SELECT" (possibly with whitespace)
    let text_upper = text_before.trim_end().to_uppercase();
    eprintln!(
        "!!! is_right_after_select_keyword: text_upper='{}'",
        text_upper
    );
    if text_upper.ends_with("SELECT")
        || text_upper.ends_with("SELECT ")
        || text_upper.ends_with("SELECT\t")
    {
        eprintln!("!!! is_right_after_select_keyword: text ends with SELECT");
        // Additional check: make sure we're after the SELECT keyword within this select_statement
        // Find the SELECT keyword position in the source
        let node_start = select_node.byte_range().start;
        let node_end = select_node.byte_range().end;

        // Look for SELECT keyword within this node
        // Use rfind to get the LAST SELECT before the cursor (handles CTEs with nested SELECT)
        let node_text = &source[node_start..node_end.min(source.len())];
        let node_text_before_cursor = if byte_offset <= node_end {
            &node_text[..byte_offset - node_start]
        } else {
            node_text
        };
        let node_text_before_cursor_upper = node_text_before_cursor.to_uppercase();

        if let Some(select_pos) = node_text_before_cursor_upper.rfind("SELECT") {
            let absolute_select_pos = node_start + select_pos + 6; // +6 for "SELECT"
            // Check if cursor is after SELECT and within reasonable distance (e.g., within 10 chars)
            if byte_offset >= absolute_select_pos && byte_offset - absolute_select_pos <= 10 {
                // Also check that there's no FROM keyword between SELECT and cursor
                let text_before_cursor_in_node = if byte_offset <= node_end {
                    &node_text[select_pos + 6..byte_offset - node_start]
                } else {
                    &node_text[select_pos + 6..]
                };

                // If we haven't reached FROM yet, we're in the projection
                !text_before_cursor_in_node.to_uppercase().contains("FROM")
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
}

/// Convert position to byte offset with error checking
fn position_to_byte_offset_checked(source: &str, position: Position) -> Result<usize, String> {
    let mut line = 0;
    let mut col = 0;
    let mut byte_offset = 0;

    for ch in source.chars() {
        if line == position.line as usize && col == position.character as usize {
            return Ok(byte_offset);
        }

        byte_offset += ch.len_utf8();

        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    // If we reach the end of the source, return the length
    if line == position.line as usize {
        Ok(byte_offset)
    } else {
        Err(format!(
            "Position {:?} is out of bounds for source of length {}",
            position,
            source.len()
        ))
    }
}

/// Extract table names from subquery text
/// For subqueries like "(SELECT | FROM orders)", extract "orders"
fn extract_tables_from_subquery_text(source: &str, position: Position) -> Vec<String> {
    let byte_offset = position_to_byte_offset(source, position);
    let text_before = &source[..byte_offset.min(source.len())];

    // Find the last unmatched opening parenthesis to locate the subquery start
    let mut open_parens = 0;
    let mut subquery_start = 0;

    for (i, ch) in text_before.chars().enumerate() {
        if ch == '(' {
            open_parens += 1;
            subquery_start = i;
        } else if ch == ')' {
            open_parens -= 1;
        }
    }

    if open_parens == 0 {
        // Not in a subquery, return empty
        return Vec::new();
    }

    // Get the text from the subquery start
    let subquery_text = &source[subquery_start..];

    // Look for the FROM clause in the subquery
    let subquery_upper = subquery_text.to_uppercase();

    // Find FROM after the last SELECT (to handle nested subqueries)
    if let Some(select_pos) = subquery_upper.rfind("SELECT") {
        let after_select = &subquery_upper[select_pos + 6..];

        if let Some(from_pos) = after_select.find("FROM") {
            let from_absolute = select_pos + 6 + from_pos;
            let after_from = &subquery_text[from_absolute + 4..];
            let after_from_trimmed = after_from.trim_start();

            // Extract the table name (up to next space, comma, parenthesis, or semicolon)
            if let Some(table_end) = after_from_trimmed.find([' ', ',', ')', ';']) {
                let table_name = after_from_trimmed[..table_end].trim().to_string();
                // Filter out obvious non-table names
                if !table_name.is_empty()
                    && table_name != "SELECT"
                    && table_name != "FROM"
                    && table_name != "WHERE"
                    && table_name != "JOIN"
                    && !table_name.contains('(')
                {
                    return vec![table_name];
                }
            }
        }
    }

    Vec::new()
}

/// Extract table names from the FROM clause
fn extract_tables_from_from_clause(select_node: &Node, source: &str) -> Vec<String> {
    let mut tables = Vec::new();

    eprintln!(
        "!!! extract_tables_from_from_clause: select_node.kind()={}",
        select_node.kind()
    );
    eprintln!(
        "!!! extract_tables_from_from_clause: select_node byte range={:?}",
        select_node.byte_range()
    );

    for child in select_node.children(&mut select_node.walk()) {
        eprintln!(
            "!!! extract_tables_from_from_clause: child.kind()={}",
            child.kind()
        );
        if child.kind() == "from_clause" {
            // Find table_reference nodes
            eprintln!("!!! extract_tables_from_from_clause: found from_clause");
            extract_table_names_recursive(&child, source, &mut tables);
            break;
        }
    }

    eprintln!(
        "!!! extract_tables_from_from_clause: extracted tables={:?}",
        tables
    );
    tables
}

/// Recursively extract table names from table_reference nodes
fn extract_table_names_recursive(node: &Node, source: &str, tables: &mut Vec<String>) {
    eprintln!(
        "!!! extract_table_names_recursive: node.kind()='{}', text='{}'",
        node.kind(),
        &source[node.byte_range()]
    );
    match node.kind() {
        "table_reference" | "table_name" => {
            if let Some(name) = extract_identifier_name(node, source) {
                eprintln!(
                    "!!! extract_table_names_recursive: extracting table name='{}'",
                    name
                );
                tables.push(name);
            }
        }
        _ => {
            // Recurse into children
            for child in node.children(&mut node.walk()) {
                extract_table_names_recursive(&child, source, tables);
            }
        }
    }
}

/// Extract table qualifier if cursor is after a dot
fn extract_qualifier(node: &Node, source: &str, position: Position) -> Option<String> {
    // Check if the node contains a dot and cursor is after it
    let node_text = &source[node.byte_range()];
    let cursor_offset = position.character as usize;

    // Find dots in the node text
    if let Some(dot_pos) = node_text.rfind('.') {
        let dot_abs_pos = node.start_position().column + dot_pos;
        if cursor_offset > dot_abs_pos {
            // Cursor is after the dot, extract qualifier (text before dot)
            let qualifier = node_text[..dot_pos].trim();
            return Some(qualifier.to_string());
        }
    }

    None
}

/// Extract table names from a from_clause node
fn extract_tables_from_from_clause_node(from_node: &Node, source: &str) -> Vec<String> {
    let mut tables = Vec::new();
    extract_table_names_recursive(from_node, source, &mut tables);
    tables
}

/// Extract table names from the FROM clause when in a JOIN context
///
/// This walks up from the join_clause to find the select_statement,
/// then extracts tables from the FROM clause.
fn extract_tables_from_join_parent(join_node: &Node, source: &str) -> Vec<String> {
    let mut tables = Vec::new();

    // Walk up to find the select_statement
    let mut current = join_node.parent();
    while let Some(node) = current {
        if node.kind() == "select_statement" {
            // Found the select statement, now find the from_clause
            for child in node.children(&mut node.walk()) {
                if child.kind() == "from_clause" {
                    extract_table_names_recursive(&child, source, &mut tables);
                    break;
                }
            }
            break;
        }
        current = node.parent();
    }

    tables
}

/// Extract left and right table names from a join clause
///
/// For a JOIN like `users JOIN orders ON users.id = orders.user_id`,
/// this extracts ("users", "orders")
fn extract_join_tables(join_node: &Node, source: &str) -> (Option<String>, Option<String>) {
    // Get parent from_clause to find the left table
    let mut left_table = None;
    let mut right_table = None;

    // First, try to get the right table (the table being joined)
    // In the join_clause node, the table_name is typically the second child (after JOIN keyword)
    let mut walk = join_node.walk();
    let mut children = join_node.children(&mut walk);
    let mut found_join_keyword = false;

    for child in &mut children {
        match child.kind() {
            "JOIN" | "INNER" | "LEFT" | "RIGHT" | "FULL" => {
                found_join_keyword = true;
            }
            "table_name" | "table_reference" if found_join_keyword => {
                if let Some(name) = extract_identifier_name(&child, source) {
                    right_table = Some(name);
                    break;
                }
            }
            _ => {}
        }
    }

    // Now, try to get the left table from the parent context
    // Walk up to find the from_clause and get tables before this join
    if let Some(parent) = join_node.parent()
        && (parent.kind() == "from_clause" || parent.kind() == "select_statement")
    {
        // Look for table_reference nodes that come before this join
        let from_tables = extract_tables_from_from_clause(&parent, source);
        if !from_tables.is_empty() {
            // The last table before the join is typically the left table
            left_table = from_tables.into_iter().next();
        }
    }

    (left_table, right_table)
}

/// Detect CTE (Common Table Expression) context
///
/// Detects if the cursor is in a WITH clause where we're defining or referencing CTEs
fn detect_cte_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let upper = text_before.to_uppercase();
    let source_upper = source.to_uppercase();

    debug!(
        "!!! LSP: detect_cte_context called, text_before='{}'",
        text_before
    );

    // Check if we're in a WITH clause at all
    if !upper.contains("WITH") {
        return None;
    }

    // Pattern 1: "WITH | AS" or "WITH | AS (" - defining first CTE, suggest all tables
    // Check: cursor is right after "WITH "
    if upper == "WITH " || upper.ends_with("WITH ") || upper.ends_with("WITH\t") {
        debug!("!!! LSP: Detected CTE definition start (Pattern 1)");
        // For CTE name completion, we want to show ALL tables from catalog
        // So we pass empty available_tables - the completion handler will fetch all tables
        return Some(CompletionContext::CteDefinition {
            available_tables: vec![], // Empty = show all tables from catalog
            defined_ctes: vec![],
        });
    }

    // Pattern 2: "WITH cte1 AS (...), | AS" - defining another CTE after comma
    // Check: cursor is after ", "
    if let Some(comma_pos) = upper.rfind(", ") {
        let after_comma = &upper[comma_pos + 2..];
        // Check if next non-whitespace would be AS
        let trimmed = after_comma.trim_start();
        if trimmed.starts_with("AS") || trimmed.is_empty() {
            debug!("!!! LSP: Detected additional CTE definition (Pattern 2)");
            let defined_ctes = extract_defined_ctes_before_cursor(source, text_before);
            // For CTE name completion, we want to show ALL tables from catalog
            return Some(CompletionContext::CteDefinition {
                available_tables: vec![], // Empty = show all tables from catalog
                defined_ctes,
            });
        }
    }

    // Pattern 3: "WITH cte AS (SELECT | FROM" - inside CTE subquery projection
    // Check if we're inside parentheses after "AS ("
    if let Some(as_pos) = upper.find(" AS (") {
        let after_as = &upper[as_pos + 5..]; // Skip " AS ("
        let before_as = &upper[..as_pos];

        // Count parentheses to check if we're still inside CTE definition
        let open_parens = before_as.matches('(').count();
        let close_parens = before_as.matches(')').count();

        if open_parens > close_parens {
            // We're inside the CTE definition
            // Check if cursor is after SELECT and before FROM
            if let Some(select_pos) = after_as.rfind("SELECT ") {
                let after_select = &after_as[select_pos + 7..];
                if !after_select.contains("FROM ") {
                    debug!("!!! LSP: Detected column completion in CTE subquery (Pattern 3)");
                    // Treat as SELECT projection for columns
                    return detect_projection_context(source, text_before);
                }
            }
        }
    }

    // Pattern 4: "WITH cte AS (...) SELECT | FROM cte" - in main query after CTE definition
    // Check if cursor is after CTE definition closes
    if let Some(with_pos) = source_upper.find("WITH ") {
        let after_with = &source_upper[with_pos + 5..];

        // Count parentheses to find where CTE definition ends
        let mut paren_count = 0;
        let mut cte_def_end = 0;

        for (i, ch) in after_with.chars().enumerate() {
            if ch == '(' {
                paren_count += 1;
            } else if ch == ')' {
                paren_count -= 1;
                if paren_count == 0 {
                    cte_def_end = with_pos + 5 + i + 1;
                    break;
                }
            }
        }

        if cte_def_end > 0 && text_before.len() > cte_def_end {
            let after_cte = &text_before[cte_def_end..].trim_start();
            // Check if cursor is in the main SELECT
            if after_cte.starts_with("SELECT") || after_cte.contains(" SELECT") {
                debug!("!!! LSP: Detected main query after CTE definition (Pattern 4)");
                // Check if cursor is after FROM to reference CTE
                if after_cte.contains("FROM ") {
                    // Suggest columns from the CTE
                    let cte_names = extract_defined_ctes(source);
                    let tables = cte_names;
                    let qualifier = None;
                    return detect_projection_context_with_tables(
                        source,
                        text_before,
                        tables,
                        qualifier,
                    );
                } else {
                    // Before FROM, suggest CTE names and columns
                    let cte_names = extract_defined_ctes(source);
                    let mut tables = cte_names.clone();
                    tables.extend(extract_tables_from_source(source));
                    let qualifier = None;
                    return detect_projection_context_with_tables(
                        source,
                        text_before,
                        tables,
                        qualifier,
                    );
                }
            }
        }
    }

    debug!("!!! LSP: No CTE pattern matched");
    None
}

/// Detect window function (OVER clause) context
///
/// Detects if the cursor is in an OVER clause for window functions
fn detect_window_function_context(source: &str, text_before: &str) -> Option<CompletionContext> {
    let upper = text_before.to_uppercase();

    debug!(
        "!!! LSP: detect_window_function_context called, text_before='{}'",
        text_before
    );

    // Check if we have an OVER clause (handle variations in spacing)
    let has_over = upper.contains("OVER(") || upper.contains("OVER (") || upper.ends_with("OVER");

    if !has_over {
        debug!("!!! LSP: No OVER clause found, returning None");
        return None;
    }

    // Pattern 1: "OVER (|" - just opened OVER clause
    if upper.ends_with("OVER (") || upper.ends_with("OVER( ") {
        debug!("!!! LSP: Detected OVER clause start");
        return Some(CompletionContext::WindowFunctionClause {
            tables: extract_tables_from_source(source),
            window_part: WindowFunctionPart::OverStart,
        });
    }

    // Pattern 2: "OVER (PARTITION BY |" - inside PARTITION BY
    if upper.ends_with("PARTITION BY ") || upper.ends_with("PARTITION BY") {
        debug!("!!! LSP: Detected PARTITION BY clause");
        return Some(CompletionContext::WindowFunctionClause {
            tables: extract_tables_from_source(source),
            window_part: WindowFunctionPart::PartitionBy,
        });
    }

    // Pattern 3: "OVER (ORDER BY |" - inside ORDER BY
    if upper.ends_with("ORDER BY ") || upper.ends_with("ORDER BY") {
        debug!("!!! LSP: Detected ORDER BY in window function");
        return Some(CompletionContext::WindowFunctionClause {
            tables: extract_tables_from_source(source),
            window_part: WindowFunctionPart::OrderBy,
        });
    }

    // Pattern 4: Check if we're inside OVER clause
    // Handle both "OVER (" and "OVER(" variants
    let over_pos = upper.rfind("OVER (").or_else(|| upper.rfind("OVER("));

    if let Some(over_pos) = over_pos {
        // Calculate skip length based on which variant we found
        let skip_len = if upper[over_pos..].starts_with("OVER (") {
            6 // "OVER ("
        } else {
            5 // "OVER("
        };

        let after_over = &upper[over_pos + skip_len..];
        let open_parens = after_over.matches('(').count();
        let close_parens = after_over.matches(')').count();

        // More opens than closes means we're still inside the OVER clause
        if open_parens > close_parens {
            debug!("!!! LSP: Detected inside OVER clause");

            // Check which part we're in
            if after_over.to_uppercase().contains("PARTITION BY") {
                // Check if cursor is after ORDER BY
                if let Some(order_pos) = after_over.to_uppercase().rfind("ORDER BY") {
                    let after_order = &after_over[order_pos + 8..];
                    if !after_order.contains(')') {
                        return Some(CompletionContext::WindowFunctionClause {
                            tables: extract_tables_from_source(source),
                            window_part: WindowFunctionPart::OrderBy,
                        });
                    }
                }
                // We're in PARTITION BY
                return Some(CompletionContext::WindowFunctionClause {
                    tables: extract_tables_from_source(source),
                    window_part: WindowFunctionPart::PartitionBy,
                });
            } else if after_over.to_uppercase().contains("ORDER BY") {
                return Some(CompletionContext::WindowFunctionClause {
                    tables: extract_tables_from_source(source),
                    window_part: WindowFunctionPart::OrderBy,
                });
            }

            // Just after OVER (
            return Some(CompletionContext::WindowFunctionClause {
                tables: extract_tables_from_source(source),
                window_part: WindowFunctionPart::OverStart,
            });
        }
    }

    debug!("!!! LSP: No window function pattern matched");
    None
}

/// Extract CTE names that have already been defined in the current WITH clause
fn extract_defined_ctes(source: &str) -> Vec<String> {
    let mut ctes = Vec::new();
    let upper = source.to_uppercase();

    // Find WITH clause
    if let Some(with_pos) = upper.find("WITH ") {
        let after_with = &source[with_pos + 5..]; // Skip "WITH "

        // Split by comma to find individual CTE definitions
        // Pattern: "cte_name AS (SELECT ...)"
        for part in after_with.split(',') {
            let trimmed = part.trim();
            if let Some(as_pos) = trimmed.to_uppercase().find(" AS ") {
                let cte_name = trimmed[..as_pos].trim();
                if !cte_name.is_empty() {
                    ctes.push(cte_name.to_string());
                }
            }
        }
    }

    ctes
}

/// Extract CTE names that have been defined before the cursor position
fn extract_defined_ctes_before_cursor(_source: &str, text_before: &str) -> Vec<String> {
    let mut ctes = Vec::new();
    let upper = text_before.to_uppercase();

    // Find WITH clause in text before cursor
    if let Some(with_pos) = upper.find("WITH ") {
        let after_with = &text_before[with_pos + 5..]; // Skip "WITH "

        // Split by comma to find individual CTE definitions
        // Pattern: "cte_name AS (SELECT ...)"
        for part in after_with.split(',') {
            let trimmed = part.trim();
            if let Some(as_pos) = trimmed.to_uppercase().find(" AS ") {
                let cte_name = trimmed[..as_pos].trim();
                if !cte_name.is_empty() {
                    ctes.push(cte_name.to_string());
                }
            }
        }
    }

    ctes
}

/// Detect projection context with provided tables and qualifier
/// This is a variant of detect_projection_context that accepts pre-computed values
fn detect_projection_context_with_tables(
    source: &str,
    text_before: &str,
    tables: Vec<String>,
    qualifier: Option<String>,
) -> Option<CompletionContext> {
    let source_upper = source.to_uppercase();
    let text_before_upper = text_before.to_uppercase();

    // Check if cursor is inside a subquery
    let open_parens = text_before.matches('(').count();
    let close_parens = text_before.matches(')').count();
    let in_subquery = open_parens > close_parens;

    debug!(
        "!!! LSP: detect_projection_context_with_tables: in_subquery={}, tables={:?}, qualifier={:?}",
        in_subquery, tables, qualifier
    );

    // If we're in SELECT context (at start or in subquery)
    if source_upper.starts_with("SELECT") || in_subquery {
        // If we haven't reached FROM yet, it's in projection
        if !text_before_upper.contains("FROM") {
            debug!("!!! LSP: Detected SELECT projection context (no FROM yet)");
            return Some(CompletionContext::SelectProjection { tables, qualifier });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_context_is_select_projection() {
        let ctx = CompletionContext::SelectProjection {
            tables: vec!["users".to_string()],
            qualifier: None,
        };
        assert!(ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_from_clause() {
        let ctx = CompletionContext::FromClause {
            exclude_tables: vec![],
        };
        assert!(!ctx.is_select_projection());
        assert!(ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_where_clause() {
        let ctx = CompletionContext::WhereClause {
            tables: vec![],
            qualifier: None,
        };
        assert!(!ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(ctx.is_where_clause());
        assert!(!ctx.is_join_condition());
    }

    #[test]
    fn test_completion_context_is_join_condition() {
        let ctx = CompletionContext::JoinCondition {
            left_table: Some("users".to_string()),
            right_table: Some("orders".to_string()),
            qualifier: None,
        };
        assert!(!ctx.is_select_projection());
        assert!(!ctx.is_from_clause());
        assert!(!ctx.is_where_clause());
        assert!(ctx.is_join_condition());
    }

    // Note: Full integration tests with real tree-sitter parsing
    // will be in the tests module
}
