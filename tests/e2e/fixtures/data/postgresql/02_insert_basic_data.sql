-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- PostgreSQL Basic Data Insertion
-- ============================================================================
-- This file inserts realistic test data for basic testing scenarios.
-- Data volume: 10-50 rows per table for easy verification
-- PostgreSQL-specific: ARRAY types, JSONB data, TIMESTAMPTZ
-- ============================================================================

-- ============================================================================
-- USERS TABLE (20 records)
-- Tests: ARRAY types, JSONB preferences, various user statuses
-- ============================================================================

INSERT INTO users (id, username, email, full_name, age, balance, is_active, status, created_at, last_login, bio, profile_image, phone, tags, preferences) VALUES
-- Active users with complete profiles and PostgreSQL-specific types
(1, 'john_doe', 'john.doe@example.com', 'John Doe', 32, 1250.50, TRUE, 'active', '2024-01-15 10:30:00+00', '2024-01-20 08:45:00+00', 'Software developer and tech enthusiast.', 'https://example.com/images/john.jpg', '+1-555-0101', ARRAY['developer', 'premium'], '{"theme": "dark", "notifications": true, "language": "en"}'),
(2, 'jane_smith', 'jane.smith@example.com', 'Jane Smith', 28, 890.00, TRUE, 'active', '2024-01-16 14:20:00+00', '2024-01-21 09:15:00+00', 'Digital marketer and content creator.', 'https://example.com/images/jane.jpg', '+1-555-0102', ARRAY['marketer', 'verified'], '{"theme": "light", "newsletter": true}'),
(3, 'bob_wilson', 'bob.wilson@example.com', 'Bob Wilson', 45, 3200.75, TRUE, 'active', '2024-01-17 08:00:00+00', '2024-01-19 16:30:00+00', 'Business consultant and entrepreneur.', NULL, '+1-555-0103', ARRAY['business', 'vip'], '{"currency": "USD", "discount_eligible": true}'),
(4, 'alice_brown', 'alice.brown@example.com', 'Alice Brown', 35, 450.25, TRUE, 'active', '2024-01-18 11:45:00+00', '2024-01-20 14:20:00+00', 'Designer and creative director.', 'https://example.com/images/alice.jpg', NULL, ARRAY['designer'], '{"theme": "light"}'),
(5, 'charlie_davis', 'charlie.davis@example.com', 'Charlie Davis', 29, 2100.00, TRUE, 'active', '2024-01-19 09:30:00+00', '2024-01-21 10:00:00+00', 'Full-stack developer and open source contributor.', 'https://example.com/images/charlie.jpg', '+1-555-0104', ARRAY['developer', 'contributor', 'verified'], '{"theme": "dark", "editor": "vim", "notifications": false}'),

-- Active users with minimal profiles
(6, 'diana_miller', 'diana.miller@example.com', 'Diana Miller', 31, 675.50, TRUE, 'active', '2024-01-10 07:15:00+00', '2024-01-18 12:00:00+00', NULL, NULL, NULL, ARRAY['standard'], NULL),
(7, 'frank_garcia', 'frank.garcia@example.com', 'Frank Garcia', 42, 1580.00, TRUE, 'active', '2024-01-11 13:40:00+00', '2024-01-20 15:30:00+00', 'Sales manager.', NULL, '+1-555-0106', ARRAY['sales'], '{"commission_rate": 0.05}'),
(8, 'grace_lee', 'grace.lee@example.com', 'Grace Lee', 26, 320.00, TRUE, 'active', '2024-01-12 10:20:00+00', '2024-01-19 09:45:00+00', NULL, 'https://example.com/images/grace.jpg', NULL, ARRAY[]::TEXT[], '{"theme": "light"}'),
(9, 'henry_martinez', 'henry.martinez@example.com', 'Henry Martinez', 38, 920.25, TRUE, 'active', '2024-01-13 15:55:00+00', '2024-01-21 11:20:00+00', 'Project manager with 10+ years experience.', NULL, '+1-555-0108', ARRAY['manager', 'verified'], '{"team_size": 8}'),
(10, 'iris_anderson', 'iris.anderson@example.com', 'Iris Anderson', 33, 780.00, TRUE, 'active', '2024-01-14 08:30:00+00', '2024-01-20 16:45:00+00', NULL, NULL, NULL, ARRAY['standard'], NULL),

