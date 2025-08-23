-- name: upsert_token_facts
-- Insert or update token metadata
-- Params: $1 mint, $2 symbol, $3 decimals, $4 logo_url, $5 ath_price, $6 atl_price, $7 liquidity_band
INSERT INTO token_facts (mint, symbol, decimals, logo_url, ath_price, atl_price, liquidity_band, updated_at)
VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
ON CONFLICT (mint) DO UPDATE SET
    symbol = COALESCE(EXCLUDED.symbol, token_facts.symbol),
    decimals = COALESCE(EXCLUDED.decimals, token_facts.decimals),
    logo_url = COALESCE(EXCLUDED.logo_url, token_facts.logo_url),
    ath_price = CASE WHEN EXCLUDED.ath_price > token_facts.ath_price THEN EXCLUDED.ath_price ELSE token_facts.ath_price END,
    atl_price = CASE WHEN EXCLUDED.atl_price < token_facts.atl_price THEN EXCLUDED.atl_price ELSE token_facts.atl_price END,
    liquidity_band = COALESCE(EXCLUDED.liquidity_band, token_facts.liquidity_band),
    updated_at = NOW();
