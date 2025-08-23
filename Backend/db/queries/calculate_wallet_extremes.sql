-- name: calculate_wallet_extremes
-- Calculate extreme values for a wallet
-- Params: $1 wallet
WITH wallet_trades AS (
    SELECT
        exit_id,
        mint,
        ts,
        realized_pnl_usd_dec,
        sig
    FROM realized_trades
    WHERE wallet = $1
),
wallet_moments AS (
    SELECT
        id,
        mint,
        kind,
        t_event,
        pct_dec,
        missed_usd_dec,
        severity_dec
    FROM oof_moments
    WHERE wallet = $1
),
highest_win AS (
    SELECT
        exit_id as id,
        mint,
        realized_pnl_usd_dec as value_usd,
        NULL::numeric as value_pct,
        ts as timestamp
    FROM wallet_trades
    WHERE realized_pnl_usd_dec > 0
    ORDER BY realized_pnl_usd_dec DESC
    LIMIT 1
),
highest_loss AS (
    SELECT
        exit_id as id,
        mint,
        realized_pnl_usd_dec as value_usd,
        NULL::numeric as value_pct,
        ts as timestamp
    FROM wallet_trades
    WHERE realized_pnl_usd_dec < 0
    ORDER BY realized_pnl_usd_dec ASC
    LIMIT 1
),
top_s2e AS (
    SELECT
        id,
        mint,
        missed_usd_dec as value_usd,
        pct_dec as value_pct,
        t_event as timestamp
    FROM wallet_moments
    WHERE kind = 'S2E'
    ORDER BY missed_usd_dec DESC, pct_dec DESC
    LIMIT 1
),
worst_bhd AS (
    SELECT
        id,
        mint,
        missed_usd_dec as value_usd,
        pct_dec as value_pct,
        t_event as timestamp
    FROM wallet_moments
    WHERE kind = 'BHD'
    ORDER BY pct_dec ASC, missed_usd_dec DESC
    LIMIT 1
),
worst_bad_route AS (
    SELECT
        id,
        mint,
        missed_usd_dec as value_usd,
        pct_dec as value_pct,
        t_event as timestamp
    FROM wallet_moments
    WHERE kind = 'BAD_ROUTE'
    ORDER BY pct_dec DESC, missed_usd_dec DESC
    LIMIT 1
),
largest_idle AS (
    SELECT
        id,
        mint,
        missed_usd_dec as value_usd,
        NULL::numeric as value_pct,
        t_event as timestamp
    FROM wallet_moments
    WHERE kind = 'IDLE_YIELD'
    ORDER BY missed_usd_dec DESC
    LIMIT 1
)
SELECT
    'highest_win' as extreme_type,
    hw.id,
    hw.mint,
    hw.value_usd,
    hw.value_pct,
    hw.timestamp
FROM highest_win hw
WHERE hw.id IS NOT NULL

UNION ALL

SELECT
    'highest_loss' as extreme_type,
    hl.id,
    hl.mint,
    hl.value_usd,
    hl.value_pct,
    hl.timestamp
FROM highest_loss hl
WHERE hl.id IS NOT NULL

UNION ALL

SELECT
    'top_s2e' as extreme_type,
    ts.id,
    ts.mint,
    ts.value_usd,
    ts.value_pct,
    ts.timestamp
FROM top_s2e ts
WHERE ts.id IS NOT NULL

UNION ALL

SELECT
    'worst_bhd' as extreme_type,
    wb.id,
    wb.mint,
    wb.value_usd,
    wb.value_pct,
    wb.timestamp
FROM worst_bhd wb
WHERE wb.id IS NOT NULL

UNION ALL

SELECT
    'worst_bad_route' as extreme_type,
    wbr.id,
    wbr.mint,
    wbr.value_usd,
    wbr.value_pct,
    wbr.timestamp
FROM worst_bad_route wbr
WHERE wbr.id IS NOT NULL

UNION ALL

SELECT
    'largest_idle' as extreme_type,
    li.id,
    li.mint,
    li.value_usd,
    li.value_pct,
    li.timestamp
FROM largest_idle li
WHERE li.id IS NOT NULL;
