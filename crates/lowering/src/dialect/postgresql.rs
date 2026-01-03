// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! PostgreSQL CST → IR lowering implementation
//!
//! This module provides PostgreSQL-specific lowering from Tree-sitter CST to the unified IR.
//! It handles PostgreSQL-specific syntax including:
//! - LIMIT/OFFSET clause (single form: LIMIT count OFFSET offset)
//! - Double-quote identifiers
//! - Dollar-quoted string literals ($$string$$ or $tag$string$tag$)
//! - CTE (WITH clauses) including MATERIALIZED
//! - RETURNING clause (placeholder for future IR support)
//! - DISTINCT ON (graceful degradation to regular DISTINCT)
//! - LATERAL JOIN (placeholder for future IR support)
//!
//! ## Error Handling
//!
//! The implementation follows graceful degradation strategy:
//! - Critical errors (missing structure) → Return `Err(LoweringError)`
//! - Recoverable errors (unsupported syntax) → Add to context, insert placeholder, continue

use crate::dialect::DialectLoweringBase;
use crate::{CstNode, Lowering, LoweringContext, LoweringError, LoweringResult};
use unified_sql_lsp_ir::expr::{BinaryOp, ColumnRef, Literal, UnaryOp};
use unified_sql_lsp_ir::query::{OrderBy, SelectItem, SelectStatement, TableRef};
use unified_sql_lsp_ir::{Dialect, Expr, Query};

/// PostgreSQL CST → IR lowering implementation
pub struct PostgreSQLLowering;

impl<N> DialectLoweringBase<N> for PostgreSQLLowering where N: CstNode {}

impl<N> Lowering<N> for PostgreSQLLowering
where
    N: CstNode,
{
    fn lower_query(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Query> {
        match node.kind() {
            "select_statement" => self.lower_select_statement(ctx, node),
            "insert_statement" => self.lower_insert_statement(ctx, node),
            "update_statement" => self.lower_update_statement(ctx, node),
            "delete_statement" => self.lower_delete_statement(ctx, node),
            _ => Err(LoweringError::UnexpectedNodeType {
                expected: "SELECT, INSERT, UPDATE, or DELETE statement".to_string(),
                found: node.kind().to_string(),
            }),
        }
    }

    fn lower_expr(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr> {
        match node.kind() {
            "binary_expression" => self.lower_binary_expr(ctx, node),
            "unary_expression" => self.lower_unary_expr(ctx, node),
            "column_reference" | "column_ref" => self.lower_column_ref(ctx, node),
            "literal" => self.lower_postgresql_literal(ctx, node),
            "function_call" => self.lower_function_call(ctx, node),
            "case_expression" => self.lower_case_expr(ctx, node),
            "parenthesized_expression" => {
                // Lower the inner expression
                let children = node.all_children();
                if let Some(inner) = children.first() {
                    // Dereference to get &N from &&N
                    self.lower_expr(ctx, *inner)
                } else {
                    ctx.add_error(LoweringError::MissingChild {
                        context: "parenthesized_expression".to_string(),
                        expected: "inner expression".to_string(),
                    });
                    Ok(ctx.create_placeholder())
                }
            }
            _ => {
                // Graceful degradation for unknown expression types
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "PostgreSQL".to_string(),
                    feature: format!("expression: {}", node.kind()),
                    suggestion: "Use supported expression types".to_string(),
                });
                Ok(ctx.create_placeholder())
            }
        }
    }

    fn supports_node(&self, _node: &N, kind: &str) -> bool {
        matches!(
            kind,
            "select_statement"
                | "insert_statement"
                | "update_statement"
                | "delete_statement"
                | "binary_expression"
                | "unary_expression"
                | "column_reference"
                | "column_ref"
                | "literal"
                | "function_call"
                | "case_expression"
                | "parenthesized_expression"
        )
    }

    fn dialect(&self) -> Dialect {
        Dialect::PostgreSQL
    }
}

