-- name: dequeue_job
-- Get next available job and lock it
-- Params: $1 worker_id
UPDATE job_queue
SET status = 'running',
    locked_by = $1,
    locked_at = NOW(),
    attempts = attempts + 1
WHERE id = (
    SELECT id
    FROM job_queue
    WHERE status = 'queued'
        AND run_after <= NOW()
        AND attempts < max_attempts
    ORDER BY run_after ASC, created_at ASC
    LIMIT 1
    FOR UPDATE SKIP LOCKED
)
RETURNING id, kind, payload_json, attempts, max_attempts;
