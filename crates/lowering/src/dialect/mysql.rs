// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! MySQL CST → IR lowering implementation
//!
//! This module provides MySQL-specific lowering from Tree-sitter CST to the unified IR.
//! It handles MySQL-specific syntax including:
//! - LIMIT clause with three forms (count, count OFFSET offset, offset,count)
//! - Backtick identifiers
//! - REPLACE statement
//! - SELECT modifiers (SQL_CALC_FOUND_ROWS, SQL_CACHE, SQL_NO_CACHE)
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

/// MySQL CST → IR lowering implementation
pub struct MySQLLowering;

impl<N> DialectLoweringBase<N> for MySQLLowering where N: CstNode {}

impl<N> Lowering<N> for MySQLLowering
where
    N: CstNode,
{
    fn lower_query(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Query> {
        match node.kind() {
            "select_statement" => self.lower_select_statement(ctx, node),
            "replace_statement" => self.lower_replace_statement(ctx, node),
            _ => Err(LoweringError::UnexpectedNodeType {
                expected: "SELECT or REPLACE statement".to_string(),
                found: node.kind().to_string(),
            }),
        }
    }

    fn lower_expr(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr> {
        match node.kind() {
            "binary_expression" => self.lower_binary_expr(ctx, node),
            "unary_expression" => self.lower_unary_expr(ctx, node),
            "column_reference" | "column_ref" => self.lower_column_ref(ctx, node),
            "literal" => self.lower_literal(ctx, node),
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
                    dialect: "MySQL".to_string(),
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
                | "replace_statement"
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
        Dialect::MySQL
    }
}

impl MySQLLowering {
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

        // Check for SELECT modifiers (MySQL-specific)
        if let Some(modifiers) = node.children("select_modifier").first() {
            self.handle_select_modifiers(ctx, *modifiers);
        }

        // Build query with SELECT body
        let mut query = Query::new(Dialect::MySQL);
        query.body = unified_sql_lsp_ir::SetOp::Select(Box::new(select));

        // Lower ORDER BY clause
        if let Some(order_node) = self.optional_child(node, "order_by_clause") {
            query.order_by = Some(self.lower_order_by_clause(ctx, order_node)?);
        }

        // Lower LIMIT clause (MySQL-specific with 3 forms)
        if let Some(limit_node) = self.optional_child(node, "limit_clause") {
            let (limit, offset) = self.lower_limit_clause(ctx, limit_node)?;
            query.limit = limit;
            query.offset = offset;
        }

        Ok(query)
    }

    /// Lower a REPLACE statement (converts to INSERT in IR)
    fn lower_replace_statement<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<Query>
    where
        N: CstNode,
    {
        // REPLACE INTO table VALUES (...)
        // Convert to INSERT structure for now
        // TODO: (LOWERING-002) Track REPLACE semantics in query metadata
        // REPLACE differs from INSERT in that it deletes duplicate rows before inserting
        // This should be tracked separately once IR supports mutation operation metadata

        let mut select = SelectStatement::default();

        // Extract table name from REPLACE
        if let Some(table_node) = self.optional_child(node, "table_name") {
            let table_name = self.normalize_identifier(table_node.text().unwrap_or(""));
            select.from = vec![TableRef {
                name: table_name,
                alias: None,
                joins: Vec::new(),
            }];
        }

        let mut query = Query::new(Dialect::MySQL);
        query.body = unified_sql_lsp_ir::SetOp::Select(Box::new(select));

        Ok(query)
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
                        let alias = children[1].text().unwrap_or("").to_string();
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
                        dialect: "MySQL".to_string(),
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
                    dialect: "MySQL".to_string(),
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
        // - Parse LEFT/RIGHT/INNER/CROSS join types
        // - Handle join conditions (ON clause)
        // - Support multiple joins and nested joins
        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
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
                // - Handle multiple ordering expressions with different directions
            }
        }

        Ok(order_by)
    }

    /// Lower LIMIT clause (MySQL-specific: 3 forms)
    ///
    /// Forms:
    /// 1. LIMIT count → (Some(count), None)
    /// 2. LIMIT count OFFSET offset → (Some(count), Some(offset))
    /// 3. LIMIT offset, count → (Some(count), Some(offset))  [MySQL-specific comma form]
    fn lower_limit_clause<N>(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> LoweringResult<(Option<Expr>, Option<Expr>)>
    where
        N: CstNode,
    {
        let children = node.all_children();

        // Filter out non-expression nodes (LIMIT keyword, commas, OFFSET keyword)
        let exprs: Vec<&N> = children
            .iter()
            .filter(|c| matches!(c.kind(), "expression" | "literal"))
            .copied()
            .collect();

        // Check for comma separator (MySQL's LIMIT offset, count syntax)
        let has_comma = children.iter().any(|c| c.kind() == ",");

        match (exprs.len(), has_comma) {
            (1, false) => {
                // Form 1: LIMIT count
                let limit = self.lower_expr(ctx, exprs[0])?;
                Ok((Some(limit), None))
            }
            (2, true) => {
                // Form 3: LIMIT offset, count (comma form)
                // exprs[0] is offset, exprs[1] is count
                let offset = self.lower_expr(ctx, exprs[0])?;
                let limit = self.lower_expr(ctx, exprs[1])?;
                Ok((Some(limit), Some(offset)))
            }
            (2, false) => {
                // Form 2: LIMIT count OFFSET offset
                // exprs[0] is count, exprs[1] is offset
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

    /// Handle SELECT modifiers (SQL_CALC_FOUND_ROWS, SQL_CACHE, SQL_NO_CACHE)
    fn handle_select_modifiers<N>(&self, ctx: &mut LoweringContext, node: &N)
    where
        N: CstNode,
    {
        let modifier = node.text().unwrap_or("");

        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
            feature: format!("SELECT modifier: {}", modifier),
            suggestion: "This modifier will be ignored in analysis".to_string(),
        });
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
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "MySQL".to_string(),
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
                    dialect: "MySQL".to_string(),
                    feature: "unary plus operator".to_string(),
                    suggestion: "Unary plus is not supported in IR".to_string(),
                });
                return Ok(operand);
            }
            "NOT" | "not" | "!" => UnaryOp::Not,
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: "MySQL".to_string(),
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
    fn lower_column_ref<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let text = node.text().unwrap_or("");

        // Check for qualified column (table.column)
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

    /// Lower literal value
    fn lower_literal<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let text = node.text().unwrap_or("").trim();

        // String literal
        if (text.starts_with('\'') && text.ends_with('\''))
            || (text.starts_with('"') && text.ends_with('"'))
        {
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
            type_name: "literal".to_string(),
        });
        Ok(ctx.create_placeholder())
    }

    /// Lower function call
    ///
    /// TODO: (COMPLETION-006) Implement full function call lowering with:
    /// - DISTINCT modifier parsing (already done for PostgreSQL, add for MySQL)
    /// - Window function support (OVER clause)
    /// - Aggregate function handling
    /// - Function type detection (aggregate vs scalar vs window)
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

        // TODO: (COMPLETION-006) Parse DISTINCT modifier (currently hardcoded to false)
        Ok(Expr::Function {
            name: func_name,
            args,
            distinct: false,
        })
    }

    /// Lower CASE expression
    fn lower_case_expr<N>(&self, ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        // TODO: (LOWERING-005) Implement CASE expression lowering
        // - Parse WHEN-THEN-ELSE clause structure from CST
        // - Handle both simple CASE and searched CASE forms
        // - Support multiple WHEN clauses with proper evaluation order
        // - Handle ELSE clause and NULL result
        ctx.add_error(LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
            feature: "CASE expression".to_string(),
            suggestion: "CASE support will be added in a future update".to_string(),
        });
        Ok(ctx.create_placeholder())
    }

    /// Normalize identifier by removing backticks
    fn normalize_identifier(&self, identifier: &str) -> String {
        identifier
            .strip_prefix('`')
            .and_then(|s| s.strip_suffix('`'))
            .unwrap_or(identifier)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::MockCstNode;

    #[test]
    fn test_mysql_lowering_dialect() {
        let lowering = MySQLLowering;
        let node = MockCstNode::new("select_statement");
        assert_eq!(Lowering::<MockCstNode>::dialect(&lowering), Dialect::MySQL);
        assert!(lowering.supports_node(&node, "select_statement"));
    }

    #[test]
    fn test_normalize_identifier_with_backticks() {
        let lowering = MySQLLowering;

        assert_eq!(lowering.normalize_identifier("`id`"), "id");
        assert_eq!(lowering.normalize_identifier("`table_name`"), "table_name");
        // For `db`.`table`, only outer backticks are stripped
        let result = lowering.normalize_identifier("`db`.`table`");
        assert!(result.contains("db"));
        assert!(result.contains("table"));
    }

    #[test]
    fn test_normalize_identifier_without_backticks() {
        let lowering = MySQLLowering;

        assert_eq!(lowering.normalize_identifier("id"), "id");
        assert_eq!(lowering.normalize_identifier("table_name"), "table_name");
    }

    #[test]
    fn test_mysql_supports_node() {
        let lowering = MySQLLowering;
        let node = MockCstNode::new("select_statement");

        assert!(lowering.supports_node(&node, "select_statement"));
        assert!(lowering.supports_node(&node, "binary_expression"));
        assert!(lowering.supports_node(&node, "column_ref"));
        assert!(!lowering.supports_node(&node, "unknown_node"));
    }
}
