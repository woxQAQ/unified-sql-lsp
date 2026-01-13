-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- PostgreSQL Edge Case Data Insertion
-- ============================================================================
-- This file inserts edge case data for testing boundary conditions.
-- PostgreSQL-specific: ARRAY operations, JSONB functions, NULL handling
-- ============================================================================

-- ============================================================================
-- EDGE CASE USERS
-- Tests: NULL values, empty arrays, special JSONB structures
-- ============================================================================

INSERT INTO users (id, username, email, full_name, age, balance, is_active, status, created_at, last_login, bio, tags, preferences) VALUES
-- NULL values and empty arrays
(101, 'null_balance', 'null.balance@example.com', 'Null Balance User', 30, NULL, TRUE, 'active', '2024-01-21 10:00:00+00', NULL, NULL, ARRAY[]::TEXT[], NULL),
(102, 'null_name', 'null.name@example.com', NULL, 25, 100.00, TRUE, 'active', '2024-01-21 10:01:00+00', '2024-01-21 10:05:00+00', 'User with NULL full name', ARRAY['standard'], '{}'),
(103, 'empty_tags', 'empty.tags@example.com', 'Empty Tags User', 28, 75.00, TRUE, 'active', '2024-01-21 10:02:00+00', NULL, NULL, ARRAY[]::TEXT[], '{"theme": "dark"}'),
(104, 'null_preferences', 'null.prefs@example.com', 'Null Preferences', 33, 200.00, TRUE, 'active', '2024-01-21 10:03:00+00', NULL, NULL, ARRAY['verified'], NULL),

-- Unicode and special characters
(105, 'user_unicode', 'unicode@example.com', 'François Müller 日本語', 29, 220.00, TRUE, 'active', '2024-01-21 10:04:00+00', NULL, 'Unicode user: café, naïve', ARRAY['international'], '{"language": "fr", "regions": ["EU", "ASIA"]}'),

-- Complex JSONB structures
(106, 'complex_json', 'complex.json@example.com', 'Complex JSON User', 35, 500.00, TRUE, 'active', '2024-01-21 10:05:00+00', NULL, NULL, ARRAY['developer'], '{"settings": {"editor": {"theme": "dark", "fontSize": 14, "tabSize": 4}, "notifications": {"email": true, "push": false, "frequency": "daily"}}, "history": {"last_login": "2024-01-21", "login_count": 42}}'),

-- Nested arrays (PostgreSQL-specific)
(107, 'array_test', 'array.test@example.com', 'Array Test User', 31, 150.00, TRUE, 'active', '2024-01-21 10:06:00+00', NULL, NULL, ARRAY['tag1', 'tag2', 'tag1', 'tag3'], '{"tags_array": ["nested", ["deeply", "nested"]]}'),

-- Boundary values
(108, 'min_age', 'min.age@example.com', 'Minimum Age', 0, 50.00, TRUE, 'active', '2024-01-21 10:07:00+00', NULL, NULL, ARRAY['new'], '{}'),
(109, 'max_age', 'max.age@example.com', 'Maximum Age', 150, 0.00, TRUE, 'active', '2024-01-21 10:08:00+00', NULL, NULL, ARRAY[]::TEXT[], NULL),

-- Special statuses
(110, 'suspended_user', 'suspended@example.com', 'Suspended User', 40, 1000.00, FALSE, 'suspended', '2024-01-21 10:09:00+00', NULL, 'Violated terms of service', ARRAY['banned'], '{"suspension_reason": "policy_violation", "suspension_date": "2024-01-21"}');

-- ============================================================================
-- EDGE CASE PRODUCTS
-- Tests: NULL categories, empty arrays, JSONB operators
-- ============================================================================

INSERT INTO products (id, name, description, price, cost, quantity_in_stock, category, is_available, weight, sku, tags, attributes) VALUES
-- NULL categories and attributes
(101, 'no_category', 'Product without category', 29.99, NULL, 50, NULL, TRUE, 0.200, 'EDGE-NO-CAT', ARRAY[]::TEXT[], NULL),
(102, 'no_attributes', 'No attributes product', 19.99, 10.00, 75, 'electronics', TRUE, 0.150, 'EDGE-NO-ATTR', ARRAY['basic'], NULL),

-- Empty and NULL arrays
(103, 'empty_tags', 'Empty tags product', 15.99, 8.00, 100, 'home', TRUE, 0.100, 'EDGE-EMPTY-TAGS', ARRAY[]::TEXT[], '{"color": "white"}'),
(104, 'complex_attributes', 'Complex JSONB attributes', 99.99, 50.00, 30, 'electronics', TRUE, 0.500, 'EDGE-COMPLEX', ARRAY['premium'], '{"specs": {"dimensions": {"width": 10, "height": 20, "depth": 15}, "features": ["wireless", "bluetooth", "waterproof"], "compatibility": ["iOS", "Android", "Windows"]}}'),

-- Boundary values
(105, 'zero_price', 'Free item', 0.00, 0.00, 200, 'toys', TRUE, 0.050, 'EDGE-ZERO', ARRAY['free'], '{"free_product": true}'),
(106, 'zero_stock', 'Out of stock', 49.99, 25.00, 0, 'clothing', FALSE, 0.600, 'EDGE-OOS', ARRAY[]::TEXT[], '{"restock_date": "2024-02-01"}');

-- ============================================================================
-- EDGE CASE ORDERS
-- Tests: NULL addresses, JSONB metadata edge cases
-- ============================================================================

