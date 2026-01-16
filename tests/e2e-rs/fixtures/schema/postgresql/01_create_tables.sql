-- ============================================================================
-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- Unified SQL LSP - E2E Test Fixtures
-- PostgreSQL Schema Definition
-- PostgreSQL Schema Definition
-- ============================================================================
-- ============================================================================
-- This file defines the complete database schema for end-to-end testing.
-- This file defines the complete database schema for end-to-end testing.
-- It covers:
-- It covers:
--   - Basic tables (users, orders, products, order_items)
--   - Basic tables (users, orders, products, order_items)
--   - Advanced scenarios (self-referencing, many-to-many, partitioning)
--   - Advanced scenarios (self-referencing, many-to-many, partitioning)
--   - PostgreSQL-specific types (ARRAY, JSONB, custom ENUMs)
--   - PostgreSQL-specific types (ARRAY, JSONB, custom ENUMs)
--   - Indexes and constraints for testing completion and diagnostics
--   - Indexes and constraints for testing completion and diagnostics
-- ============================================================================
-- ============================================================================


-- Drop existing tables if they exist (in correct dependency order)
DROP TABLE IF EXISTS order_items CASCADE;
DROP TABLE IF EXISTS orders CASCADE;
DROP TABLE IF EXISTS products CASCADE;
DROP TABLE IF EXISTS users CASCADE;
DROP TABLE IF EXISTS post_tags CASCADE;
DROP TABLE IF EXISTS tags CASCADE;
DROP TABLE IF EXISTS posts CASCADE;
DROP TABLE IF EXISTS employees CASCADE;
DROP TABLE IF EXISTS logs CASCADE;

-- Drop enums
DROP TYPE IF EXISTS product_category CASCADE;
DROP TYPE IF EXISTS order_status CASCADE;
DROP TYPE IF EXISTS payment_method CASCADE;
DROP TYPE IF EXISTS user_status CASCADE;
DROP TYPE IF EXISTS post_status CASCADE;
DROP TYPE IF EXISTS log_level CASCADE;

-- ============================================================================
-- ENUM DEFINITIONS (PostgreSQL-specific)
-- ============================================================================

CREATE TYPE product_category AS ENUM ('electronics', 'clothing', 'books', 'home', 'sports', 'toys');
CREATE TYPE order_status AS ENUM ('pending', 'processing', 'shipped', 'delivered', 'cancelled', 'refunded');
CREATE TYPE payment_method AS ENUM ('credit_card', 'debit_card', 'paypal', 'bank_transfer', 'cash');
CREATE TYPE user_status AS ENUM ('active', 'inactive', 'suspended');
CREATE TYPE post_status AS ENUM ('draft', 'published', 'archived');
CREATE TYPE log_level AS ENUM ('debug', 'info', 'warning', 'error', 'critical');

-- ============================================================================
-- BASIC TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Users table
-- Tests: basic CRUD, various data types, NOT NULL, DEFAULT, UNIQUE
-- PostgreSQL-specific: ARRAY type, custom ENUMs
-- ----------------------------------------------------------------------------
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(50) NOT NULL UNIQUE,
    email VARCHAR(100) NOT NULL UNIQUE,
    full_name VARCHAR(100),
    age INTEGER,
    balance NUMERIC(10, 2) DEFAULT 0.00,
    is_active BOOLEAN DEFAULT TRUE,
    status user_status DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    last_login TIMESTAMP WITH TIME ZONE,
    bio TEXT,
    profile_image VARCHAR(255),
    phone VARCHAR(20),
    tags TEXT[],  -- PostgreSQL ARRAY type
    preferences JSONB  -- PostgreSQL JSONB type
);

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ----------------------------------------------------------------------------
-- Products table
-- Tests: NUMERIC precision, PostgreSQL ENUM, ARRAY types, CHECK constraints
-- ----------------------------------------------------------------------------
CREATE TABLE products (
    id SERIAL PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    price NUMERIC(10, 2) NOT NULL,
    cost NUMERIC(10, 2),
    quantity_in_stock INTEGER DEFAULT 0,
    category product_category,
    is_available BOOLEAN DEFAULT TRUE,
    weight NUMERIC(8, 3),
    sku VARCHAR(50) UNIQUE,
    tags TEXT[],  -- PostgreSQL ARRAY type
    attributes JSONB,  -- PostgreSQL JSONB type
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    -- Constraints
    CONSTRAINT chk_price_positive CHECK (price >= 0),
    CONSTRAINT chk_quantity_non_negative CHECK (quantity_in_stock >= 0),
    CONSTRAINT chk_cost_not_greater_than_price CHECK (cost IS NULL OR cost <= price)
);

