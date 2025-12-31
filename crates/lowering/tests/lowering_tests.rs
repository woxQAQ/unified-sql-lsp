// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Integration and unit tests for the lowering layer

use unified_sql_lsp_ir::{Dialect, Expr, Query};
use unified_sql_lsp_lowering::cst::MockCstNode;
use unified_sql_lsp_lowering::dialect::DialectLoweringBase;
use unified_sql_lsp_lowering::{
    CstNode, ErrorSeverity, Lowering, LoweringContext, LoweringError, LoweringOutcome,
    SourceLocation,
};

/// Mock lowering implementation for testing
struct TestLowering;

impl<N> Lowering<N> for TestLowering
where
    N: CstNode,
{
    fn lower_query(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> unified_sql_lsp_lowering::LoweringResult<Query> {
        if node.kind() == "select_statement" {
            // Check for unsupported features
            if let Some(_lateral_children) = node.children("lateral").first() {
                if !ctx.supports_feature("LATERAL") {
                    ctx.add_error(LoweringError::UnsupportedSyntax {
                        dialect: format!("{:?}", ctx.dialect()),
                        feature: "LATERAL JOIN".to_string(),
                        suggestion: "Use a subquery instead".to_string(),
                    });
                }
            }
            Ok(Query::new(ctx.dialect()))
        } else if node.kind() == "empty" {
            Err(LoweringError::Generic {
                message: "Empty query".to_string(),
            })
        } else {
            Err(LoweringError::UnexpectedNodeType {
                expected: "select_statement".to_string(),
                found: node.kind().to_string(),
            })
        }
    }

    fn lower_expr(
        &self,
        ctx: &mut LoweringContext,
        node: &N,
    ) -> unified_sql_lsp_lowering::LoweringResult<Expr> {
        match node.kind() {
            "column_ref" => Ok(Expr::Column(unified_sql_lsp_ir::ColumnRef::new("id"))),
            "literal" => Ok(Expr::Literal(unified_sql_lsp_ir::Literal::Integer(42))),
            "unsupported" => {
                ctx.add_error(LoweringError::UnsupportedSyntax {
                    dialect: format!("{:?}", ctx.dialect()),
                    feature: "this expression".to_string(),
                    suggestion: "Use a supported expression".to_string(),
                });
                Ok(ctx.create_placeholder())
            }
            _ => Ok(ctx.create_placeholder()),
        }
    }

    fn supports_node(&self, node: &N, kind: &str) -> bool {
        node.kind() == kind
    }

    fn dialect(&self) -> Dialect {
        Dialect::MySQL
    }
}

impl<N> DialectLoweringBase<N> for TestLowering where N: CstNode {}

#[test]
fn test_successful_lowering() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);
    let node = MockCstNode::new("select_statement");

    let result = lowering.lower_query(&mut ctx, &node);
    assert!(result.is_ok());

    let query = result.unwrap();
    assert_eq!(query.dialect, Dialect::MySQL);
    assert_eq!(ctx.outcome(), LoweringOutcome::Success);
    assert!(!ctx.has_errors());
}

#[test]
fn test_failed_lowering() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);
    let node = MockCstNode::new("invalid_statement");

    let result = lowering.lower_query(&mut ctx, &node);
    assert!(result.is_err());

    if let Err(LoweringError::UnexpectedNodeType { expected, found }) = result {
        assert_eq!(expected, "select_statement");
        assert_eq!(found, "invalid_statement");
    } else {
        panic!("Expected UnexpectedNodeType error");
    }
}

#[test]
fn test_partial_success_with_placeholders() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Create an unsupported expression node
    let node = MockCstNode::new("unsupported");
    let _ = lowering.lower_expr(&mut ctx, &node);

    // Should have error and placeholder
    assert!(ctx.has_errors());
    assert_eq!(ctx.errors().len(), 1);

    match ctx.outcome() {
        LoweringOutcome::Partial(errors) => {
            assert_eq!(errors.len(), 1);
        }
        _ => panic!("Expected Partial outcome"),
    }
}

#[test]
fn test_recursion_limit() {
    let mut ctx = LoweringContext::with_max_depth(Dialect::MySQL, 3);

    // Exceed recursion limit
    for _ in 0..4 {
        let _ = ctx.enter_recursive_context();
    }

    assert!(ctx.has_errors());
    assert_eq!(ctx.errors().len(), 1);

    if let Some(LoweringError::RecursionLimitExceeded { depth, limit, .. }) = ctx.errors().first() {
        assert_eq!(*depth, 4);
        assert_eq!(*limit, 3);
    } else {
        panic!("Expected RecursionLimitExceeded error");
    }
}

