-- 0008_plans_policy.sql
CREATE TABLE IF NOT EXISTS plans (
  code TEXT PRIMARY KEY,
  price_usd_dec NUMERIC(10,2) NOT NULL,
  daily_wallets INT NOT NULL,
  backfill_days INT NOT NULL,
  cadence TEXT,
  alerts INT,
  api_rows BIGINT,
  perks_json JSONB
);

CREATE TABLE IF NOT EXISTS user_plans (
  user_id TEXT NOT NULL,
  plan_code TEXT NOT NULL REFERENCES plans(code),
  started_at TIMESTAMPTZ NOT NULL,
  expires_at TIMESTAMPTZ,
  PRIMARY KEY(user_id, plan_code)
);

CREATE TABLE IF NOT EXISTS policy_state (
  user_id TEXT PRIMARY KEY,
  analyses_today INT DEFAULT 0,
  last_reset_at TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS wallet_cursors (
  wallet TEXT PRIMARY KEY,
  from_ts TIMESTAMPTZ,
  to_ts TIMESTAMPTZ,
  last_cursor_sig TEXT
);
