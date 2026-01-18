-- Basic WHERE clause with comparison
SELECT *
FROM orders
WHERE status = 'pending'
  AND amount > 100;
