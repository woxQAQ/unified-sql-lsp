// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Unit tests for IR Query representation

use unified_sql_lsp_ir::{
    BinaryOp, ColumnRef, Dialect, Expr, Join, JoinCondition, JoinType, Literal, Query,
    SelectItem, SelectStatement, SetOp, TableRef,
};

#[test]
fn test_query_new() {
    let query = Query::new(Dialect::MySQL);
    assert_eq!(query.dialect, Dialect::MySQL);
    assert!(matches!(query.body, SetOp::Select(_)));
}

#[test]
fn test_query_with_limit() {
    let query = Query::new(Dialect::MySQL)
        .with_limit(Expr::Literal(Literal::Integer(10)))
        .with_offset(Expr::Literal(Literal::Integer(20)));

    assert!(query.limit.is_some());
    assert!(query.offset.is_some());
    if let Some(Expr::Literal(Literal::Integer(limit))) = query.limit {
        assert_eq!(limit, 10);
    }
    if let Some(Expr::Literal(Literal::Integer(offset))) = query.offset {
        assert_eq!(offset, 20);
    }
}

#[test]
fn test_query_with_order_by() {
    use unified_sql_lsp_ir::query::SortDirection;

    let order_by = vec![unified_sql_lsp_ir::OrderBy {
        expr: Expr::Column(ColumnRef::new("id")),
        direction: Some(SortDirection::Asc),
    }];

    let query = Query::new(Dialect::PostgreSQL)
        .with_order_by(order_by);

    assert!(query.order_by.is_some());
    assert_eq!(query.order_by.as_ref().unwrap().len(), 1);
}

#[test]
fn test_select_statement_default() {
    let select = SelectStatement::default();
    assert!(select.projection.is_empty());
    assert!(select.from.is_empty());
    assert!(!select.distinct);
}

#[test]
fn test_select_statement_with_projection() {
    let mut select = SelectStatement::default();
    select.projection = vec![
        SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("id"))),
        SelectItem::UnnamedExpr(Expr::Column(ColumnRef::new("name"))),
    ];

    assert_eq!(select.projection.len(), 2);
}

#[test]
fn test_select_statement_with_from() {
    let mut select = SelectStatement::default();
    select.from = vec![TableRef {
        name: "users".to_string(),
        alias: None,
        joins: Vec::new(),
    }];

    assert_eq!(select.from.len(), 1);
    assert_eq!(select.from[0].name, "users");
}

#[test]
fn test_table_ref_construction() {
    let table = TableRef {
        name: "users".to_string(),
        alias: None,
        joins: Vec::new(),
    };
    assert_eq!(table.name, "users");
    assert!(table.alias.is_none());
    assert!(table.joins.is_empty());
}

#[test]
fn test_table_ref_with_alias() {
    let table = TableRef {
        name: "users".to_string(),
        alias: Some("u".to_string()),
        joins: Vec::new(),
    };
    assert_eq!(table.alias, Some("u".to_string()));
}

#[test]
fn test_join_construction() {
    let table_ref = TableRef {
        name: "orders".to_string(),
        alias: None,
        joins: Vec::new(),
    };
    let join = Join {
        join_type: JoinType::Inner,
        table: table_ref,
        condition: JoinCondition::Natural,
    };

    assert!(matches!(join.join_type, JoinType::Inner));
    assert_eq!(join.table.name, "orders");
}

#[test]
fn test_join_with_condition() {
    let table_ref = TableRef {
        name: "orders".to_string(),
        alias: None,
        joins: Vec::new(),
    };

    let join = Join {
        join_type: JoinType::Inner,
        table: table_ref,
        condition: JoinCondition::On(Expr::BinaryOp {
            left: Box::new(Expr::Column(ColumnRef {
                table: Some("users".to_string()),
                column: "id".to_string(),
            })),
            op: BinaryOp::Eq,
            right: Box::new(Expr::Column(ColumnRef {
                table: Some("orders".to_string()),
                column: "user_id".to_string(),
            })),
        }),
    };

    assert!(matches!(join.condition, JoinCondition::On(_)));
}