-- Inactive users
(11, 'jack_thomas', 'jack.thomas@example.com', 'Jack Thomas', 50, 120.00, FALSE, 'inactive', '2024-01-05 12:00:00+00', '2024-01-10 09:30:00+00', 'Retired teacher.', NULL, '+1-555-0110', ARRAY['retired'], '{"notifications": false}'),
(12, 'kate_white', 'kate.white@example.com', 'Kate White', 27, 0.00, FALSE, 'inactive', '2024-01-06 14:20:00+00', '2024-01-12 11:00:00+00', NULL, NULL, NULL, ARRAY[]::TEXT[], NULL),
(13, 'leo_harris', 'leo.harris@example.com', 'Leo Harris', 44, 85.50, FALSE, 'suspended', '2024-01-07 09:10:00+00', '2024-01-11 14:45:00+00', 'Freelance writer.', 'https://example.com/images/leo.jpg', '+1-555-0112', ARRAY['writer'], '{"suspension_reason": "policy_violation"}'),

-- Users with edge case balances
(14, 'mia_clark', 'mia.clark@example.com', 'Mia Clark', 30, 0.00, TRUE, 'active', '2024-01-08 11:30:00+00', '2024-01-20 10:15:00+00', 'New user, first purchase pending.', NULL, NULL, ARRAY['new'], '{"onboarding_completed": false}'),
(15, 'noah_lewis', 'noah.lewis@example.com', 'Noah Lewis', 36, 5000.00, TRUE, 'active', '2024-01-09 13:45:00+00', '2024-01-21 08:30:00+00', 'Premium customer with VIP status.', 'https://example.com/images/noah.jpg', '+1-555-0114', ARRAY['vip', 'premium', 'verified'], '{"tier": "platinum", "personal_rep": true}'),

-- Recent users
(16, 'olivia_walker', 'olivia.walker@example.com', 'Olivia Walker', 24, 150.00, TRUE, 'active', '2024-01-20 10:00:00+00', '2024-01-20 10:05:00+00', 'Student.', NULL, NULL, ARRAY['student'], '{"educational_discount": true}'),
(17, 'paul_hall', 'paul.hall@example.com', 'Paul Hall', 41, 980.75, TRUE, 'active', '2024-01-20 11:30:00+00', '2024-01-20 12:00:00+00', NULL, NULL, '+1-555-0116', ARRAY[]::TEXT[], NULL),
(18, 'quinta_allen', 'quinta.allen@example.com', 'Quinta Allen', 29, 420.00, TRUE, 'active', '2024-01-20 14:15:00+00', '2024-01-20 14:20:00+00', 'Travel blogger.', 'https://example.com/images/quinta.jpg', NULL, ARRAY['blogger', 'verified'], '{"blog_url": "https://quinta.travel"}'),
(19, 'ryan_young', 'ryan.young@example.com', 'Ryan Young', 37, 2150.00, TRUE, 'active', '2024-01-21 09:00:00+00', '2024-01-21 09:30:00+00', 'Software architect.', NULL, '+1-555-0118', ARRAY['developer', 'architect', 'premium'], '{"languages": ["Python", "Rust", "Go"]}'),
(20, 'sophia_king', 'sophia.king@example.com', 'Sophia King', 23, 75.00, TRUE, 'active', '2024-01-21 15:45:00+00', '2024-01-21 16:00:00+00', 'Recent graduate.', 'https://example.com/images/sophia.jpg', NULL, ARRAY['new', 'student'], '{"job_seeking": true}');

-- ============================================================================
-- PRODUCTS TABLE (25 records)
-- Tests: PostgreSQL ENUM, ARRAY tags, JSONB attributes
-- ============================================================================

INSERT INTO products (id, name, description, price, cost, quantity_in_stock, category, is_available, weight, sku, tags, attributes) VALUES
-- Electronics with PostgreSQL-specific fields
(1, 'Wireless Bluetooth Headphones', 'Premium noise-cancelling headphones with 30hr battery life', 149.99, 75.00, 45, 'electronics', TRUE, 0.350, 'ELEC-BT-HP-001', ARRAY['audio', 'wireless', 'bluetooth'], '{"brand": "SoundMax", "battery_life": "30 hours", "noise_cancelling": true, "color": ["black", "white", "blue"]}'),
(2, 'USB-C Charging Cable', 'Fast charging cable, 6ft braided nylon', 12.99, 3.50, 200, 'electronics', TRUE, 0.080, 'ELEC-USB-C-001', ARRAY['cable', 'charging', 'usb-c'], '{"length": "6ft", "material": "braided nylon", "color": "black"}'),
(3, 'Portable Power Bank 20000mAh', 'High-capacity power bank with Quick Charge 3.0', 39.99, 18.00, 85, 'electronics', TRUE, 0.450, 'ELEC-PB-20K-001', ARRAY['power', 'charging', 'portable'], '{"capacity": "20000mAh", "quick_charge": true, "ports": 2}'),
(4, 'Smart Watch Series 5', 'Fitness tracking, heart rate monitor, GPS', 299.99, 150.00, 30, 'electronics', TRUE, 0.150, 'ELEC-SW-S5-001', ARRAY['wearable', 'fitness', 'smart'], '{"water_resistant": true, "gps": true, "heart_rate": true, "battery": "5 days"}'),
(5, 'Mechanical Keyboard RGB', 'Gaming keyboard with Cherry MX switches', 129.99, 65.00, 60, 'electronics', TRUE, 1.200, 'ELEC-MK-RGB-001', ARRAY['keyboard', 'gaming', 'mechanical'], '{"switch_type": "Cherry MX", "backlight": "RGB", "numpad": true}'),
(6, 'Wireless Mouse Ergonomic', 'Vertical ergonomic mouse with adjustable DPI', 49.99, 22.00, 0, 'electronics', FALSE, 0.180, 'ELEC-WM-ERG-001', ARRAY['mouse', 'ergonomic', 'wireless'], '{"dpi": "adjustable", "buttons": 6}'),

