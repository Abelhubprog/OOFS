-- name: select_price_at_time
-- Get price closest to a specific timestamp
-- Params: $1 mint, $2 target_ts, $3 max_age_minutes
SELECT mint, ts, price, source
FROM token_prices
WHERE mint = $1
    AND ts <= $2
    AND ts >= $2 - INTERVAL '1 minute' * $3
ORDER BY ABS(EXTRACT(EPOCH FROM (ts - $2)))
LIMIT 1;
