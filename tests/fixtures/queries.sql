-- Copyright (c) 2025 woxQAQ
--
-- Licensed under the MIT License or Apache License 2.0
-- See LICENSE files for details

-- Sample SQL queries for testing
-- These queries are used across multiple test files

-- ===== Basic SELECT Queries =====

-- Simple SELECT with column list
SELECT id, email, name FROM users;

-- SELECT with all columns
SELECT * FROM users;

-- SELECT with WHERE clause
SELECT id, name FROM users WHERE email LIKE '%@example.com';

-- SELECT with ORDER BY
SELECT id, name FROM users ORDER BY name ASC;

-- SELECT with LIMIT (MySQL)
SELECT * FROM users LIMIT 10;

-- SELECT with LIMIT and OFFSET
SELECT * FROM users LIMIT 10 OFFSET 20;

-- ===== JOIN Queries =====

-- INNER JOIN
SELECT users.name, orders.total
FROM users
INNER JOIN orders ON users.id = orders.user_id;

-- LEFT JOIN
SELECT users.name, orders.total
FROM users
LEFT JOIN orders ON users.id = orders.user_id;

-- Multiple JOINs
SELECT u.name, o.total, p.name
FROM users u
INNER JOIN orders o ON u.id = o.user_id
INNER JOIN products p ON o.product_id = p.id;

-- ===== Aggregation Queries =====

-- Simple COUNT
SELECT COUNT(*) FROM users;

-- GROUP BY
SELECT status, COUNT(*) as count
FROM orders
GROUP BY status;

-- GROUP BY with HAVING
SELECT user_id, SUM(total) as total_spent
FROM orders
GROUP BY user_id
HAVING SUM(total) > 1000;

-- ===== Subqueries =====

-- Simple subquery
SELECT name FROM users
WHERE id IN (SELECT user_id FROM orders WHERE total > 100);

-- Correlated subquery
SELECT u.name, (
    SELECT COUNT(*) FROM orders o WHERE o.user_id = u.id
) as order_count
FROM users u;

-- ===== CTE (Common Table Expressions) =====

WITH user_orders AS (
    SELECT user_id, COUNT(*) as order_count
    FROM orders
    GROUP BY user_id
)
SELECT u.name, uo.order_count
FROM users u
INNER JOIN user_orders uo ON u.id = uo.user_id;

-- ===== INSERT/UPDATE/DELETE =====

-- Simple INSERT
INSERT INTO users (email, name) VALUES ('test@example.com', 'Test User');

-- Bulk INSERT
INSERT INTO users (email, name) VALUES
    ('user1@example.com', 'User 1'),
    ('user2@example.com', 'User 2'),
    ('user3@example.com', 'User 3');

-- Simple UPDATE
UPDATE users SET name = 'Updated Name' WHERE id = 1;

-- DELETE with WHERE
DELETE FROM users WHERE created_at < '2020-01-01';

-- ===== DDL (Data Definition Language) =====

-- CREATE TABLE
CREATE TABLE users (
    id BIGINT PRIMARY KEY,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(100),
    created_at TIMESTAMP
);

-- CREATE INDEX
CREATE INDEX idx_users_email ON users(email);

-- ===== Error Cases =====

-- Missing FROM clause (syntax error)
SELECT id, name, email;

-- Undefined table (semantic error)
SELECT * FROM non_existent_table;

-- Undefined column (semantic error)
SELECT non_existent_column FROM users;

-- Ambiguous column (semantic error)
SELECT id FROM users u INNER JOIN orders o;
