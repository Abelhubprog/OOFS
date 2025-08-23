-- name: upsert_wallet_cursor
-- Update wallet cursor for backfill tracking
-- Params: $1 wallet, $2 from_ts, $3 to_ts, $4 last_cursor_sig
INSERT INTO wallet_cursors (wallet, from_ts, to_ts, last_cursor_sig)
VALUES ($1, $2, $3, $4)
ON CONFLICT (wallet) DO UPDATE SET
    from_ts = CASE WHEN EXCLUDED.from_ts < wallet_cursors.from_ts THEN EXCLUDED.from_ts ELSE wallet_cursors.from_ts END,
    to_ts = CASE WHEN EXCLUDED.to_ts > wallet_cursors.to_ts THEN EXCLUDED.to_ts ELSE wallet_cursors.to_ts END,
    last_cursor_sig = EXCLUDED.last_cursor_sig;
