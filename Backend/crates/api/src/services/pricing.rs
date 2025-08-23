use shared::{
    error::{ApiError, ApiResult},
    types::{
        common::{UsdAmount, TokenAmount},
        price::{PricePoint, PriceHistory, PriceSource},
    },
};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

/// Pricing service for fetching and managing token prices
pub struct PricingService {
    pool: PgPool,
    redis: Option<redis::Client>,
    price_cache: Arc<RwLock<HashMap<String, CachedPrice>>>,
    jupiter_client: reqwest::Client,
    coingecko_client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct CachedPrice {
    price: UsdAmount,
    timestamp: OffsetDateTime,
    source: PriceSource,
}

impl PricingService {
    pub fn new(pool: PgPool, redis: Option<redis::Client>) -> Self {
        Self {
            pool,
            redis,
            price_cache: Arc::new(RwLock::new(HashMap::new())),
            jupiter_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create Jupiter HTTP client"),
            coingecko_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create CoinGecko HTTP client"),
        }
    }

    /// Get current price for a token
    #[instrument(skip(self))]
    pub async fn get_current_price(&self, token_mint: &str) -> ApiResult<UsdAmount> {
        info!("Getting current price for token: {}", token_mint);

        // Check cache first
        if let Some(cached_price) = self.get_cached_price(token_mint).await {
            if self.is_price_fresh(&cached_price) {
                debug!("Using cached price for token: {}", token_mint);
                return Ok(cached_price.price);
            }
        }

        // Try multiple sources in order of preference
        let price = self.fetch_price_from_sources(token_mint).await?;

        // Cache the result
        self.cache_price(token_mint, &price, PriceSource::Jupiter).await;

        Ok(price)
    }

    /// Get multiple token prices in batch
    #[instrument(skip(self))]
    pub async fn get_multiple_prices(&self, token_mints: &[String]) -> ApiResult<HashMap<String, UsdAmount>> {
        info!("Getting prices for {} tokens", token_mints.len());

        let mut prices = HashMap::new();
        let mut uncached_tokens = Vec::new();

        // Check cache for all tokens
        for token_mint in token_mints {
            if let Some(cached_price) = self.get_cached_price(token_mint).await {
                if self.is_price_fresh(&cached_price) {
                    prices.insert(token_mint.clone(), cached_price.price);
                } else {
                    uncached_tokens.push(token_mint.clone());
                }
            } else {
                uncached_tokens.push(token_mint.clone());
            }
        }

        // Fetch uncached prices in batch
        if !uncached_tokens.is_empty() {
            let batch_prices = self.fetch_batch_prices(&uncached_tokens).await?;
            prices.extend(batch_prices);
        }

        Ok(prices)
    }

    /// Get historical prices for a token
    #[instrument(skip(self))]
    pub async fn get_price_history(
        &self,
        token_mint: &str,
        start_time: OffsetDateTime,
        end_time: OffsetDateTime,
        interval: PriceInterval,
    ) -> ApiResult<PriceHistory> {
        info!("Getting price history for token: {} from {} to {}", token_mint, start_time, end_time);

        // Check if we have sufficient data in database
        if let Some(history) = self.get_cached_price_history(token_mint, start_time, end_time, interval).await? {
            return Ok(history);
        }

        // Fetch from external sources and store
        let history = self.fetch_historical_prices(token_mint, start_time, end_time, interval).await?;
        self.store_price_history(&history).await?;

        Ok(history)
    }

