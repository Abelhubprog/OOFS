-- 0001_tx_raw.sql
-- Stores each observed transaction raw payload (compressed in object store)
CREATE TABLE IF NOT EXISTS tx_raw (
  sig TEXT PRIMARY KEY,
  slot BIGINT NOT NULL,
  ts TIMESTAMPTZ NOT NULL,
  status TEXT,
  object_key TEXT, -- e.g. tx/<prefix>/<sig>.json.zst
  size_bytes INT
);

CREATE INDEX IF NOT EXISTS idx_tx_raw_slot ON tx_raw(slot);
