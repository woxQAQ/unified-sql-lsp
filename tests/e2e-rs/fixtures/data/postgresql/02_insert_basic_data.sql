-- ============================================================================
-- Unified SQL LSP - E2E Test Fixtures
-- PostgreSQL Basic Data Insertion (Simplified)
-- ============================================================================

INSERT INTO users (id, username, email, full_name, age, balance, is_active, created_at) VALUES
(1, 'john_doe', 'john.doe@example.com', 'John Doe', 32, 1250.50, TRUE, '2024-01-15 10:30:00+00'),
(2, 'jane_smith', 'jane.smith@example.com', 'Jane Smith', 28, 890.00, TRUE, '2024-01-16 14:20:00+00'),
(3, 'bob_wilson', 'bob.wilson@example.com', 'Bob Wilson', 45, 3200.75, TRUE, '2024-01-17 08:00:00+00'),
(4, 'alice_brown', 'alice.brown@example.com', 'Alice Brown', 35, 450.25, TRUE, '2024-01-18 11:45:00+00'),
(5, 'charlie_davis', 'charlie.davis@example.com', 'Charlie Davis', 29, 2100.00, TRUE, '2024-01-19 09:30:00+00');

INSERT INTO products (id, name, description, price, cost, quantity_in_stock, is_available, created_at) VALUES
(1, 'Wireless Bluetooth Headphones', 'Premium noise-cancelling headphones', 149.99, 75.00, 45, TRUE, '2024-01-15 10:00:00+00'),
(2, 'USB-C Charging Cable', 'Fast charging cable', 12.99, 3.50, 200, TRUE, '2024-01-15 10:00:00+00'),
(3, 'Portable Power Bank 20000mAh', 'High-capacity power bank', 39.99, 18.00, 85, TRUE, '2024-01-15 10:00:00+00'),
(4, 'Smart Watch Series 5', 'Fitness tracking watch', 299.99, 150.00, 30, TRUE, '2024-01-15 10:00:00+00'),
(5, 'Mechanical Keyboard RGB', 'Gaming keyboard', 129.99, 65.00, 60, TRUE, '2024-01-15 10:00:00+00');

INSERT INTO orders (id, user_id, order_date, total_amount, notes) VALUES
(1, 1, '2024-01-15 11:00:00+00', 189.98, 'Gift wrapping requested'),
(2, 2, '2024-01-16 15:30:00+00', 49.99, NULL),
(3, 3, '2024-01-17 10:15:00+00', 449.98, 'Corporate order'),
(4, 1, '2024-01-18 13:45:00+00', 79.97, 'Quick delivery'),
(5, 4, '2024-01-19 09:20:00+00', 149.99, NULL);

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price, notes) VALUES
(1, 1, 1, 1, 149.99, 'Gift wrapping'),
(2, 1, 2, 2, 12.99, NULL),
(3, 2, 3, 1, 49.99, NULL),
(4, 3, 4, 1, 299.99, NULL),
(5, 3, 5, 1, 129.99, NULL);