#[test]
fn test_multiple_errors_accumulation() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Generate multiple errors
    let unsupported_node = MockCstNode::new("unsupported");
    let _ = lowering.lower_expr(&mut ctx, &unsupported_node);
    let _ = lowering.lower_expr(&mut ctx, &unsupported_node);
    let _ = lowering.lower_expr(&mut ctx, &unsupported_node);

    assert!(ctx.has_errors());
    assert_eq!(ctx.errors().len(), 3);

    match ctx.outcome() {
        LoweringOutcome::Partial(errors) => {
            assert_eq!(errors.len(), 3);
        }
        _ => panic!("Expected Partial outcome"),
    }
}

#[test]
fn test_placeholder_uniqueness() {
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let p1 = ctx.create_placeholder();
    let p2 = ctx.create_placeholder();
    let p3 = ctx.create_placeholder();

    // All placeholders should be different
    if let (Expr::Column(c1), Expr::Column(c2), Expr::Column(c3)) = (p1, p2, p3) {
        assert_ne!(c1.column, c2.column);
        assert_ne!(c2.column, c3.column);
        assert_ne!(c1.column, c3.column);
    } else {
        panic!("Expected Column expressions");
    }
}

#[test]
fn test_source_mapping() {
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let location = SourceLocation {
        byte_offset: 100,
        line: 5,
        column: 10,
    };

    ctx.add_source_mapping("query:0".to_string(), location.clone());
    ctx.add_source_mapping(
        "expr:1".to_string(),
        SourceLocation {
            byte_offset: 150,
            line: 7,
            column: 5,
        },
    );

    let query_loc = ctx.get_source_location("query:0");
    assert!(query_loc.is_some());
    assert_eq!(query_loc.unwrap().line, 5);

    let expr_loc = ctx.get_source_location("expr:1");
    assert!(expr_loc.is_some());
    assert_eq!(expr_loc.unwrap().column, 5);

    let missing = ctx.get_source_location("missing");
    assert!(missing.is_none());
}

#[test]
fn test_context_with_different_dialects() {
    let mysql_ctx = LoweringContext::new(Dialect::MySQL);
    let pg_ctx = LoweringContext::new(Dialect::PostgreSQL);

    assert_eq!(mysql_ctx.dialect(), Dialect::MySQL);
    assert_eq!(pg_ctx.dialect(), Dialect::PostgreSQL);
}

#[test]
fn test_recursion_depth_tracking() {
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    // Test that we can enter and exit recursive contexts
    assert!(ctx.enter_recursive_context().is_ok());
    assert!(ctx.enter_recursive_context().is_ok());
    ctx.exit_recursive_context();
    ctx.exit_recursive_context();

    // Should still be able to enter after exiting
    assert!(ctx.enter_recursive_context().is_ok());
    ctx.exit_recursive_context();
}

#[test]
fn test_dialect_lowering_base_require_child() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let child = MockCstNode::new("from_clause");
    let node = MockCstNode::new("select_statement").with_child(Some("from"), child);

    let result = lowering.require_child(&mut ctx, &node, "from");
    assert!(result.is_ok());
    assert_eq!(result.unwrap().kind(), "from_clause");
}

#[test]
fn test_dialect_lowering_base_require_child_missing() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let node = MockCstNode::new("select_statement");

    let result = lowering.require_child(&mut ctx, &node, "from");
    assert!(result.is_err());
}

#[test]
fn test_dialect_lowering_base_optional_child() {
    let lowering = TestLowering;

    let child = MockCstNode::new("where_clause");
    let node_with = MockCstNode::new("select_statement").with_child(Some("where"), child);
    let node_without = MockCstNode::new("select_statement");

    assert!(lowering.optional_child(&node_with, "where").is_some());
    assert!(lowering.optional_child(&node_without, "where").is_none());
}

#[test]
fn test_dialect_lowering_base_lower_children() {
    let lowering = TestLowering;
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    let node1 = MockCstNode::new("column_ref");
    let node2 = MockCstNode::new("literal");
    let node3 = MockCstNode::new("column_ref");

    let nodes = vec![&node1, &node2, &node3];

    let result =
        lowering.lower_children(&mut ctx, &nodes, |ctx, node| lowering.lower_expr(ctx, node));

    assert!(result.is_ok());
    let exprs = result.unwrap();
    assert_eq!(exprs.len(), 3);
}

