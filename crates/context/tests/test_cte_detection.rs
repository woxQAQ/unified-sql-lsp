use tree_sitter::Parser;
use unified_sql_grammar::{DialectVersion, language_for_dialect_with_version};
use unified_sql_lsp_context::completion::detect_completion_context;
use unified_sql_lsp_context::cst_utils::Position;
use unified_sql_lsp_ir::Dialect;

#[test]
fn test_cte_name_completion_detection() {
    let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
        .expect("Failed to get MySQL 8.0 language");

    let mut parser = Parser::new();
    parser.set_language(&lang).expect("Failed to set language");

    // Test case 1: "WITH | AS (SELECT * FROM users)"
    let source = "WITH  AS (SELECT * FROM users)"; // Two spaces after WITH (cursor marker removed)
    let tree = parser.parse(source, None).expect("Failed to parse");
    let root = tree.root_node();
    let position = Position::new(0, 5); // Cursor at character 5 (after "WITH ")

    let ctx = detect_completion_context(&root, position, source);
    println!("Test 1 - source='{}', position={:?}", source, position);
    println!("Test 1 - context: {:?}", ctx);

    match ctx {
        unified_sql_lsp_context::CompletionContext::CteDefinition { .. } => {
            println!("Test 1 PASSED: Detected CteDefinition context");
        }
        _ => {
            println!("Test 1 FAILED: Expected CteDefinition, got {:?}", ctx);
            panic!("Test 1 failed");
        }
    }
}

#[test]
fn test_cte_column_completion_detection() {
    let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
        .expect("Failed to get MySQL 8.0 language");

    let mut parser = Parser::new();
    parser.set_language(&lang).expect("Failed to set language");

    // Test case 2: "WITH user_cte AS (SELECT | FROM users)"
    let source = "WITH user_cte AS (SELECT  FROM users)"; // Two spaces after SELECT (cursor marker removed)
    let tree = parser.parse(source, None).expect("Failed to parse");
    let root = tree.root_node();
    let position = Position::new(0, 27); // Cursor at character 27 (after "SELECT ")

    let ctx = detect_completion_context(&root, position, source);
    println!("\nTest 2 - source='{}', position={:?}", source, position);
    println!("Test 2 - context: {:?}", ctx);

    match ctx {
        unified_sql_lsp_context::CompletionContext::SelectProjection { tables, .. } => {
            println!("Test 2: Detected SelectProjection with tables={:?}", tables);
            if tables.contains(&"users".to_string()) {
                println!("Test 2 PASSED: Detected SelectProjection with users table");
            } else {
                println!(
                    "Test 2 FAILED: Expected 'users' in tables, got {:?}",
                    tables
                );
                panic!("Test 2 failed: 'users' not in tables");
            }
        }
        unified_sql_lsp_context::CompletionContext::Keywords { .. } => {
            println!("Test 2 FAILED: Got Keywords context instead of SelectProjection");
            println!("This is the bug - it's returning keywords instead of columns");
            panic!("Test 2 failed: Got Keywords");
        }
        _ => {
            println!("Test 2 FAILED: Expected SelectProjection, got {:?}", ctx);
            panic!("Test 2 failed");
        }
    }
}

#[test]
fn test_window_function_partition_by_detection() {
    let lang = language_for_dialect_with_version(Dialect::MySQL, Some(DialectVersion::MySQL80))
        .expect("Failed to get MySQL 8.0 language");

    let mut parser = Parser::new();
    parser.set_language(&lang).expect("Failed to set language");

    // Test case 3: "SELECT ROW_NUMBER() OVER (PARTITION BY |) FROM users"
    // The cursor marker | is at position 39 (after "PARTITION BY " and before ")")
    // After stripping, the source is "SELECT ROW_NUMBER() OVER (PARTITION BY ) FROM users"
    // The cursor position is where | was, which is 39
    let source = "SELECT ROW_NUMBER() OVER (PARTITION BY ) FROM users"; // Cursor marker removed
    let tree = parser.parse(source, None).expect("Failed to parse");
    let root = tree.root_node();
    let position = Position::new(0, 39); // Cursor at position 39 (where | was, between "BY " and ")")

    // Debug: walk the tree to find what nodes exist
    println!("\nTest 3 - source='{}', position={:?}", source, position);
    print_tree(&root, source, 0, 4);

    let ctx = detect_completion_context(&root, position, source);
    println!("Test 3 - context: {:?}", ctx);

    match ctx {
        unified_sql_lsp_context::CompletionContext::WindowFunctionClause {
            tables,
            window_part,
            ..
        } => {
            println!(
                "Test 3: Detected WindowFunctionClause with tables={:?}, window_part={:?}",
                tables, window_part
            );
            if tables.contains(&"users".to_string()) {
                println!("Test 3 PASSED: Detected WindowFunctionClause with users table");
            } else {
                println!("Test 3 WARNING: 'users' not in tables, got {:?}", tables);
            }
        }
        _ => {
            println!(
                "Test 3 FAILED: Expected WindowFunctionClause, got {:?}",
                ctx
            );
            panic!("Test 3 failed");
        }
    }
}

fn print_tree(node: &tree_sitter::Node, source: &str, depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let indent = "  ".repeat(depth);
    let text = &source[node.start_byte()..node.end_byte().min(source.len())];
    let text_preview: String = text.chars().take(30).collect();
    if text.len() > 30 {
        println!(
            "{}[{}] '{}'... ({} bytes)",
            indent,
            node.kind(),
            text_preview,
            node.end_byte() - node.start_byte()
        );
    } else {
        println!(
            "{}[{}] '{}' ({} bytes)",
            indent,
            node.kind(),
            text_preview,
            node.end_byte() - node.start_byte()
        );
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(&child, source, depth + 1, max_depth);
    }
    drop(cursor);
}