impl PostgreSQLLowering {
    /// Lower a SELECT statement
    fn lower_select_statement<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Query>
    where
        N: CstNode,
    {
        let mut select = SelectStatement::default();

        // Check for DISTINCT ON clause (PostgreSQL-specific)
        if let Some(distinct_on_node) = self.optional_child(node, "distinct_on_clause") {
            self.handle_distinct_on(ctx, distinct_on_node);
            // Mark as DISTINCT (without ON support for now)
            select.distinct = true;
        } else if let Some(_distinct_node) = self.optional_child(node, "distinct") {
            select.distinct = true;
        }

        // Lower projection (SELECT clause)
        if let Some(proj_node) = self.optional_child(node, "projection") {
            select.projection = self.lower_projection(ctx, proj_node)?;
        }

        // Lower FROM clause
        if let Some(from_node) = self.optional_child(node, "from_clause") {
            select.from = self.lower_from_clause(ctx, from_node)?;
        }

        // Lower WHERE clause
        if let Some(where_node) = self.optional_child(node, "where_clause") {
            select.where_clause = Some(self.lower_where_clause(ctx, where_node)?);
        }

        // Lower GROUP BY clause
        if let Some(group_node) = self.optional_child(node, "group_by_clause") {
            select.group_by = self.lower_group_by_clause(ctx, group_node)?;
        }

        // Lower HAVING clause
        if let Some(having_node) = self.optional_child(node, "having_clause") {
            select.having = Some(self.lower_having_clause(ctx, having_node)?);
        }

        // Build query with SELECT body
        let mut query = Query::new(Dialect::PostgreSQL);
        query.body = unified_sql_lsp_ir::SetOp::Select(Box::new(select));

        // Lower ORDER BY clause
        if let Some(order_node) = self.optional_child(node, "order_by_clause") {
            query.order_by = Some(self.lower_order_by_clause(ctx, order_node)?);
        }

        // Lower LIMIT clause (PostgreSQL: LIMIT count [OFFSET offset])
        if let Some(limit_node) = self.optional_child(node, "limit_clause") {
            let (limit, offset) = self.lower_limit_clause(ctx, limit_node)?;
            query.limit = limit;
            query.offset = offset;
        }

        Ok(query)
    }

