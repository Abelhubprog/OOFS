-- name: insert_action
INSERT INTO actions (id, sig, log_idx, slot, ts, program_id, kind, mint, amount_dec, exec_px_usd_dec, route, flags_json)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
ON CONFLICT (sig, log_idx) DO NOTHING;
