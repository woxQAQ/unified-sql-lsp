// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration tests for MySQL CST → IR lowering
//!
//! This module contains integration tests for the MySQL lowering implementation,
//! covering major features including SELECT statements, LIMIT clauses, expressions,
//! and MySQL-specific syntax.

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
        assert_eq!(val, true);
    } else {
        panic!("Expected Boolean literal");
    }

    let false_literal = MockCstNode::new("literal").with_text("FALSE");
    let result = lowering.lower_expr(&mut ctx, &false_literal);

    assert!(result.is_ok());
    if let Ok(Expr::Literal(Literal::Boolean(val))) = result {
        assert_eq!(val, false);
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
