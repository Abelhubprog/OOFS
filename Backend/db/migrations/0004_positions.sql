-- 0004_positions.sql
-- Lots, realized trades, and episodes to record position state and realized PnL

CREATE TABLE IF NOT EXISTS lots (
  lot_id TEXT PRIMARY KEY,          -- ULID
  wallet TEXT NOT NULL,
  mint TEXT NOT NULL,
  episode_id TEXT,                  -- FK to episodes(episode_id) when part of an episode
  entry_ts TIMESTAMPTZ NOT NULL,
  qty_initial NUMERIC(38,18) NOT NULL,
  qty_remaining NUMERIC(38,18) NOT NULL,
  entry_px_usd_dec NUMERIC(38,18) NOT NULL,
  meta_json JSONB
);

CREATE INDEX IF NOT EXISTS idx_lots_wallet ON lots(wallet);
CREATE INDEX IF NOT EXISTS idx_lots_mint ON lots(mint);

CREATE TABLE IF NOT EXISTS realized_trades (
  exit_id TEXT PRIMARY KEY,         -- ULID
  wallet TEXT NOT NULL,
  mint TEXT NOT NULL,
  episode_id TEXT,
  ts TIMESTAMPTZ NOT NULL,
  qty NUMERIC(38,18) NOT NULL,
  vwavg_exit_px_usd_dec NUMERIC(38,18) NOT NULL,
  realized_pnl_usd_dec NUMERIC(38,18) NOT NULL,
  sig TEXT
);

CREATE INDEX IF NOT EXISTS idx_realized_trades_wallet_ts ON realized_trades(wallet, ts DESC);

CREATE TABLE IF NOT EXISTS episodes (
  episode_id TEXT PRIMARY KEY,      -- ULID
  wallet TEXT NOT NULL,
  mint TEXT NOT NULL,
  start_ts TIMESTAMPTZ NOT NULL,
  end_ts TIMESTAMPTZ,
  basis_usd_dec NUMERIC(38,18),
  realized_pnl_usd_dec NUMERIC(38,18),
  roi_pct_dec NUMERIC(38,18),
  meta_json JSONB
);

CREATE INDEX IF NOT EXISTS idx_episodes_wallet_mint_start ON episodes(wallet, mint, start_ts);
