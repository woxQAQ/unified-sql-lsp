-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- MySQL Edge Case Data Insertion
-- ============================================================================
-- This file inserts edge case data for testing boundary conditions and special
-- scenarios. It covers:
--   - NULL values in various contexts
--   - Empty strings and whitespace
--   - Special characters and Unicode
--   - Boundary values (min/max)
--   - Duplicate-like scenarios
--   - Date/time edge cases
-- ============================================================================

-- ============================================================================
-- EDGE CASE USERS
-- Tests: NULL values, empty strings, special characters, boundary ages
-- ============================================================================

INSERT INTO users (id, username, email, full_name, age, balance, is_active, created_at, last_login, bio, profile_image, phone) VALUES
-- NULL values in different columns
(101, 'null_balance', 'null.balance@example.com', 'Null Balance User', 30, NULL, TRUE, '2024-01-21 10:00:00', NULL, NULL, NULL, NULL),
(102, 'null_name', 'null.name@example.com', NULL, 25, 100.00, TRUE, '2024-01-21 10:01:00', '2024-01-21 10:05:00', 'User with NULL full name', NULL, NULL),
(103, 'null_age', 'null.age@example.com', 'Unknown Age User', NULL, 50.00, TRUE, '2024-01-21 10:02:00', NULL, NULL, NULL, NULL),

-- Empty strings (not NULL)
(104, 'empty_bio', 'empty.bio@example.com', 'Empty Bio User', 28, 75.00, TRUE, '2024-01-21 10:03:00', NULL, '', NULL, NULL),
(105, 'empty_phone', 'empty.phone@example.com', 'Empty Phone User', 33, 200.00, TRUE, '2024-01-21 10:04:00', NULL, NULL, NULL, ''),

-- Whitespace tests
(106, 'whitespace_user', 'whitespace@example.com', '   Whitespace User   ', 35, 150.00, TRUE, '2024-01-21 10:05:00', NULL, '   Bio with spaces   ', NULL, NULL),

-- Special characters and Unicode
(107, 'user_quotes', 'quotes.test@example.com', 'O''Brien "Quotes" Test', 40, 180.00, TRUE, '2024-01-21 10:06:00', NULL, 'User with single '' and double " quotes', NULL, NULL),
(108, 'user_unicode', 'unicode@example.com', 'François Müller 日本語', 29, 220.00, TRUE, '2024-01-21 10:07:00', NULL, 'Unicode user: café, naïve, 北京', NULL, NULL),
(109, 'user_emoji', 'emoji@example.com', 'Emoji User 😀 🎉', 26, 95.00, TRUE, '2024-01-21 10:08:00', NULL, 'Bio with emoji: 💼 🚀 ✨', NULL, NULL),

-- Boundary ages
(110, 'min_age_user', 'min.age@example.com', 'Minimum Age User', 0, 500.00, TRUE, '2024-01-21 10:09:00', NULL, NULL, NULL, NULL),
(111, 'max_age_user', 'max.age@example.com', 'Maximum Age User', 150, 0.00, TRUE, '2024-01-21 10:10:00', NULL, NULL, NULL, NULL),

-- Boundary balances
(112, 'negative_balance_pending', 'negative@example.com', 'Negative Balance', 45, -50.00, TRUE, '2024-01-21 10:11:00', NULL, 'User with negative balance (overdraft)', NULL, NULL),
(113, 'zero_balance', 'zero.balance@example.com', 'Zero Balance User', 31, 0.00, TRUE, '2024-01-21 10:12:00', NULL, NULL, NULL, NULL),
(114, 'high_balance', 'high.balance@example.com', 'High Balance User', 50, 999999.99, TRUE, '2024-01-21 10:13:00', NULL, 'VIP user with maximum balance', NULL, NULL),

-- Very long strings (test VARCHAR limits)
(115, 'long_name', 'long.name@example.com', 'This is a very long full name that tests the varchar limit but still within the 100 character maximum allowed', 27, 120.00, TRUE, '2024-01-21 10:14:00', NULL, NULL, NULL, NULL),

