SELECT u.id, u.name, o.order_id, p.product_name
FROM users u
INNER JOIN orders o ON u.id = o.user_id
LEFT JOIN order_items oi ON o.order_id = oi.order_id
LEFT JOIN products p ON oi.product_id = p.id
WHERE o.created_at > '2024-01-01'
