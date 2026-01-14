// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for MySQL CST → IR lowering
//!
//! This module contains integration tests for the MySQL lowering implementation,
//! covering major features including SELECT statements, LIMIT clauses, expressions,
//! and MySQL-specific syntax.

use unified_sql_lsp_ir::query::SortDirection;
use unified_sql_lsp_ir::{BinaryOp, Dialect, Expr, Literal};
use unified_sql_lsp_lowering::cst::MockCstNode;
use unified_sql_lsp_lowering::dialect::MySQLLowering;
use unified_sql_lsp_lowering::{Lowering, LoweringContext, LoweringError};

// =============================================================================
// Basic SELECT Statement Tests
// =============================================================================

#[test]
fn test_mysql_simple_select() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
    assert_eq!(query.dialect, Dialect::MySQL);
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_mysql_select_with_wildcard() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // SELECT * FROM users
    let wildcard = MockCstNode::new("*");
    let projection = MockCstNode::new("projection").with_child(None, wildcard);

    let table = MockCstNode::new("table_name").with_text("users");
    let from = MockCstNode::new("from_clause").with_child(Some("table"), table);

    let cst = MockCstNode::new("select_statement")
        .with_child(Some("projection"), projection)
        .with_child(Some("from"), from);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok());
    assert!(!ctx.has_errors());
}

// =============================================================================
// Expression Tests
// =============================================================================

#[test]
fn test_mysql_column_reference() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test unqualified column
    let col = MockCstNode::new("column_ref").with_text("id");
    let result = lowering.lower_expr(&mut ctx, &col);

    assert!(result.is_ok());
    if let Ok(Expr::Column(col_ref)) = result {
        assert_eq!(col_ref.column, "id");
        assert!(col_ref.table.is_none());
    } else {
        panic!("Expected Column expression");
    }

    // Test qualified column
    let qualified_col = MockCstNode::new("column_ref").with_text("users.id");
    let result = lowering.lower_expr(&mut ctx, &qualified_col);

    assert!(result.is_ok());
    if let Ok(Expr::Column(col_ref)) = result {
        assert_eq!(col_ref.column, "id");
        assert_eq!(col_ref.table.as_deref(), Some("users"));
    } else {
        panic!("Expected Column expression with table");
    }
}

#[test]
fn test_mysql_binary_expressions() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test: a + b
    let expr = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("a"))
        .with_child(None, MockCstNode::new("operator").with_text("+"))
        .with_child(None, MockCstNode::new("column_ref").with_text("b"));

    let result = lowering.lower_expr(&mut ctx, &expr);

    assert!(result.is_ok());
    if let Ok(Expr::BinaryOp { op, .. }) = result {
        assert_eq!(op, BinaryOp::Add);
    } else {
        panic!("Expected BinaryOp expression");
    }
}

#[test]
fn test_mysql_comparison_expressions() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test: price > 100
    let expr = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("price"))
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("100"));

    let result = lowering.lower_expr(&mut ctx, &expr);

    assert!(result.is_ok());
    if let Ok(Expr::BinaryOp { op, .. }) = result {
        assert_eq!(op, BinaryOp::Gt);
    } else {
        panic!("Expected BinaryOp with Gt operator");
    }
}

#[test]
fn test_mysql_literal_integer() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let literal = MockCstNode::new("literal").with_text("42");
    let result = lowering.lower_expr(&mut ctx, &literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Integer(val))) = result {
        assert_eq!(val, 42);
    } else {
        panic!("Expected Integer literal");
    }
}

#[test]
fn test_mysql_literal_float() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let literal = MockCstNode::new("literal").with_text("3.14");
    let result = lowering.lower_expr(&mut ctx, &literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Float(val))) = result {
        assert!((val - 3.14).abs() < 0.001);
    } else {
        panic!("Expected Float literal");
    }
}

#[test]
fn test_mysql_literal_string() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let literal = MockCstNode::new("literal").with_text("'hello'");
    let result = lowering.lower_expr(&mut ctx, &literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::String(val))) = result {
        assert_eq!(val, "hello");
    } else {
        panic!("Expected String literal");
    }
}

#[test]
fn test_mysql_literal_boolean() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let true_literal = MockCstNode::new("literal").with_text("TRUE");
    let result = lowering.lower_expr(&mut ctx, &true_literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Boolean(val))) = result {
        assert!(val);
    } else {
        panic!("Expected Boolean literal");
    }

    let false_literal = MockCstNode::new("literal").with_text("FALSE");
    let result = lowering.lower_expr(&mut ctx, &false_literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Boolean(val))) = result {
        assert!(!val);
    } else {
        panic!("Expected Boolean literal");
    }
}

#[test]
fn test_mysql_literal_null() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let literal = MockCstNode::new("literal").with_text("NULL");
    let result = lowering.lower_expr(&mut ctx, &literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Null)) = result {
        // OK
    } else {
        panic!("Expected Null literal");
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_mysql_unsupported_expression_placeholder() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test unsupported expression type
    let unsupported = MockCstNode::new("unsupported_expression");
    let result = lowering.lower_expr(&mut ctx, &unsupported);

    // Should return placeholder
    assert!(result.is_ok());
    assert!(
        ctx.has_errors(),
        "Should have error for unsupported expression"
    );

    // Verify a placeholder was created
    let expr = result.unwrap();
    if let Expr::Column(col) = expr {
        assert!(
            col.column.starts_with("__placeholder_"),
            "Should be a placeholder"
        );
    } else {
        panic!("Expected placeholder Column expression");
    }
}

