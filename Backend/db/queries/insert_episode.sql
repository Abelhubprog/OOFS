-- name: insert_episode
INSERT INTO episodes (episode_id, wallet, mint, start_ts, end_ts, basis_usd_dec, realized_pnl_usd_dec, roi_pct_dec, meta_json)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (episode_id) DO UPDATE SET
  end_ts = EXCLUDED.end_ts,
  basis_usd_dec = EXCLUDED.basis_usd_dec,
  realized_pnl_usd_dec = EXCLUDED.realized_pnl_usd_dec,
  roi_pct_dec = EXCLUDED.roi_pct_dec,
  meta_json = EXCLUDED.meta_json;
