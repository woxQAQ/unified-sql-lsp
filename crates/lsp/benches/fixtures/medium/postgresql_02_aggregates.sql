-- Aggregates with GROUP BY (PostgreSQL)
SELECT u.id, u.name, COUNT(o.id) as order_count, SUM(o.amount) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name
HAVING COUNT(o.id) > 5
ORDER BY total_spent DESC;
