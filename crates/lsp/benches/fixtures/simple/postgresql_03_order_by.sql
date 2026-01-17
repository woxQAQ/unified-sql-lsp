-- Simple ORDER BY with LIMIT (PostgreSQL)
SELECT product_id, name, price
FROM products
ORDER BY price DESC
LIMIT 10;
