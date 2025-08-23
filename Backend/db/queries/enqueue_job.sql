-- name: enqueue_job
-- Insert a new job into the queue
-- Params: $1 id, $2 kind, $3 payload_json, $4 run_after, $5 max_attempts
INSERT INTO job_queue (id, kind, payload_json, run_after, max_attempts)
VALUES ($1, $2, $3, $4, $5);
