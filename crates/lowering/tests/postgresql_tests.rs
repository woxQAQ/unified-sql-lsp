// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for PostgreSQL CST → IR lowering
//!
//! This module contains integration tests for the PostgreSQL lowering implementation,
//! covering major features including SELECT statements, expressions, and
//! PostgreSQL-specific syntax.

use unified_sql_lsp_ir::query::SortDirection;
use unified_sql_lsp_ir::{BinaryOp, Dialect, Expr, Literal};
use unified_sql_lsp_lowering::cst::MockCstNode;
use unified_sql_lsp_lowering::dialect::PostgreSQLLowering;
use unified_sql_lsp_lowering::{Lowering, LoweringContext, LoweringError};

// =============================================================================
// Basic SELECT Statement Tests
// =============================================================================

#[test]
fn test_postgresql_simple_select() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // Create a simple SELECT statement CST: SELECT id FROM users
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();
    assert_eq!(query.dialect, Dialect::PostgreSQL);
    assert!(!ctx.has_errors(), "Should have no errors");
}

// =============================================================================
// CASE Expression Tests
// =============================================================================

#[test]
fn test_postgresql_case_expression_with_else() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // CASE WHEN status = 'active' THEN 1 ELSE 0 END
    let when_condition = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("status"))
        .with_child(None, MockCstNode::new("operator").with_text("="))
        .with_child(None, MockCstNode::new("literal").with_text("'active'"));

    let then_result = MockCstNode::new("literal").with_text("1");

    let when_clause = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, when_condition)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, then_result);

    let else_expr = MockCstNode::new("literal").with_text("0");
    let else_clause = MockCstNode::new("else_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("ELSE"))
        .with_child(None, else_expr);

    let case_expr = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, when_clause)
        .with_child(None, else_clause)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &case_expr);

    assert!(result.is_ok(), "Lowering should succeed");
    let expr = result.unwrap();

    match expr {
        Expr::Case {
            conditions,
            results,
            else_result,
        } => {
            assert_eq!(conditions.len(), 1, "Should have 1 condition");
            assert_eq!(results.len(), 1, "Should have 1 result");
            assert!(else_result.is_some(), "Should have ELSE result");
            assert!(!ctx.has_errors(), "Should have no errors");
        }
        _ => panic!("Expected Expr::Case, got {:?}", expr),
    }
}

#[test]
fn test_postgresql_case_expression_multiple_when() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // CASE
    //   WHEN score > 90 THEN 'A'
    //   WHEN score > 80 THEN 'B'
    //   WHEN score > 70 THEN 'C'
    //   ELSE 'F'
    // END

    let when1_cond = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("score"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("90"));

    let when1 = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, when1_cond)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, MockCstNode::new("literal").with_text("'A'"));

    let when2_cond = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("score"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("80"));

    let when2 = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, when2_cond)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, MockCstNode::new("literal").with_text("'B'"));

    let when3_cond = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("score"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("70"));

    let when3 = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, when3_cond)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, MockCstNode::new("literal").with_text("'C'"));

    let else_clause = MockCstNode::new("else_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("ELSE"))
        .with_child(None, MockCstNode::new("literal").with_text("'F'"));

    let case_expr = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, when1)
        .with_child(None, when2)
        .with_child(None, when3)
        .with_child(None, else_clause)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &case_expr);

    assert!(result.is_ok(), "Lowering should succeed");
    let expr = result.unwrap();

    match expr {
        Expr::Case {
            conditions,
            results,
            else_result,
        } => {
            assert_eq!(conditions.len(), 3, "Should have 3 conditions");
            assert_eq!(results.len(), 3, "Should have 3 results");
            assert!(else_result.is_some(), "Should have ELSE result");
            assert!(!ctx.has_errors(), "Should have no errors");
        }
        _ => panic!("Expected Expr::Case, got {:?}", expr),
    }
}

