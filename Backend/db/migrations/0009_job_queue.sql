-- 0009_job_queue.sql
CREATE TABLE IF NOT EXISTS job_queue (
  id TEXT PRIMARY KEY,                 -- ULID
  kind TEXT NOT NULL,                  -- e.g., backfill, compute, refresh_prices
  payload_json JSONB NOT NULL,
  run_after TIMESTAMPTZ DEFAULT NOW(),
  attempts INT DEFAULT 0,
  max_attempts INT DEFAULT 5,
  locked_by TEXT,
  locked_at TIMESTAMPTZ,
  status TEXT DEFAULT 'queued',        -- queued|running|done|failed
  created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_job_queue_run ON job_queue(status, run_after);