-- Inactive edge cases
(116, 'never_logged_in', 'never.login@example.com', 'Never Logged In', 38, 300.00, FALSE, '2024-01-21 10:15:00', NULL, 'Account created but never accessed', NULL, NULL),
(117, 'inactive_no_balance', 'inactive.zero@example.com', 'Inactive No Balance', 22, 0.00, FALSE, '2024-01-21 10:16:00', NULL, NULL, NULL, NULL);

-- ============================================================================
-- EDGE CASE PRODUCTS
-- Tests: NULL costs, zero quantities, boundary prices, special characters
-- ============================================================================

INSERT INTO products (id, name, description, price, cost, quantity_in_stock, category, is_available, weight, sku) VALUES
-- NULL and zero costs
(101, 'no_cost_info', 'Product without cost information', 29.99, NULL, 50, 'electronics', TRUE, 0.200, 'EDGE-NO-COST-001'),
(102, 'zero_cost', 'Free sample product', 0.00, 0.00, 100, 'home', TRUE, 0.050, 'EDGE-ZERO-COST-001'),

-- Zero and negative quantities
(103, 'out_of_stock', 'Permanently out of stock', 99.99, 50.00, 0, 'clothing', FALSE, 0.500, 'EDGE-OOS-001'),

-- Boundary prices
(104, 'min_price', 'Minimum price item', 0.01, 0.00, 200, 'toys', TRUE, 0.010, 'EDGE-MIN-PRICE-001'),
(105, 'max_price_decimal', 'Maximum decimal precision', 99999999.99, 80000000.00, 5, 'electronics', TRUE, 50.000, 'EDGE-MAX-DEC-001'),

-- Very light and heavy items
(106, 'ultra_light', 'Ultra light item', 15.99, 8.00, 75, 'sports', TRUE, 0.001, 'EDGE-LIGHT-001'),
(107, 'very_heavy', 'Very heavy item', 299.99, 150.00, 10, 'sports', TRUE, 999.999, 'EDGE-HEAVY-001'),

-- NULL descriptions and special names
(108, 'no_description', NULL, 49.99, 25.00, 30, 'books', TRUE, 0.700, 'EDGE-NO-DESC-001'),
(109, 'special ''chars''', 'Product with "special" characters', 19.99, 10.00, 40, 'home', TRUE, 0.300, 'EDGE-QUOTE-001'),
(110, 'product_with_emoji_🎮', 'Gaming accessory with emoji', 39.99, 20.00, 25, 'electronics', TRUE, 0.250, 'EDGE-EMOJI-001'),

-- NULL category (if allowed)
(111, 'uncategorized', NULL, 9.99, 5.00, 60, NULL, TRUE, 0.100, 'EDGE-NO-CAT-001');

-- ============================================================================
-- EDGE CASE ORDERS
-- Tests: boundary amounts, NULL dates, special statuses
-- ============================================================================

INSERT INTO orders (id, user_id, order_date, total_amount, status, payment_method, shipping_address, billing_address, shipped_at, delivered_at) VALUES
-- Zero and minimal amounts
(101, 101, '2024-01-21 10:00:00', 0.00, 'pending', 'cash', 'Test Address 1', 'Test Address 1', NULL, NULL),
(102, 102, '2024-01-21 10:01:00', 0.01, 'pending', 'credit_card', 'Test Address 2', 'Test Address 2', NULL, NULL),

-- Very high amounts
(103, 114, '2024-01-21 10:02:00', 999999.99, 'pending', 'bank_transfer', 'VIP Address 1', 'VIP Address 1', NULL, NULL),

-- NULL addresses (digital products)
(104, 101, '2024-01-21 10:03:00', 29.99, 'delivered', 'paypal', NULL, NULL, '2024-01-21 10:30:00', '2024-01-21 10:35:00'),

-- NULL payment method (cash on delivery)
(105, 103, '2024-01-21 10:04:00', 149.99, 'processing', NULL, 'Cash Delivery Address', 'Cash Delivery Address', NULL, NULL),

