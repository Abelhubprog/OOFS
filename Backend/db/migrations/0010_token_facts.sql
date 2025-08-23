-- 0010_token_facts.sql
CREATE TABLE IF NOT EXISTS token_facts (
  mint TEXT PRIMARY KEY,
  symbol TEXT,
  decimals INT,
  logo_url TEXT,
  ath_price NUMERIC(38,18),
  atl_price NUMERIC(38,18),
  liquidity_band TEXT,
  updated_at TIMESTAMPTZ
);
