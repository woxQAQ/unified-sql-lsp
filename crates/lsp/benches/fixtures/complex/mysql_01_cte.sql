-- Common Table Expression (MySQL 8.0+)
WITH user_stats AS (
    SELECT
        user_id,
        COUNT(*) as total_orders,
        SUM(amount) as total_spent
    FROM orders
    GROUP BY user_id
),
high_value_users AS (
    SELECT user_id
    FROM user_stats
    WHERE total_spent > 5000
)
SELECT u.id, u.name, us.total_orders, us.total_spent
FROM users u
JOIN user_stats us ON u.id = us.user_id
WHERE u.id IN (SELECT user_id FROM high_value_users)
ORDER BY us.total_spent DESC;