CREATE TRIGGER update_products_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ----------------------------------------------------------------------------
-- Orders table
-- Tests: foreign keys, PostgreSQL ENUMs, date ranges
-- ----------------------------------------------------------------------------
CREATE TABLE orders (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL,
    order_date TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    total_amount NUMERIC(12, 2) NOT NULL,
    status order_status NOT NULL DEFAULT 'pending',
    payment_method payment_method,
    shipping_address TEXT,
    billing_address TEXT,
    notes TEXT,
    metadata JSONB,  -- PostgreSQL JSONB for flexible metadata
    shipped_at TIMESTAMP WITH TIME ZONE,
    delivered_at TIMESTAMP WITH TIME ZONE,
    -- Foreign key to users
    CONSTRAINT fk_orders_user_id
        FOREIGN KEY (user_id) REFERENCES users(id)
        ON DELETE RESTRICT
        ON UPDATE CASCADE,
    -- Constraints
    CONSTRAINT chk_total_amount_positive CHECK (total_amount >= 0),
    CONSTRAINT chk_delivered_after_shipped
        CHECK (delivered_at IS NULL OR shipped_at IS NOT NULL)
);

-- ----------------------------------------------------------------------------
-- Order Items table
-- Tests: many-to-one relationship, computed columns (via generated columns in PG 12+)
-- ----------------------------------------------------------------------------
CREATE TABLE order_items (
    id SERIAL PRIMARY KEY,
    order_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price NUMERIC(10, 2) NOT NULL,
    discount_percent NUMERIC(5, 2) DEFAULT 0.00,
    subtotal NUMERIC(10, 2) GENERATED ALWAYS AS (quantity * unit_price * (1 - discount_percent / 100.0)) STORED,
    notes TEXT,
    metadata JSONB,
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
    -- Unique constraint to prevent duplicate products in same order
    UNIQUE (order_id, product_id)
);

-- ============================================================================
-- ADVANCED SCENARIO TABLES
-- ============================================================================

-- ----------------------------------------------------------------------------
-- Employees table (self-referencing)
-- Tests: hierarchical data, recursive relationships
-- ----------------------------------------------------------------------------
CREATE TABLE employees (
    id SERIAL PRIMARY KEY,
    first_name VARCHAR(50) NOT NULL,
    last_name VARCHAR(50) NOT NULL,
    email VARCHAR(100) NOT NULL UNIQUE,
    manager_id INTEGER,
    department VARCHAR(50),
    position VARCHAR(100),
    salary NUMERIC(12, 2),
    hire_date DATE NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    skills TEXT[],  -- PostgreSQL ARRAY for skills
    metadata JSONB,
    -- Self-referencing foreign key for organizational hierarchy
    CONSTRAINT fk_employees_manager_id
        FOREIGN KEY (manager_id) REFERENCES employees(id)
        ON DELETE SET NULL
        ON UPDATE CASCADE,
    -- Constraints
    CONSTRAINT chk_salary_positive CHECK (salary IS NULL OR salary > 0)
);

-- ----------------------------------------------------------------------------
-- Posts table
-- Tests: content management, ARRAY tags, JSONB metadata
-- ----------------------------------------------------------------------------
CREATE TABLE posts (
    id SERIAL PRIMARY KEY,
    title VARCHAR(200) NOT NULL,
    slug VARCHAR(200) UNIQUE,
    content TEXT NOT NULL,
    excerpt TEXT,
    author_id INTEGER NOT NULL,
    status post_status DEFAULT 'draft',
    tags TEXT[],  -- PostgreSQL ARRAY for tags
    view_count INTEGER DEFAULT 0,
    metadata JSONB,  -- PostgreSQL JSONB for flexible metadata
    published_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    -- Foreign key to users (author)
    CONSTRAINT fk_posts_author_id
        FOREIGN KEY (author_id) REFERENCES users(id)
        ON DELETE CASCADE
        ON UPDATE CASCADE
);

