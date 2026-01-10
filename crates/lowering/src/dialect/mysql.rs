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

use crate::dialect::{DialectLoweringBase, SharedLowering};
use crate::{CstNode, Lowering, LoweringContext, LoweringError, LoweringResult};
use unified_sql_lsp_ir::{InsertSource, InsertStatement, OnConflict};
use unified_sql_lsp_ir::query::{SelectStatement, TableRef};
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
            "binary_expression" => {
                let lower_fn = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
                SharedLowering::lower_binary_expr(ctx, node, "MySQL", None, lower_fn)
            }
            "unary_expression" => SharedLowering::lower_unary_expr(ctx, node, "MySQL"),
            "column_reference" | "column_ref" => {
                SharedLowering::lower_column_ref(ctx, node, Self::normalize_identifier)
            }
            "literal" => SharedLowering::lower_literal(ctx, node),
            "function_call" => self.lower_function_call(ctx, node),
            "case_expression" => SharedLowering::lower_case_expr(ctx, node, "MySQL"),
            "parenthesized_expression" => {
                let children = node.all_children();
                if let Some(inner) = children.first() {
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
            let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
            select.projection = SharedLowering::lower_projection(
                ctx,
                proj_node,
                "MySQL",
                Self::normalize_identifier,
                lower_expr,
            )?;
        }

        // Lower FROM clause
        if let Some(from_node) = self.optional_child(node, "from_clause") {
            select.from =
                SharedLowering::lower_from_clause(ctx, from_node, "MySQL", Self::normalize_identifier)?;
        }

        // Lower WHERE clause
        if let Some(where_node) = self.optional_child(node, "where_clause") {
            let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
            select.where_clause = Some(SharedLowering::lower_where_clause_with(
                ctx, where_node, "MySQL", lower_expr,
            )?);
        }

        // Lower GROUP BY clause
        if let Some(group_node) = self.optional_child(node, "group_by_clause") {
            let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
            select.group_by =
                SharedLowering::lower_group_by_clause_with(ctx, group_node, "MySQL", lower_expr)?;
        }

        // Lower HAVING clause
        if let Some(having_node) = self.optional_child(node, "having_clause") {
            let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
            select.having = Some(SharedLowering::lower_having_clause_with(
                ctx, having_node, "MySQL", lower_expr,
            )?);
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
            let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
            query.order_by = Some(SharedLowering::lower_order_by_clause_with(
                ctx, order_node, "MySQL", lower_expr,
            )?);
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
        let mut insert = InsertStatement {
            table: TableRef {
                name: String::new(),
                alias: None,
                joins: Vec::new(),
            },
            columns: Vec::new(),
            source: InsertSource::DefaultValues,
            on_conflict: Some(OnConflict::ReplaceMode),
            returning: None,
        };

        // Extract table name from REPLACE
        if let Some(table_node) = self.optional_child(node, "table_name") {
            let table_name = Self::normalize_identifier(table_node.text().unwrap_or(""));
            insert.table.name = table_name;
        }

        // Extract columns if present
        insert.columns = SharedLowering::extract_column_list(node, Self::normalize_identifier);

        // Extract VALUES if present
        let lower_expr = |ctx: &mut LoweringContext, n: &N| self.lower_expr(ctx, n);
        if let Some(all_rows) = SharedLowering::extract_values_clause_with(ctx, node, lower_expr) {
            insert.source = InsertSource::Values(all_rows);
        }

        let mut query = Query::new(Dialect::MySQL);
        query.body = unified_sql_lsp_ir::SetOp::Insert(Box::new(insert));

        Ok(query)
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
                let offset = self.lower_expr(ctx, exprs[0])?;
                let limit = self.lower_expr(ctx, exprs[1])?;
                Ok((Some(limit), Some(offset)))
            }
            (2, false) => {
                // Form 2: LIMIT count OFFSET offset
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

    /// Lower function call
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

        Ok(Expr::Function {
            name: func_name,
            args,
            distinct: false,
            filter: None,
            over: None,
        })
    }

    /// Normalize identifier by removing backticks
    fn normalize_identifier(identifier: &str) -> String {
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

    #[test]
    fn test_normalize_identifier_with_backticks() {
        assert_eq!(MySQLLowering::normalize_identifier("`id`"), "id");
        assert_eq!(MySQLLowering::normalize_identifier("`table_name`"), "table_name");
        let result = MySQLLowering::normalize_identifier("`db`.`table`");
        assert!(result.contains("db"));
        assert!(result.contains("table"));
    }

    #[test]
    fn test_normalize_identifier_without_backticks() {
        assert_eq!(MySQLLowering::normalize_identifier("id"), "id");
        assert_eq!(MySQLLowering::normalize_identifier("table_name"), "table_name");
    }
}
