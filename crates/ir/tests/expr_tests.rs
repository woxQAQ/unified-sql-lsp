// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Unit tests for IR Expression representation

use unified_sql_lsp_ir::{BinaryOp, ColumnRef, Expr, Literal, UnaryOp};

#[test]
fn test_expr_column_ref() {
    let col = Expr::Column(ColumnRef::new("id"));
    assert!(matches!(col, Expr::Column(_)));
}

#[test]
fn test_column_ref_new() {
    let col = ColumnRef::new("user_id");
    assert_eq!(col.column, "user_id");
    assert!(col.table.is_none());
}

#[test]
fn test_column_ref_with_table() {
    let col = ColumnRef::new("id").with_table("users");
    assert_eq!(col.column, "id");
    assert_eq!(col.table, Some("users".to_string()));
}

#[test]
fn test_expr_literal_integer() {
    let lit = Expr::Literal(Literal::Integer(42));
    assert!(matches!(lit, Expr::Literal(Literal::Integer(42))));
}

#[test]
fn test_expr_literal_string() {
    let lit = Expr::Literal(Literal::String("hello".to_string()));
    assert!(matches!(lit, Expr::Literal(Literal::String(_))));
    if let Expr::Literal(Literal::String(s)) = lit {
        assert_eq!(s, "hello");
    }
}

#[test]
fn test_expr_literal_null() {
    let lit = Expr::Literal(Literal::Null);
    assert!(matches!(lit, Expr::Literal(Literal::Null)));
}

#[test]
fn test_expr_literal_boolean() {
    let lit_true = Expr::Literal(Literal::Boolean(true));
    let lit_false = Expr::Literal(Literal::Boolean(false));

    assert!(matches!(lit_true, Expr::Literal(Literal::Boolean(true))));
    assert!(matches!(lit_false, Expr::Literal(Literal::Boolean(false))));
}

#[test]
fn test_expr_literal_float() {
    let lit = Expr::Literal(Literal::Float(3.14));
    assert!(matches!(lit, Expr::Literal(Literal::Float(_))));
    if let Expr::Literal(Literal::Float(f)) = lit {
        assert!((f - 3.14).abs() < 0.001);
    }
}

#[test]
fn test_expr_binary_op_arithmetic() {
    let add = Expr::BinaryOp {
        left: Box::new(Expr::Literal(Literal::Integer(10))),
        op: BinaryOp::Add,
        right: Box::new(Expr::Literal(Literal::Integer(5))),
    };
    assert!(matches!(
        add,
        Expr::BinaryOp {
            op: BinaryOp::Add,
            ..
        }
    ));

    let sub = Expr::BinaryOp {
        left: Box::new(Expr::Literal(Literal::Integer(10))),
        op: BinaryOp::Sub,
        right: Box::new(Expr::Literal(Literal::Integer(5))),
    };
    assert!(matches!(
        sub,
        Expr::BinaryOp {
            op: BinaryOp::Sub,
            ..
        }
    ));

    let mul = Expr::BinaryOp {
        left: Box::new(Expr::Literal(Literal::Integer(10))),
        op: BinaryOp::Mul,
        right: Box::new(Expr::Literal(Literal::Integer(5))),
    };
    assert!(matches!(
        mul,
        Expr::BinaryOp {
            op: BinaryOp::Mul,
            ..
        }
    ));

    let div = Expr::BinaryOp {
        left: Box::new(Expr::Literal(Literal::Integer(10))),
        op: BinaryOp::Div,
        right: Box::new(Expr::Literal(Literal::Integer(5))),
    };
    assert!(matches!(
        div,
        Expr::BinaryOp {
            op: BinaryOp::Div,
            ..
        }
    ));
}

#[test]
fn test_expr_binary_op_comparison() {
    let eq = Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("id"))),
        op: BinaryOp::Eq,
        right: Box::new(Expr::Literal(Literal::Integer(1))),
    };
    assert!(matches!(
        eq,
        Expr::BinaryOp {
            op: BinaryOp::Eq,
            ..
        }
    ));

    let lt = Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("age"))),
        op: BinaryOp::Lt,
        right: Box::new(Expr::Literal(Literal::Integer(18))),
    };
    assert!(matches!(
        lt,
        Expr::BinaryOp {
            op: BinaryOp::Lt,
            ..
        }
    ));

    let gt_eq = Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("price"))),
        op: BinaryOp::GtEq,
        right: Box::new(Expr::Literal(Literal::Float(100.0))),
    };
    assert!(matches!(
        gt_eq,
        Expr::BinaryOp {
            op: BinaryOp::GtEq,
            ..
        }
    ));
}