CREATE TRIGGER update_posts_updated_at
    BEFORE UPDATE ON posts
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- ----------------------------------------------------------------------------
-- Tags table
-- Tests: simple reference table for many-to-many relationships
-- ----------------------------------------------------------------------------
CREATE TABLE tags (
    id SERIAL PRIMARY KEY,
    name VARCHAR(50) NOT NULL UNIQUE,
    slug VARCHAR(50) UNIQUE,
    description TEXT,
    color VARCHAR(7),  -- Hex color code
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- ----------------------------------------------------------------------------
-- Post_Tags table (many-to-many)
-- Tests: junction tables, composite foreign keys
-- ----------------------------------------------------------------------------
CREATE TABLE post_tags (
    post_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    tagged_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    metadata JSONB,
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
        ON UPDATE CASCADE
);

-- ----------------------------------------------------------------------------
-- Logs table (partitioned)
-- Tests: time-series data, native partitioning (PostgreSQL 10+)
-- ----------------------------------------------------------------------------
CREATE TABLE logs (
    id BIGSERIAL PRIMARY KEY,
    level log_level NOT NULL,
    message TEXT NOT NULL,
    context JSONB,
    source VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP
) PARTITION BY RANGE (created_at);

-- Create partitions for different years
CREATE TABLE logs_2023 PARTITION OF logs
    FOR VALUES FROM ('2023-01-01') TO ('2024-01-01');

CREATE TABLE logs_2024 PARTITION OF logs
    FOR VALUES FROM ('2024-01-01') TO ('2025-01-01');

CREATE TABLE logs_2025 PARTITION OF logs
    FOR VALUES FROM ('2025-01-01') TO ('2026-01-01');

-- ============================================================================
-- VIEWS FOR TESTING COMPLETION
-- ============================================================================

-- View for active users
CREATE VIEW v_active_users AS
SELECT
    id,
    username,
    email,
    full_name,
    created_at
FROM users
WHERE is_active = TRUE;

-- View for order summaries
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

-- Simple function for testing procedural code completion
CREATE OR REPLACE FUNCTION get_user_orders(p_user_id INTEGER)
RETURNS TABLE (
    id INTEGER,
    order_date TIMESTAMP WITH TIME ZONE,
    total_amount NUMERIC,
    status order_status,
    item_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        o.id,
        o.order_date,
        o.total_amount,
        o.status,
        COUNT(oi.id)
    FROM orders o
    LEFT JOIN order_items oi ON o.id = oi.order_id
    WHERE o.user_id = p_user_id
    GROUP BY o.id, o.order_date, o.total_amount, o.status
    ORDER BY o.order_date DESC;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- POSTGRESQL-SPECIFIC FEATURES FOR TESTING
-- ============================================================================

-- Materialized view (refreshable)
CREATE MATERIALIZED VIEW mv_product_stats AS
SELECT
    category,
    COUNT(*) AS product_count,
    AVG(price) AS avg_price,
    SUM(quantity_in_stock) AS total_stock
FROM products
GROUP BY category;

-- Unique index with partial condition (PostgreSQL-specific)
CREATE UNIQUE INDEX uniq_users_username_active
    ON users(username)
    WHERE is_active = TRUE;

-- Trigger function example
CREATE OR REPLACE FUNCTION log_order_creation()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO logs (level, message, context, source)
    VALUES ('info', 'Order created', jsonb_build_object('order_id', NEW.id, 'user_id', NEW.user_id), 'order-service');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_log_order_creation
    AFTER INSERT ON orders
    FOR EACH ROW
    EXECUTE FUNCTION log_order_creation();

-- ============================================================================
-- END OF SCHEMA DEFINITION
-- ============================================================================

-- ============================================================================
-- INDEXES
-- ============================================================================
CREATE INDEX idx_username ON users(username);
CREATE INDEX idx_email ON users(email);
CREATE INDEX idx_created_at ON users(created_at);
CREATE INDEX idx_is_active ON users(is_active);
CREATE INDEX idx_status ON users(status);
CREATE UNIQUE INDEX uniq_users_username_active ON users(username) WHERE is_active = TRUE;

CREATE INDEX idx_name ON products(name);
CREATE INDEX idx_category ON products(category);
CREATE INDEX idx_price ON products(price);
CREATE INDEX idx_is_available ON products(is_active);
CREATE INDEX idx_sku ON products(sku);
CREATE INDEX idx_attributes_gin ON products USING GIN (attributes);

CREATE INDEX idx_user_id ON orders(user_id);
CREATE INDEX idx_order_date ON orders(order_date);
CREATE INDEX idx_status ON orders(status);
CREATE INDEX idx_total_amount ON orders(total_amount);
CREATE INDEX idx_user_status ON orders(user_id, status);
CREATE INDEX idx_date_status ON orders(order_date, status);

CREATE INDEX idx_order_id ON order_items(order_id);
CREATE INDEX idx_product_id ON order_items(product_id);

CREATE INDEX idx_manager_id ON employees(manager_id);
CREATE INDEX idx_department ON employees(department);
CREATE INDEX idx_email ON employees(email);
CREATE INDEX idx_name_emp ON employees(last_name, first_name);

CREATE INDEX idx_slug ON posts(slug);
CREATE INDEX idx_author_id ON posts(author_id);
CREATE INDEX idx_status_posts ON posts(status);
CREATE INDEX idx_published_at ON posts(published_at);
CREATE INDEX idx_view_count ON posts(view_count);
CREATE INDEX idx_tags_gin ON posts USING GIN (tags);

CREATE INDEX idx_name_tags ON tags(name);
CREATE INDEX idx_slug_tags ON tags(slug);
CREATE INDEX idx_tag_id ON post_tags(tag_id);

CREATE INDEX idx_level ON logs(level);
CREATE INDEX idx_created_at_logs ON logs(created_at);
CREATE INDEX idx_source ON logs(source);

CREATE VIEW v_active_users AS
SELECT
    id,
    username,
    email,
    full_name,
    created_at
FROM users
WHERE is_active = TRUE;

-- View for order summaries
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

-- Simple function for testing procedural code completion
CREATE OR REPLACE FUNCTION get_user_orders(p_user_id INTEGER)
RETURNS TABLE (
    id INTEGER,
    order_date TIMESTAMP WITH TIME ZONE,
    total_amount NUMERIC,
    status order_status,
    item_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        o.id,
        o.order_date,
        o.total_amount,
        o.status,
        COUNT(oi.id)
    FROM orders o
    LEFT JOIN order_items oi ON o.id = oi.order_id
    WHERE o.user_id = p_user_id
    GROUP BY o.id, o.order_date, o.total_amount, o.status
    ORDER BY o.order_date DESC;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- POSTGRESQL-SPECIFIC FEATURES FOR TESTING
-- ============================================================================

-- Materialized view (refreshable)
CREATE MATERIALIZED VIEW mv_product_stats AS
SELECT
    category,
    COUNT(*) AS product_count,
    AVG(price) AS avg_price,
    SUM(quantity_in_stock) AS total_stock
FROM products
GROUP BY category;

-- Unique index with partial condition (PostgreSQL-specific)
CREATE UNIQUE INDEX uniq_users_username_active
    ON users(username)
    WHERE is_active = TRUE;

-- Trigger function example
CREATE OR REPLACE FUNCTION log_order_creation()
RETURNS TRIGGER AS $$
BEGIN
    INSERT INTO logs (level, message, context, source)
    VALUES ('info', 'Order created', jsonb_build_object('order_id', NEW.id, 'user_id', NEW.user_id), 'order-service');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_log_order_creation
    AFTER INSERT ON orders
    FOR EACH ROW
    EXECUTE FUNCTION log_order_creation();

-- ============================================================================
-- END OF SCHEMA DEFINITION
-- ============================================================================
