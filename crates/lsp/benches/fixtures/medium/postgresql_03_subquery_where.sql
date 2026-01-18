-- Subquery in WHERE (PostgreSQL)
SELECT id, name, email
FROM users
WHERE id IN (
    SELECT user_id
    FROM orders
    WHERE amount > 1000
);
