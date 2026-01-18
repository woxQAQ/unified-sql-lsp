-- Basic WHERE clause (PostgreSQL)
SELECT *
FROM orders
WHERE status = 'pending'
  AND amount > 100;
