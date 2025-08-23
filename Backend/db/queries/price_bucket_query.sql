-- name: price_bucket_query
-- Fetches price buckets depending on lookback window.
-- Params: $1 mint, $2 from_ts, $3 to_ts
(
  SELECT ts, price FROM mv_price_1m WHERE mint = $1 AND ts BETWEEN $2 AND $3
)
UNION ALL
(
  SELECT ts, price FROM mv_price_5m WHERE mint = $1 AND ts BETWEEN $2 AND $3
)
UNION ALL
(
  SELECT ts, price FROM mv_price_1h WHERE mint = $1 AND ts BETWEEN $2 AND $3
)
ORDER BY ts ASC;
