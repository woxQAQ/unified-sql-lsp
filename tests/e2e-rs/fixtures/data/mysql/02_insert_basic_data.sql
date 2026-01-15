-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- MySQL Basic Data Insertion
-- ============================================================================
-- This file inserts realistic test data for basic testing scenarios.
-- Data volume: 10-50 rows per table for easy verification
-- Focus: Realistic values, proper relationships, diverse scenarios
-- ============================================================================

-- ============================================================================
-- USERS TABLE (20 records)
-- Tests: various data types, realistic user profiles, different statuses
-- ============================================================================

INSERT INTO users (id, username, email, full_name, age, balance, is_active, created_at, last_login, bio, profile_image, phone) VALUES
-- Active users with complete profiles
(1, 'john_doe', 'john.doe@example.com', 'John Doe', 32, 1250.50, TRUE, '2024-01-15 10:30:00', '2024-01-20 08:45:00', 'Software developer and tech enthusiast.', 'https://example.com/images/john.jpg', '+1-555-0101'),
(2, 'jane_smith', 'jane.smith@example.com', 'Jane Smith', 28, 890.00, TRUE, '2024-01-16 14:20:00', '2024-01-21 09:15:00', 'Digital marketer and content creator.', 'https://example.com/images/jane.jpg', '+1-555-0102'),
(3, 'bob_wilson', 'bob.wilson@example.com', 'Bob Wilson', 45, 3200.75, TRUE, '2024-01-17 08:00:00', '2024-01-19 16:30:00', 'Business consultant and entrepreneur.', NULL, '+1-555-0103'),
(4, 'alice_brown', 'alice.brown@example.com', 'Alice Brown', 35, 450.25, TRUE, '2024-01-18 11:45:00', '2024-01-20 14:20:00', 'Designer and creative director.', 'https://example.com/images/alice.jpg', NULL),
(5, 'charlie_davis', 'charlie.davis@example.com', 'Charlie Davis', 29, 2100.00, TRUE, '2024-01-19 09:30:00', '2024-01-21 10:00:00', 'Full-stack developer and open source contributor.', 'https://example.com/images/charlie.jpg', '+1-555-0104'),

-- Active users with minimal profiles
(6, 'diana_miller', 'diana.miller@example.com', 'Diana Miller', 31, 675.50, TRUE, '2024-01-10 07:15:00', '2024-01-18 12:00:00', NULL, NULL, NULL),
(7, 'frank_garcia', 'frank.garcia@example.com', 'Frank Garcia', 42, 1580.00, TRUE, '2024-01-11 13:40:00', '2024-01-20 15:30:00', 'Sales manager.', NULL, '+1-555-0106'),
(8, 'grace_lee', 'grace.lee@example.com', 'Grace Lee', 26, 320.00, TRUE, '2024-01-12 10:20:00', '2024-01-19 09:45:00', NULL, 'https://example.com/images/grace.jpg', NULL),
(9, 'henry_martinez', 'henry.martinez@example.com', 'Henry Martinez', 38, 920.25, TRUE, '2024-01-13 15:55:00', '2024-01-21 11:20:00', 'Project manager with 10+ years experience.', NULL, '+1-555-0108'),
(10, 'iris_anderson', 'iris.anderson@example.com', 'Iris Anderson', 33, 780.00, TRUE, '2024-01-14 08:30:00', '2024-01-20 16:45:00', NULL, NULL, NULL),

-- Inactive users
(11, 'jack_thomas', 'jack.thomas@example.com', 'Jack Thomas', 50, 120.00, FALSE, '2024-01-05 12:00:00', '2024-01-10 09:30:00', 'Retired teacher.', NULL, '+1-555-0110'),
(12, 'kate_white', 'kate.white@example.com', 'Kate White', 27, 0.00, FALSE, '2024-01-06 14:20:00', '2024-01-12 11:00:00', NULL, NULL, NULL),
(13, 'leo_harris', 'leo.harris@example.com', 'Leo Harris', 44, 85.50, FALSE, '2024-01-07 09:10:00', '2024-01-11 14:45:00', 'Freelance writer.', 'https://example.com/images/leo.jpg', '+1-555-0112'),

-- Users with edge case balances
(14, 'mia_clark', 'mia.clark@example.com', 'Mia Clark', 30, 0.00, TRUE, '2024-01-08 11:30:00', '2024-01-20 10:15:00', 'New user, first purchase pending.', NULL, NULL),
(15, 'noah_lewis', 'noah.lewis@example.com', 'Noah Lewis', 36, 5000.00, TRUE, '2024-01-09 13:45:00', '2024-01-21 08:30:00', 'Premium customer with VIP status.', 'https://example.com/images/noah.jpg', '+1-555-0114'),