-- Clothing
(7, 'Classic Cotton T-Shirt', '100% cotton, pre-shrunk, multiple colors', 19.99, 5.00, 150, 'clothing', TRUE, 0.200, 'CLTH-TSH-001', ARRAY['casual', 'cotton'], '{"material": "100% cotton", "sizes": ["XS", "S", "M", "L", "XL"], "colors": ["white", "black", "navy"]}'),
(8, 'Slim Fit Jeans', 'Stretch denim, classic blue wash', 59.99, 25.00, 80, 'clothing', TRUE, 0.600, 'CLTH-JN-SL-001', ARRAY['jeans', 'denim'], '{"fit": "slim", "wash": "blue", "stretch": true}'),
(9, 'Running Shoes Performance', 'Breathable mesh, cushioned sole, size 8-13', 89.99, 40.00, 45, 'clothing', TRUE, 0.850, 'CLTH-SH-RUN-001', ARRAY['footwear', 'running', 'sports'], '{"sizes": [8, 9, 10, 11, 12, 13], "mesh": true, "cushioning": "high"}'),
(10, 'Winter Jacket Waterproof', 'Insulated jacket with removable hood', 159.99, 70.00, 25, 'clothing', TRUE, 1.100, 'CLTH-JK-WIN-001', ARRAY['jacket', 'winter', 'waterproof'], '{"temperature_rating": "-20C", "hood": "removable", "waterproof": true}'),
(11, 'Wool Sweater', 'Merino wool blend, crew neck', 79.99, 35.00, 55, 'clothing', TRUE, 0.450, 'CLTH-SW-WL-001', ARRAY['sweater', 'wool', 'warm'], '{"material": "Merino wool blend", "style": "crew neck"}'),

-- Books
(12, 'The Art of Programming', 'Comprehensive guide to software development', 49.99, 20.00, 100, 'books', TRUE, 0.900, 'BOOK-PROG-001', ARRAY['programming', 'education'], '{"pages": 500, "author": "John Smith", "isbn": "978-0123456789"}'),
(13, 'Introduction to Machine Learning', 'Textbook covering ML fundamentals', 89.99, 45.00, 60, 'books', TRUE, 1.500, 'BOOK-ML-001', ARRAY['machine-learning', 'textbook', 'ai'], '{"pages": 750, "hardcover": true, "edition": "3rd"}'),
(14, 'Mystery Novel Collection', 'Box set of 5 bestselling mystery novels', 34.99, 15.00, 40, 'books', TRUE, 1.200, 'BOOK-MYS-001', ARRAY['fiction', 'mystery', 'collection'], '{"books_count": 5, "format": "paperback"}'),
(15, 'Cooking Mastery Guide', '100+ recipes from professional chefs', 29.99, 12.00, 75, 'books', TRUE, 0.800, 'BOOK-CK-001', ARRAY['cooking', 'recipes'], '{"recipes": 150, "difficulty": "beginner to advanced"}'),

