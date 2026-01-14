// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Shared lowering logic for all SQL dialects
//!
//! This module provides common lowering implementations that are shared
//! across MySQL, PostgreSQL, and other dialects to reduce code duplication.

use crate::{CstNode, LoweringContext, LoweringError, LoweringResult};
use unified_sql_lsp_ir::expr::{BinaryOp, ColumnRef, Literal, UnaryOp};
use unified_sql_lsp_ir::query::{OrderBy, SelectItem, SortDirection, TableRef};
use unified_sql_lsp_ir::{Expr, Join, JoinCondition, JoinType};

/// Shared lowering utilities for all dialects
pub struct SharedLowering;

impl SharedLowering {
    /// Lower binary expression (a + b, x = 5, etc.)
    ///
    /// This implementation is shared across all dialects with support for
    /// dialect-specific operator extensions via the dialect_name parameter.
    pub fn lower_binary_expr<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
        dialect_operators: Option<&[(&str, BinaryOp)]>,
        lower_fn: E,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let children = node.all_children();
        if children.len() < 3 {
            ctx.add_error(LoweringError::MissingChild {
                context: "binary_expression".to_string(),
                expected: "left, operator, right".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        let left = lower_fn(ctx, children[0])?;
        let op_str = children[1].text().unwrap_or("");
        let right = lower_fn(ctx, children[2])?;

        let op = Self::parse_binary_op(op_str, dialect_name, dialect_operators, ctx);

        Ok(Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
        })
    }

    /// Parse a binary operator string into a BinaryOp enum
    fn parse_binary_op(
        op_str: &str,
        dialect_name: &str,
        dialect_operators: Option<&[(&str, BinaryOp)]>,
        ctx: &mut LoweringContext,
    ) -> BinaryOp {
        // Standard SQL operators

        match op_str {
            "+" => BinaryOp::Add,
            "-" => BinaryOp::Sub,
            "*" => BinaryOp::Mul,
            "/" => BinaryOp::Div,
            "%" => BinaryOp::Mod,
            "=" | "==" => BinaryOp::Eq,
            "!=" | "<>" => BinaryOp::NotEq,
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
                // Check dialect-specific operators
                if let Some(dialect_ops) = dialect_operators {
                    for &(variant, op) in dialect_ops {
                        if variant == op_str {
                            return op;
                        }
                    }
                }

                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: dialect_name.to_string(),
                    feature: format!("binary operator: {}", op_str),
                    suggestion: "Use supported operators".to_string(),
                });
                return BinaryOp::Eq; // Default fallback
            }
        }
    }

    /// Lower unary expression (NOT, negation)
    ///
    /// Shared implementation for all dialects.
    pub fn lower_unary_expr<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();
        if children.is_empty() {
            return Ok(ctx.create_placeholder());
        }

        let op_str = children[0].text().unwrap_or("");
        let operand = if children.len() > 1 {
            Self::lower_expr_generic(ctx, children[1], dialect_name)?
        } else {
            ctx.create_placeholder()
        };

        let op = match op_str {
            "-" => UnaryOp::Neg,
            "+" => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: dialect_name.to_string(),
                    feature: "unary plus operator".to_string(),
                    suggestion: "Unary plus is not supported in IR".to_string(),
                });
                return Ok(operand);
            }
            "NOT" | "not" | "!" => UnaryOp::Not,
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: dialect_name.to_string(),
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

    /// Lower column reference (table.column or column)
    ///
    /// Shared implementation that handles qualified and unqualified column names.
    /// Dialect-specific identifier normalization is handled via the normalize_fn parameter.
    pub fn lower_column_ref<N, F>(
        _ctx: &mut LoweringContext,
        node: &N,
        normalize_fn: F,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
    {
        let text = node.text().unwrap_or("");

        // Check for qualified column (table.column)
        if let Some(dot_pos) = text.find('.') {
            let table = normalize_fn(&text[..dot_pos]);
            let column = normalize_fn(&text[dot_pos + 1..]);
            Ok(Expr::Column(ColumnRef {
                table: Some(table),
                column,
            }))
        } else {
            let column = normalize_fn(text);
            Ok(Expr::Column(ColumnRef {
                table: None,
                column,
            }))
        }
    }

    /// Lower literal value (string, number, boolean, NULL)
    ///
    /// Shared implementation for standard SQL literals.
    /// Dialect-specific literal handling (e.g., dollar-quoted strings) should
    /// be handled by the dialect implementation before calling this.
    pub fn lower_literal<N>(ctx: &mut LoweringContext, node: &N) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let text = node.text().unwrap_or("").trim();

        // String literal (single or double quoted)
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

    /// Lower WHERE clause
    pub fn lower_where_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let lower_expr =
            |ctx: &mut LoweringContext, n: &N| Self::lower_expr_generic(ctx, n, dialect_name);
        Self::lower_where_clause_with(ctx, node, dialect_name, lower_expr)
    }

    /// Lower WHERE clause with custom expression lowering function
    pub fn lower_where_clause_with<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        _dialect_name: &str,
        lower_expr: E,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let children = node.all_children();
        if let Some(cond) = children.first() {
            lower_expr(ctx, *cond)
        } else {
            ctx.add_error(LoweringError::MissingChild {
                context: "where_clause".to_string(),
                expected: "condition expression".to_string(),
            });
            Ok(ctx.create_placeholder())
        }
    }

    /// Lower GROUP BY clause
    pub fn lower_group_by_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Vec<Expr>>
    where
        N: CstNode,
    {
        let lower_expr =
            |ctx: &mut LoweringContext, n: &N| Self::lower_expr_generic(ctx, n, dialect_name);
        Self::lower_group_by_clause_with(ctx, node, dialect_name, lower_expr)
    }

    /// Lower GROUP BY clause with custom expression lowering function
    pub fn lower_group_by_clause_with<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        _dialect_name: &str,
        lower_expr: E,
    ) -> LoweringResult<Vec<Expr>>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let mut group_by = Vec::new();

        for child in node.all_children() {
            if matches!(child.kind(), "expression" | "column_ref") {
                group_by.push(lower_expr(ctx, child)?);
            }
        }

        Ok(group_by)
    }

    /// Lower HAVING clause
    pub fn lower_having_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let lower_expr =
            |ctx: &mut LoweringContext, n: &N| Self::lower_expr_generic(ctx, n, dialect_name);
        Self::lower_having_clause_with(ctx, node, dialect_name, lower_expr)
    }

    /// Lower HAVING clause with custom expression lowering function
    pub fn lower_having_clause_with<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        _dialect_name: &str,
        lower_expr: E,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let children = node.all_children();
        if let Some(cond) = children.first() {
            lower_expr(ctx, *cond)
        } else {
            ctx.add_error(LoweringError::MissingChild {
                context: "having_clause".to_string(),
                expected: "condition expression".to_string(),
            });
            Ok(ctx.create_placeholder())
        }
    }

    /// Lower ORDER BY clause
    pub fn lower_order_by_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Vec<OrderBy>>
    where
        N: CstNode,
    {
        let lower_expr =
            |ctx: &mut LoweringContext, n: &N| Self::lower_expr_generic(ctx, n, dialect_name);
        Self::lower_order_by_clause_with(ctx, node, dialect_name, lower_expr)
    }

    /// Lower ORDER BY clause with custom expression lowering function
    pub fn lower_order_by_clause_with<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        _dialect_name: &str,
        lower_expr: E,
    ) -> LoweringResult<Vec<OrderBy>>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let mut order_by = Vec::new();

        for child in node.all_children() {
            if matches!(child.kind(), "expression" | "column_ref") {
                let expr = lower_expr(ctx, child)?;
                order_by.push(OrderBy {
                    expr,
                    direction: None,
                });
            } else if matches!(child.kind(), "ASC" | "DESC")
                && let Some(last) = order_by.last_mut()
            {
                last.direction = if child.kind() == "DESC" {
                    Some(SortDirection::Desc)
                } else {
                    Some(SortDirection::Asc)
                };
            }
        }

        Ok(order_by)
    }

    /// Lower a joined table (JOIN syntax)
    ///
    /// Shared implementation for all dialects.
    pub fn lower_joined_table<N, F>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
        normalize_fn: F,
    ) -> LoweringResult<Option<TableRef>>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
    {
        // Parse JOIN type
        let join_type = if let Some(join_kind_node) = Self::optional_child(node, "join_kind") {
            match join_kind_node.text().unwrap_or("").to_uppercase().as_str() {
                "LEFT" => JoinType::Left,
                "RIGHT" => JoinType::Right,
                "INNER" => JoinType::Inner,
                "CROSS" => JoinType::Cross,
                "FULL" => JoinType::Full,
                _ => JoinType::Inner,
            }
        } else {
            JoinType::Inner
        };

        // Get the joined table
        let table = if let Some(table_node) = Self::optional_child(node, "table_name") {
            let table_name = normalize_fn(table_node.text().unwrap_or(""));
            TableRef {
                name: table_name,
                alias: None,
                joins: Vec::new(),
            }
        } else {
            ctx.add_error(LoweringError::MissingChild {
                context: "joined_table".to_string(),
                expected: "table name".to_string(),
            });
            return Ok(None);
        };

        // Parse join condition
        let condition = if let Some(on_node) = Self::optional_child(node, "join_on") {
            if let Some(expr_node) = Self::optional_child(on_node, "expression") {
                match Self::lower_expr_generic(ctx, expr_node, dialect_name) {
                    Ok(expr) => JoinCondition::On(expr),
                    Err(_) => JoinCondition::On(ctx.create_placeholder()),
                }
            } else {
                JoinCondition::On(ctx.create_placeholder())
            }
        } else if let Some(using_node) = Self::optional_child(node, "join_using") {
            let mut columns = Vec::new();
            for child in using_node.all_children() {
                if child.kind() == "identifier"
                    && let Some(name) = child.text()
                {
                    columns.push(normalize_fn(name));
                }
            }
            JoinCondition::Using(columns)
        } else {
            JoinCondition::On(ctx.create_placeholder())
        };

        let join = Join {
            join_type,
            table,
            condition,
        };

        Ok(Some(TableRef {
            name: String::new(),
            alias: None,
            joins: vec![join],
        }))
    }

    /// Lower FROM clause with table references
    pub fn lower_from_clause<N, F>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
        normalize_fn: F,
    ) -> LoweringResult<Vec<TableRef>>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
    {
        let mut tables = Vec::new();

        for child in node.all_children() {
            if matches!(
                child.kind(),
                "table_reference" | "table_name" | "joined_table"
            ) && let Some(table) =
                Self::lower_table_reference(ctx, child, dialect_name, normalize_fn)?
            {
                tables.push(table);
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
    pub fn lower_table_reference<N, F>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
        normalize_fn: F,
    ) -> LoweringResult<Option<TableRef>>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
    {
        match node.kind() {
            "table_name" | "identifier" => Ok(Some(TableRef {
                name: normalize_fn(node.text().unwrap_or("")),
                alias: None,
                joins: Vec::new(),
            })),
            "aliased_table" => {
                let children = node.all_children();
                if children.len() >= 2 {
                    Ok(Some(TableRef {
                        name: normalize_fn(children[0].text().unwrap_or("")),
                        alias: Some(normalize_fn(children[1].text().unwrap_or(""))),
                        joins: Vec::new(),
                    }))
                } else {
                    Ok(Some(TableRef {
                        name: normalize_fn(node.text().unwrap_or("")),
                        alias: None,
                        joins: Vec::new(),
                    }))
                }
            }
            "joined_table" => Self::lower_joined_table(ctx, node, dialect_name, normalize_fn),
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: dialect_name.to_string(),
                    feature: format!("table reference: {}", node.kind()),
                    suggestion: "Use valid table references".to_string(),
                });
                Ok(None)
            }
        }
    }

    /// Lower projection list (SELECT clause)
    ///
    /// Uses a callback function for expression lowering to allow dialect-specific
    /// handling of expressions.
    pub fn lower_projection<N, F, E>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
        normalize_fn: F,
        lower_expr_fn: E,
    ) -> LoweringResult<Vec<SelectItem>>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr> + Copy,
    {
        let mut items = Vec::new();

        for child in node.all_children() {
            match child.kind() {
                "expression" | "column_ref" => {
                    let expr = lower_expr_fn(ctx, child)?;
                    items.push(SelectItem::UnnamedExpr(expr));
                }
                "aliased_expression" => {
                    let children = child.all_children();
                    if children.len() >= 2 {
                        let expr = lower_expr_fn(ctx, children[0])?;
                        let alias = normalize_fn(children[1].text().unwrap_or(""));
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
                    let text = child.text().unwrap_or("");
                    let table_name = text.trim_end_matches('.').to_string();
                    items.push(SelectItem::QualifiedWildcard(table_name));
                }
                _ => {
                    ctx.add_error(LoweringError::UnsupportedSyntax {
                        dialect: dialect_name.to_string(),
                        feature: format!("projection item: {}", child.kind()),
                        suggestion: "Use valid SELECT expressions".to_string(),
                    });
                    items.push(SelectItem::UnnamedExpr(ctx.create_placeholder()));
                }
            }
        }

        Ok(items)
    }

    /// Extract VALUES clause from INSERT statement
    pub fn extract_values_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> Option<Vec<Vec<Expr>>>
    where
        N: CstNode,
    {
        let lower_expr =
            |ctx: &mut LoweringContext, n: &N| Self::lower_expr_generic(ctx, n, dialect_name);
        Self::extract_values_clause_with(ctx, node, lower_expr)
    }

    /// Extract VALUES clause from INSERT statement with custom expression lowering
    pub fn extract_values_clause_with<N, E>(
        ctx: &mut LoweringContext,
        node: &N,
        lower_expr: E,
    ) -> Option<Vec<Vec<Expr>>>
    where
        N: CstNode,
        E: Fn(&mut LoweringContext, &N) -> LoweringResult<Expr>,
    {
        let values_node = Self::optional_child(node, "values")?;
        let rows_node = Self::optional_child(values_node, "value_row_list")?;

        let mut all_rows = Vec::new();
        for row in rows_node.all_children() {
            if row.kind() != "value_row" {
                continue;
            }

            let mut row_values = Vec::new();
            for expr in row.all_children() {
                if expr.kind() == "expression" {
                    match lower_expr(ctx, expr) {
                        Ok(lowered_expr) => row_values.push(lowered_expr),
                        Err(_) => row_values.push(ctx.create_placeholder()),
                    }
                }
            }
            all_rows.push(row_values);
        }

        Some(all_rows)
    }

    /// Extract column list from INSERT statement
    pub fn extract_column_list<N, F>(node: &N, normalize_fn: F) -> Vec<String>
    where
        N: CstNode,
        F: Fn(&str) -> String + Copy,
    {
        let mut columns = Vec::new();

        if let Some(columns_node) = Self::optional_child(node, "column_list")
            && let Some(identifiers_node) = Self::optional_child(columns_node, "identifier_list")
        {
            for ident in identifiers_node.all_children() {
                if ident.kind() == "identifier"
                    && let Some(name) = ident.text()
                {
                    columns.push(normalize_fn(name));
                }
            }
        }

        columns
    }

    /// Lower CASE expression
    pub fn lower_case_expr<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let mut conditions = Vec::new();
        let mut results = Vec::new();
        let mut else_result = None;

        for child in node.all_children() {
            match child.kind() {
                "when_clause" => {
                    let (condition, result) = Self::lower_when_clause(ctx, child, dialect_name)?;
                    conditions.push(condition);
                    results.push(result);
                }
                "else_clause" => {
                    else_result =
                        Some(Box::new(Self::lower_else_clause(ctx, child, dialect_name)?));
                }
                _ => continue,
            }
        }

        if conditions.is_empty() {
            ctx.add_error(LoweringError::MissingChild {
                context: "case_expression".to_string(),
                expected: "when_clause".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        if conditions.len() != results.len() {
            ctx.add_error(LoweringError::InvalidLiteral {
                value: "Mismatched conditions and results in CASE expression".to_string(),
                type_name: "Case".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        Ok(Expr::Case {
            conditions,
            results,
            else_result,
        })
    }

    /// Lower a WHEN clause in CASE expression
    fn lower_when_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<(Expr, Expr)>
    where
        N: CstNode,
    {
        let children = node.all_children();

        if children.len() < 4 {
            ctx.add_error(LoweringError::MissingChild {
                context: "when_clause".to_string(),
                expected: "condition or result".to_string(),
            });
            return Ok((ctx.create_placeholder(), ctx.create_placeholder()));
        }

        let condition = Self::lower_expr_generic(ctx, children[1], dialect_name)?;
        let result = Self::lower_expr_generic(ctx, children[3], dialect_name)?;

        Ok((condition, result))
    }

    /// Lower an ELSE clause in CASE expression
    fn lower_else_clause<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        let children = node.all_children();

        if children.len() < 2 {
            ctx.add_error(LoweringError::MissingChild {
                context: "else_clause".to_string(),
                expected: "expression".to_string(),
            });
            return Ok(ctx.create_placeholder());
        }

        Self::lower_expr_generic(ctx, children[1], dialect_name)
    }

    /// Generic expression lowering that delegates to the appropriate handler
    ///
    /// This is used internally by shared functions when they need to lower
    /// sub-expressions. It only handles a limited set of expression types
    /// and returns placeholders for anything else.
    fn lower_expr_generic<N>(
        ctx: &mut LoweringContext,
        node: &N,
        dialect_name: &str,
    ) -> LoweringResult<Expr>
    where
        N: CstNode,
    {
        match node.kind() {
            "binary_expression" => {
                Self::lower_binary_expr(ctx, node, dialect_name, None, |ctx, n| {
                    Self::lower_expr_generic(ctx, n, dialect_name)
                })
            }
            "unary_expression" => Self::lower_unary_expr(ctx, node, dialect_name),
            "column_reference" | "column_ref" => {
                Self::lower_column_ref(ctx, node, |s| s.to_string())
            }
            "literal" => Self::lower_literal(ctx, node),
            "parenthesized_expression" => {
                let children = node.all_children();
                if let Some(inner) = children.first() {
                    Self::lower_expr_generic(ctx, *inner, dialect_name)
                } else {
                    ctx.add_error(LoweringError::MissingChild {
                        context: "parenthesized_expression".to_string(),
                        expected: "inner expression".to_string(),
                    });
                    Ok(ctx.create_placeholder())
                }
            }
            "case_expression" => Self::lower_case_expr(ctx, node, dialect_name),
            _ => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: dialect_name.to_string(),
                    feature: format!("expression: {}", node.kind()),
                    suggestion: "Use supported expression types".to_string(),
                });
                Ok(ctx.create_placeholder())
            }
        }
    }

    /// Helper: Extract an optional child node
    fn optional_child<'a, N>(node: &'a N, field: &str) -> Option<&'a N>
    where
        N: CstNode,
    {
        node.children(field).first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use unified_sql_lsp_ir::Dialect;

    #[test]
    fn test_parse_binary_op_standard() {
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        assert_eq!(
            SharedLowering::parse_binary_op("+", "MySQL", None, &mut ctx),
            BinaryOp::Add
        );
        assert_eq!(
            SharedLowering::parse_binary_op("=", "MySQL", None, &mut ctx),
            BinaryOp::Eq
        );
        assert_eq!(
            SharedLowering::parse_binary_op("AND", "MySQL", None, &mut ctx),
            BinaryOp::And
        );
    }

    #[test]
    fn test_parse_binary_op_dialect_specific() {
        let mut ctx = LoweringContext::new(Dialect::PostgreSQL);
        let dialect_ops = &[("~", BinaryOp::Like), ("~*", BinaryOp::Like)];

        assert_eq!(
            SharedLowering::parse_binary_op("~", "PostgreSQL", Some(dialect_ops), &mut ctx),
            BinaryOp::Like
        );
    }
}