-- Recent users
(16, 'olivia_walker', 'olivia.walker@example.com', 'Olivia Walker', 24, 150.00, TRUE, '2024-01-20 10:00:00', '2024-01-20 10:05:00', 'Student.', NULL, NULL),
(17, 'paul_hall', 'paul.hall@example.com', 'Paul Hall', 41, 980.75, TRUE, '2024-01-20 11:30:00', '2024-01-20 12:00:00', NULL, NULL, '+1-555-0116'),
(18, 'quinta_allen', 'quinta.allen@example.com', 'Quinta Allen', 29, 420.00, TRUE, '2024-01-20 14:15:00', '2024-01-20 14:20:00', 'Travel blogger.', 'https://example.com/images/quinta.jpg', NULL),
(19, 'ryan_young', 'ryan.young@example.com', 'Ryan Young', 37, 2150.00, TRUE, '2024-01-21 09:00:00', '2024-01-21 09:30:00', 'Software architect.', NULL, '+1-555-0118'),
(20, 'sophia_king', 'sophia.king@example.com', 'Sophia King', 23, 75.00, TRUE, '2024-01-21 15:45:00', '2024-01-21 16:00:00', 'Recent graduate.', 'https://example.com/images/sophia.jpg', NULL);

-- ============================================================================
-- PRODUCTS TABLE (25 records)
-- Tests: various categories, price ranges, stock levels, ENUM values
-- ============================================================================

INSERT INTO products (id, name, description, price, cost, quantity_in_stock, category, is_available, weight, sku) VALUES
-- Electronics
(1, 'Wireless Bluetooth Headphones', 'Premium noise-cancelling headphones with 30hr battery life', 149.99, 75.00, 45, 'electronics', TRUE, 0.350, 'ELEC-BT-HP-001'),
(2, 'USB-C Charging Cable', 'Fast charging cable, 6ft braided nylon', 12.99, 3.50, 200, 'electronics', TRUE, 0.080, 'ELEC-USB-C-001'),
(3, 'Portable Power Bank 20000mAh', 'High-capacity power bank with Quick Charge 3.0', 39.99, 18.00, 85, 'electronics', TRUE, 0.450, 'ELEC-PB-20K-001'),
(4, 'Smart Watch Series 5', 'Fitness tracking, heart rate monitor, GPS', 299.99, 150.00, 30, 'electronics', TRUE, 0.150, 'ELEC-SW-S5-001'),
(5, 'Mechanical Keyboard RGB', 'Gaming keyboard with Cherry MX switches', 129.99, 65.00, 60, 'electronics', TRUE, 1.200, 'ELEC-MK-RGB-001'),
(6, 'Wireless Mouse Ergonomic', 'Vertical ergonomic mouse with adjustable DPI', 49.99, 22.00, 0, 'electronics', FALSE, 0.180, 'ELEC-WM-ERG-001'),

-- Clothing
(7, 'Classic Cotton T-Shirt', '100% cotton, pre-shrunk, multiple colors', 19.99, 5.00, 150, 'clothing', TRUE, 0.200, 'CLTH-TSH-001'),
(8, 'Slim Fit Jeans', 'Stretch denim, classic blue wash', 59.99, 25.00, 80, 'clothing', TRUE, 0.600, 'CLTH-JN-SL-001'),
(9, 'Running Shoes Performance', 'Breathable mesh, cushioned sole, size 8-13', 89.99, 40.00, 45, 'clothing', TRUE, 0.850, 'CLTH-SH-RUN-001'),
(10, 'Winter Jacket Waterproof', 'Insulated jacket with removable hood', 159.99, 70.00, 25, 'clothing', TRUE, 1.100, 'CLTH-JK-WIN-001'),
(11, 'Wool Sweater', 'Merino wool blend, crew neck', 79.99, 35.00, 55, 'clothing', TRUE, 0.450, 'CLTH-SW-WL-001'),

-- Books
(12, 'The Art of Programming', 'Comprehensive guide to software development', 49.99, 20.00, 100, 'books', TRUE, 0.900, 'BOOK-PROG-001'),
(13, 'Introduction to Machine Learning', 'Textbook covering ML fundamentals', 89.99, 45.00, 60, 'books', TRUE, 1.500, 'BOOK-ML-001'),
(14, 'Mystery Novel Collection', 'Box set of 5 bestselling mystery novels', 34.99, 15.00, 40, 'books', TRUE, 1.200, 'BOOK-MYS-001'),
(15, 'Cooking Mastery Guide', '100+ recipes from professional chefs', 29.99, 12.00, 75, 'books', TRUE, 0.800, 'BOOK-CK-001'),