-- Same timestamp (test order of events)
(106, 101, '2024-01-21 10:05:00', 49.99, 'pending', 'credit_card', 'Same Time Addr 1', 'Same Time Addr 1', NULL, NULL),
(107, 102, '2024-01-21 10:05:00', 59.99, 'pending', 'credit_card', 'Same Time Addr 2', 'Same Time Addr 2', NULL, NULL),

-- Special characters in addresses
(108, 108, '2024-01-21 10:06:00', 89.99, 'pending', 'credit_card', '123 Main St, Apt "4B"', '123 Main St, Apt "4B"', NULL, NULL),
(109, 108, '2024-01-21 10:07:00', 99.99, 'pending', 'debit_card', 'O''Connor Street, Building 5', 'O''Connor Street, Building 5', NULL, NULL),

-- Very long notes (stored in separate column)
(110, 101, '2024-01-21 10:08:00', 199.99, 'processing', 'credit_card', 'Long Note Address', 'Long Note Address', NULL, NULL);

-- Add notes separately
UPDATE orders SET notes = 'This is a very long note that contains extensive customer instructions. The customer has requested special handling for this order, including gift wrapping, a personalized message, and specific delivery time preferences. Additional details: please call upon arrival, leave at door if no answer, do not ring doorbell after 9pm.' WHERE id = 110;

-- Edge case: shipped but not delivered (long gap)
(111, 102, '2024-01-01 10:00:00', 79.99, 'shipped', 'credit_card', 'Lost in Transit Address', 'Lost in Transit Address', '2024-01-02 10:00:00', NULL),

-- Edge case: delivered before shipped (data quality issue - should be caught by constraint)
-- Note: This would fail due to CHECK constraint, so commenting out
-- INSERT INTO orders (user_id, order_date, total_amount, status, shipped_at, delivered_at)
-- VALUES (101, '2024-01-21 10:00:00', 99.99, 'delivered', '2024-01-22 10:00:00', '2024-01-21 11:00:00');

-- ============================================================================
-- EDGE CASE ORDER ITEMS
-- Tests: zero quantities (should fail), maximum discounts
-- ============================================================================

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, discount_percent, notes) VALUES
-- Note: quantity must be > 0 due to CHECK constraint

-- Maximum discounts
(101, 101, 101, 10, 29.99, 0.00, NULL),
(102, 102, 102, 1, 0.01, 100.00, '100% discount - free item'),
(103, 103, 105, 5, 999999.99, 50.00, '50% discount on expensive item'),

-- Boundary prices
(104, 104, 104, 1, 0.01, 0.00, 'Minimum price item'),
(105, 105, 105, 2, 99999999.99, 0.00, 'Maximum price item'),

-- Special characters in notes
(106, 106, 109, 1, 19.99, 0.00, 'Special request: "Handle with care"'),
(107, 107, 109, 1, 19.99, 0.00, 'O''Connor''s special order'),

-- Large quantities
(108, 108, 16, 1000, 24.99, 25.00, 'Bulk order - 1000 units');

-- ============================================================================
-- EDGE CASE EMPLOYEES
-- Tests: NULL managers, boundary salaries, self-referencing edge cases
-- ============================================================================

INSERT INTO employees (id, first_name, last_name, email, manager_id, department, position, salary, hire_date, is_active) VALUES
-- NULL salaries
(101, 'Volunteer', 'One', 'volunteer.one@company.com', 9, 'Sales', 'Sales Intern', NULL, '2024-01-21', TRUE),

-- Boundary salaries
(102, 'Minimum', 'Wage', 'minimum.wage@company.com', 10, 'Sales', 'Associate', 0.01, '2024-01-21', TRUE),
(103, 'Executive', 'Maximum', 'executive.max@company.com', NULL, 'Executive', 'Chairman', 999999.99, '2024-01-21', TRUE),

-- Very long names
(104, 'Verylongfirstname thatexceeds', 'Verylonglastname thatgoeson', 'verylong.name@company.com', 5, 'Engineering', 'Developer', 95000.00, '2024-01-21', TRUE),

