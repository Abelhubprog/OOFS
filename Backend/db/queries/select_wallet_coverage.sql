-- name: select_wallet_coverage
-- Check existing coverage for a wallet in a time range
-- Params: $1 wallet, $2 from_ts, $3 to_ts
SELECT
    MIN(a.ts) as earliest_ts,
    MAX(a.ts) as latest_ts,
    COUNT(DISTINCT a.sig) as signature_count,
    COUNT(*) as action_count
FROM actions a
JOIN participants p ON p.sig = a.sig
WHERE p.wallet = $1
    AND a.ts >= $2
    AND a.ts <= $3;