INSERT INTO orders (id, user_id, order_date, total_amount, status, payment_method, shipping_address, billing_address, metadata, shipped_at, delivered_at) VALUES
-- NULL addresses (digital products)
(101, 101, '2024-01-21 10:00:00+00', 29.99, 'delivered', 'paypal', NULL, NULL, '{"digital_product": true, "download_link": "https://example.com/download/123"}', '2024-01-21 10:05:00+00', '2024-01-21 10:05:00+00'),

-- Empty JSONB
(102, 102, '2024-01-21 10:01:00+00', 49.99, 'pending', 'credit_card', 'Address 1', 'Address 1', '{}', NULL, NULL),

-- Complex JSONB metadata
(103, 106, '2024-01-21 10:02:00+00', 199.99, 'processing', 'credit_card', 'Complex Address', 'Complex Address', '{"coupons": ["SAVE10", "FREESHIP"], "gift": {"wrap": true, "message": "Happy Birthday!", "sender": "Anonymous"}, "shipping": {"method": "express", "instructions": "Leave at door", "signature_required": false}, "custom_fields": {"field1": "value1", "field2": null, "field3": [1, 2, 3]}}', NULL, NULL),

-- Zero amount
(104, 108, '2024-01-21 10:03:00+00', 0.00, 'pending', 'cash', 'Zero Amount Address', 'Zero Amount Address', '{"promo_order": true}', NULL, NULL);

-- ============================================================================
-- EDGE CASE LOGS
-- Tests: JSONB context, NULL values, special characters
-- ============================================================================

INSERT INTO logs (id, level, message, context, source, created_at) VALUES
-- NULL and empty JSONB
(101, 'info', 'Simple log', NULL, 'test', '2024-01-21 10:00:00+00'),
(102, 'debug', 'Empty context', '{}', 'test', '2024-01-21 10:01:00+00'),

-- Complex nested JSONB
(103, 'info', 'Complex nested context', '{"user": {"id": 1, "profile": {"name": "Test", "prefs": {"theme": "dark", "lang": "en"}}}, "meta": {"v": "1.0", "ts": "2024-01-21T10:00:00Z"}, "data": [1, 2, 3, {"nested": "value"}]}', 'api', '2024-01-21 10:02:00+00'),

-- JSONB with special characters
(104, 'warning', 'Special chars in JSONB', '{"msg": "Test with \"quotes\" and ''apostrophes''", "path": "/path/to/file", "unicode": "café, 日本語"}', 'test', '2024-01-21 10:03:00+00'),

-- JSONB arrays and null values
(105, 'info', 'JSONB with nulls', '{"items": [1, null, 3, {"key": null}], "empty_array": [], "nested_null": {"a": {"b": null}}}', 'test', '2024-01-21 10:04:00+00');

-- ============================================================================
-- POSTGRESQL-SPECIFIC EDGE CASES
-- ============================================================================

-- Test table for array operations
CREATE TABLE IF NOT EXISTS array_tests (
    id SERIAL PRIMARY KEY,
    int_array INTEGER[],
    text_array TEXT[],
    mixed_array TEXT[],
    empty_array INTEGER[],
    null_array INTEGER[]
);

INSERT INTO array_tests (int_array, text_array, mixed_array, empty_array, null_array) VALUES
(ARRAY[1, 2, 3, 4, 5], ARRAY['a', 'b', 'c'], ARRAY[1, 'two', 3, 'four'], ARRAY[]::INTEGER[], NULL),
(ARRAY[10, 20, 30], ARRAY['x', 'y', 'z'], ARRAY['mixed', 42, NULL], ARRAY[]::INTEGER[], ARRAY[]::INTEGER[]),
(ARRAY[1, 1, 2, 2, 3], ARRAY['dup', 'dup', 'unique'], ARRAY[NULL, 'null', ''], ARRAY[]::INTEGER[], NULL);

-- Test table for JSONB operations
CREATE TABLE IF NOT EXISTS jsonb_tests (
    id SERIAL PRIMARY KEY,
    data JSONB,
    data_array JSONB[],
    metadata JSONB
);

INSERT INTO jsonb_tests (data, data_array, metadata) VALUES
('{"name": "test", "value": 42}', ARRAY['{"a": 1}', '{"b": 2}']::JSONB[], '{"created": "2024-01-21"}'),
('{"nested": {"deep": {"value": true}}}', ARRAY[]::JSONB[], '{}'),
('{"array": [1, 2, 3], "null": null, "bool": true}', ARRAY['{"x": null}']::JSONB[], NULL),
('{"empty": {}, "empty_array": []}', ARRAY[]::JSONB[], '{"keys": ["a", "b", "c"]}');

-- Test table for NULL vs empty string comparison
CREATE TABLE IF NOT EXISTS null_comparison_tests (
    id SERIAL PRIMARY KEY,
    text_value TEXT,
    int_value INTEGER,
    json_value JSONB,
    array_value INTEGER[]
);

INSERT INTO null_comparison_tests (text_value, int_value, json_value, array_value) VALUES
(NULL, NULL, NULL, NULL),
('', 0, '{}', ARRAY[]::INTEGER[]),
('   ', 0, '{"key": "value"}', ARRAY[1, 2, 3]),
('NULL', 1, 'null', ARRAY[NULL]),
('0', 0, '0', ARRAY[0]);

-- ============================================================================
-- END OF EDGE CASE DATA INSERTION
-- ============================================================================
