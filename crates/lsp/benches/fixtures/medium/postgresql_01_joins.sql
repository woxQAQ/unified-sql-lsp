-- Two-table JOIN (PostgreSQL)
SELECT u.id, u.name, o.order_id, o.total
FROM users u
INNER JOIN orders o ON u.id = o.user_id
WHERE o.status = 'completed';