-- Home & Garden
(16, 'Stainless Steel Water Bottle', 'Insulated bottle, keeps drinks cold 24hrs', 24.99, 8.00, 180, 'home', TRUE, 0.350, 'HOME-WB-SS-001', ARRAY['bottle', 'insulated', 'eco-friendly'], '{"capacity": "750ml", "insulation": "24 hours cold", "material": "stainless steel"}'),
(17, 'LED Desk Lamp Adjustable', 'Touch control, 5 brightness levels, USB port', 34.99, 15.00, 90, 'home', TRUE, 0.800, 'HOME-DL-LED-001', ARRAY['lamp', 'led', 'desk'], '{"brightness_levels": 5, "usb_port": true, "color_temperature": "adjustable"}'),
(18, 'Throw Pillow Set (4-pack)', 'Soft decorative pillows, removable covers', 39.99, 16.00, 120, 'home', TRUE, 1.500, 'HOME-TP-4P-001', ARRAY['decor', 'pillows'], '{"count": 4, "covers": "removable, washable", "filling": "hypoallergenic"}'),
(19, 'Plant Pot Ceramic', 'Modern minimalist design, drainage hole', 19.99, 7.00, 65, 'home', TRUE, 0.700, 'HOME-PP-CER-001', ARRAY['planter', 'ceramic', 'modern'], '{"diameter": "15cm", "drainage": true, "saucer_included": true}'),
(20, 'Coffee Maker Programmable', '12-cup capacity, auto shut-off', 69.99, 30.00, 35, 'home', TRUE, 2.800, 'HOME-CM-12-001', ARRAY['coffee', 'appliance'], '{"capacity": "12 cups", "timer": "programmable", "auto_shutoff": true}'),

-- Sports
(21, 'Yoga Mat Premium', 'Non-slip, extra thick, eco-friendly material', 29.99, 12.00, 95, 'sports', TRUE, 1.000, 'SPR-YM-PRE-001', ARRAY['yoga', 'fitness', 'mat'], '{"thickness": "6mm", "material": "eco-friendly TPE", "size": "183x61cm"}'),
(22, 'Resistance Bands Set', '5-piece set with different tension levels', 24.99, 8.00, 0, 'sports', FALSE, 0.400, 'SPR-RB-5P-001', ARRAY['fitness', 'resistance', 'bands'], '{"bands_count": 5, "tension_levels": ["light", "medium", "heavy", "x-heavy"]}'),
(23, 'Dumbbell Set Adjustable', '5-50lbs per dumbbell, storage rack included', 199.99, 90.00, 20, 'sports', TRUE, 25.000, 'SPR-DB-ADJ-001', ARRAY['weights', 'dumbbell', 'adjustable'], '{"weight_range": "5-50 lbs", "rack_included": true}'),

-- Toys
(24, 'Building Blocks Classic', '500-piece set, compatible with major brands', 34.99, 14.00, 110, 'toys', TRUE, 1.200, 'TOY-BB-500-001', ARRAY['building', 'blocks', 'educational'], '{"pieces": 500, "age_range": "4+", "compatible": "universal"}'),
(25, 'Board Game Strategy', '2-4 players, ages 12+, average playtime 60min', 44.99, 18.00, 50, 'toys', TRUE, 1.800, 'TOY-BG-STR-001', ARRAY['board-game', 'strategy', 'family'], '{"players": "2-4", "age": "12+", "playtime": "60 minutes"}');

-- ============================================================================
-- ORDERS TABLE (30 records)
-- Tests: PostgreSQL ENUM for status and payment_method, JSONB metadata
-- ============================================================================

