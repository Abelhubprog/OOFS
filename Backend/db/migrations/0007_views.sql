-- 0007_views.sql
-- Materialized views for price buckets (lightweight examples)
-- In production, refresh schedules should be managed by workers.

CREATE MATERIALIZED VIEW IF NOT EXISTS mv_price_1m AS
SELECT
  mint,
  date_trunc('minute', ts) AS ts,
  avg(price) AS price
FROM token_prices
WHERE ts >= NOW() - INTERVAL '7 days'
GROUP BY mint, date_trunc('minute', ts);

CREATE MATERIALIZED VIEW IF NOT EXISTS mv_price_5m AS
SELECT
  mint,
  date_trunc('minute', ts) - (EXTRACT(MINUTE FROM ts)::int % 5) * INTERVAL '1 minute' AS ts,
  avg(price) AS price
FROM token_prices
WHERE ts >= NOW() - INTERVAL '180 days' AND ts < NOW() - INTERVAL '7 days'
GROUP BY mint, date_trunc('minute', ts) - (EXTRACT(MINUTE FROM ts)::int % 5) * INTERVAL '1 minute';

CREATE MATERIALIZED VIEW IF NOT EXISTS mv_price_1h AS
SELECT
  mint,
  date_trunc('hour', ts) AS ts,
  avg(price) AS price
FROM token_prices
WHERE ts >= NOW() - INTERVAL '2 years' AND ts < NOW() - INTERVAL '180 days'
GROUP BY mint, date_trunc('hour', ts);

CREATE INDEX IF NOT EXISTS idx_mv_price_1m_mint_ts ON mv_price_1m(mint, ts);
CREATE INDEX IF NOT EXISTS idx_mv_price_5m_mint_ts ON mv_price_5m(mint, ts);
CREATE INDEX IF NOT EXISTS idx_mv_price_1h_mint_ts ON mv_price_1h(mint, ts);
