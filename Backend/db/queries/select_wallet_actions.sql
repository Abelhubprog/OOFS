-- name: select_wallet_actions
-- Params: $1 wallet, $2 from_ts
SELECT a.*
FROM actions a
JOIN participants p ON p.sig = a.sig
WHERE p.wallet = $1 AND a.ts >= $2
ORDER BY a.slot ASC, a.sig ASC, a.log_idx ASC;
