-- 0005_moments_extremes.sql
-- OOF moments and wallet extremes

CREATE TABLE IF NOT EXISTS oof_moments (
  id TEXT PRIMARY KEY,              -- ULID
  wallet TEXT NOT NULL,
  mint TEXT,
  kind TEXT NOT NULL,               -- e.g. S2E, BHD, BadRoute, Idle, Rug
  t_event TIMESTAMPTZ NOT NULL,
  window INTERVAL,
  pct_dec NUMERIC(38,18),
  missed_usd_dec NUMERIC(38,18),
  severity_dec NUMERIC(38,18),
  sig_ref TEXT,
  slot_ref BIGINT,
  version TEXT,
  explain_json JSONB,
  preview_png_url TEXT,
  nft_minted BOOL DEFAULT FALSE,
  nft_mint TEXT,
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_oof_moments_wallet_tevent ON oof_moments(wallet, t_event DESC);
CREATE INDEX IF NOT EXISTS idx_oof_moments_kind_tevent ON oof_moments(kind, t_event DESC);

CREATE TABLE IF NOT EXISTS wallet_extremes (
  wallet TEXT PRIMARY KEY,
  computed_at TIMESTAMPTZ NOT NULL,
  json JSONB NOT NULL
);
