-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- MySQL Schema Definition
-- ============================================================================
-- This file defines the complete database schema for end-to-end testing.
-- It covers:
--   - Basic tables (users, orders, products, order_items)
--   - Advanced scenarios (self-referencing, many-to-many, partitioning)
--   - Various data types and constraints
--   - Indexes for testing completion and diagnostics
-- ============================================================================

-- Drop existing tables if they exist (in correct dependency order)
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS post_tags;
DROP TABLE IF EXISTS posts;
DROP TABLE IF EXISTS users;
DROP TABLE IF EXISTS tags;
DROP TABLE IF EXISTS employees;
DROP TABLE IF EXISTS logs;

-- ============================================================================
-- BASIC TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Users table
-- Tests: basic CRUD, various data types, NOT NULL, DEFAULT, UNIQUE
-- ----------------------------------------------------------------------------
CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    full_name VARCHAR(100),
    age INT,
    balance DECIMAL(10, 2) DEFAULT 0.00,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    last_login DATETIME,
    bio TEXT,
    profile_image VARCHAR(255),
    phone VARCHAR(20),
    -- Indexes for testing completion
    INDEX idx_username (username),
    INDEX idx_email (email),
    INDEX idx_created_at (created_at),
    INDEX idx_is_active (is_active)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Products table
