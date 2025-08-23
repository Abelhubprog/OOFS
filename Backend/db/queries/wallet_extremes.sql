-- name: wallet_extremes
-- Params: $1 wallet
WITH win AS (
  SELECT to_jsonb(rt.*) AS j FROM realized_trades rt WHERE wallet = $1 ORDER BY realized_pnl_usd_dec DESC LIMIT 1
), loss AS (
  SELECT to_jsonb(rt.*) AS j FROM realized_trades rt WHERE wallet = $1 ORDER BY realized_pnl_usd_dec ASC LIMIT 1
), s2e AS (
  SELECT to_jsonb(m.*) AS j FROM oof_moments m WHERE wallet = $1 AND kind='S2E' ORDER BY missed_usd_dec DESC, pct_dec DESC LIMIT 1
), bhd AS (
  SELECT to_jsonb(m.*) AS j FROM oof_moments m WHERE wallet = $1 AND kind='BHD' ORDER BY pct_dec ASC LIMIT 1
), badroute AS (
  SELECT to_jsonb(m.*) AS j FROM oof_moments m WHERE wallet = $1 AND kind='BadRoute' ORDER BY pct_dec DESC LIMIT 1
), idle AS (
  SELECT to_jsonb(m.*) AS j FROM oof_moments m WHERE wallet = $1 AND kind='Idle' ORDER BY missed_usd_dec DESC LIMIT 1
)
SELECT jsonb_build_object(
  'win', (SELECT j FROM win),
  'loss', (SELECT j FROM loss),
  's2e', (SELECT j FROM s2e),
  'bhd', (SELECT j FROM bhd),
  'badroute', (SELECT j FROM badroute),
  'idle', (SELECT j FROM idle)
) AS extremes;