INSERT INTO orders (id, user_id, order_date, total_amount, status, payment_method, shipping_address, billing_address, metadata, shipped_at, delivered_at) VALUES
-- Delivered orders with JSONB metadata
(1, 1, '2024-01-15 11:00:00+00', 189.98, 'delivered', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', '{"gift_wrapping": true, "gift_message": "Happy Birthday!"}', '2024-01-16 10:00:00+00', '2024-01-18 15:30:00+00'),
(2, 2, '2024-01-16 15:30:00+00', 49.99, 'delivered', 'paypal', '456 Oak Ave, Los Angeles, CA 90001', '456 Oak Ave, Los Angeles, CA 90001', '{"expedited_shipping": false}', '2024-01-17 09:00:00+00', '2024-01-19 12:00:00+00'),
(3, 3, '2024-01-17 10:15:00+00', 449.98, 'delivered', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', '{"corporate_order": true, "po_number": "PO-2024-001"}', '2024-01-18 14:00:00+00', '2024-01-21 10:30:00+00'),
(4, 1, '2024-01-18 13:45:00+00', 79.97, 'delivered', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', '{"coupons": ["SAVE10"]}', '2024-01-19 11:00:00+00', '2024-01-22 14:15:00+00'),
(5, 4, '2024-01-19 09:20:00+00', 149.99, 'delivered', 'debit_card', '321 Elm St, Houston, TX 77001', '321 Elm St, Houston, TX 77001', NULL, '2024-01-20 08:30:00+00', '2024-01-23 16:45:00+00'),

-- Shipped orders
(6, 5, '2024-01-19 14:30:00+00', 279.97, 'shipped', 'credit_card', '654 Maple Dr, Phoenix, AZ 85001', '654 Maple Dr, Phoenix, AZ 85001', '{"tracking_number": "1Z999AA10123456784", "carrier": "UPS"}', '2024-01-20 10:00:00+00', NULL),
(7, 6, '2024-01-20 10:00:00+00', 89.98, 'shipped', 'paypal', '987 Cedar Ln, Philadelphia, PA 19101', '987 Cedar Ln, Philadelphia, PA 19101', '{"tracking_number": "1Z999AA10123456785", "carrier": "UPS"}', '2024-01-21 09:30:00+00', NULL),
(8, 7, '2024-01-20 16:45:00+00', 199.99, 'shipped', 'credit_card', '147 Birch Blvd, San Antonio, TX 78201', '147 Birch Blvd, San Antonio, TX 78201', NULL, '2024-01-21 13:00:00+00', NULL),

-- Processing orders
(9, 8, '2024-01-21 08:30:00+00', 64.98, 'processing', 'debit_card', '258 Walnut Way, San Diego, CA 92101', '258 Walnut Way, San Diego, CA 92101', NULL, NULL, NULL),
(10, 9, '2024-01-21 11:15:00+00', 329.98, 'processing', 'credit_card', '369 Spruce St, Dallas, TX 75201', '369 Spruce St, Dallas, TX 75201', '{"priority_processing": true}', NULL, NULL),
(11, 10, '2024-01-21 13:00:00+00', 109.97, 'processing', 'paypal', '741 Ash Ave, San Jose, CA 95101', '741 Ash Ave, San Jose, CA 95101', NULL, NULL, NULL),
(12, 2, '2024-01-21 14:30:00+00', 39.99, 'processing', 'credit_card', '456 Oak Ave, Los Angeles, CA 90001', '456 Oak Ave, Los Angeles, CA 90001', NULL, NULL, NULL),

-- Pending orders
(13, 3, '2024-01-21 09:00:00+00', 149.99, 'pending', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', NULL, NULL, NULL),
(14, 5, '2024-01-21 10:30:00+00', 84.98, 'pending', 'credit_card', '654 Maple Dr, Phoenix, AZ 85001', '654 Maple Dr, Phoenix, AZ 85001', NULL, NULL, NULL),
(15, 1, '2024-01-21 11:45:00+00', 299.99, 'pending', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', NULL, NULL, NULL),
(16, 14, '2024-01-21 12:00:00+00', 19.99, 'pending', 'debit_card', '963 Willow Ct, Austin, TX 78701', '963 Willow Ct, Austin, TX 78701', NULL, NULL, NULL),
(17, 15, '2024-01-21 15:00:00+00', 859.96, 'pending', 'credit_card', '852 Aspen Pl, Jacksonville, FL 32201', '852 Aspen Pl, Jacksonville, FL 32201', '{"vip_order": true}', NULL, NULL),
(18, 16, '2024-01-21 16:15:00+00', 34.99, 'pending', 'paypal', '741 Oak Ln, Fort Worth, TX 76101', '741 Oak Ln, Fort Worth, TX 76101', NULL, NULL, NULL),

-- Cancelled orders
(19, 11, '2024-01-10 10:00:00+00', 199.99, 'cancelled', 'credit_card', '159 Pine St, Columbus, OH 43201', '159 Pine St, Columbus, OH 43201', '{"cancellation_reason": "payment_failed"}', NULL, NULL),
(20, 12, '2024-01-12 14:30:00+00', 49.99, 'cancelled', 'debit_card', '357 Maple Dr, Charlotte, NC 28201', '357 Maple Dr, Charlotte, NC 28201', '{"cancellation_reason": "customer_request"}', NULL, NULL),
(21, 11, '2024-01-15 09:15:00+00', 89.98, 'cancelled', 'paypal', '159 Pine St, Columbus, OH 43201', '159 Pine St, Columbus, OH 43201', NULL, NULL, NULL),

-- Refunded orders
(22, 13, '2024-01-08 11:00:00+00', 129.99, 'refunded', 'credit_card', '951 Cedar Ave, San Francisco, CA 94101', '951 Cedar Ave, San Francisco, CA 94101', '{"refund_reason": "defective", "refund_amount": 129.99}', '2024-01-09 10:00:00+00', '2024-01-12 14:00:00+00'),
(23, 3, '2024-01-14 13:30:00+00', 349.99, 'refunded', 'bank_transfer', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', '{"refund_reason": "customer_changed_mind"}', '2024-01-15 11:00:00+00', '2024-01-18 09:30:00+00'),

-- High-value orders
(24, 15, '2024-01-12 10:00:00+00', 1249.95, 'delivered', 'credit_card', '852 Aspen Pl, Jacksonville, FL 32201', '852 Aspen Pl, Jacksonville, FL 32201', '{"insurance": true, "signature_required": true}', '2024-01-13 09:00:00+00', '2024-01-17 15:00:00+00'),
(25, 3, '2024-01-16 08:30:00+00', 899.97, 'delivered', 'credit_card', '789 Pine Rd, Chicago, IL 60601', '789 Pine Rd, Chicago, IL 60601', NULL, '2024-01-17 10:30:00+00', '2024-01-20 12:00:00+00'),

-- Recent orders (same day)
(26, 17, '2024-01-21 10:00:00+00', 174.97, 'pending', 'credit_card', '456 Elm St, Denver, CO 80201', '456 Elm St, Denver, CO 80201', NULL, NULL, NULL),
(27, 18, '2024-01-21 11:30:00+00', 54.98, 'pending', 'paypal', '789 Oak Ave, Boston, MA 02101', '789 Oak Ave, Boston, MA 02101', NULL, NULL, NULL),
(28, 19, '2024-01-21 13:45:00+00', 399.99, 'processing', 'credit_card', '234 Pine Rd, Seattle, WA 98101', '234 Pine Rd, Seattle, WA 98101', NULL, NULL, NULL),
(29, 20, '2024-01-21 14:15:00+00', 24.99, 'pending', 'debit_card', '567 Maple Dr, Nashville, TN 37201', '567 Maple Dr, Nashville, TN 37201', NULL, NULL, NULL),
(30, 1, '2024-01-21 16:00:00+00', 149.98, 'pending', 'credit_card', '123 Main St, New York, NY 10001', '123 Main St, New York, NY 10001', NULL, NULL, NULL);

-- ============================================================================
-- ORDER_ITEMS TABLE (50 records)
-- Tests: foreign keys, computed columns, JSONB metadata
-- ============================================================================

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, discount_percent, notes, metadata) VALUES
(1, 1, 1, 1, 149.99, 0.00, 'Gift wrapping requested', '{"gift": true}'),
(2, 1, 7, 2, 19.99, 0.00, NULL, NULL),
(3, 2, 12, 1, 49.99, 0.00, NULL, NULL),
(4, 3, 4, 1, 299.99, 0.00, NULL, NULL),
(5, 3, 5, 1, 129.99, 10.00, 'Minor scratch on box', '{"condition": "open_box"}'),
(6, 3, 20, 1, 69.99, 0.00, NULL, NULL),
(7, 4, 16, 4, 24.99, 5.00, 'Bulk order', '{"bulk": true}'),
(8, 4, 17, 1, 29.99, 0.00, NULL, NULL),
(9, 5, 1, 1, 149.99, 0.00, NULL, NULL),
(10, 6, 2, 3, 12.99, 0.00, NULL, NULL),
(11, 6, 3, 2, 39.99, 0.00, NULL, NULL),
(12, 6, 6, 1, 49.99, 15.00, 'Open box discount', '{"condition": "open_box"}'),
(13, 7, 7, 3, 19.99, 0.00, NULL, NULL),
(14, 7, 8, 1, 59.99, 0.00, NULL, NULL),
(15, 8, 13, 1, 89.99, 20.00, 'Textbook sale', '{"sale_type": "academic"}'),
(16, 8, 18, 1, 39.99, 0.00, NULL, NULL),
(17, 8, 19, 2, 19.99, 0.00, NULL, NULL),
(18, 9, 21, 1, 29.99, 0.00, NULL, NULL),
(19, 9, 22, 1, 24.99, 0.00, NULL, NULL),
(20, 9, 21, 1, 29.99, 0.00, 'Gift - separate shipping', '{"gift_wrap": true}'),
(21, 10, 1, 2, 149.99, 5.00, 'Bundle discount', '{"bundle": true, "bundle_size": 2}'),
(22, 10, 4, 1, 299.99, 0.00, NULL, NULL),
(23, 11, 1, 1, 149.99, 0.00, NULL, NULL),
(24, 11, 12, 1, 49.99, 0.00, NULL, NULL),
(25, 11, 16, 2, 24.99, 10.00, NULL, '{"discount_code": "SAVE10"}'),
(26, 12, 15, 1, 39.99, 0.00, NULL, NULL),
(27, 13, 1, 1, 149.99, 0.00, NULL, NULL),
(28, 14, 2, 2, 12.99, 0.00, NULL, NULL),
(29, 14, 16, 1, 24.99, 0.00, NULL, NULL),
(30, 14, 21, 1, 29.99, 0.00, NULL, NULL),
(31, 15, 4, 1, 299.99, 0.00, NULL, NULL),
(32, 16, 7, 1, 19.99, 0.00, NULL, NULL),
(33, 17, 1, 5, 149.99, 15.00, 'VIP bulk discount', '{"vip_discount": true, "bulk_order": true}'),
(34, 17, 4, 3, 299.99, 10.00, NULL, NULL),
(35, 17, 5, 2, 129.99, 0.00, NULL, NULL),
(36, 18, 25, 1, 34.99, 0.00, NULL, NULL),
(37, 19, 3, 2, 39.99, 0.00, 'Payment issue', NULL),
(38, 19, 9, 1, 89.99, 0.00, NULL, NULL),
(39, 20, 10, 1, 159.99, 0.00, 'Customer request', NULL),
(40, 21, 6, 1, 49.99, 0.00, 'Item unavailable', NULL),
(41, 22, 5, 1, 129.99, 0.00, 'Returned - defective', '{"return_reason": "defective"}'),
(42, 23, 1, 2, 149.99, 0.00, 'Customer changed mind', NULL),
(43, 23, 20, 1, 69.99, 0.00, NULL, NULL),
(44, 24, 23, 1, 199.99, 0.00, NULL, NULL),
(45, 24, 1, 1, 149.99, 0.00, NULL, NULL),
(46, 24, 4, 3, 299.99, 0.00, NULL, NULL),
(47, 25, 12, 10, 49.99, 25.00, 'Educational bulk discount', '{"educational": true}'),
(48, 25, 13, 5, 89.99, 20.00, NULL, NULL),
(49, 26, 1, 1, 149.99, 0.00, NULL, NULL),
(50, 26, 21, 1, 29.99, 0.00, NULL, NULL);

-- ============================================================================
-- EMPLOYEES TABLE (12 records)
-- Tests: self-referencing, ARRAY skills, JSONB metadata
-- ============================================================================

INSERT INTO employees (id, first_name, last_name, email, manager_id, department, position, salary, hire_date, is_active, skills, metadata) VALUES
(1, 'James', 'Wilson', 'james.wilson@company.com', NULL, 'Executive', 'CEO', 250000.00, '2015-01-15', TRUE, ARRAY['leadership', 'strategy'], '{"office": "Corner Office", "executive_level": true}'),
(2, 'Sarah', 'Johnson', 'sarah.johnson@company.com', NULL, 'Executive', 'CTO', 220000.00, '2015-03-01', TRUE, ARRAY['technology', 'architecture', 'leadership'], '{"executive_level": true}'),
(3, 'Michael', 'Chen', 'michael.chen@company.com', NULL, 'Executive', 'CFO', 210000.00, '2015-02-20', TRUE, ARRAY['finance', 'accounting', 'strategy'], '{"executive_level": true}'),
(4, 'Emily', 'Rodriguez', 'emily.rodriguez@company.com', 2, 'Engineering', 'VP of Engineering', 180000.00, '2016-05-10', TRUE, ARRAY['management', 'coding', 'architecture'], '{"direct_reports": 3}'),
(5, 'David', 'Kim', 'david.kim@company.com', 4, 'Engineering', 'Engineering Manager', 140000.00, '2017-03-15', TRUE, ARRAY['management', 'coding'], '{"direct_reports": 3}'),
(6, 'Lisa', 'Patel', 'lisa.patel@company.com', 5, 'Engineering', 'Senior Developer', 120000.00, '2018-07-20', TRUE, ARRAY['Python', 'JavaScript', 'PostgreSQL', 'Docker'], '{"senior_level": true}'),
(7, 'Robert', 'Martinez', 'robert.martinez@company.com', 5, 'Engineering', 'Developer', 95000.00, '2019-01-10', TRUE, ARRAY['JavaScript', 'React', 'Node.js'], NULL),
(8, 'Amanda', 'Taylor', 'amanda.taylor@company.com', 5, 'Engineering', 'Junior Developer', 75000.00, '2020-06-15', TRUE, ARRAY['Python', 'SQL'], '{"junior_level": true, "mentor_required": true}'),
(9, 'Christopher', 'Lee', 'christopher.lee@company.com', 1, 'Sales', 'VP of Sales', 160000.00, '2016-08-01', TRUE, ARRAY['sales', 'negotiation', 'leadership'], '{"direct_reports": 2}'),
(10, 'Jessica', 'Brown', 'jessica.brown@company.com', 9, 'Sales', 'Sales Manager', 110000.00, '2017-11-20', TRUE, ARRAY['sales', 'management'], '{"direct_reports": 1}'),
(11, 'Daniel', 'Garcia', 'daniel.garcia@company.com', 10, 'Sales', 'Sales Representative', 65000.00, '2019-04-05', TRUE, ARRAY['sales', 'customer_service'], '{"quota": 500000, "ytd_sales": 350000}'),
(12, 'Michelle', 'Davis', 'michelle.davis@company.com', 3, 'Finance', 'Financial Analyst', 85000.00, '2018-09-15', TRUE, ARRAY['accounting', 'Excel', 'financial_modeling'], NULL);

-- ============================================================================
-- POSTS, TAGS, POST_TAGS (shorter version for PostgreSQL)
-- ============================================================================

INSERT INTO posts (id, title, slug, content, excerpt, author_id, status, tags, view_count, published_at) VALUES
(1, 'Getting Started with SQL', 'getting-started-with-sql', 'SQL is a powerful language...', 'Learn SQL fundamentals', 1, 'published', ARRAY['sql', 'tutorial', 'beginner'], 1520, '2024-01-16 10:00:00+00'),
(2, 'Advanced Query Optimization', 'advanced-query-optimization', 'Optimizing SQL queries...', 'Master query performance', 1, 'published', ARRAY['sql', 'performance', 'advanced'], 890, '2024-01-17 14:30:00+00'),
(3, 'Understanding Database Normalization', 'understanding-normalization', 'Database normalization helps...', 'A deep dive into normalization', 3, 'published', ARRAY['sql', 'database', 'design'], 645, '2024-01-18 09:15:00+00'),
(4, 'Top 10 SQL Best Practices', 'sql-best-practices', 'Following best practices...', 'Write better SQL code', 5, 'published', ARRAY['sql', 'best-practices'], 2100, '2024-01-19 11:45:00+00'),
(5, 'Introduction to Window Functions', 'introduction-window-functions', 'Window functions are powerful...', 'Unlock advanced analytics', 1, 'published', ARRAY['sql', 'advanced', 'analytics'], 1234, '2024-01-20 08:30:00+00'),
(6, 'Database Security Fundamentals', 'database-security-fundamentals', 'Securing your database...', 'Protect your data', 3, 'published', ARRAY['security', 'database'], 567, '2024-01-20 15:00:00+00'),
(7, 'SQL vs NoSQL: When to Use Which', 'sql-vs-nosql', 'This comparison guide...', 'Choose the right database', 5, 'draft', ARRAY['sql', 'nosql', 'architecture'], 0, NULL);

INSERT INTO tags (id, name, slug, description, color) VALUES
(1, 'SQL', 'sql', 'SQL and relational database topics', '#3b82f6'),
(2, 'Tutorial', 'tutorial', 'Step-by-step guides', '#10b981'),
(3, 'Performance', 'performance', 'Query optimization', '#f59e0b'),
(4, 'Security', 'security', 'Database security', '#ef4444'),
(5, 'MySQL', 'mysql', 'MySQL-specific articles', '#00758f'),
(6, 'PostgreSQL', 'postgresql', 'PostgreSQL features', '#336791'),
(7, 'Beginner', 'beginner', 'Content for beginners', '#8b5cf6'),
(8, 'Advanced', 'advanced', 'Advanced concepts', '#ec4899'),
(9, 'Best Practices', 'best-practices', 'Industry standards', '#6366f1'),
(10, 'Architecture', 'architecture', 'Database architecture', '#14b8a6');

INSERT INTO post_tags (post_id, tag_id) VALUES
(1, 1), (1, 2), (1, 7),
(2, 1), (2, 3), (2, 8),
(3, 1), (3, 7), (3, 9),
(4, 1), (4, 9), (4, 8),
(5, 1), (5, 8),
(6, 1), (6, 4),
(7, 1), (7, 10);

-- ============================================================================
-- LOGS TABLE (sample with PostgreSQL-specific JSONB context)
-- ============================================================================

INSERT INTO logs (id, level, message, context, source, created_at) VALUES
(1, 'error', 'Database connection timeout', '{"error_code": "ETIMEDOUT", "retry_count": 3}', 'db-connection', '2024-01-15 10:30:00+00'),
(2, 'error', 'Failed to execute query', '{"query": "SELECT * FROM users", "error": "Table not found"}', 'query-executor', '2024-01-15 11:45:00+00'),
(3, 'warning', 'High memory usage detected', '{"usage_percent": 85, "threshold": 80}', 'system-monitor', '2024-01-15 12:00:00+00'),
(4, 'info', 'User logged in successfully', '{"user_id": 1, "username": "john_doe"}', 'auth-service', '2024-01-15 10:00:00+00'),
(5, 'info', 'New order created', '{"order_id": 1, "user_id": 1, "total": 189.98}', 'order-service', '2024-01-15 11:00:00+00'),
(6, 'debug', 'Cache hit', '{"key": "user_1", "ttl": 3600}', 'cache-service', '2024-01-15 10:01:00+00');

-- ============================================================================
-- END OF BASIC DATA INSERTION
-- ============================================================================
