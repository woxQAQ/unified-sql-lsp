-- Nested correlated subqueries (PostgreSQL)
SELECT
    u.id,
    u.name,
    (SELECT COUNT(*) FROM orders WHERE user_id = u.id) as order_count,
    (SELECT AVG(amount) FROM orders WHERE user_id = u.id) as avg_order,
    (SELECT MAX(amount) FROM orders WHERE user_id = u.id) as max_order
FROM users u
WHERE EXISTS (
    SELECT 1 FROM orders o
    WHERE o.user_id = u.id
    AND o.amount > (
        SELECT AVG(amount) * 2 FROM orders
    )
);
