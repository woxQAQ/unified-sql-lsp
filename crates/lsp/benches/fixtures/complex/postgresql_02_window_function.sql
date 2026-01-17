-- Window functions (PostgreSQL)
SELECT
    id,
    name,
    amount,
    ROW_NUMBER() OVER (ORDER BY amount DESC) as row_num,
    RANK() OVER (ORDER BY amount DESC) as rank_num,
    DENSE_RANK() OVER (ORDER BY amount DESC) as dense_rank,
    SUM(amount) OVER (ORDER BY amount DESC) as running_total
FROM orders
WHERE status = 'completed';