    /// Get price at specific timestamp
    #[instrument(skip(self))]
    pub async fn get_price_at_time(&self, token_mint: &str, timestamp: OffsetDateTime) -> ApiResult<UsdAmount> {
        debug!("Getting price for token: {} at time: {}", token_mint, timestamp);

        // Query database for closest price point
        let price_point = sqlx::query_as!(
            PricePoint,
            r#"
            SELECT token_mint, price_usd as "price_usd: UsdAmount", timestamp, source as "source: PriceSource"
            FROM prices
            WHERE token_mint = $1
            AND timestamp <= $2
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
            token_mint, timestamp
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch historical price: {}", e)))?;

        match price_point {
            Some(point) => {
                // Check if price is recent enough (within 1 hour)
                if (timestamp - point.timestamp).abs() < time::Duration::hours(1) {
                    Ok(point.price_usd)
                } else {
                    // Price is too old, try to fetch from external source
                    self.fetch_historical_price_point(token_mint, timestamp).await
                }
            }
            None => {
                // No historical data, try external source
                self.fetch_historical_price_point(token_mint, timestamp).await
            }
        }
    }

    /// Update prices for a list of tokens
    #[instrument(skip(self))]
    pub async fn refresh_prices(&self, token_mints: &[String]) -> ApiResult<u32> {
        info!("Refreshing prices for {} tokens", token_mints.len());

        let mut updated_count = 0;

        // Process in batches to avoid overwhelming APIs
        for batch in token_mints.chunks(50) {
            match self.fetch_batch_prices(batch).await {
                Ok(prices) => {
                    for (token_mint, price) in prices {
                        // Store in database
                        if self.store_current_price(&token_mint, price).await.is_ok() {
                            updated_count += 1;
                        }

                        // Update cache
                        self.cache_price(&token_mint, &price, PriceSource::Jupiter).await;
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch batch prices: {}", e);
                }
            }

            // Rate limiting delay
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        Ok(updated_count)
    }

    /// Get price statistics for a token
    #[instrument(skip(self))]
    pub async fn get_price_statistics(
        &self,
        token_mint: &str,
        period: time::Duration,
    ) -> ApiResult<PriceStatistics> {
        debug!("Getting price statistics for token: {} over period: {:?}", token_mint, period);

        let end_time = OffsetDateTime::now_utc();
        let start_time = end_time - period;

        // Fetch price data from database
        let prices = sqlx::query!(
            "SELECT price_usd, timestamp FROM prices WHERE token_mint = $1 AND timestamp >= $2 ORDER BY timestamp",
            token_mint, start_time
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch price statistics: {}", e)))?;

        if prices.is_empty() {
            return Err(ApiError::NotFound("No price data found for token".to_string()));
        }

        // Calculate statistics
        let price_values: Vec<f64> = prices.iter().map(|p| p.price_usd).collect();

        let min_price = price_values.iter().copied().fold(f64::INFINITY, f64::min);
        let max_price = price_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let avg_price = price_values.iter().sum::<f64>() / price_values.len() as f64;

        // Calculate volatility (standard deviation)
        let variance = price_values.iter()
            .map(|price| {
                let diff = price - avg_price;
                diff * diff
            })
            .sum::<f64>() / price_values.len() as f64;
        let volatility = variance.sqrt();

        // Calculate price change
        let first_price = prices.first().unwrap().price_usd;
        let last_price = prices.last().unwrap().price_usd;
        let change_amount = last_price - first_price;
        let change_pct = (change_amount / first_price) * 100.0;

        Ok(PriceStatistics {
            token_mint: token_mint.to_string(),
            period_start: start_time,
            period_end: end_time,
            min_price: UsdAmount::from(min_price),
            max_price: UsdAmount::from(max_price),
            avg_price: UsdAmount::from(avg_price),
            current_price: UsdAmount::from(last_price),
            change_amount: UsdAmount::from(change_amount),
            change_percentage: change_pct,
            volatility,
            data_points: prices.len() as u32,
        })
    }

    // Private helper methods

    async fn get_cached_price(&self, token_mint: &str) -> Option<CachedPrice> {
        let cache = self.price_cache.read().await;
        cache.get(token_mint).cloned()
    }

    fn is_price_fresh(&self, cached_price: &CachedPrice) -> bool {
        let age = OffsetDateTime::now_utc() - cached_price.timestamp;
        age < time::Duration::minutes(1) // Consider prices fresh for 1 minute
    }

    async fn cache_price(&self, token_mint: &str, price: &UsdAmount, source: PriceSource) {
        let mut cache = self.price_cache.write().await;
        cache.insert(token_mint.to_string(), CachedPrice {
            price: *price,
            timestamp: OffsetDateTime::now_utc(),
            source,
        });
    }

    async fn fetch_price_from_sources(&self, token_mint: &str) -> ApiResult<UsdAmount> {
        // Try Jupiter first
        if let Ok(price) = self.fetch_jupiter_price(token_mint).await {
            return Ok(price);
        }

        // Try CoinGecko as fallback
        if let Ok(price) = self.fetch_coingecko_price(token_mint).await {
            return Ok(price);
        }

        // Try database as last resort
        self.get_last_known_price(token_mint).await
    }

    async fn fetch_jupiter_price(&self, token_mint: &str) -> ApiResult<UsdAmount> {
        debug!("Fetching price from Jupiter for token: {}", token_mint);

        let url = format!("https://price.jup.ag/v4/price?ids={}", token_mint);
        let response: serde_json::Value = self.jupiter_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Jupiter".to_string(),
                error: e.to_string()
            })?
            .json()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Jupiter".to_string(),
                error: e.to_string()
            })?;

        let price = response
            .get("data")
            .and_then(|data| data.get(token_mint))
            .and_then(|token_data| token_data.get("price"))
            .and_then(|price| price.as_f64())
            .ok_or_else(|| ApiError::ExternalService {
                service: "Jupiter".to_string(),
                error: "Invalid price response format".to_string()
            })?;

        Ok(UsdAmount::from(price))
    }

