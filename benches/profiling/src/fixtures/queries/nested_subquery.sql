SELECT u.name, u.email
FROM users u
WHERE u.id IN (
    SELECT o.user_id
    FROM orders o
    WHERE o.total > (
        SELECT AVG(o2.total)
        FROM orders o2
        WHERE o2.user_id = o.user_id
    )
)