#[test]
fn test_expr_binary_op_logical() {
    let and = Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("is_active"))),
        op: BinaryOp::And,
        right: Box::new(Expr::Column(ColumnRef::new("is_verified"))),
    };
    assert!(matches!(
        and,
        Expr::BinaryOp {
            op: BinaryOp::And,
            ..
        }
    ));

    let or = Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("status"))),
        op: BinaryOp::Or,
        right: Box::new(Expr::Literal(Literal::String("pending".to_string()))),
    };
    assert!(matches!(
        or,
        Expr::BinaryOp {
            op: BinaryOp::Or,
            ..
        }
    ));
}

#[test]
fn test_expr_unary_op() {
    let neg = Expr::UnaryOp {
        op: UnaryOp::Neg,
        expr: Box::new(Expr::Literal(Literal::Integer(10))),
    };
    assert!(matches!(
        neg,
        Expr::UnaryOp {
            op: UnaryOp::Neg,
            ..
        }
    ));

    let not = Expr::UnaryOp {
        op: UnaryOp::Not,
        expr: Box::new(Expr::Column(ColumnRef::new("is_active"))),
    };
    assert!(matches!(
        not,
        Expr::UnaryOp {
            op: UnaryOp::Not,
            ..
        }
    ));
}

#[test]
fn test_expr_function_call() {
    let func = Expr::Function {
        name: "COUNT".to_string(),
        args: vec![],
        distinct: false,
    };
    assert!(matches!(func, Expr::Function { .. }));
    if let Expr::Function { name, distinct, .. } = func {
        assert_eq!(name, "COUNT");
        assert!(!distinct);
    }
}

#[test]
fn test_expr_function_with_args() {
    let func = Expr::Function {
        name: "SUM".to_string(),
        args: vec![Expr::Column(ColumnRef::new("total"))],
        distinct: false,
    };
    assert!(matches!(func, Expr::Function { .. }));
    if let Expr::Function { name, args, .. } = func {
        assert_eq!(name, "SUM");
        assert_eq!(args.len(), 1);
    }
}

#[test]
fn test_expr_function_distinct() {
    let func = Expr::Function {
        name: "COUNT".to_string(),
        args: vec![Expr::Column(ColumnRef::new("user_id"))],
        distinct: true,
    };
    assert!(matches!(func, Expr::Function { .. }));
    if let Expr::Function {
        name,
        args,
        distinct,
    } = func
    {
        assert_eq!(name, "COUNT");
        assert!(distinct);
        assert_eq!(args.len(), 1);
    }
}

#[test]
fn test_expr_list() {
    let list = Expr::List(vec![
        Expr::Literal(Literal::Integer(1)),
        Expr::Literal(Literal::Integer(2)),
        Expr::Literal(Literal::Integer(3)),
    ]);
    assert!(matches!(list, Expr::List(_)));
    if let Expr::List(items) = list {
        assert_eq!(items.len(), 3);
    }
}

#[test]
fn test_expr_nested_binary_ops() {
    // Represents: (age > 18) AND (status = 'active')
    let expr = Expr::BinaryOp {
        left: Box::new(Expr::BinaryOp {
            left: Box::new(Expr::Column(ColumnRef::new("age"))),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(Literal::Integer(18))),
        }),
        op: BinaryOp::And,
        right: Box::new(Expr::BinaryOp {
            left: Box::new(Expr::Column(ColumnRef::new("status"))),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Literal(Literal::String("active".to_string()))),
        }),
    };

    assert!(matches!(
        expr,
        Expr::BinaryOp {
            op: BinaryOp::And,
            ..
        }
    ));
}

#[test]
fn test_expr_paren() {
    let inner = Box::new(Expr::Literal(Literal::Integer(42)));
    let paren = Expr::Paren(inner);
    assert!(matches!(paren, Expr::Paren(_)));
}

#[test]
fn test_expr_case() {
    let expr = Expr::Case {
        conditions: vec![Expr::BinaryOp {
            left: Box::new(Expr::Column(ColumnRef::new("score"))),
            op: BinaryOp::Gt,
            right: Box::new(Expr::Literal(Literal::Integer(90))),
        }],
        results: vec![Expr::Literal(Literal::String("A".to_string()))],
        else_result: Some(Box::new(Expr::Literal(Literal::String("F".to_string())))),
    };

    assert!(matches!(expr, Expr::Case { .. }));
}

#[test]
fn test_expr_cast() {
    let expr = Expr::Cast {
        expr: Box::new(Expr::Column(ColumnRef::new("value"))),
        type_name: "INTEGER".to_string(),
    };

    assert!(matches!(expr, Expr::Cast { .. }));
}

#[test]
fn test_column_ref_qualified() {
    let col = ColumnRef::new("id").with_table("users");
    assert_eq!(col.qualified(), "users.id");
}

#[test]
fn test_column_ref_unqualified() {
    let col = ColumnRef::new("id");
    assert_eq!(col.qualified(), "id");
}
