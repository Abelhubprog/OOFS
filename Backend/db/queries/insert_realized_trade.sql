-- name: insert_realized_trade
INSERT INTO realized_trades (exit_id, wallet, mint, episode_id, ts, qty, vwavg_exit_px_usd_dec, realized_pnl_usd_dec, sig)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (exit_id) DO NOTHING;
