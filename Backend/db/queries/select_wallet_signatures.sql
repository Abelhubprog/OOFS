-- name: select_wallet_signatures
-- Get all signatures for a wallet in a time range
-- Params: $1 wallet, $2 from_ts, $3 to_ts, $4 limit
SELECT DISTINCT p.sig, a.ts, a.slot
FROM participants p
JOIN actions a ON a.sig = p.sig
WHERE p.wallet = $1
    AND a.ts >= $2
    AND a.ts <= $3
ORDER BY a.ts DESC, a.slot DESC
LIMIT $4;