#[test]
fn test_postgresql_case_expression_without_else() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // CASE WHEN quantity > 100 THEN 'bulk' END
    let when_condition = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("quantity"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("100"));

    let when_clause = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, when_condition)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, MockCstNode::new("literal").with_text("'bulk'"));

    let case_expr = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, when_clause)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &case_expr);

    assert!(result.is_ok(), "Lowering should succeed");
    let expr = result.unwrap();

    match expr {
        Expr::Case {
            conditions,
            results,
            else_result,
        } => {
            assert_eq!(conditions.len(), 1, "Should have 1 condition");
            assert_eq!(results.len(), 1, "Should have 1 result");
            assert!(
                else_result.is_none(),
                "Should not have ELSE result (implicit NULL)"
            );
            assert!(!ctx.has_errors(), "Should have no errors");
        }
        _ => panic!("Expected Expr::Case, got {:?}", expr),
    }
}

#[test]
fn test_postgresql_case_expression_nested() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // Outer CASE expression
    // CASE
    //   WHEN type = 'X' THEN (inner CASE)
    //   ELSE 'other'
    // END

    // Inner CASE: CASE WHEN value > 10 THEN 'high' ELSE 'low' END
    let inner_when_cond = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("value"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("10"));

    let inner_when = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, inner_when_cond)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, MockCstNode::new("literal").with_text("'high'"));

    let inner_else = MockCstNode::new("else_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("ELSE"))
        .with_child(None, MockCstNode::new("literal").with_text("'low'"));

    let inner_case = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, inner_when)
        .with_child(None, inner_else)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    // Outer CASE
    let outer_when_cond = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("type"))
        .with_child(None, MockCstNode::new("operator").with_text("="))
        .with_child(None, MockCstNode::new("literal").with_text("'X'"));

    let outer_when = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, outer_when_cond)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, inner_case);

    let outer_else = MockCstNode::new("else_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("ELSE"))
        .with_child(None, MockCstNode::new("literal").with_text("'other'"));

    let outer_case = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, outer_when)
        .with_child(None, outer_else)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &outer_case);

    assert!(result.is_ok(), "Lowering should succeed");
    let expr = result.unwrap();

    match expr {
        Expr::Case {
            conditions,
            results,
            else_result,
        } => {
            assert_eq!(conditions.len(), 1, "Should have 1 condition");
            assert_eq!(results.len(), 1, "Should have 1 result");
            assert!(else_result.is_some(), "Should have ELSE result");

            // Check that the result contains a nested CASE
            match &results[0] {
                Expr::Case { .. } => {
                    // Successfully parsed nested CASE
                }
                _ => panic!("Expected nested Expr::Case in result, got {:?}", results[0]),
            }

            assert!(!ctx.has_errors(), "Should have no errors");
        }
        _ => panic!("Expected Expr::Case, got {:?}", expr),
    }
}

#[test]
fn test_postgresql_case_expression_missing_when_error() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // CASE without any WHEN clauses - should produce error
    let case_expr = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &case_expr);

    // Should succeed but with placeholder and error in context
    assert!(
        result.is_ok(),
        "Lowering should succeed with graceful degradation"
    );
    assert!(
        ctx.has_errors(),
        "Should have error for missing WHEN clause"
    );
}

#[test]
fn test_postgresql_case_expression_malformed_when_error() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // WHEN clause with insufficient children - should produce error
    let malformed_when = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, MockCstNode::new("column_ref").with_text("status"));

    let case_expr = MockCstNode::new("case_expression")
        .with_child(None, MockCstNode::new("keyword").with_text("CASE"))
        .with_child(None, malformed_when)
        .with_child(None, MockCstNode::new("keyword").with_text("END"));

    let result = lowering.lower_expr(&mut ctx, &case_expr);

    // Should succeed but with placeholder and error in context
    assert!(
        result.is_ok(),
        "Lowering should succeed with graceful degradation"
    );
    assert!(
        ctx.has_errors(),
        "Should have error for malformed WHEN clause"
    );
}

// =============================================================================
// ORDER BY Direction Tests (LOWERING-004)
// =============================================================================

