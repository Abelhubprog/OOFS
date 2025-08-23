-- name: update_job_status
-- Update job status
-- Params: $1 job_id, $2 status, $3 error_message
UPDATE job_queue
SET status = $2,
    locked_by = NULL,
    locked_at = NULL,
    error_message = $3,
    completed_at = CASE WHEN $2 IN ('done', 'failed') THEN NOW() ELSE NULL END
WHERE id = $1;