#[test]
fn test_mysql_partial_success() {
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Create an invalid literal that will generate an error
    ctx.add_error(unified_sql_lsp_lowering::LoweringError::InvalidLiteral {
        value: "not_a_number".to_string(),
        type_name: "number".to_string(),
    });

    // Should have error
    assert!(ctx.has_errors());
    assert_eq!(ctx.errors().len(), 1);
}

#[test]
fn test_mysql_recursion_limit() {
    let mut ctx = LoweringContext::with_max_depth(Dialect::MySQL, 3);

    // Should be able to enter up to the limit
    assert!(ctx.enter_recursive_context().is_ok());
    assert!(ctx.enter_recursive_context().is_ok());
    assert!(ctx.enter_recursive_context().is_ok());

    // Fourth call should exceed the limit and return an error
    let result = ctx.enter_recursive_context();
    assert!(result.is_err());
    if let Err(LoweringError::RecursionLimitExceeded { depth, limit, .. }) = result {
        assert_eq!(depth, 4);
        assert_eq!(limit, 3);
    } else {
        panic!("Expected RecursionLimitExceeded error");
    }
}

// =============================================================================
// MySQL-Specific Feature Tests
// =============================================================================

#[test]
fn test_mysql_backtick_identifiers() {
    // Test backtick identifier normalization via column reference
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test that backticked column works correctly
    let col = MockCstNode::new("column_ref").with_text("`id`");
    let result = lowering.lower_expr(&mut ctx, &col);

    assert!(result.is_ok());
    if let Ok(Expr::Column(col_ref)) = result {
        // Backticks should be stripped
        assert_eq!(col_ref.column, "id");
        assert!(!col_ref.column.contains('`'));
    } else {
        panic!("Expected Column expression");
    }
}

#[test]
fn test_mysql_replace_statement() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // REPLACE INTO users
    let table_node = MockCstNode::new("table_name").with_text("users");
    let cst = MockCstNode::new("replace_statement").with_child(Some("table"), table_node);

    let result = lowering.lower_query(&mut ctx, &cst);

    assert!(result.is_ok());
    let query = result.unwrap();
    assert_eq!(query.dialect, Dialect::MySQL);
    // REPLACE is converted to INSERT structure in IR
    let _ = query; // Suppress unused warning
}

// =============================================================================
// Integration Tests
// =============================================================================

#[test]
fn test_mysql_full_stack_lowering() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test complete lowering pipeline: SELECT id FROM users
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

    assert_eq!(query.dialect, Dialect::MySQL);
    assert!(!ctx.has_errors(), "Should have no errors");
}

#[test]
fn test_mysql_dialect_check() {
    let lowering = MySQLLowering;
    let ctx = LoweringContext::new(Dialect::MySQL);

    // Verify dialect is set correctly
    assert_eq!(
        <MySQLLowering as Lowering<MockCstNode>>::dialect(&lowering),
        Dialect::MySQL
    );
    assert_eq!(ctx.dialect(), Dialect::MySQL);
}

// =============================================================================
// CASE Expression Tests
// =============================================================================

#[test]
fn test_mysql_case_expression_with_else() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_case_expression_multiple_when() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_case_expression_without_else() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_case_expression_nested() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_case_expression_complex_conditions() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // CASE
    //   WHEN price * quantity > 1000 THEN price * 0.9
    //   ELSE price
    // END

    // Complex condition: price * quantity > 1000
    let left_operand = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("price"))
        .with_child(None, MockCstNode::new("operator").with_text("*"))
        .with_child(None, MockCstNode::new("column_ref").with_text("quantity"));

    let complex_condition = MockCstNode::new("binary_expression")
        .with_child(None, left_operand)
        .with_child(None, MockCstNode::new("operator").with_text(">"))
        .with_child(None, MockCstNode::new("literal").with_text("1000"));

    // Complex result: price * 0.9
    let complex_result = MockCstNode::new("binary_expression")
        .with_child(None, MockCstNode::new("column_ref").with_text("price"))
        .with_child(None, MockCstNode::new("operator").with_text("*"))
        .with_child(None, MockCstNode::new("literal").with_text("0.9"));

    let when_clause = MockCstNode::new("when_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("WHEN"))
        .with_child(None, complex_condition)
        .with_child(None, MockCstNode::new("keyword").with_text("THEN"))
        .with_child(None, complex_result);

    let else_clause = MockCstNode::new("else_clause")
        .with_child(None, MockCstNode::new("keyword").with_text("ELSE"))
        .with_child(None, MockCstNode::new("column_ref").with_text("price"));

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
fn test_mysql_case_expression_missing_when_error() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_case_expression_malformed_when_error() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_default_direction() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_with_asc() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_with_desc() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_multiple_columns_mixed_directions() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_partial_directions() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
fn test_mysql_order_by_with_limit() {
    let lowering = MySQLLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

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