-- Special characters in names
(105, 'O''Brien', 'Mc Donald', 'obrien.mcdonald@company.com', 5, 'Engineering', 'Developer', 100000.00, '2024-01-21', TRUE),

-- Inactive employees
(106, 'Former', 'Employee', 'former.employee@company.com', NULL, 'Engineering', 'Ex-Developer', 85000.00, '2020-01-15', FALSE),
(107, 'Retired', 'Founder', 'retired.founder@company.com', NULL, 'Executive', 'Founder', 1.00, '2010-01-01', FALSE),

-- Self-referencing edge case (employee manages themselves - data quality issue)
-- Note: This would cause a cycle, so not including
-- INSERT INTO employees (first_name, last_name, email, manager_id, department, position, salary, hire_date, is_active)
-- VALUES ('Self', 'Manager', 'self.manager@company.com', <would_be_self_id>, 'Management', 'Self-Manager', 150000.00, '2024-01-21', TRUE);

-- ============================================================================
-- EDGE CASE POSTS
-- Tests: NULL excerpts, very long content, Unicode, boundary view counts
-- ============================================================================

INSERT INTO posts (id, title, slug, content, excerpt, author_id, status, view_count, published_at) VALUES
-- NULL excerpts
(101, 'Post Without Excerpt', 'post-without-excerpt', 'This is a full article content without an excerpt...', NULL, 1, 'published', 0, '2024-01-21 10:00:00'),

-- Very long content
(102, 'Very Long Article', 'very-long-article', REPEAT('This is a long article. ', 500), 'Very long article excerpt', 1, 'published', 100, '2024-01-21 10:01:00'),

-- Unicode and special characters in title
(103, 'Tutorial: café & naïve approaches', 'unicode-tutorial', 'Content with Unicode characters...', 'Unicode post', 3, 'published', 50, '2024-01-21 10:02:00'),
(104, 'Post with "quotes" and ''apostrophes''', 'quotes-post', 'Testing special characters...', NULL, 5, 'published', 25, '2024-01-21 10:03:00'),

-- Boundary view counts
(105, 'Most Popular Post', 'most-popular', 'This is the most viewed post...', 'Popular content', 1, 'published', 9999999, '2024-01-15 10:00:00'),
(106, 'Unpopular Post', 'unpopular-post', 'This post has no views yet...', NULL, 3, 'published', 0, '2024-01-21 10:04:00'),

-- NULL published_at (drafts)
(107, 'Future Post', 'future-post', 'Content scheduled for future...', 'Coming soon', 5, 'draft', 0, NULL),

-- Duplicate slugs (should fail due to UNIQUE constraint - commenting out)
-- INSERT INTO posts (title, slug, content, author_id, status)
-- VALUES ('Another Post', 'most-popular', 'Duplicate slug test', 1, 'published');

-- ============================================================================
-- EDGE CASE TAGS
-- Tests: special characters, NULL descriptions, duplicate-like names
-- ============================================================================

INSERT INTO tags (id, name, slug, description, color) VALUES
-- NULL descriptions
(101, 'Mystery', 'mystery', NULL, '#ff00ff'),
(102, 'No Desc', 'no-desc', NULL, NULL),

-- Special characters in names
(103, 'C++', 'cpp', 'C++ programming articles', '#00599C'),
(104, 'C#', 'csharp', 'C# programming articles', '#68217A'),
(105, 'Node.js', 'nodejs', 'Node.js articles', '#68A063'),

-- Unicode names
(106, 'Français', 'francais', 'French language content', '#0055A4'),
(107, 'Español', 'espanol', 'Spanish language content', '#AA151B'),

-- Very long names
(108, 'This Is A Very Long Tag Name That Tests Limits', 'very-long-tag', 'Testing tag name length', '#999999'),

-- Edge case colors
(109, 'No Color', 'no-color', 'Tag without color', NULL),
(110, 'Invalid Color', 'invalid-color', 'Tag with non-hex color', 'not-a-color');