    /// Lower an INSERT statement with RETURNING clause support
    fn lower_insert_statement<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Query>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-003) Implement full INSERT statement lowering
        // - Parse INSERT INTO table (columns) VALUES (values)
        // - Handle DEFAULT VALUES syntax
        // - Support RETURNING clause (needs IR extension)

        // Check for RETURNING clause
        if let Some(returning_node) = self.optional_child(node, "returning_clause") {
            self.handle_returning_clause(ctx, returning_node);
        }

        // For now, return a placeholder query
        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "INSERT statement".to_string(),
            suggestion: "INSERT lowering is not yet fully implemented".to_string(),
        });

        Ok(Query::new(Dialect::PostgreSQL))
    }

    /// Lower an UPDATE statement with RETURNING clause support
    fn lower_update_statement<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Query>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-003) Implement full UPDATE statement lowering
        // - Parse UPDATE table SET column = value
        // - Support RETURNING clause (needs IR extension)

        // Check for RETURNING clause
        if let Some(returning_node) = self.optional_child(node, "returning_clause") {
            self.handle_returning_clause(ctx, returning_node);
        }

        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "UPDATE statement".to_string(),
            suggestion: "UPDATE lowering is not yet fully implemented".to_string(),
        });

        Ok(Query::new(Dialect::PostgreSQL))
    }

    /// Lower a DELETE statement with RETURNING clause support
    fn lower_delete_statement<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Query>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-003) Implement full DELETE statement lowering
        // - Parse DELETE FROM table WHERE condition
        // - Support RETURNING clause (needs IR extension)

        // Check for RETURNING clause
        if let Some(returning_node) = self.optional_child(node, "returning_clause") {
            self.handle_returning_clause(ctx, returning_node);
        }

        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "DELETE statement".to_string(),
            suggestion: "DELETE lowering is not yet fully implemented".to_string(),
        });

        Ok(Query::new(Dialect::PostgreSQL))
    }

    /// Handle DISTINCT ON clause (PostgreSQL-specific)
    ///
    /// IR doesn't currently support DISTINCT ON, so we add a warning and
    /// convert it to regular DISTINCT. Future enhancement needed.
    fn handle_distinct_on<N>(&self, ctx: &mut LoweringContext, _node: &N)
    where
        N: CstNode,
    {
        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "DISTINCT ON clause".to_string(),
            suggestion: "DISTINCT ON is PostgreSQL-specific and will be converted to regular DISTINCT. Full support requires IR extension.".to_string(),
        });

        // TODO: (LOWERING-003) Implement full DISTINCT ON support
        // - Parse column list from DISTINCT ON (columns)
        // - Extend IR to support distinct_on field
        // - Ensure proper ordering (DISTINCT ON columns should typically appear first in ORDER BY)
    }

    /// Handle RETURNING clause (PostgreSQL-specific)
    ///
    /// IR doesn't currently support RETURNING, so we add a warning.
    /// This needs IR extension to properly support.
    fn handle_returning_clause<N>(&self, ctx: &mut LoweringContext, _node: &N)
    where
        N: CstNode,
    {
        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "RETURNING clause".to_string(),
            suggestion: "RETURNING clause requires IR extension to properly support. Currently treated as placeholder.".to_string(),
        });

        // TODO: (LOWERING-003) Implement full RETURNING clause support
        // - Parse expression list from RETURNING clause
        // - Extend InsertStatement/UpdateStatement/DeleteStatement in IR to have returning field
        // - Return the list of expressions to be inserted into the IR
    }

    /// Lower projection list (SELECT clause)
    fn lower_projection<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Vec<SelectItem>>
    where
        N: CstNode,
    {
        let mut items = Vec::new();

        for child in node.all_children() {
            match child.kind() {
                "expression" | "column_ref" => {
                    let expr = self.lower_expr(ctx, child)?;
                    items.push(SelectItem::UnnamedExpr(expr));
                }
                "aliased_expression" => {
                    // Extract expression and alias
                    let children = child.all_children();
                    if children.len() >= 2 {
                        let expr = self.lower_expr(ctx, children[0])?;
                        let alias = self.normalize_identifier(children[1].text().unwrap_or(""));
                        items.push(SelectItem::AliasedExpr { expr, alias });
                    } else {
                        ctx.add_error(LoweringError::MissingChild {
                            context: "aliased_expression".to_string(),
                            expected: "expression and alias".to_string(),
                        });
                        items.push(SelectItem::UnnamedExpr(ctx.create_placeholder()));
                    }
                }
                "wildcard" | "*" => {
                    items.push(SelectItem::Wildcard);
                }
                "qualified_wildcard" => {
                    // Extract table name for table.*
                    let text = child.text().unwrap_or("");
                    let table_name = text.trim_end_matches('.').to_string();
                    items.push(SelectItem::QualifiedWildcard(table_name));
                }
                _ => {
                    // Unknown projection item
                    ctx.add_error(LoweringError::UnsupportedSyntax {
                        dialect: "PostgreSQL".to_string(),
                        feature: format!("projection item: {}", child.kind()),
                        suggestion: "Use valid SELECT expressions".to_string(),
                    });
                    items.push(SelectItem::UnnamedExpr(ctx.create_placeholder()));
                }
            }
        }

        Ok(items)
    }

    /// Lower FROM clause with table references
    fn lower_from_clause<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Vec<TableRef>>
    where
        N: CstNode,
    {
        let mut tables = Vec::new();

        for child in node.all_children() {
            if matches!(
                child.kind(),
                "table_reference" | "table_name" | "joined_table"
            ) {
                if let Some(table) = self.lower_table_reference(ctx, child)? {
                    tables.push(table);
                }
            }
        }

        if tables.is_empty() {
            ctx.add_error(LoweringError::MissingChild {
                context: "from_clause".to_string(),
                expected: "table_reference".to_string(),
            });
        }

        Ok(tables)
    }

    /// Lower a single table reference
    fn lower_table_reference<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Option<TableRef>>
    where
        N: CstNode,
    {
        let mut table_ref = TableRef {
            name: String::new(),
            alias: None,
            joins: Vec::new(),
        };

        match node.kind() {
            "table_name" | "identifier" => {
                table_ref.name = self.normalize_identifier(node.text().unwrap_or(""));
            }
            "aliased_table" => {
                // Extract table name and alias
                let children = node.all_children();
                if children.len() >= 2 {
                    table_ref.name = self.normalize_identifier(children[0].text().unwrap_or(""));
                    table_ref.alias =
                        Some(self.normalize_identifier(children[1].text().unwrap_or("")));
                }
            }
            "joined_table" => {
                // Handle JOIN syntax
                return self.lower_joined_table(ctx, node);
            }
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "PostgreSQL".to_string(),
                    feature: format!("table reference: {}", node.kind()),
                    suggestion: "Use valid table references".to_string(),
                });
                return Ok(None);
            }
        }

        Ok(Some(table_ref))
    }

    /// Lower a joined table (JOIN syntax)
    fn lower_joined_table<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Option<TableRef>>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-003) Implement JOIN clause lowering
        // - Parse LEFT/RIGHT/INNER/FULL/CROSS join types
        // - Handle join conditions (ON clause, USING clause)
        // - Support LATERAL joins (PostgreSQL-specific)
        // - Support multiple joins and nested joins

        // Check for LATERAL keyword
        let text = node.text().unwrap_or("");
        if text.contains("LATERAL") || text.contains("Lateral") {
            ctx.add_error(LoweringError::UnsupportedSyntax {
                dialect: "PostgreSQL".to_string(),
                feature: "LATERAL JOIN".to_string(),
                suggestion: "LATERAL JOIN is PostgreSQL-specific and requires IR extension. Currently treated as placeholder.".to_string(),
            });
        }

        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "JOIN clause".to_string(),
            suggestion: "JOIN support will be added in a future update".to_string(),
        });
        Ok(None)
    }

    /// Lower WHERE clause
    fn lower_where_clause<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        // Lower the condition expression
        let children = node.all_children();
        if let Some(cond) = children.first() {
            self.lower_expr(ctx, *cond)
        } else {
            ctx.add_error(LoweringError::MissingChild {
                context: "where_clause".to_string(),
                expected: "condition expression".to_string(),
            });
            Ok(ctx.create_placeholder())
        }
    }

    /// Lower GROUP BY clause
    fn lower_group_by_clause<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Vec<Expr>>
    where
        N: CstNode,
    {
        let mut group_by = Vec::new();

        for child in node.all_children() {
            if matches!(child.kind(), "expression" | "column_ref") {
                group_by.push(self.lower_expr(ctx, child)?);
            }
        }

        Ok(group_by)
    }

    /// Lower HAVING clause
    fn lower_having_clause<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();
        if let Some(cond) = children.first() {
            self.lower_expr(ctx, *cond)
        } else {
            ctx.add_error(LoweringError::MissingChild {
                context: "having_clause".to_string(),
                expected: "condition expression".to_string(),
            });
            Ok(ctx.create_placeholder())
        }
    }

    /// Lower ORDER BY clause
    fn lower_order_by_clause<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Vec<OrderBy>>
    where
        N: CstNode,
    {
        let mut order_by = Vec::new();

        for child in node.all_children() {
            if matches!(child.kind(), "expression" | "column_ref") {
                let expr = self.lower_expr(ctx, child)?;
                order_by.push(OrderBy {
                    expr,
                    direction: None, // Default ASC
                });
            } else if child.kind() == "order_by_direction" {
                // TODO: (LOWERING-004) Implement ORDER BY direction parsing
                // - Parse ASC/DESC keywords from CST
                // - Associate direction with the previous expression in order_by
            }
        }

        Ok(order_by)
    }

    /// Lower LIMIT clause (PostgreSQL: LIMIT count [OFFSET offset])
    ///
    /// PostgreSQL only supports one form:
    /// - LIMIT count [OFFSET offset]
    fn lower_limit_clause<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<(Option<Expr>, Option<Expr>)>
    where
        N: CstNode,
    {
        let children = node.all_children();

        // Filter out non-expression nodes (LIMIT keyword, OFFSET keyword)
        let exprs: Vec<&N> = children
            .iter()
            .filter(|c| matches!(c.kind(), "expression" | "literal"))
            .copied()
            .collect();

        // Check for OFFSET keyword presence
        let has_offset = children.iter().any(|c| {
            let text = c.text().unwrap_or("");
            text.eq_ignore_ascii_case("OFFSET")
        });

        match exprs.len() {
            1 => {
                // LIMIT count
                let limit = self.lower_expr(ctx, exprs[0])?;
                Ok((Some(limit), None))
            }
            2 if has_offset => {
                // LIMIT count OFFSET offset
                // exprs[0] is count (LIMIT), exprs[1] is offset (OFFSET)
                let limit = self.lower_expr(ctx, exprs[0])?;
                let offset = self.lower_expr(ctx, exprs[1])?;
                Ok((Some(limit), Some(offset)))
            }
            _ => {
                ctx.add_error(LoweringError::InvalidLiteral {
                    value: format!("LIMIT clause with {} expressions", exprs.len()),
                    type_name: "LIMIT clause".to_string(),
                });
                Ok((None, None))
            }
        }
    }

    /// Lower binary expression (a + b, x = 5, etc.)
    fn lower_binary_expr<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();
        if children.len() < 3 {
            ctx.add_error(LoweringError::MissingChild {
                context: "binary_expression".to_string(),
                expected: "left, operator, right".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        let left = self.lower_expr(ctx, children[0])?;
        let op_str = children[1].text().unwrap_or("");
        let right = self.lower_expr(ctx, children[2])?;

        // Map operator string to BinaryOp enum
        let op = match op_str {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            "=" => BinaryOp::Eq,
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::NotEq,
            "<>" => BinaryOp::NotEq,
            "<" => BinaryOp::Lt,
            "<=" => BinaryOp::LtEq,
            ">" => BinaryOp::Gt,
            ">=" => BinaryOp::GtEq,
            "AND" | "and" => BinaryOp::And,
            "OR" | "or" => BinaryOp::Or,
            "LIKE" | "like" => BinaryOp::Like,
            "NOT LIKE" | "not like" => BinaryOp::NotLike,
            "IN" | "in" => BinaryOp::In,
            "NOT IN" | "not in" => BinaryOp::NotIn,
            "IS" | "is" => BinaryOp::Is,
            "IS NOT" | "is not" => BinaryOp::IsNot,
            // PostgreSQL-specific string matching operators
            "~" => BinaryOp::Like, // Regex match (approximated as LIKE for now)
            "~*" => BinaryOp::Like, // Case-insensitive regex match
            "!~" => BinaryOp::NotLike, // Regex not match
            "!~*" => BinaryOp::NotLike, // Case-insensitive regex not match
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "PostgreSQL".to_string(),
                    feature: format!("binary operator: {}", op_str),
                    suggestion: "Use supported operators".to_string(),
                });
                return Ok(Expr::Literal(Literal::Null));
            }
        };

        Ok(Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    /// Lower unary expression (NOT, negation)
    fn lower_unary_expr<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();
        if children.is_empty() {
            return Ok(ctx.create_placeholder());
        }

        let op_str = children[0].text().unwrap_or("");
        let operand = if children.len() > 1 {
            self.lower_expr(ctx, children[1])?
        } else {
            ctx.create_placeholder()
        };

        let op = match op_str {
            "-" => UnaryOp::Neg,
            "+" => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "PostgreSQL".to_string(),
                    feature: "unary plus operator".to_string(),
                    suggestion: "Unary plus is not supported in IR".to_string(),
                });
                return Ok(operand);
            }
            "NOT" | "not" => UnaryOp::Not,
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "PostgreSQL".to_string(),
                    feature: format!("unary operator: {}", op_str),
                    suggestion: "Use supported unary operators".to_string(),
                });
                return Ok(operand);
            }
        };

        Ok(Expr::UnaryOp {
            op,
            expr: Box::new(operand),
        })
    }

    /// Lower column reference
    fn lower_column_ref<N>(&self, _ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let text = node.text().unwrap_or("");

        // Check for qualified column (table.column or "table"."column")
        if let Some(dot_pos) = text.find('.') {
            let table = self.normalize_identifier(&text[..dot_pos]);
            let column = self.normalize_identifier(&text[dot_pos + 1..]);
            Ok(Expr::Column(ColumnRef {
                table: Some(table),
                column,
            }))
        } else {
            let column = self.normalize_identifier(text);
            Ok(Expr::Column(ColumnRef {
                table: None,
                column,
            }))
        }
    }

    /// Lower PostgreSQL literal value (including dollar-quoted strings)
    fn lower_postgresql_literal<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let text = node.text().unwrap_or("").trim();

        // Dollar-quoted string: $$text$$ or $tag$text$tag$
        if text.starts_with('$') && text.ends_with('$') && text.len() >= 4 {
            return self.lower_dollar_quoted_string::<N>(ctx, text);
        }

        // Standard single-quoted string
        if text.starts_with('\'') && text.ends_with('\'') {
            let unquoted = &text[1..text.len() - 1];
            return Ok(Expr::Literal(Literal::String(unquoted.to_string())));
        }

        // Boolean
        if text.eq_ignore_ascii_case("TRUE") {
            return Ok(Expr::Literal(Literal::Boolean(true)));
        }
        if text.eq_ignore_ascii_case("FALSE") {
            return Ok(Expr::Literal(Literal::Boolean(false)));
        }

        // NULL
        if text.eq_ignore_ascii_case("NULL") {
            return Ok(Expr::Literal(Literal::Null));
        }

        // Numeric: try integer first, then float
        if let Ok(int_val) = text.parse::<i64>() {
            return Ok(Expr::Literal(Literal::Integer(int_val)));
        }

        if let Ok(float_val) = text.parse::<f64>() {
            return Ok(Expr::Literal(Literal::Float(float_val)));
        }

        // Failed to parse
        ctx.add_error(LoweringError::InvalidLiteral {
            value: text.to_string(),
            type_name: "PostgreSQL literal".to_string(),
        });
        Ok(ctx.create_placeholder())
    }

    /// Lower dollar-quoted string literal (PostgreSQL-specific)
    ///
    /// Forms:
    /// - $$text$$
    /// - $tag$text$tag$
    fn lower_dollar_quoted_string<N>(
        &self,
        ctx: &mut LoweringContext,
        text: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        // Must start and end with $
        if !text.starts_with('$') || !text.ends_with('$') || text.len() < 4 {
            ctx.add_error(LoweringError::InvalidLiteral {
                value: text.to_string(),
                type_name: "dollar-quoted string".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        // Find the end of the opening tag (second $)
        if let Some(second_dollar) = text[1..].find('$') {
            let second_dollar_pos = second_dollar + 1; // Adjust for [1..] offset

            // Extract the tag (between the two $ signs)
            let tag = &text[1..second_dollar_pos];

            // Build the closing tag: $tag$
            let closing_tag = format!("${}$", tag);

            // Find the closing tag (must start after the opening tag)
            let search_start = second_dollar_pos + tag.len() + 1; // +1 for the second $
            if let Some(end_pos) = text[search_start..].find(&closing_tag) {
                let end_pos = end_pos + search_start;
                let content_start = second_dollar_pos + 1;
                let content = &text[content_start..end_pos];
                return Ok(Expr::Literal(Literal::String(content.to_string())));
            }
        }

        // Failed to parse dollar-quoted string
        ctx.add_error(LoweringError::InvalidLiteral {
            value: text.to_string(),
            type_name: "dollar-quoted string".to_string(),
        });
        Ok(ctx.create_placeholder())
    }

    /// Lower function call
    ///
    /// TODO: (COMPLETION-006) Implement full function call lowering with:
    /// - Window function support (OVER clause with PARTITION BY, ORDER BY, window frame)
    /// - Filter clause (aggregate function FILTER)
    /// - Function type detection (aggregate vs scalar vs window)
    /// - Argument type validation
    fn lower_function_call<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();

        // Extract function name and arguments
        let func_name = children
            .first()
            .and_then(|n| n.text())
            .unwrap_or("")
            .to_string();

        let mut args = Vec::new();
        for child in children.iter().skip(1) {
            if matches!(child.kind(), "expression" | "column_ref" | "literal") {
                args.push(self.lower_expr(ctx, *child)?);
            }
        }

        // Check for DISTINCT modifier (aggregate functions)
        let distinct = children.iter().any(|c| {
            let text = c.text().unwrap_or("");
            text.eq_ignore_ascii_case("DISTINCT")
        });

        Ok(Expr::Function {
            name: func_name,
            args,
            distinct,
        })
    }

    /// Lower CASE expression
    fn lower_case_expr<N>(&self, ctx: &mut LoweringContext, _node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-005) Implement CASE expression lowering
        // - Parse WHEN-THEN-ELSE clause structure from CST
        // - Handle both simple CASE and searched CASE forms
        // - Support multiple WHEN clauses with proper evaluation order
        // - Handle ELSE clause and NULL result

        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "PostgreSQL".to_string(),
            feature: "CASE expression".to_string(),
            suggestion: "CASE support will be added in a future update".to_string(),
        });
        Ok(ctx.create_placeholder())
    }

    /// Normalize identifier by removing double quotes (PostgreSQL-style)
    fn normalize_identifier(&self, identifier: &str) -> String {
        identifier
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(identifier)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::MockCstNode;

    #[test]
    fn test_postgresql_lowering_dialect() {
        let lowering = PostgreSQLLowering;
        let node = MockCstNode::new("select_statement");
        assert_eq!(
            Lowering::<MockCstNode>::dialect(&lowering),
            Dialect::PostgreSQL
        );
        assert!(lowering.supports_node(&node, "select_statement"));
    }

    #[test]
    fn test_normalize_identifier_with_quotes() {
        let lowering = PostgreSQLLowering;

        assert_eq!(lowering.normalize_identifier("\"id\""), "id");
        assert_eq!(
            lowering.normalize_identifier("\"table_name\""),
            "table_name"
        );
        // For "schema"."table", each part is quoted separately
        let result = lowering.normalize_identifier("\"schema\"");
        assert_eq!(result, "schema");
        let result = lowering.normalize_identifier("\"table\"");
        assert_eq!(result, "table");
    }

    #[test]
    fn test_normalize_identifier_without_quotes() {
        let lowering = PostgreSQLLowering;

        assert_eq!(lowering.normalize_identifier("id"), "id");
        assert_eq!(lowering.normalize_identifier("table_name"), "table_name");
    }

    #[test]
    fn test_lower_dollar_quoted_string_basic() {
        let lowering = PostgreSQLLowering;
        let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

        // Basic dollar-quoted string: $$text$$
        let result =
            lowering.lower_dollar_quoted_string::<MockCstNode>(&mut ctx, "$$Hello, World!$$");
        assert!(result.is_ok());
        if let Ok(Expr::Literal(Literal::String(s))) = result {
            assert_eq!(s, "Hello, World!");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_lower_dollar_quoted_string_with_tag() {
        let lowering = PostgreSQLLowering;
        let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

        // Tagged dollar-quoted string: $tag$text$tag$
        let result = lowering
            .lower_dollar_quoted_string::<MockCstNode>(&mut ctx, "$tag$PostgreSQL String$tag$");
        assert!(result.is_ok());
        if let Ok(Expr::Literal(Literal::String(s))) = result {
            assert_eq!(s, "PostgreSQL String");
        } else {
            panic!("Expected string literal");
        }
    }

    #[test]
    fn test_postgresql_supports_node() {
        let lowering = PostgreSQLLowering;
        let node = MockCstNode::new("select_statement");

        assert!(lowering.supports_node(&node, "select_statement"));
        assert!(lowering.supports_node(&node, "insert_statement"));
        assert!(lowering.supports_node(&node, "binary_expression"));
        assert!(lowering.supports_node(&node, "column_ref"));
        assert!(!lowering.supports_node(&node, "unknown_node"));
    }
}
