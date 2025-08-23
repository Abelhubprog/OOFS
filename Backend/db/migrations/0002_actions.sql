-- 0002_actions.sql
-- Canonical normalized actions extracted from tx_raw (one row per meaningful instr/log)
CREATE TABLE IF NOT EXISTS actions (
  id TEXT PRIMARY KEY, -- ULID
  sig TEXT NOT NULL, -- references tx_raw(sig)
  log_idx INT NOT NULL,
  slot BIGINT NOT NULL,
  ts TIMESTAMPTZ NOT NULL,
  program_id TEXT,
  kind TEXT,
  mint TEXT,
  amount_dec NUMERIC(38,18),
  exec_px_usd_dec NUMERIC(38,18),
  route TEXT,
  flags_json JSONB
);

-- Unique constraint to prevent duplicate (sig, log_idx) inserts
CREATE UNIQUE INDEX IF NOT EXISTS ux_actions_sig_logidx ON actions(sig, log_idx);

-- Indexes for queries by mint/time and GIN for flags
CREATE INDEX IF NOT EXISTS idx_actions_mint_ts ON actions(mint, ts);
CREATE INDEX IF NOT EXISTS idx_actions_flags_gin ON actions USING GIN (flags_json);