#[test]
fn test_postgresql_order_by_default_direction() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name (no direction specified)
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column);

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 1, "Should have one ORDER BY item");

    // Default direction is None (which means ASC in SQL)
    assert_eq!(
        order_by_items[0].direction, None,
        "Default direction should be None (ASC)"
    );
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_postgresql_order_by_with_asc() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name ASC
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name ASC
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column)
        .with_child(None, MockCstNode::new("ASC"));

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 1, "Should have one ORDER BY item");

    assert_eq!(
        order_by_items[0].direction,
        Some(SortDirection::Asc),
        "Direction should be ASC"
    );
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_postgresql_order_by_with_desc() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name DESC
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name DESC
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column)
        .with_child(None, MockCstNode::new("DESC"));

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 1, "Should have one ORDER BY item");

    assert_eq!(
        order_by_items[0].direction,
        Some(SortDirection::Desc),
        "Direction should be DESC"
    );
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_postgresql_order_by_multiple_columns_mixed_directions() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name ASC, id DESC
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name ASC, id DESC
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let id_order_column = MockCstNode::new("column_ref").with_text("id");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column)
        .with_child(None, MockCstNode::new("ASC"))
        .with_child(None, MockCstNode::new(","))
        .with_child(None, id_order_column)
        .with_child(None, MockCstNode::new("DESC"));

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 2, "Should have two ORDER BY items");

    assert_eq!(
        order_by_items[0].direction,
        Some(SortDirection::Asc),
        "First item should be ASC"
    );
    assert_eq!(
        order_by_items[1].direction,
        Some(SortDirection::Desc),
        "Second item should be DESC"
    );
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_postgresql_order_by_partial_directions() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name, id DESC
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name, id DESC (first has no direction, second is DESC)
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let id_order_column = MockCstNode::new("column_ref").with_text("id");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column)
        .with_child(None, MockCstNode::new(","))
        .with_child(None, id_order_column)
        .with_child(None, MockCstNode::new("DESC"));

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 2, "Should have two ORDER BY items");

    assert_eq!(
        order_by_items[0].direction, None,
        "First item should have no direction (default ASC)"
    );
    assert_eq!(
        order_by_items[1].direction,
        Some(SortDirection::Desc),
        "Second item should be DESC"
    );
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_postgresql_order_by_with_limit() {
    let lowering = PostgreSQLLowering;
    let mut ctx = LoweringContext::new(Dialect::PostgreSQL);

    // SELECT id FROM users ORDER BY name DESC LIMIT 10
    let id_column = MockCstNode::new("column_ref").with_text("id");
    let projection = MockCstNode::new("projection").with_child(None, id_column);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    // ORDER BY name DESC
    let name_column = MockCstNode::new("column_ref").with_text("name");
    let order_by = MockCstNode::new("order_by_clause")
        .with_child(None, MockCstNode::new("ORDER"))
        .with_child(None, MockCstNode::new("BY"))
        .with_child(None, name_column)
        .with_child(None, MockCstNode::new("DESC"));

    // LIMIT 10
    let limit_literal = MockCstNode::new("literal").with_text("10");
    let limit = MockCstNode::new("limit_clause")
        .with_child(None, MockCstNode::new("LIMIT"))
        .with_child(None, limit_literal);

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from)
        .with_child(Some("order_by_clause"), order_by)
        .with_child(Some("limit_clause"), limit);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok(), "Lowering should succeed");
    let query = result.unwrap();

    assert!(
        query.order_by.is_some(),
        "Query should have ORDER BY clause"
    );
    let order_by_items = query.order_by.unwrap();
    assert_eq!(order_by_items.len(), 1, "Should have one ORDER BY item");

    assert_eq!(
        order_by_items[0].direction,
        Some(SortDirection::Desc),
        "Direction should be DESC"
    );

    assert!(query.limit.is_some(), "Query should have LIMIT clause");
    assert!(!ctx.has_errors(), "Should have no errors");
}
