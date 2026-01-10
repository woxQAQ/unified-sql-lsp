// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Debug tests to understand parse failures

use tree_sitter::Parser;
use unified_sql_grammar::language_for_dialect;
use unified_sql_lsp_ir::Dialect;

fn print_tree(node: tree_sitter::Node, depth: usize) {
    let indent = "  ".repeat(depth);
    let marker = if node.is_error() {
        " ERROR"
    } else if node.is_missing() {
        " MISSING"
    } else {
        ""
    };
    println!(
        "{}{}: {}",
        indent,
        node.kind(),
        marker
    );
    for i in 0..node.child_count() {
        print_tree(node.child(i).unwrap(), depth + 1);
    }
}

#[test]
fn debug_complex_query() {
    let language = language_for_dialect(Dialect::MySQL).expect("MySQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Complex query with JOINs, aggregates, and subquery
    let source = r#"
        SELECT u.id, u.name, COUNT(o.id) as order_count
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        WHERE u.created_at > '2024-01-01'
        GROUP BY u.id, u.name
        HAVING COUNT(o.id) > 5
        ORDER BY order_count DESC
        LIMIT 10
    "#;

    let tree = parser.parse(source, None).expect("Failed to parse");

    println!("=== Has error: {} ===", tree.root_node().has_error());
    println!("=== Parse tree ===");
    print_tree(tree.root_node(), 0);
}

#[test]
fn debug_multiple_statements() {
    let language =
        language_for_dialect(Dialect::PostgreSQL).expect("PostgreSQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Multiple statements separated by semicolons
    let source = "CREATE TABLE users (id INT); INSERT INTO users VALUES (1); SELECT * FROM users;";
    let tree = parser.parse(source, None).expect("Failed to parse");

    println!("=== Has error: {} ===", tree.root_node().has_error());
    println!("=== Parse tree ===");
    print_tree(tree.root_node(), 0);
}

#[test]
fn debug_postgresql_distinct_on() {
    let language =
        language_for_dialect(Dialect::PostgreSQL).expect("PostgreSQL language not found");

    let mut parser = Parser::new();
    parser
        .set_language(&language)
        .expect("Failed to set language");

    // Test PostgreSQL-specific DISTINCT ON syntax
    let source = "SELECT DISTINCT ON (name) name, id FROM users";
    let tree = parser.parse(source, None).expect("Failed to parse");

    println!("=== Has error: {} ===", tree.root_node().has_error());
    println!("=== Parse tree ===");
    print_tree(tree.root_node(), 0);
}
