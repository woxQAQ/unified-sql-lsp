use std::collections::HashMap;
use std::path::Path;

pub struct TestQuery {
    pub name: String,
    pub sql: String,
    pub dialect: String,
}

pub fn load_test_queries() -> HashMap<String, TestQuery> {
    let mut queries = HashMap::new();

    queries.insert(
        "simple_select".to_string(),
        TestQuery {
            name: "simple_select".to_string(),
            sql: "SELECT id, name FROM users WHERE active = true".to_string(),
            dialect: "mysql".to_string(),
        }
    );

    queries.insert(
        "complex_join".to_string(),
        TestQuery {
            name: "complex_join".to_string(),
            sql: r#"
                SELECT u.id, u.name, o.order_id, p.product_name
                FROM users u
                INNER JOIN orders o ON u.id = o.user_id
                LEFT JOIN order_items oi ON o.order_id = oi.order_id
                LEFT JOIN products p ON oi.product_id = p.id
                WHERE o.created_at > '2024-01-01'
            "#.trim().to_string(),
            dialect: "mysql".to_string(),
        }
    );

    queries.insert(
        "nested_subquery".to_string(),
        TestQuery {
            name: "nested_subquery".to_string(),
            sql: r#"
                SELECT u.name, u.email
                FROM users u
                WHERE u.id IN (
                    SELECT o.user_id
                    FROM orders o
                    WHERE o.total > (
                        SELECT AVG(o2.total)
                        FROM orders o2
                        WHERE o2.user_id = o.user_id
                    )
                )
            "#.trim().to_string(),
            dialect: "postgresql".to_string(),
        }
    );

    queries
}

pub fn load_queries_from_files() -> Vec<TestQuery> {
    // For future: load from src/fixtures/queries/*.sql
    vec![]
}
