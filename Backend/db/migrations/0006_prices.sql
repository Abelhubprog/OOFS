-- 0006_prices.sql
-- Token price buckets

CREATE TABLE IF NOT EXISTS token_prices (
  mint TEXT NOT NULL,
  ts TIMESTAMPTZ NOT NULL,
  price NUMERIC(38,18) NOT NULL,
  source TEXT,
  PRIMARY KEY (mint, ts)
);

-- Optional BRIN index for ts to speed large-range queries in production
CREATE INDEX IF NOT EXISTS idx_token_prices_ts ON token_prices(ts);
