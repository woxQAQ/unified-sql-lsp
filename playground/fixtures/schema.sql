-- Playground Test Schema
--
-- This schema is used for the SQL LSP playground to demonstrate
-- code completion, hover information, and diagnostics.

-- Users table
CREATE TABLE users (
  id INT PRIMARY KEY AUTO_INCREMENT,
  name VARCHAR(100) NOT NULL,
  email VARCHAR(255) UNIQUE NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Orders table
CREATE TABLE orders (
  id INT PRIMARY KEY AUTO_INCREMENT,
  user_id INT NOT NULL,
  total DECIMAL(10, 2) NOT NULL,
  status VARCHAR(20) DEFAULT 'pending',
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Order items table
CREATE TABLE order_items (
  id INT PRIMARY KEY AUTO_INCREMENT,
  order_id INT NOT NULL,
  product_name VARCHAR(255) NOT NULL,
  quantity INT NOT NULL,
  price DECIMAL(10, 2) NOT NULL,
  FOREIGN KEY (order_id) REFERENCES orders(id)
);

-- Sample data for testing
INSERT INTO users (name, email) VALUES
  ('Alice', 'alice@example.com'),
  ('Bob', 'bob@example.com'),
  ('Charlie', 'charlie@example.com');

INSERT INTO orders (user_id, total, status) VALUES
  (1, 150.00, 'completed'),
  (2, 200.50, 'pending'),
  (3, 75.25, 'shipped');

INSERT INTO order_items (order_id, product_name, quantity, price) VALUES
  (1, 'Widget A', 5, 30.00),
  (1, 'Widget B', 3, 10.00),
  (2, 'Gadget X', 2, 100.25),
  (3, 'Tool Y', 1, 75.25);
