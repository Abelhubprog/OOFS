-- name: select_moments_for_wallet
-- Get moments for a wallet with pagination
-- Params: $1 wallet, $2 kinds (array), $3 since_ts, $4 min_usd, $5 limit, $6 cursor_t_event, $7 cursor_id
SELECT id, wallet, mint, kind, t_event, window, pct_dec, missed_usd_dec,
       severity_dec, sig_ref, slot_ref, version, explain_json,
       preview_png_url, nft_minted, nft_mint
FROM oof_moments
WHERE wallet = $1
    AND ($2::text[] IS NULL OR kind = ANY($2))
    AND ($3::timestamptz IS NULL OR t_event >= $3)
    AND ($4::numeric IS NULL OR missed_usd_dec >= $4)
    AND (
        $6::timestamptz IS NULL OR
        t_event < $6 OR
        (t_event = $6 AND id < $7)
    )
ORDER BY t_event DESC, id DESC
LIMIT $5;
