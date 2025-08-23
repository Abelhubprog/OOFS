-- name: upsert_tx_raw
INSERT INTO tx_raw (sig, slot, ts, status, object_key, size_bytes)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (sig) DO UPDATE SET
  slot = EXCLUDED.slot,
  ts = EXCLUDED.ts,
  status = EXCLUDED.status,
  object_key = EXCLUDED.object_key,
  size_bytes = EXCLUDED.size_bytes;
