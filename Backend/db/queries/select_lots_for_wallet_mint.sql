-- name: select_lots_for_wallet_mint
-- Get lots for position calculation (FIFO order)
-- Params: $1 wallet, $2 mint
SELECT lot_id, episode_id, entry_ts, qty_initial, qty_remaining, entry_px_usd_dec
FROM lots
WHERE wallet = $1
    AND mint = $2
    AND qty_remaining > 0
ORDER BY entry_ts ASC, lot_id ASC;
