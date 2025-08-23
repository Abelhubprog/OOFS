-- name: upsert_policy_state
-- Update or insert policy state for quota tracking
-- Params: $1 user_id, $2 analyses_today, $3 last_reset_at
INSERT INTO policy_state (user_id, analyses_today, last_reset_at)
VALUES ($1, $2, $3)
ON CONFLICT (user_id) DO UPDATE SET
    analyses_today = EXCLUDED.analyses_today,
    last_reset_at = EXCLUDED.last_reset_at;
