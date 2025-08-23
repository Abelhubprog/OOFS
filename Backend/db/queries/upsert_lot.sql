-- name: upsert_lot
-- Insert or update a position lot
-- Params: $1 lot_id, $2 wallet, $3 mint, $4 episode_id, $5 entry_ts, $6 qty_initial, $7 qty_remaining, $8 entry_px_usd_dec
INSERT INTO lots (lot_id, wallet, mint, episode_id, entry_ts, qty_initial, qty_remaining, entry_px_usd_dec)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (lot_id) DO UPDATE SET
    qty_remaining = EXCLUDED.qty_remaining,
    entry_px_usd_dec = EXCLUDED.entry_px_usd_dec;
