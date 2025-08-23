-- name: select_price_range_min_max
-- Get min and max prices in a time range
-- Params: $1 mint, $2 from_ts, $3 to_ts
SELECT
    mint,
    MIN(price) as min_price,
    MAX(price) as max_price,
    AVG(price) as avg_price,
    COUNT(*) as sample_count,
    (SELECT ts FROM token_prices WHERE mint = $1 AND ts BETWEEN $2 AND $3 AND price = MIN(token_prices.price) LIMIT 1) as min_ts,
    (SELECT ts FROM token_prices WHERE mint = $1 AND ts BETWEEN $2 AND $3 AND price = MAX(token_prices.price) LIMIT 1) as max_ts
FROM token_prices
WHERE mint = $1
    AND ts BETWEEN $2 AND $3
GROUP BY mint;