#[test]
fn test_join_types() {
    let table = TableRef {
        name: "orders".to_string(),
        alias: None,
        joins: Vec::new(),
    };

    let inner_join = Join {
        join_type: JoinType::Inner,
        table: table.clone(),
        condition: JoinCondition::Natural,
    };
    assert!(matches!(inner_join.join_type, JoinType::Inner));

    let left_join = Join {
        join_type: JoinType::Left,
        table: table.clone(),
        condition: JoinCondition::Natural,
    };
    assert!(matches!(left_join.join_type, JoinType::Left));

    let right_join = Join {
        join_type: JoinType::Right,
        table: table.clone(),
        condition: JoinCondition::Natural,
    };
    assert!(matches!(right_join.join_type, JoinType::Right));

    let full_join = Join {
        join_type: JoinType::Full,
        table: table.clone(),
        condition: JoinCondition::Natural,
    };
    assert!(matches!(full_join.join_type, JoinType::Full));

    let cross_join = Join {
        join_type: JoinType::Cross,
        table,
        condition: JoinCondition::Natural,
    };
    assert!(matches!(cross_join.join_type, JoinType::Cross));
}

#[test]
fn test_table_ref_with_joins() {
    let mut table = TableRef {
        name: "users".to_string(),
        alias: None,
        joins: Vec::new(),
    };

    let join1 = Join {
        join_type: JoinType::Inner,
        table: TableRef {
            name: "orders".to_string(),
            alias: Some("o".to_string()),
            joins: Vec::new(),
        },
        condition: JoinCondition::Natural,
    };

    let join2 = Join {
        join_type: JoinType::Left,
        table: TableRef {
            name: "products".to_string(),
            alias: Some("p".to_string()),
            joins: Vec::new(),
        },
        condition: JoinCondition::Natural,
    };

    table.joins.push(join1);
    table.joins.push(join2);

    assert_eq!(table.joins.len(), 2);
}

#[test]
fn test_select_item_wildcard() {
    let wildcard = SelectItem::Wildcard;
    assert!(matches!(wildcard, SelectItem::Wildcard));
}

#[test]
fn test_select_item_qualified_wildcard() {
    let qw = SelectItem::QualifiedWildcard("users".to_string());
    assert!(matches!(qw, SelectItem::QualifiedWildcard(_)));
    if let SelectItem::QualifiedWildcard(table) = qw {
        assert_eq!(table, "users");
    }
}

#[test]
fn test_select_item_aliased_expr() {
    let expr = Expr::Column(ColumnRef::new("id"));
    let aliased = SelectItem::AliasedExpr {
        expr,
        alias: "user_id".to_string(),
    };

    assert!(matches!(aliased, SelectItem::AliasedExpr { .. }));
    if let SelectItem::AliasedExpr { expr: e, alias } = aliased {
        assert_eq!(alias, "user_id");
        assert!(matches!(e, Expr::Column(_)));
    }
}

#[test]
fn test_query_dialects() {
    let mysql_query = Query::new(Dialect::MySQL);
    assert_eq!(mysql_query.dialect, Dialect::MySQL);

    let pg_query = Query::new(Dialect::PostgreSQL);
    assert_eq!(pg_query.dialect, Dialect::PostgreSQL);
}

#[test]
fn test_select_statement_with_where() {
    let mut select = SelectStatement::default();
    select.where_clause = Some(Expr::BinaryOp {
        left: Box::new(Expr::Column(ColumnRef::new("id"))),
        op: BinaryOp::Gt,
        right: Box::new(Expr::Literal(Literal::Integer(10))),
    });

    assert!(select.where_clause.is_some());
}

#[test]
fn test_select_statement_with_group_by() {
    let mut select = SelectStatement::default();
    select.group_by = vec![
        Expr::Column(ColumnRef::new("category")),
        Expr::Column(ColumnRef::new("status")),
    ];

    assert_eq!(select.group_by.len(), 2);
}

#[test]
fn test_query_with_ctes() {
    use unified_sql_lsp_ir::CommonTableExpr;

    let mut query = Query::new(Dialect::PostgreSQL);
    query.ctes = vec![CommonTableExpr {
        name: "user_counts".to_string(),
        columns: Vec::new(),
        query: Box::new(Query::new(Dialect::PostgreSQL)),
        materialized: None,
    }];

    assert_eq!(query.ctes.len(), 1);
    assert_eq!(query.ctes[0].name, "user_counts");
}
