-- name: select_leaderboard
-- Get leaderboard for different metrics
-- Params: $1 period_days, $2 metric_type, $3 limit
WITH period_filter AS (
    SELECT NOW() - INTERVAL '1 day' * $1 as since_ts
),
wallet_stats AS (
    CASE WHEN $2 = 'highest_gains' THEN
        SELECT
            rt.wallet,
            SUM(rt.realized_pnl_usd_dec) as total_pnl,
            COUNT(rt.exit_id) as trade_count,
            MAX(rt.realized_pnl_usd_dec) as best_trade
        FROM realized_trades rt, period_filter pf
        WHERE rt.ts >= pf.since_ts
            AND rt.realized_pnl_usd_dec > 0
        GROUP BY rt.wallet
        HAVING SUM(rt.realized_pnl_usd_dec) > 100
        ORDER BY total_pnl DESC
        LIMIT $3

    WHEN $2 = 'most_s2e' THEN
        SELECT
            om.wallet,
            SUM(om.missed_usd_dec) as total_missed,
            COUNT(om.id) as moment_count,
            MAX(om.missed_usd_dec) as biggest_s2e
        FROM oof_moments om, period_filter pf
        WHERE om.t_event >= pf.since_ts
            AND om.kind = 'S2E'
        GROUP BY om.wallet
        ORDER BY total_missed DESC
        LIMIT $3

    WHEN $2 = 'worst_bhd' THEN
        SELECT
            om.wallet,
            MIN(om.pct_dec) as worst_drawdown,
            COUNT(om.id) as moment_count,
            AVG(om.severity_dec) as avg_severity
        FROM oof_moments om, period_filter pf
        WHERE om.t_event >= pf.since_ts
            AND om.kind = 'BHD'
        GROUP BY om.wallet
        ORDER BY worst_drawdown ASC
        LIMIT $3

    ELSE
        SELECT
            '' as wallet,
            0::numeric as value1,
            0::bigint as value2,
            0::numeric as value3
        WHERE false
    END
)
SELECT * FROM wallet_stats;