    async fn fetch_coingecko_price(&self, token_mint: &str) -> ApiResult<UsdAmount> {
        debug!("Fetching price from CoinGecko for token: {}", token_mint);

        // This would require mapping Solana mints to CoinGecko IDs
        // For now, return an error to indicate unavailability
        Err(ApiError::ExternalService {
            service: "CoinGecko".to_string(),
            error: "CoinGecko integration not implemented".to_string()
        })
    }

    async fn get_last_known_price(&self, token_mint: &str) -> ApiResult<UsdAmount> {
        debug!("Getting last known price from database for token: {}", token_mint);

        let price = sqlx::query_scalar!(
            "SELECT price_usd FROM prices WHERE token_mint = $1 ORDER BY timestamp DESC LIMIT 1",
            token_mint
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch last known price: {}", e)))?;

        price
            .map(UsdAmount::from)
            .ok_or_else(|| ApiError::NotFound("No price data available for token".to_string()))
    }

    async fn fetch_batch_prices(&self, token_mints: &[String]) -> ApiResult<HashMap<String, UsdAmount>> {
        debug!("Fetching batch prices for {} tokens", token_mints.len());

        let ids = token_mints.join(",");
        let url = format!("https://price.jup.ag/v4/price?ids={}", ids);

        let response: serde_json::Value = self.jupiter_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Jupiter".to_string(),
                error: e.to_string()
            })?
            .json()
            .await
            .map_err(|e| ApiError::ExternalService {
                service: "Jupiter".to_string(),
                error: e.to_string()
            })?;

        let mut prices = HashMap::new();

        if let Some(data) = response.get("data").and_then(|d| d.as_object()) {
            for (token_mint, token_data) in data {
                if let Some(price) = token_data.get("price").and_then(|p| p.as_f64()) {
                    prices.insert(token_mint.clone(), UsdAmount::from(price));
                }
            }
        }

        Ok(prices)
    }

    async fn store_current_price(&self, token_mint: &str, price: UsdAmount) -> ApiResult<()> {
        sqlx::query!(
            "INSERT INTO prices (token_mint, price_usd, timestamp, source) VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
            token_mint, price.0, OffsetDateTime::now_utc(), PriceSource::Jupiter as _
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to store price: {}", e)))?;

        Ok(())
    }

    // Additional helper methods would be implemented here for:
    // - get_cached_price_history
    // - fetch_historical_prices
    // - store_price_history
    // - fetch_historical_price_point
}

// Supporting types
#[derive(Debug, Clone)]
pub enum PriceInterval {
    OneMinute,
    FiveMinutes,
    FifteenMinutes,
    OneHour,
    FourHours,
    OneDay,
}

#[derive(Debug, Clone)]
pub struct PriceStatistics {
    pub token_mint: String,
    pub period_start: OffsetDateTime,
    pub period_end: OffsetDateTime,
    pub min_price: UsdAmount,
    pub max_price: UsdAmount,
    pub avg_price: UsdAmount,
    pub current_price: UsdAmount,
    pub change_amount: UsdAmount,
    pub change_percentage: f64,
    pub volatility: f64,
    pub data_points: u32,
}
