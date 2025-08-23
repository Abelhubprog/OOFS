-- name: insert_lot
INSERT INTO lots (lot_id, wallet, mint, episode_id, entry_ts, qty_initial, qty_remaining, entry_px_usd_dec, meta_json)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
ON CONFLICT (lot_id) DO NOTHING;