-- Home & Garden
(16, 'Stainless Steel Water Bottle', 'Insulated bottle, keeps drinks cold 24hrs', 24.99, 8.00, 180, 'home', TRUE, 0.350, 'HOME-WB-SS-001'),
(17, 'LED Desk Lamp Adjustable', 'Touch control, 5 brightness levels, USB port', 34.99, 15.00, 90, 'home', TRUE, 0.800, 'HOME-DL-LED-001'),
(18, 'Throw Pillow Set (4-pack)', 'Soft decorative pillows, removable covers', 39.99, 16.00, 120, 'home', TRUE, 1.500, 'HOME-TP-4P-001'),
(19, 'Plant Pot Ceramic', 'Modern minimalist design, drainage hole', 19.99, 7.00, 65, 'home', TRUE, 0.700, 'HOME-PP-CER-001'),
(20, 'Coffee Maker Programmable', '12-cup capacity, auto shut-off', 69.99, 30.00, 35, 'home', TRUE, 2.800, 'HOME-CM-12-001'),

-- Sports
(21, 'Yoga Mat Premium', 'Non-slip, extra thick, eco-friendly material', 29.99, 12.00, 95, 'sports', TRUE, 1.000, 'SPR-YM-PRE-001'),
(22, 'Resistance Bands Set', '5-piece set with different tension levels', 24.99, 8.00, 0, 'sports', FALSE, 0.400, 'SPR-RB-5P-001'),
(23, 'Dumbbell Set Adjustable', '5-50lbs per dumbbell, storage rack included', 199.99, 90.00, 20, 'sports', TRUE, 25.000, 'SPR-DB-ADJ-001'),

-- Toys
(24, 'Building Blocks Classic', '500-piece set, compatible with major brands', 34.99, 14.00, 110, 'toys', TRUE, 1.200, 'TOY-BB-500-001'),
(25, 'Board Game Strategy', '2-4 players, ages 12+, average playtime 60min', 44.99, 18.00, 50, 'toys', TRUE, 1.800, 'TOY-BG-STR-001');

-- ============================================================================
-- ORDERS TABLE (30 records)
-- Tests: various statuses, dates, amounts, foreign key relationships
-- ============================================================================