#[test]
fn test_clear_errors() {
    let mut ctx = LoweringContext::new(Dialect::MySQL);

    ctx.add_error(LoweringError::Generic {
        message: "Error 1".to_string(),
    });
    ctx.add_error(LoweringError::Generic {
        message: "Error 2".to_string(),
    });

    assert!(ctx.has_errors());
    assert_eq!(ctx.errors().len(), 2);

    ctx.clear_errors();

    assert!(!ctx.has_errors());
    assert_eq!(ctx.errors().len(), 0);
    assert!(matches!(ctx.outcome(), LoweringOutcome::Success));
}

#[test]
fn test_error_recoverable() {
    use LoweringError;

    assert!(
        LoweringError::UnsupportedSyntax {
            dialect: "MySQL".to_string(),
            feature: "CTE".to_string(),
            suggestion: "Upgrade".to_string(),
        }
        .is_recoverable()
    );

    assert!(
        LoweringError::InvalidLiteral {
            value: "abc".to_string(),
            type_name: "int".to_string(),
        }
        .is_recoverable()
    );

    assert!(
        !LoweringError::MissingChild {
            context: "query".to_string(),
            expected: "SELECT".to_string(),
        }
        .is_recoverable()
    );
}

#[test]
fn test_error_severity_levels() {
    use unified_sql_lsp_lowering::{ErrorSeverity, LoweringError};

    let unsupported = LoweringError::UnsupportedSyntax {
        dialect: "MySQL".to_string(),
        feature: "CTE".to_string(),
        suggestion: "Upgrade".to_string(),
    };
    assert_eq!(unsupported.severity(), ErrorSeverity::Warning);

    let missing = LoweringError::MissingChild {
        context: "query".to_string(),
        expected: "SELECT".to_string(),
    };
    assert_eq!(missing.severity(), ErrorSeverity::Error);

    let ambiguous = LoweringError::AmbiguousSyntax {
        message: "Ambiguous".to_string(),
        suggestion: "Fix it".to_string(),
    };
    assert_eq!(ambiguous.severity(), ErrorSeverity::Info);
}

#[test]
fn test_mock_cst_node_tree() {
    let column1 = MockCstNode::new("column_ref").with_text("id");
    let column2 = MockCstNode::new("column_ref").with_text("name");
    let select_list = MockCstNode::new("select_list")
        .with_child(None, column1)
        .with_child(None, column2);

    let from_clause = MockCstNode::new("from_clause").with_child(
        Some("table"),
        MockCstNode::new("table_ref").with_text("users"),
    );

    let select_stmt = MockCstNode::new("select_statement")
        .with_child(Some("projection"), select_list)
        .with_child(Some("from"), from_clause);

    assert_eq!(select_stmt.kind(), "select_statement");
    assert_eq!(select_stmt.child_count(), 2);

    let projection = select_stmt.children("projection");
    assert_eq!(projection.len(), 1);
    assert_eq!(projection[0].kind(), "select_list");
    assert_eq!(projection[0].child_count(), 2);
}

#[test]
fn test_supports_feature() {
    let ctx = LoweringContext::new(Dialect::MySQL);

    // Core SQL features should be supported
    assert!(ctx.supports_feature("SELECT"));
    assert!(ctx.supports_feature("FROM"));
    assert!(ctx.supports_feature("WHERE"));
    assert!(ctx.supports_feature("JOIN"));

    // Advanced features return false for now
    assert!(!ctx.supports_feature("CTE"));
    assert!(!ctx.supports_feature("WINDOW"));
}

#[test]
fn test_outcome_variants() {
    // Success
    let ctx_success = LoweringContext::new(Dialect::MySQL);
    assert!(matches!(ctx_success.outcome(), LoweringOutcome::Success));

    // Partial
    let mut ctx_partial = LoweringContext::new(Dialect::MySQL);
    ctx_partial.add_error(LoweringError::Generic {
        message: "Error".to_string(),
    });
    assert!(matches!(ctx_partial.outcome(), LoweringOutcome::Partial(_)));

    // Failed is constructed manually
    let failed = LoweringOutcome::Failed(LoweringError::Generic {
        message: "Critical".to_string(),
    });
    assert!(matches!(failed, LoweringOutcome::Failed(_)));
}