-- ============================================================================
-- EDGE CASE POST_TAGS
-- Tests: duplicate relationships (should fail), timestamp edge cases
-- ============================================================================

-- Note: Duplicate (post_id, tag_id) combinations should fail due to PRIMARY KEY
-- INSERT INTO post_tags (post_id, tag_id) VALUES (101, 1);
-- INSERT INTO post_tags (post_id, tag_id) VALUES (101, 1); -- Would fail

-- Add some edge case tag relationships
INSERT INTO post_tags (post_id, tag_id) VALUES
(101, 101),  -- Mystery tag
(102, 8),    -- Advanced
(103, 108),  -- Français
(104, 103),  -- C++
(105, 9);    -- Best Practices

-- ============================================================================
-- EDGE CASE LOGS
-- Tests: very long messages, NULL contexts, boundary timestamps
-- ============================================================================

INSERT INTO logs (id, level, message, context, source, created_at) VALUES
-- NULL context
(101, 'info', 'Simple log without context', NULL, 'test-source', '2024-01-21 10:00:00'),

-- Very long messages
(102, 'error', REPEAT('Error message repeated. ', 100), '{"error_code": "LONG_ERR"}', 'error-generator', '2024-01-21 10:01:00'),

-- Empty JSON context
(103, 'debug', 'Log with empty context', '{}', 'debug-source', '2024-01-21 10:02:00'),

-- Complex JSON context
(104, 'info', 'Complex nested JSON context', '{"user": {"id": 1, "profile": {"name": "Test", "preferences": {"theme": "dark", "notifications": true}}}, "metadata": {"version": "1.0", "timestamp": "2024-01-21T10:00:00Z"}}', 'api-gateway', '2024-01-21 10:03:00'),

-- Special characters in message
(105, 'warning', 'Warning with "quotes" and ''apostrophes''', '{}', 'test-source', '2024-01-21 10:04:00'),
(106, 'info', 'Unicode message: café, naïve, 北京', '{}', 'test-source', '2024-01-21 10:05:00'),

-- Boundary timestamps
(107, 'info', 'Earliest timestamp', '{}', 'time-test', '1970-01-01 00:00:01'),
(108, 'info', 'Future timestamp', '{}', 'time-test', '2099-12-31 23:59:59'),

-- NULL source
(109, 'debug', 'Log without source', '{}', NULL, '2024-01-21 10:06:00'),

-- Very long source name
(110, 'info', 'Long source name', '{}', 'this-is-a-very-long-source-name-that-tests-the-limit', '2024-01-21 10:07:00');

-- ============================================================================
-- EDGE CASE: NULL AND EMPTY STRING COMPARISON TESTS
-- ============================================================================

-- Create a separate test table for NULL vs empty string comparisons
CREATE TABLE IF NOT EXISTS null_tests (
    id INT PRIMARY KEY AUTO_INCREMENT,
    test_value VARCHAR(100),
    description VARCHAR(255)
);

INSERT INTO null_tests (test_value, description) VALUES
(NULL, 'This is NULL'),
('', 'This is an empty string'),
('   ', 'This is whitespace'),
('NULL', 'This is the string "NULL"'),
('null', 'This is the string "null"'),
('0', 'This is the string "0"'),
(0, 'This is the integer 0 (converted to string)');

-- ============================================================================
-- EDGE CASE: AUTO_INCREMENT RESET TEST
-- ============================================================================

-- Note: Auto-increment values continue from basic data
-- This tests that the LSP correctly handles gaps in IDs

-- ============================================================================
-- EDGE CASE: FOREIGN KEY CONSTRAINT TESTS
-- ============================================================================

-- These should fail due to foreign key constraints (commented out)
-- INSERT INTO orders (user_id, order_date, total_amount, status)
-- VALUES (99999, '2024-01-21 10:00:00', 99.99, 'pending');  -- Non-existent user

-- INSERT INTO order_items (order_id, product_id, quantity, unit_price)
-- VALUES (99999, 1, 1, 29.99);  -- Non-existent order

-- ============================================================================
-- END OF EDGE CASE DATA INSERTION
-- ============================================================================