INSERT INTO orders (id, user_id, order_date, total_amount, status, payment_method, shipping_address, billing_address, shipped_at, delivered_at) VALUES
-- Delivered orders
(1, 1, '2024-01-15 11:00:00', 189.98, 'delivered', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', '2024-01-16 10:00:00', '2024-01-18 15:30:00'),
(2, 2, '2024-01-16 15:30:00', 49.99, 'delivered', 'paypal', '456 Oak Ave, Los Angeles, CA 90001', '456 Oak Ave, Los Angeles, CA 90001', '2024-01-17 09:00:00', '2024-01-19 12:00:00'),
(3, 3, '2024-01-17 10:15:00', 449.98, 'delivered', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', '2024-01-18 14:00:00', '2024-01-21 10:30:00'),
(4, 1, '2024-01-18 13:45:00', 79.97, 'delivered', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', '2024-01-19 11:00:00', '2024-01-22 14:15:00'),
(5, 4, '2024-01-19 09:20:00', 149.99, 'delivered', 'debit_card', '321 Elm St, Houston, TX 77001', '321 Elm St, Houston, TX 77001', '2024-01-20 08:30:00', '2024-01-23 16:45:00'),

-- Shipped orders
(6, 5, '2024-01-19 14:30:00', 279.97, 'shipped', 'credit_card', '654 Maple Dr, Phoenix, AZ 85001', '654 Maple Dr, Phoenix, AZ 85001', '2024-01-20 10:00:00', NULL),
(7, 6, '2024-01-20 10:00:00', 89.98, 'shipped', 'paypal', '987 Cedar Ln, Philadelphia, PA 19101', '987 Cedar Ln, Philadelphia, PA 19101', '2024-01-21 09:30:00', NULL),
(8, 7, '2024-01-20 16:45:00', 199.99, 'shipped', 'credit_card', '147 Birch Blvd, San Antonio, TX 78201', '147 Birch Blvd, San Antonio, TX 78201', '2024-01-21 13:00:00', NULL),

-- Processing orders
(9, 8, '2024-01-21 08:30:00', 64.98, 'processing', 'debit_card', '258 Walnut Way, San Diego, CA 92101', '258 Walnut Way, San Diego, CA 92101', NULL, NULL),
(10, 9, '2024-01-21 11:15:00', 329.98, 'processing', 'credit_card', '369 Spruce St, Dallas, TX 75201', '369 Spruce St, Dallas, TX 75201', NULL, NULL),
(11, 10, '2024-01-21 13:00:00', 109.97, 'processing', 'paypal', '741 Ash Ave, San Jose, CA 95101', '741 Ash Ave, San Jose, CA 95101', NULL, NULL),
(12, 2, '2024-01-21 14:30:00', 39.99, 'processing', 'credit_card', '456 Oak Ave, Los Angeles, CA 90001', '456 Oak Ave, Los Angeles, CA 90001', NULL, NULL),

-- Pending orders
(13, 3, '2024-01-21 09:00:00', 149.99, 'pending', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', NULL, NULL),
(14, 5, '2024-01-21 10:30:00', 84.98, 'pending', 'credit_card', '654 Maple Dr, Phoenix, AZ 85001', '654 Maple Dr, Phoenix, AZ 85001', NULL, NULL),
(15, 1, '2024-01-21 11:45:00', 299.99, 'pending', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', NULL, NULL),
(16, 14, '2024-01-21 12:00:00', 19.99, 'pending', 'debit_card', '963 Willow Ct, Austin, TX 78701', '963 Willow Ct, Austin, TX 78701', NULL, NULL),
(17, 15, '2024-01-21 15:00:00', 859.96, 'pending', 'credit_card', '852 Aspen Pl, Jacksonville, FL 32201', '852 Aspen Pl, Jacksonville, FL 32201', NULL, NULL),
(18, 16, '2024-01-21 16:15:00', 34.99, 'pending', 'paypal', '741 Oak Ln, Fort Worth, TX 76101', '741 Oak Ln, Fort Worth, TX 76101', NULL, NULL),

-- Cancelled orders
(19, 11, '2024-01-10 10:00:00', 199.99, 'cancelled', 'credit_card', '159 Pine St, Columbus, OH 43201', '159 Pine St, Columbus, OH 43201', NULL, NULL),
(20, 12, '2024-01-12 14:30:00', 49.99, 'cancelled', 'debit_card', '357 Maple Dr, Charlotte, NC 28201', '357 Maple Dr, Charlotte, NC 28201', NULL, NULL),
(21, 11, '2024-01-15 09:15:00', 89.98, 'cancelled', 'paypal', '159 Pine St, Columbus, OH 43201', '159 Pine St, Columbus, OH 43201', NULL, NULL),

-- Refunded orders
(22, 13, '2024-01-08 11:00:00', 129.99, 'refunded', 'credit_card', '951 Cedar Ave, San Francisco, CA 94101', '951 Cedar Ave, San Francisco, CA 94101', '2024-01-09 10:00:00', '2024-01-12 14:00:00'),
(23, 3, '2024-01-14 13:30:00', 349.99, 'refunded', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', '2024-01-15 11:00:00', '2024-01-18 09:30:00'),

-- High-value orders
(24, 15, '2024-01-12 10:00:00', 1249.95, 'delivered', 'credit_card', '852 Aspen Pl, Jacksonville, FL 32201', '852 Aspen Pl, Jacksonville, FL 32201', '2024-01-13 09:00:00', '2024-01-17 15:00:00'),
(25, 3, '2024-01-16 08:30:00', 899.97, 'delivered', 'credit_card', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', '2024-01-17 10:30:00', '2024-01-20 12:00:00'),

-- Recent orders (same day)
(26, 17, '2024-01-21 10:00:00', 174.97, 'pending', 'credit_card', '456 Elm St, Denver, CO 80201', '456 Elm St, Denver, CO 80201', NULL, NULL),
(27, 18, '2024-01-21 11:30:00', 54.98, 'pending', 'paypal', '789 Oak Ave, Boston, MA 02101', '789 Oak Ave, Boston, MA 02101', NULL, NULL),
(28, 19, '2024-01-21 13:45:00', 399.99, 'processing', 'credit_card', '234 Pine Rd, Seattle, WA 98101', '234 Pine Rd, Seattle, WA 98101', NULL, NULL),
(29, 20, '2024-01-21 14:15:00', 24.99, 'pending', 'debit_card', '567 Maple Dr, Nashville, TN 37201', '567 Maple Dr, Nashville, TN 37201', NULL, NULL),
(30, 1, '2024-01-21 16:00:00', 149.98, 'pending', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', NULL, NULL);

-- ============================================================================
-- ORDER_ITEMS TABLE (50 records)
-- Tests: foreign keys, various quantities, discounts, computed columns
-- ============================================================================

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, discount_percent, notes) VALUES
-- Order 1: Delivered, multiple items
(1, 1, 1, 1, 149.99, 0.00, 'Gift wrapping requested'),
(2, 1, 7, 2, 19.99, 0.00, NULL),

-- Order 2: Single item
(3, 2, 12, 1, 49.99, 0.00, NULL),

-- Order 3: High-value order with multiple items
(4, 3, 4, 1, 299.99, 0.00, NULL),
(5, 3, 5, 1, 129.99, 10.00, 'Minor scratch on box'),
(6, 3, 20, 1, 69.99, 0.00, NULL),

-- Order 4: Multiple quantities with discount
(7, 4, 16, 4, 24.99, 5.00, 'Bulk order'),
(8, 4, 17, 1, 29.99, 0.00, NULL),

-- Order 5: Single expensive item
(9, 5, 1, 1, 149.99, 0.00, NULL),

-- Order 6: Shipped, electronics bundle
(10, 6, 2, 3, 12.99, 0.00, NULL),
(11, 6, 3, 2, 39.99, 0.00, NULL),
(12, 6, 6, 1, 49.99, 15.00, 'Open box discount'),

-- Order 7: Clothing items
(13, 7, 7, 3, 19.99, 0.00, NULL),
(14, 7, 8, 1, 59.99, 0.00, NULL),

-- Order 8: Books and home goods
(15, 8, 13, 1, 89.99, 20.00, 'Textbook sale'),
(16, 8, 18, 1, 39.99, 0.00, NULL),
(17, 8, 19, 2, 19.99, 0.00, NULL),

-- Order 9: Sports equipment
(18, 9, 21, 1, 29.99, 0.00, NULL),
(19, 9, 22, 1, 24.99, 0.00, NULL),
(20, 9, 21, 1, 29.99, 0.00, 'Gift - separate shipping'),

-- Order 10: High-value electronics
(21, 10, 1, 2, 149.99, 5.00, 'Bundle discount'),
(22, 10, 4, 1, 299.99, 0.00, NULL),

-- Order 11: Mixed categories
(23, 11, 1, 1, 149.99, 0.00, NULL),
(24, 11, 12, 1, 49.99, 0.00, NULL),
(25, 11, 16, 2, 24.99, 10.00, NULL),

-- Order 12: Single book
(26, 12, 15, 1, 39.99, 0.00, NULL),

-- Order 13: Pending, high-value
(27, 13, 1, 1, 149.99, 0.00, NULL),

-- Order 14: Multiple small items
(28, 14, 2, 2, 12.99, 0.00, NULL),
(29, 14, 16, 1, 24.99, 0.00, NULL),
(30, 14, 21, 1, 29.99, 0.00, NULL),

-- Order 15: Expensive electronics
(31, 15, 4, 1, 299.99, 0.00, NULL),

-- Order 16: Single clothing item
(32, 16, 7, 1, 19.99, 0.00, NULL),

-- Order 17: Large bulk order (VIP customer)
(33, 17, 1, 5, 149.99, 15.00, 'VIP bulk discount'),
(34, 17, 4, 3, 299.99, 10.00, NULL),
(35, 17, 5, 2, 129.99, 0.00, NULL),

-- Order 18: Toy and book
(36, 18, 25, 1, 34.99, 0.00, NULL),

-- Order 19: Cancelled - multiple items
(37, 19, 3, 2, 39.99, 0.00, 'Payment issue'),
(38, 19, 9, 1, 89.99, 0.00, NULL),

-- Order 20: Cancelled - single item
(39, 20, 10, 1, 159.99, 0.00, 'Customer request'),

-- Order 21: Cancelled - out of stock
(40, 21, 6, 1, 49.99, 0.00, 'Item unavailable'),

-- Order 22: Refunded
(41, 22, 5, 1, 129.99, 0.00, 'Returned - defective'),

-- Order 23: Refunded - multiple items
(42, 23, 1, 2, 149.99, 0.00, 'Customer changed mind'),
(43, 23, 20, 1, 69.99, 0.00, NULL),

-- Order 24: High-value delivered
(44, 24, 23, 1, 199.99, 0.00, NULL),
(45, 24, 1, 1, 149.99, 0.00, NULL),
(46, 24, 4, 3, 299.99, 0.00, NULL),

-- Order 25: Bulk books order
(47, 25, 12, 10, 49.99, 25.00, 'Educational bulk discount'),
(48, 25, 13, 5, 89.99, 20.00, NULL),

-- Recent orders
(49, 26, 1, 1, 149.99, 0.00, NULL),
(50, 26, 21, 1, 29.99, 0.00, NULL);

-- ============================================================================
-- EMPLOYEES TABLE (12 records)
-- Tests: self-referencing hierarchy, organizational structure
-- ============================================================================

INSERT INTO employees (id, first_name, last_name, email, manager_id, department, position, salary, hire_date, is_active) VALUES
-- Top management (no manager)
(1, 'James', 'Wilson', 'james.wilson@company.com', NULL, 'Executive', 'CEO', 250000.00, '2015-01-15', TRUE),
(2, 'Sarah', 'Johnson', 'sarah.johnson@company.com', NULL, 'Executive', 'CTO', 220000.00, '2015-03-01', TRUE),
(3, 'Michael', 'Chen', 'michael.chen@company.com', NULL, 'Executive', 'CFO', 210000.00, '2015-02-20', TRUE),

-- Engineering department (under CTO)
(4, 'Emily', 'Rodriguez', 'emily.rodriguez@company.com', 2, 'Engineering', 'VP of Engineering', 180000.00, '2016-05-10', TRUE),
(5, 'David', 'Kim', 'david.kim@company.com', 4, 'Engineering', 'Engineering Manager', 140000.00, '2017-03-15', TRUE),
(6, 'Lisa', 'Patel', 'lisa.patel@company.com', 5, 'Engineering', 'Senior Developer', 120000.00, '2018-07-20', TRUE),
(7, 'Robert', 'Martinez', 'robert.martinez@company.com', 5, 'Engineering', 'Developer', 95000.00, '2019-01-10', TRUE),
(8, 'Amanda', 'Taylor', 'amanda.taylor@company.com', 5, 'Engineering', 'Junior Developer', 75000.00, '2020-06-15', TRUE),

-- Sales department (under CEO)
(9, 'Christopher', 'Lee', 'christopher.lee@company.com', 1, 'Sales', 'VP of Sales', 160000.00, '2016-08-01', TRUE),
(10, 'Jessica', 'Brown', 'jessica.brown@company.com', 9, 'Sales', 'Sales Manager', 110000.00, '2017-11-20', TRUE),
(11, 'Daniel', 'Garcia', 'daniel.garcia@company.com', 10, 'Sales', 'Sales Representative', 65000.00, '2019-04-05', TRUE),

-- Finance department (under CFO)
(12, 'Michelle', 'Davis', 'michelle.davis@company.com', 3, 'Finance', 'Financial Analyst', 85000.00, '2018-09-15', TRUE);

-- ============================================================================
-- POSTS TABLE (15 records)
-- Tests: content management, publication workflow
-- ============================================================================

INSERT INTO posts (id, title, slug, content, excerpt, author_id, status, view_count, published_at) VALUES
-- Published posts
(1, 'Getting Started with SQL', 'getting-started-with-sql', 'SQL is a powerful language for managing relational databases. In this comprehensive guide, we will cover the basics...', 'Learn SQL fundamentals from scratch', 1, 'published', 1520, '2024-01-16 10:00:00'),
(2, 'Advanced Query Optimization Techniques', 'advanced-query-optimization', 'Optimizing SQL queries is crucial for performance. We will explore indexing strategies, query execution plans...', 'Master query performance tuning', 1, 'published', 890, '2024-01-17 14:30:00'),
(3, 'Understanding Database Normalization', 'understanding-normalization', 'Database normalization helps eliminate data redundancy and improve data integrity. Learn about 1NF, 2NF, 3NF...', 'A deep dive into normalization', 3, 'published', 645, '2024-01-18 09:15:00'),
(4, 'Top 10 SQL Best Practices', 'sql-best-practices', 'Following best practices ensures your SQL code is maintainable and efficient. Here are our top recommendations...', 'Write better SQL code', 5, 'published', 2100, '2024-01-19 11:45:00'),
(5, 'Introduction to Window Functions', 'introduction-window-functions', 'Window functions are a powerful feature for advanced data analysis. In this tutorial...', 'Unlock advanced analytics', 1, 'published', 1234, '2024-01-20 08:30:00'),
(6, 'Database Security Fundamentals', 'database-security-fundamentals', 'Securing your database is critical. We will cover authentication, authorization, encryption...', 'Protect your data', 3, 'published', 567, '2024-01-20 15:00:00'),

-- Draft posts
(7, 'SQL vs NoSQL: When to Use Which', 'sql-vs-nosql', 'This comparison guide will help you choose the right database technology for your project...', 'Choose the right database', 5, 'draft', 0, NULL),
(8, 'MySQL Performance Tuning Guide', 'mysql-performance-tuning', 'Detailed guide on optimizing MySQL databases for maximum performance...', 'Optimize your MySQL server', 1, 'draft', 0, NULL),
(9, 'PostgreSQL Features Overview', 'postgresql-features-overview', 'Explore the powerful features that make PostgreSQL a top choice for enterprise applications...', 'Discover PostgreSQL capabilities', 3, 'draft', 0, NULL),
(10, 'Building Scalable Database Architectures', 'scalable-database-architectures', 'Learn how to design database architectures that can handle growth...', 'Scale your databases effectively', 5, 'draft', 0, NULL),

-- Archived posts
(11, 'Legacy Database Migration Strategies', 'legacy-database-migration', 'Strategies for migrating from legacy database systems to modern platforms...', 'Updated content available in newer posts', 1, 'archived', 3450, '2023-06-15 10:00:00'),
(12, 'SQL Injection Prevention', 'sql-injection-prevention', 'Protect your applications from SQL injection attacks with these proven techniques...', 'Security fundamentals', 3, 'archived', 2890, '2023-08-20 14:00:00'),

-- Recently published
(13, 'Common Table Expressions Explained', 'ctes-explained', 'Common Table Expressions (CTEs) make complex queries more readable and maintainable...', 'Simplify complex queries', 5, 'published', 98, '2024-01-21 10:00:00'),
(14, 'Database Indexing Strategies', 'database-indexing-strategies', 'Learn different types of indexes and when to use them for optimal query performance...', 'Master database indexing', 1, 'published', 45, '2024-01-21 12:30:00'),
(15, 'Understanding Transaction Isolation Levels', 'transaction-isolation-levels', 'Transaction isolation levels control how transactions interact. This guide explains...', 'Ensure data consistency', 3, 'published', 23, '2024-01-21 14:00:00');

-- ============================================================================
-- TAGS TABLE (10 records)
-- Tests: categorization, metadata
-- ============================================================================

INSERT INTO tags (id, name, slug, description, color) VALUES
(1, 'SQL', 'sql', 'SQL and relational database topics', '#3b82f6'),
(2, 'Tutorial', 'tutorial', 'Step-by-step guides and tutorials', '#10b981'),
(3, 'Performance', 'performance', 'Query optimization and performance tuning', '#f59e0b'),
(4, 'Security', 'security', 'Database security best practices', '#ef4444'),
(5, 'MySQL', 'mysql', 'MySQL-specific articles and tips', '#00758f'),
(6, 'PostgreSQL', 'postgresql', 'PostgreSQL features and guides', '#336791'),
(7, 'Beginner', 'beginner', 'Content for SQL beginners', '#8b5cf6'),
(8, 'Advanced', 'advanced', 'Advanced database concepts', '#ec4899'),
(9, 'Best Practices', 'best-practices', 'Industry best practices and standards', '#6366f1'),
(10, 'Architecture', 'architecture', 'Database architecture and design', '#14b8a6');

-- ============================================================================
-- POST_TAGS TABLE (25 records)
-- Tests: many-to-many relationships
-- ============================================================================

INSERT INTO post_tags (post_id, tag_id) VALUES
-- Post 1: Getting Started with SQL
(1, 1), (1, 2), (1, 7), (1, 5),

-- Post 2: Advanced Query Optimization
(2, 1), (2, 3), (2, 8), (2, 9),

-- Post 3: Understanding Database Normalization
(3, 1), (3, 2), (3, 7), (3, 9),

-- Post 4: Top 10 SQL Best Practices
(4, 1), (4, 9), (4, 8),

-- Post 5: Introduction to Window Functions
(5, 1), (5, 8), (5, 2),

-- Post 6: Database Security Fundamentals
(6, 1), (6, 4), (6, 9),

-- Post 13: Common Table Expressions Explained
(13, 1), (13, 2), (13, 8),

-- Post 14: Database Indexing Strategies
(14, 1), (14, 3), (14, 8), (14, 10),

-- Post 15: Understanding Transaction Isolation Levels
(15, 1), (15, 8), (15, 4),

-- Add some tags to draft posts
(7, 1), (7, 10),
(8, 3), (8, 5),
(9, 6), (9, 8),
(10, 10), (10, 9);

-- ============================================================================
-- LOGS TABLE (30 records)
-- Tests: time-series data, various log levels, JSON context
-- ============================================================================

INSERT INTO logs (id, level, message, context, source, created_at) VALUES
-- Error logs
(1, 'error', 'Database connection timeout', '{"error_code": "ETIMEDOUT", "retry_count": 3}', 'db-connection', '2024-01-15 10:30:00'),
(2, 'error', 'Failed to execute query', '{"query": "SELECT * FROM users", "error": "Table not found"}', 'query-executor', '2024-01-15 11:45:00'),
(3, 'error', 'Authentication failed for user', '{"user_id": 999, "ip": "192.168.1.100"}', 'auth-service', '2024-01-16 08:20:00'),

-- Warning logs
(4, 'warning', 'High memory usage detected', '{"usage_percent": 85, "threshold": 80}', 'system-monitor', '2024-01-15 12:00:00'),
(5, 'warning', 'Slow query detected', '{"query_duration_ms": 2500, "threshold_ms": 1000}', 'query-monitor', '2024-01-15 14:30:00'),
(6, 'warning', 'Disk space running low', '{"available_gb": 5, "threshold_gb": 10}', 'storage-monitor', '2024-01-16 09:15:00'),
(7, 'warning', 'Rate limit approaching', '{"requests_per_minute": 95, "limit": 100}', 'api-gateway', '2024-01-16 10:45:00'),

-- Info logs
(8, 'info', 'User logged in successfully', '{"user_id": 1, "username": "john_doe"}', 'auth-service', '2024-01-15 10:00:00'),
(9, 'info', 'New order created', '{"order_id": 1, "user_id": 1, "total": 189.98}', 'order-service', '2024-01-15 11:00:00'),
(10, 'info', 'Order status updated', '{"order_id": 1, "old_status": "pending", "new_status": "processing"}', 'order-service', '2024-01-15 11:05:00'),
(11, 'info', 'Payment processed successfully', '{"order_id": 1, "amount": 189.98, "method": "credit_card"}', 'payment-service', '2024-01-15 11:10:00'),
(12, 'info', 'Order shipped', '{"order_id": 1, "carrier": "UPS", "tracking_number": "1Z999AA10123456784"}', 'shipping-service', '2024-01-16 10:00:00'),
(13, 'info', 'Order delivered', '{"order_id": 1, "delivered_at": "2024-01-18 15:30:00"}', 'shipping-service', '2024-01-18 15:30:00'),
(14, 'info', 'User registration completed', '{"user_id": 20, "username": "sophia_king"}', 'auth-service', '2024-01-21 15:45:00'),
(15, 'info', 'Password reset requested', '{"email": "john.doe@example.com"}', 'auth-service', '2024-01-20 08:30:00'),
(16, 'info', 'Database backup completed', '{"backup_size_mb": 256, "duration_seconds": 45}', 'backup-service', '2024-01-21 02:00:00'),

-- Debug logs
(17, 'debug', 'Query execution plan generated', '{"query": "SELECT * FROM orders", "plan_type": "nested_loop"}', 'query-optimizer', '2024-01-15 11:00:00'),
(18, 'debug', 'Cache hit', '{"key": "user_1", "ttl": 3600}', 'cache-service', '2024-01-15 10:01:00'),
(19, 'debug', 'Cache miss', '{"key": "user_999", "reason": "expired"}', 'cache-service', '2024-01-15 10:02:00'),
(20, 'debug', 'API request received', '{"endpoint": "/api/users", "method": "GET"}', 'api-gateway', '2024-01-15 10:00:00'),

-- Critical logs
(21, 'critical', 'Database server down', '{"server": "db-primary", "uptime": "72 hours"}', 'db-monitor', '2024-01-17 03:00:00'),
(22, 'critical', 'Out of memory error', '{"available_mb": 0, "required_mb": 512}', 'system-monitor', '2024-01-18 06:30:00'),

-- More recent logs (current day)
(23, 'info', 'System health check passed', '{"checks": 12, "failed": 0}', 'health-monitor', '2024-01-21 08:00:00'),
(24, 'info', 'Daily analytics report generated', '{"report_type": "daily", "records_processed": 15234}', 'analytics-service', '2024-01-21 06:00:00'),
(25, 'warning', 'Unusual login pattern detected', '{"user_id": 1, "login_count": 15, "timeframe": "1 hour"}', 'security-monitor', '2024-01-21 14:30:00'),
(26, 'info', 'New product added', '{"product_id": 25, "name": "Board Game Strategy"}', 'catalog-service', '2024-01-20 10:00:00'),
(27, 'info', 'Inventory updated', '{"product_id": 6, "old_quantity": 10, "new_quantity": 0}', 'inventory-service', '2024-01-20 11:00:00'),
(28, 'error', 'Payment gateway timeout', '{"gateway": "stripe", "timeout_ms": 30000}', 'payment-service', '2024-01-21 13:00:00'),
(29, 'info', 'Email notification sent', '{"recipient": "john.doe@example.com", "template": "order_confirmation"}', 'notification-service', '2024-01-21 12:30:00'),
(30, 'debug', 'Session created', '{"session_id": "abc123", "user_id": 1}', 'session-service', '2024-01-21 10:00:00');

-- ============================================================================
-- END OF BASIC DATA INSERTION
-- ============================================================================
