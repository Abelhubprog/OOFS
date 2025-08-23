-- name: upsert_episode
-- Insert or update an episode
-- Params: $1 episode_id, $2 wallet, $3 mint, $4 start_ts, $5 end_ts, $6 basis_usd_dec, $7 realized_pnl_usd_dec, $8 roi_pct_dec
INSERT INTO episodes (episode_id, wallet, mint, start_ts, end_ts, basis_usd_dec, realized_pnl_usd_dec, roi_pct_dec)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (episode_id) DO UPDATE SET
    end_ts = EXCLUDED.end_ts,
    basis_usd_dec = EXCLUDED.basis_usd_dec,
    realized_pnl_usd_dec = EXCLUDED.realized_pnl_usd_dec,
    roi_pct_dec = EXCLUDED.roi_pct_dec;
