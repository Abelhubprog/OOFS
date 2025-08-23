use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::{
    Candle, MaybeRedis, PriceBucket, PriceConfidence, PricePoint, PriceProvider, PriceRange,
    PriceSource,
};
use sqlx::PgPool;
use std::collections::HashMap;
use time::{Duration, OffsetDateTime};

/// Jupiter API response structures
#[derive(Debug, Deserialize)]
struct JupiterPriceResponse {
    data: HashMap<String, JupiterTokenPrice>,
}

#[derive(Debug, Deserialize)]
struct JupiterTokenPrice {
    id: String,
    price: String,
    #[serde(rename = "vsToken")]
    vs_token: String,
    #[serde(rename = "vsTokenSymbol")]
    vs_token_symbol: String,
    timestamp: i64,
}

/// Comprehensive price provider with multiple sources
pub struct CompositePriceProvider {
    pool: PgPool,
    redis: MaybeRedis,
    http_client: Client,
    jupiter_base_url: String,
}

impl CompositePriceProvider {
    pub fn new(pool: PgPool, redis: MaybeRedis, jupiter_base_url: String) -> Self {
        Self {
            pool,
            redis,
            http_client: Client::new(),
            jupiter_base_url,
        }
    }

    /// Refresh prices for a list of mints from Jupiter
    pub async fn refresh_prices_from_jupiter(&self, mints: &[String]) -> Result<Vec<PricePoint>> {
        let mut results = Vec::new();

        // Process mints in batches to avoid hitting API limits
        const BATCH_SIZE: usize = 50;
        for batch in mints.chunks(BATCH_SIZE) {
            let ids = batch.join(",");
            let url = format!("{}/price?ids={}", self.jupiter_base_url, ids);

            let response = self
                .http_client
                .get(&url)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await?
                .error_for_status()?;

            let jupiter_response: JupiterPriceResponse = response.json().await?;

            for (mint, price_data) in jupiter_response.data {
                if let Ok(price) = price_data.price.parse::<Decimal>() {
                    let timestamp = OffsetDateTime::from_unix_timestamp(price_data.timestamp)
                        .unwrap_or_else(|_| OffsetDateTime::now_utc());

                    let price_point = PricePoint {
                        mint: mint.clone(),
                        timestamp,
                        price,
                        source: PriceSource::Jupiter,
                        confidence: PriceConfidence::High,
                    };

                    // Store in database
                    self.store_price(&price_point).await?;

                    // Cache in Redis
                    let cache_key = format!("price:{}:latest", mint);
                    let price_json = serde_json::to_string(&price_point)?;
                    let _ = self
                        .redis
                        .set(cache_key, price_json, Some(Duration::minutes(5)))
                        .await;

                    results.push(price_point);
                }
            }

            // Rate limiting: wait between batches
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        Ok(results)
    }

    /// Store price point in database
    async fn store_price(&self, price: &PricePoint) -> Result<()> {
        sqlx::query!(
            "INSERT INTO token_prices (mint, ts, price, source) VALUES ($1, $2, $3, $4) ON CONFLICT (mint, ts) DO UPDATE SET price = EXCLUDED.price, source = EXCLUDED.source",
            price.mint,
            price.timestamp,
            price.price,
            price.source.as_str()
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get price using best available source with fallbacks
    pub async fn get_price_with_fallback(
        &self,
        mint: &str,
        timestamp: OffsetDateTime,
    ) -> Result<Option<PricePoint>> {
        // 1. Try cache first
        if let Ok(Some(cached)) = self.get_cached_price(mint).await {
            if (OffsetDateTime::now_utc() - cached.timestamp).whole_minutes() < 5 {
                return Ok(Some(cached));
            }
        }

        // 2. Try database with appropriate bucket
        if let Some(price_point) = self.get_price_from_db(mint, timestamp).await? {
            return Ok(Some(price_point));
        }

        // 3. Try live Jupiter API
        if let Ok(prices) = self.refresh_prices_from_jupiter(&[mint.to_string()]).await {
            if let Some(price) = prices.first() {
                return Ok(Some(price.clone()));
            }
        }

        // 4. Fallback to VWAP calculation from observed swaps
        if let Some(vwap_price) = self.calculate_vwap_fallback(mint, timestamp).await? {
            return Ok(Some(vwap_price));
        }

        Ok(None)
    }

    /// Get cached price from Redis
    async fn get_cached_price(&self, mint: &str) -> Result<Option<PricePoint>> {
        let cache_key = format!("price:{}:latest", mint);
        if let Ok(Some(price_json)) = self.redis.get::<_, String>(cache_key).await {
            if let Ok(price_point) = serde_json::from_str::<PricePoint>(&price_json) {
                return Ok(Some(price_point));
            }
        }
        Ok(None)
    }

    /// Get price from database with time-based bucketing
    async fn get_price_from_db(
        &self,
        mint: &str,
        timestamp: OffsetDateTime,
    ) -> Result<Option<PricePoint>> {
        // Determine which bucket to use based on age
        let now = OffsetDateTime::now_utc();
        let age_days = (now - timestamp).whole_days();

        let (table_suffix, max_age_minutes) = if age_days <= 7 {
            ("1m", 5) // 1-minute buckets, 5 minute tolerance
        } else if age_days <= 180 {
            ("5m", 15) // 5-minute buckets, 15 minute tolerance
        } else {
            ("1h", 60) // 1-hour buckets, 60 minute tolerance
        };

        // Try materialized view first
        let mv_result = sqlx::query!(
            "SELECT mint, ts, price, source FROM mv_price_{} WHERE mint = $1 AND ts <= $2 AND ts >= $2 - INTERVAL '1 minute' * $3 ORDER BY ABS(EXTRACT(EPOCH FROM (ts - $2))) LIMIT 1",
            table_suffix,
            mint,
            timestamp,
            max_age_minutes
        )
        .fetch_optional(&self.pool)
        .await;

        if let Ok(Some(row)) = mv_result {
            let confidence = PriceConfidence::from_source_and_age(
                &match row.source.as_str() {
                    "jupiter" => PriceSource::Jupiter,
                    "exec_obs" => PriceSource::ExecutionObserved,
                    _ => PriceSource::Fallback,
                },
                (now - row.ts).whole_minutes(),
            );

            return Ok(Some(PricePoint {
                mint: row.mint,
                timestamp: row.ts,
                price: row.price,
                source: match row.source.as_str() {
                    "jupiter" => PriceSource::Jupiter,
                    "exec_obs" => PriceSource::ExecutionObserved,
                    _ => PriceSource::Fallback,
                },
                confidence,
            }));
        }

        // Fallback to raw token_prices table
        let raw_result = sqlx::query!(
            "SELECT mint, ts, price, source FROM token_prices WHERE mint = $1 AND ts <= $2 ORDER BY ts DESC LIMIT 1",
            mint,
            timestamp
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = raw_result {
            let age_minutes = (now - row.ts).whole_minutes();
            if age_minutes <= 1440 {
                // 24 hours max age
                let confidence = PriceConfidence::from_source_and_age(
                    &match row.source.as_str() {
                        "jupiter" => PriceSource::Jupiter,
                        "exec_obs" => PriceSource::ExecutionObserved,
                        _ => PriceSource::Fallback,
                    },
                    age_minutes,
                );

                return Ok(Some(PricePoint {
                    mint: row.mint,
                    timestamp: row.ts,
                    price: row.price,
                    source: match row.source.as_str() {
                        "jupiter" => PriceSource::Jupiter,
                        "exec_obs" => PriceSource::ExecutionObserved,
                        _ => PriceSource::Fallback,
                    },
                    confidence,
                }));
            }
        }

        Ok(None)
    }

    /// Calculate VWAP from observed swap executions as fallback
    async fn calculate_vwap_fallback(
        &self,
        mint: &str,
        around_timestamp: OffsetDateTime,
    ) -> Result<Option<PricePoint>> {
        let window_start = around_timestamp - Duration::hours(1);
        let window_end = around_timestamp + Duration::hours(1);

        let vwap_result = sqlx::query!(
            "SELECT
                SUM(amount_dec * exec_px_usd_dec) / NULLIF(SUM(amount_dec), 0) as vwap_price,
                COUNT(*) as trade_count,
                SUM(amount_dec) as total_volume
            FROM actions
            WHERE mint = $1
                AND ts BETWEEN $2 AND $3
                AND exec_px_usd_dec IS NOT NULL
                AND amount_dec > 0
                AND kind IN ('swap', 'buy', 'sell')",
            mint,
            window_start,
            window_end
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = vwap_result {
            if let (Some(vwap), Some(count), Some(volume)) =
                (row.vwap_price, row.trade_count, row.total_volume)
            {
                if count >= 3 && volume > Decimal::from(100) {
                    // Minimum thresholds for confidence
                    let confidence = if count >= 10 && volume > Decimal::from(1000) {
                        PriceConfidence::Medium
                    } else {
                        PriceConfidence::Low
                    };

                    return Ok(Some(PricePoint {
                        mint: mint.to_string(),
                        timestamp: around_timestamp,
                        price: vwap,
                        source: PriceSource::Vwap,
                        confidence,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Refresh materialized views
    pub async fn refresh_materialized_views(&self) -> Result<()> {
        sqlx::query!("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_price_1m")
            .execute(&self.pool)
            .await?;

        sqlx::query!("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_price_5m")
            .execute(&self.pool)
            .await?;

        sqlx::query!("REFRESH MATERIALIZED VIEW CONCURRENTLY mv_price_1h")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get mints that need price updates
    pub async fn get_mints_needing_updates(&self, max_age_minutes: i64) -> Result<Vec<String>> {
        let cutoff = OffsetDateTime::now_utc() - Duration::minutes(max_age_minutes);

        let mints = sqlx::query!(
            "SELECT DISTINCT mint FROM (
                SELECT DISTINCT mint FROM actions WHERE ts >= $1 AND mint IS NOT NULL
                UNION
                SELECT DISTINCT mint FROM oof_moments WHERE t_event >= $1
            ) AS active_mints
            WHERE mint NOT IN (
                SELECT DISTINCT mint FROM token_prices
                WHERE ts >= $1 AND source = 'jupiter'
            )",
            cutoff
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| row.mint)
        .collect();

        Ok(mints)
    }
}

#[async_trait]
impl PriceProvider for CompositePriceProvider {
    async fn get_price_at(
        &self,
        mint: &str,
        timestamp: OffsetDateTime,
    ) -> Result<Option<PricePoint>> {
        self.get_price_with_fallback(mint, timestamp).await
    }

    async fn get_max_in_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>> {
        let result = sqlx::query!(
            "SELECT
                MIN(price) as min_price,
                MAX(price) as max_price,
                AVG(price) as avg_price,
                (SELECT ts FROM token_prices WHERE mint = $1 AND ts BETWEEN $2 AND $3 AND price = MIN(token_prices.price) LIMIT 1) as min_ts,
                (SELECT ts FROM token_prices WHERE mint = $1 AND ts BETWEEN $2 AND $3 AND price = MAX(token_prices.price) LIMIT 1) as max_ts,
                (SELECT source FROM token_prices WHERE mint = $1 AND ts BETWEEN $2 AND $3 AND price = MAX(token_prices.price) LIMIT 1) as source
            FROM token_prices
            WHERE mint = $1 AND ts BETWEEN $2 AND $3",
            mint,
            from,
            to
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = result {
            if let (Some(min_price), Some(max_price), Some(avg_price), Some(min_ts), Some(max_ts)) = (
                row.min_price,
                row.max_price,
                row.avg_price,
                row.min_ts,
                row.max_ts,
            ) {
                let source = match row.source.as_deref() {
                    Some("jupiter") => PriceSource::Jupiter,
                    Some("exec_obs") => PriceSource::ExecutionObserved,
                    _ => PriceSource::Fallback,
                };

                return Ok(Some(PriceRange {
                    mint: mint.to_string(),
                    from,
                    to,
                    min_price,
                    min_timestamp: min_ts,
                    max_price,
                    max_timestamp: max_ts,
                    avg_price,
                    source,
                    confidence: PriceConfidence::Medium,
                }));
            }
        }

        Ok(None)
    }

    async fn get_min_in_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>> {
        self.get_max_in_range(mint, from, to).await // Same query returns both min and max
    }

    async fn get_price_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>> {
        self.get_max_in_range(mint, from, to).await
    }

    async fn get_candles(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
        bucket: PriceBucket,
    ) -> Result<Vec<Candle>> {
        let interval = match bucket {
            PriceBucket::OneMinute => "1 minute",
            PriceBucket::FiveMinutes => "5 minutes",
            PriceBucket::OneHour => "1 hour",
            PriceBucket::OneDay => "1 day",
        };

        let candles = sqlx::query!(
            "SELECT
                date_trunc($4, ts) as bucket_ts,
                (array_agg(price ORDER BY ts ASC))[1] as open_price,
                MAX(price) as high_price,
                MIN(price) as low_price,
                (array_agg(price ORDER BY ts DESC))[1] as close_price,
                COUNT(*) as sample_count
            FROM token_prices
            WHERE mint = $1 AND ts BETWEEN $2 AND $3
            GROUP BY date_trunc($4, ts)
            ORDER BY bucket_ts ASC",
            mint,
            from,
            to,
            interval
        )
        .fetch_all(&self.pool)
        .await?;

        let mut result = Vec::new();
        for row in candles {
            if let (Some(ts), Some(open), Some(high), Some(low), Some(close)) = (
                row.bucket_ts,
                row.open_price,
                row.high_price,
                row.low_price,
                row.close_price,
            ) {
                result.push(Candle {
                    mint: mint.to_string(),
                    timestamp: ts,
                    open,
                    high,
                    low,
                    close,
                    volume: Decimal::ZERO, // Volume not tracked in current schema
                    source: PriceSource::Jupiter,
                });
            }
        }

        Ok(result)
    }

    async fn get_current_price(&self, mint: &str) -> Result<Option<PricePoint>> {
        self.get_price_with_fallback(mint, OffsetDateTime::now_utc())
            .await
    }

    async fn get_prices_bulk(
        &self,
        mints: &[String],
        timestamp: OffsetDateTime,
    ) -> Result<Vec<PricePoint>> {
        let mut results = Vec::new();

        // Process in batches to avoid query size limits
        for batch in mints.chunks(50) {
            for mint in batch {
                if let Some(price) = self.get_price_with_fallback(mint, timestamp).await? {
                    results.push(price);
                }
            }
        }

        Ok(results)
    }
}

/// Legacy database price provider for backwards compatibility
pub struct DbPriceProvider<'a> {
    pub pg: &'a PgPool,
}

impl<'a> DbPriceProvider<'a> {
    pub fn new(pg: &'a PgPool) -> Self {
        Self { pg }
    }

    pub async fn at_minute(&self, mint: &str, t: OffsetDateTime) -> Option<Decimal> {
        sqlx::query_scalar!(
            "SELECT price FROM token_prices WHERE mint=$1 AND ts<= $2 ORDER BY ts DESC LIMIT 1",
            mint,
            t
        )
        .fetch_optional(self.pg)
        .await
        .ok()
        .flatten()
    }

    pub async fn max_in(
        &self,
        mint: &str,
        t0: OffsetDateTime,
        t1: OffsetDateTime,
    ) -> Option<(OffsetDateTime, Decimal)> {
        sqlx::query!(
            "SELECT ts, price FROM token_prices WHERE mint=$1 AND ts BETWEEN $2 AND $3 ORDER BY price DESC LIMIT 1",
            mint,
            t0,
            t1
        )
        .fetch_optional(self.pg)
        .await
        .ok()
        .flatten()
        .map(|r| (r.ts, r.price))
    }

    pub async fn min_in(
        &self,
        mint: &str,
        t0: OffsetDateTime,
        t1: OffsetDateTime,
    ) -> Option<(OffsetDateTime, Decimal)> {
        sqlx::query!(
            "SELECT ts, price FROM token_prices WHERE mint=$1 AND ts BETWEEN $2 AND $3 ORDER BY price ASC LIMIT 1",
            mint,
            t0,
            t1
        )
        .fetch_optional(self.pg)
        .await
        .ok()
        .flatten()
        .map(|r| (r.ts, r.price))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::datetime;

    #[tokio::test]
    async fn test_vwap_calculation() {
        // This would require a test database setup
        // For now, just test the struct creation
        let provider = CompositePriceProvider::new(
            // Would need a test pool
            todo!(),
            MaybeRedis::Disabled,
            "https://price.jup.ag/v3".to_string(),
        );

        // Test would verify VWAP calculation logic
    }
}
