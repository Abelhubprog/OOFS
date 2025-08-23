-- Select current wallet holdings with value calculations
-- Used by: wallet_summary endpoint
-- Parameters: $1 = wallet address

SELECT
    h.mint,
    tf.symbol,
    h.balance_dec,
    CASE
        WHEN tp.price IS NOT NULL AND h.balance_dec > 0
        THEN h.balance_dec * tp.price
        ELSE NULL
    END as value_usd_dec,
    CASE
        WHEN rt.avg_cost IS NOT NULL AND tp.price IS NOT NULL AND h.balance_dec > 0
        THEN h.balance_dec * (tp.price - rt.avg_cost)
        ELSE NULL
    END as unrealized_pnl_dec
FROM holdings h
LEFT JOIN token_facts tf ON h.mint = tf.mint
LEFT JOIN LATERAL (
    SELECT price
    FROM token_prices
    WHERE mint = h.mint
    ORDER BY ts DESC
    LIMIT 1
) tp ON true
LEFT JOIN LATERAL (
    SELECT
        CASE
            WHEN SUM(quantity_dec) > 0
            THEN SUM(cost_basis_usd_dec) / SUM(quantity_dec)
            ELSE NULL
        END as avg_cost
    FROM lots
    WHERE wallet = h.wallet
    AND mint = h.mint
    AND quantity_dec > 0
) rt ON true
WHERE h.wallet = $1
AND h.balance_dec > 0
ORDER BY value_usd_dec DESC NULLS LAST, h.balance_dec DESC;
