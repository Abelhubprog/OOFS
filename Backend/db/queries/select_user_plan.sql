-- name: select_user_plan
-- Get user's current plan
-- Params: $1 user_id
SELECT
    p.code,
    p.price_usd_dec,
    p.daily_wallets,
    p.backfill_days,
    p.cadence,
    p.alerts,
    p.api_rows,
    p.perks_json,
    up.started_at,
    up.expires_at
FROM plans p
JOIN user_plans up ON up.plan_code = p.code
WHERE up.user_id = $1
    AND up.expires_at > NOW()
ORDER BY up.started_at DESC
LIMIT 1;
