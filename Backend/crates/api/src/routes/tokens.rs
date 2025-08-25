use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    error::{ApiError, ApiResult},
    validation::validate_pagination,
};
use sqlx::Row;
use time::OffsetDateTime;

use crate::routes::AppState;

#[derive(Deserialize)]
pub struct TrendingTokensQuery {
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Serialize)]
pub struct TrendingToken {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub price: String,
    pub price_change_24h: String,
    pub volume_24h: String,
}

#[derive(Serialize)]
pub struct TrendingTokensResponse {
    pub data: Vec<TrendingToken>,
    pub cursor: Option<String>,
}

pub async fn trending_tokens(
    State(state): State<AppState>,
    Query(query): Query<TrendingTokensQuery>,
) -> ApiResult<Json<TrendingTokensResponse>> {
    let (limit, cursor) = validate_pagination(query.limit, query.cursor.as_deref())?;

    // Query to get trending tokens by 24h volume
    let rows = sqlx::query(
        r#"
        WITH token_volumes AS (
            SELECT
                mint,
                SUM(amount_dec * exec_px_usd_dec) as volume_24h
            FROM actions
            WHERE ts >= NOW() - INTERVAL '24 hours'
                AND exec_px_usd_dec IS NOT NULL
                AND amount_dec > 0
                AND kind IN ('swap', 'buy', 'sell')
            GROUP BY mint
            HAVING SUM(amount_dec * exec_px_usd_dec) > 1000  -- Minimum $1000 volume
        ),
        latest_prices AS (
            SELECT DISTINCT ON (mint)
                mint,
                price,
                ts
            FROM token_prices
            ORDER BY mint, ts DESC
        ),
        price_changes AS (
            SELECT
                lp.mint,
                lp.price as current_price,
                hp.price as price_24h_ago,
                (lp.price - hp.price) / NULLIF(hp.price, 0) AS price_change_24h
            FROM latest_prices lp
            JOIN token_prices hp ON lp.mint = hp.mint
            WHERE hp.ts = (
                SELECT ts FROM token_prices tp2 
                WHERE tp2.mint = lp.mint 
                AND tp2.ts >= NOW() - INTERVAL '24 hours'
                ORDER BY ts ASC LIMIT 1
            )
        )
        SELECT
            tv.mint,
            tf.symbol,
            tf.name,
            tf.logo_url,
            lp.price::TEXT as price,
            COALESCE(pc.price_change_24h, 0)::TEXT as price_change_24h,
            tv.volume_24h::TEXT as volume_24h
        FROM token_volumes tv
        JOIN latest_prices lp ON tv.mint = lp.mint
        LEFT JOIN price_changes pc ON tv.mint = pc.mint
        LEFT JOIN token_facts tf ON tv.mint = tf.mint
        ORDER BY tv.volume_24h DESC
        LIMIT $1
        "#,
    )
    .bind(limit as i64)
    .fetch_all(&state.pg.0)
    .await?;

    let data: Vec<TrendingToken> = rows
        .iter()
        .map(|row| {
            TrendingToken {
                mint: row.get("mint"),
                symbol: row.try_get("symbol").ok(),
                name: row.try_get("name").ok(),
                logo_url: row.try_get("logo_url").ok(),
                price: row.get("price"),
                price_change_24h: row.get("price_change_24h"),
                volume_24h: row.get("volume_24h"),
            }
        })
        .collect();

    // Pagination cursor would be implemented here if needed
    let next_cursor = None;

    Ok(Json(TrendingTokensResponse {
        data,
        cursor: next_cursor,
    }))
}