-- Tests: DECIMAL precision, ENUM type, CHECK constraints (MySQL 8.0.16+)
-- ----------------------------------------------------------------------------
CREATE TABLE products (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    cost DECIMAL(10, 2),
    quantity_in_stock INT DEFAULT 0,
    category ENUM('electronics', 'clothing', 'books', 'home', 'sports', 'toys'),
    is_available BOOLEAN DEFAULT TRUE,
    weight DECIMAL(8, 3),
    sku VARCHAR(50) UNIQUE,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    -- Constraints
    CONSTRAINT chk_price_positive CHECK (price >= 0),
    CONSTRAINT chk_quantity_non_negative CHECK (quantity_in_stock >= 0),
    CONSTRAINT chk_cost_not_greater_than_price CHECK (cost IS NULL OR cost <= price),
    -- Indexes
    INDEX idx_name (name),
    INDEX idx_category (category),
    INDEX idx_price (price),
    INDEX idx_is_available (is_available),
    INDEX idx_sku (sku)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Orders table
-- Tests: foreign keys, status management, date ranges
-- ----------------------------------------------------------------------------
CREATE TABLE orders (
    id INT PRIMARY KEY AUTO_INCREMENT,
    user_id INT NOT NULL,
    order_date DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    total_amount DECIMAL(12, 2) NOT NULL,
    status ENUM('pending', 'processing', 'shipped', 'delivered', 'cancelled', 'refunded')
        NOT NULL DEFAULT 'pending',
    payment_method ENUM('credit_card', 'debit_card', 'paypal', 'bank_transfer', 'cash'),
    shipping_address TEXT,
    billing_address TEXT,
    notes TEXT,
    shipped_at DATETIME,
    delivered_at DATETIME,
    -- Foreign key to users
    CONSTRAINT fk_orders_user_id
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE,
    -- Constraints
    CONSTRAINT chk_total_amount_positive CHECK (total_amount >= 0),
    CONSTRAINT chk_delivered_after_shipped
        CHECK (delivered_at IS NULL OR shipped_at IS NOT NULL),
    -- Indexes
    INDEX idx_user_id (user_id),
    INDEX idx_order_date (order_date),
    INDEX idx_status (status),
    INDEX idx_total_amount (total_amount),
    -- Composite index for testing multi-column completion
    INDEX idx_user_status (user_id, status),
    INDEX idx_date_status (order_date, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Order Items table
-- Tests: many-to-one relationship, composite keys, quantity calculations
-- ----------------------------------------------------------------------------
CREATE TABLE order_items (
    id INT PRIMARY KEY AUTO_INCREMENT,
    order_id INT NOT NULL,
    product_id INT NOT NULL,
    quantity INT NOT NULL DEFAULT 1,
    unit_price DECIMAL(10, 2) NOT NULL,
    discount_percent DECIMAL(5, 2) DEFAULT 0.00,
    subtotal DECIMAL(10, 2) GENERATED ALWAYS AS (quantity * unit_price * (1 - discount_percent / 100)) STORED,
    notes TEXT,
    -- Foreign keys
    CONSTRAINT fk_order_items_order_id
        FOREIGN KEY (order_id) REFERENCES orders(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,
    CONSTRAINT fk_order_items_product_id
        FOREIGN KEY (product_id) REFERENCES products(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE,
    -- Constraints
    CONSTRAINT chk_quantity_positive CHECK (quantity > 0),
    CONSTRAINT chk_unit_price_positive CHECK (unit_price >= 0),
    CONSTRAINT chk_discount_valid CHECK (discount_percent >= 0 AND discount_percent <= 100),
    -- Indexes
    INDEX idx_order_id (order_id),
    INDEX idx_product_id (product_id),
    -- Unique constraint to prevent duplicate products in same order
    UNIQUE KEY uk_order_product (order_id, product_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ============================================================================
-- ADVANCED SCENARIO TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Employees table (self-referencing)
-- Tests: hierarchical data, recursive relationships, organizational structure
-- ----------------------------------------------------------------------------
CREATE TABLE employees (
    id INT PRIMARY KEY AUTO_INCREMENT,
    first_name VARCHAR(50) NOT NULL,
    last_name VARCHAR(50) NOT NULL,
    email VARCHAR(100) NOT NULL UNIQUE,
    manager_id INT,
    department VARCHAR(50),
    position VARCHAR(100),
    salary DECIMAL(12, 2),
    hire_date DATE NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    -- Self-referencing foreign key for organizational hierarchy
    CONSTRAINT fk_employees_manager_id
        FOREIGN KEY (manager_id) REFERENCES employees(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE,
    -- Constraints
    CONSTRAINT chk_salary_positive CHECK (salary IS NULL OR salary > 0),
    -- Indexes
    INDEX idx_manager_id (manager_id),
    INDEX idx_department (department),
    INDEX idx_email (email),
    INDEX idx_name (last_name, first_name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Posts table
-- Tests: content management, publication workflow, authorship
-- ----------------------------------------------------------------------------
CREATE TABLE posts (
    id INT PRIMARY KEY AUTO_INCREMENT,
    title VARCHAR(200) NOT NULL,
    slug VARCHAR(200) UNIQUE,
    content TEXT NOT NULL,
    excerpt TEXT,
    author_id INT NOT NULL,
    status ENUM('draft', 'published', 'archived') DEFAULT 'draft',
    view_count INT DEFAULT 0,
    published_at DATETIME,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    -- Foreign key to users (author)
    CONSTRAINT fk_posts_author_id
        FOREIGN KEY (author_id) REFERENCES users(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,
    -- Indexes
    INDEX idx_slug (slug),
    INDEX idx_author_id (author_id),
    INDEX idx_status (status),
    INDEX idx_published_at (published_at),
    INDEX idx_view_count (view_count)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Tags table
-- Tests: simple reference table for many-to-many relationships
-- ----------------------------------------------------------------------------
CREATE TABLE tags (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) UNIQUE,
    description TEXT,
    color VARCHAR(7),  -- Hex color code
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    -- Indexes
    INDEX idx_name (name),
    INDEX idx_slug (slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Post_Tags table (many-to-many)
-- Tests: junction tables, composite foreign keys, many-to-many relationships
-- ----------------------------------------------------------------------------
CREATE TABLE post_tags (
    post_id INT NOT NULL,
    tag_id INT NOT NULL,
    tagged_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    -- Composite primary key
    PRIMARY KEY (post_id, tag_id),
    -- Foreign keys
    CONSTRAINT fk_post_tags_post_id
        FOREIGN KEY (post_id) REFERENCES posts(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,
    CONSTRAINT fk_post_tags_tag_id
        FOREIGN KEY (tag_id) REFERENCES tags(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE,
    -- Index for reverse lookup
    INDEX idx_tag_id (tag_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ----------------------------------------------------------------------------
-- Logs table (partitioned)
-- Tests: time-series data, partitioning (if supported), large data scenarios
-- Note: Actual partitioning syntax depends on MySQL version
-- ----------------------------------------------------------------------------
CREATE TABLE logs (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    level ENUM('debug', 'info', 'warning', 'error', 'critical') NOT NULL,
    message TEXT NOT NULL,
    context JSON,
    source VARCHAR(100),
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    -- Indexes
    INDEX idx_level (level),
    INDEX idx_created_at (created_at),
    INDEX idx_source (source)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
-- Partition by year (MySQL 8.0+)
-- PARTITION BY RANGE (YEAR(created_at)) (
--     PARTITION p2023 VALUES LESS THAN (2024),
--     PARTITION p2024 VALUES LESS THAN (2025),
--     PARTITION p2025 VALUES LESS THAN (2026),
--     PARTITION p_future VALUES LESS THAN MAXVALUE
-- );

-- ============================================================================
-- VIEWS FOR TESTING COMPLETION
-- ============================================================================

-- View for active users (tests view completion and hover)
CREATE VIEW v_active_users AS
SELECT
    id,
    username,
    email,
    full_name,
    created_at
FROM users
WHERE is_active = TRUE;

-- View for order summaries (tests aggregate functions in views)
CREATE VIEW v_order_summaries AS
SELECT
    o.id,
    o.user_id,
    u.username,
    o.order_date,
    o.total_amount,
    o.status,
    COUNT(oi.id) AS item_count,
    SUM(oi.quantity) AS total_items
FROM orders o
JOIN users u ON o.user_id = u.id
LEFT JOIN order_items oi ON o.id = oi.order_id
GROUP BY o.id, o.user_id, u.username, o.order_date, o.total_amount, o.status;

-- ============================================================================
-- STORED PROCEDURES FOR TESTING
-- ============================================================================

DELIMITER //

-- Simple procedure for testing procedural code completion
CREATE PROCEDURE get_user_orders(IN p_user_id INT)
BEGIN
    SELECT
        o.id,
        o.order_date,
        o.total_amount,
        o.status,
        COUNT(oi.id) AS item_count
    FROM orders o
    LEFT JOIN order_items oi ON o.id = oi.order_id
    WHERE o.user_id = p_user_id
    GROUP BY o.id, o.order_date, o.total_amount, o.status
    ORDER BY o.order_date DESC;
END //

DELIMITER ;

-- ============================================================================
-- END OF SCHEMA DEFINITION
-- ============================================================================
