-- Aggregate functions with GROUP BY and HAVING
SELECT u.id, u.name, COUNT(o.id) as order_count, SUM(o.amount) as total_spent
FROM users u
LEFT JOIN orders o ON u.id = o.user_id
GROUP BY u.id, u.name
HAVING order_count > 5
ORDER BY total_spent DESC;
