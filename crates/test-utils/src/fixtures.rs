// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Test fixtures and sample SQL queries

/// Sample SQL queries for testing
pub struct SqlFixtures;

impl SqlFixtures {
    // ===== Basic SELECT queries =====

    /// Simple SELECT with column list
    pub const fn simple_select() -> &'static str {
        "SELECT id, email, name FROM users"
    }

    /// SELECT with all columns
    pub const fn select_all() -> &'static str {
        "SELECT * FROM users"
    }

    /// SELECT with WHERE clause
    pub const fn select_with_where() -> &'static str {
        "SELECT id, name FROM users WHERE email LIKE '%@example.com'"
    }

    /// SELECT with ORDER BY
    pub const fn select_with_order() -> &'static str {
        "SELECT id, name FROM users ORDER BY name ASC"
    }

    /// SELECT with LIMIT
    pub const fn select_with_limit() -> &'static str {
        "SELECT * FROM users LIMIT 10"
    }

    /// SELECT with LIMIT and OFFSET (MySQL style)
    pub const fn select_with_limit_offset_mysql() -> &'static str {
        "SELECT * FROM users LIMIT 10 OFFSET 20"
    }

    /// SELECT with LIMIT and OFFSET (PostgreSQL style)
    pub const fn select_with_limit_offset_postgres() -> &'static str {
        "SELECT * FROM users LIMIT 10 OFFSET 20"
    }

    // ===== JOIN queries =====

    /// INNER JOIN
    pub const fn inner_join() -> &'static str {
        "SELECT users.name, orders.total
         FROM users
         INNER JOIN orders ON users.id = orders.user_id"
    }

    /// LEFT JOIN
    pub const fn left_join() -> &'static str {
        "SELECT users.name, orders.total
         FROM users
         LEFT JOIN orders ON users.id = orders.user_id"
    }

    /// Multiple JOINs
    pub const fn multiple_joins() -> &'static str {
        "SELECT u.name, o.total, p.name
         FROM users u
         INNER JOIN orders o ON u.id = o.user_id
         INNER JOIN products p ON o.product_id = p.id"
    }

    // ===== Aggregation queries =====

    /// Simple COUNT
    pub const fn count_aggregation() -> &'static str {
        "SELECT COUNT(*) FROM users"
    }

    /// GROUP BY
    pub const fn group_by() -> &'static str {
        "SELECT status, COUNT(*) as count
         FROM orders
         GROUP BY status"
    }

    /// GROUP BY with HAVING
    pub const fn group_by_having() -> &'static str {
        "SELECT user_id, SUM(total) as total_spent
         FROM orders
         GROUP BY user_id
         HAVING SUM(total) > 1000"
    }

    // ===== Subqueries =====

    /// Simple subquery
    pub const fn simple_subquery() -> &'static str {
        "SELECT name FROM users
         WHERE id IN (SELECT user_id FROM orders WHERE total > 100)"
    }

    /// Correlated subquery
    pub const fn correlated_subquery() -> &'static str {
        "SELECT u.name, (SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id) as order_count
         FROM users u"
    }

    // ===== CTE (Common Table Expressions) =====

    /// CTE query
    pub const fn with_cte() -> &'static str {
        "WITH user_orders AS (
             SELECT user_id, COUNT(*) as order_count
             FROM orders
             GROUP BY user_id
         )
         SELECT u.name, uo.order_count
         FROM users u
         INNER JOIN user_orders uo ON u.id = uo.user_id"
    }

    // ===== INSERT/UPDATE/DELETE =====

    /// Simple INSERT
    pub const fn simple_insert() -> &'static str {
        "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User')"
    }

    /// Bulk INSERT
    pub const fn bulk_insert() -> &'static str {
        "INSERT INTO users (email, name) VALUES
         ('user1@example.com', 'User 1'),
         ('user2@example.com', 'User 2'),
         ('user3@example.com', 'User 3')"
    }

    /// Simple UPDATE
    pub const fn simple_update() -> &'static str {
        "UPDATE users SET name = 'Updated Name' WHERE id = 1"
    }

    /// DELETE with WHERE
    pub const fn simple_delete() -> &'static str {
        "DELETE FROM users WHERE created_at < '2020-01-01'"
    }

    // ===== DDL (Data Definition Language) =====

    /// CREATE TABLE
    pub const fn create_table() -> &'static str {
        "CREATE TABLE users (
            id BIGINT PRIMARY KEY,
            email VARCHAR(255) NOT NULL,
            name VARCHAR(100),
            created_at TIMESTAMP
        )"
    }

    /// CREATE INDEX
    pub const fn create_index() -> &'static str {
        "CREATE INDEX idx_users_email ON users(email)"
    }

    // ===== MySQL-specific queries =====

    /// MySQL REPLACE
    pub const fn mysql_replace() -> &'static str {
        "REPLACE INTO users (id, email, name) VALUES (1, 'test@example.com', 'Test')"
    }

    /// MySQL SHOW TABLES
    pub const fn mysql_show_tables() -> &'static str {
        "SHOW TABLES"
    }

    /// MySQL DESCRIBE
    pub const fn mysql_describe() -> &'static str {
        "DESCRIBE users"
    }

    // ===== PostgreSQL-specific queries =====

    /// PostgreSQL RETURNING clause
    pub const fn postgres_insert_returning() -> &'static str {
        "INSERT INTO users (email, name) VALUES ('test@example.com', 'Test') RETURNING id"
    }

    /// PostgreSQL DISTINCT ON
    pub const fn postgres_distinct_on() -> &'static str {
        "SELECT DISTINCT ON (user_id) user_id, created_at
         FROM orders
         ORDER BY user_id, created_at DESC"
    }

    // ===== Error cases =====

    /// Query with syntax error (missing FROM)
    pub const fn error_missing_from() -> &'static str {
        "SELECT id, name, email"
    }

    /// Query with undefined table
    pub const fn error_undefined_table() -> &'static str {
        "SELECT * FROM non_existent_table"
    }

    /// Query with undefined column
    pub const fn error_undefined_column() -> &'static str {
        "SELECT non_existent_column FROM users"
    }

    /// Query with ambiguous column
    pub const fn error_ambiguous_column() -> &'static str {
        "SELECT id FROM users u INNER JOIN orders o"
    }
}

/// Sample schema definitions for testing
pub struct SchemaFixtures;

impl SchemaFixtures {
    /// Get the standard test schema SQL
    pub fn standard_schema() -> String {
        format!(
            "{}\n\n{}\n\n{}",
            Self::users_table(),
            Self::orders_table(),
            Self::products_table()
        )
    }

    /// Users table definition
    pub fn users_table() -> String {
        r#"
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    email VARCHAR(255) NOT NULL UNIQUE,
    name VARCHAR(100),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#
        .to_string()
    }

    /// Orders table definition
    pub fn orders_table() -> String {
        r#"
CREATE TABLE orders (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    user_id BIGINT NOT NULL,
    total DECIMAL(10, 2),
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#
        .to_string()
    }

    /// Products table definition
    pub fn products_table() -> String {
        r#"
CREATE TABLE products (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL,
    price DECIMAL(10, 2) NOT NULL,
    stock INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#
        .to_string()
    }

    /// Order items table (many-to-many)
    pub fn order_items_table() -> String {
        r#"
CREATE TABLE order_items (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    order_id BIGINT NOT NULL,
    product_id BIGINT NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    price DECIMAL(10, 2) NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (product_id) REFERENCES products(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4;
"#
        .to_string()
    }
}
