use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use async_trait::async_trait;
use anyhow::Result;

/// Price point with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricePoint {
    pub mint: String,
    pub timestamp: OffsetDateTime,
    pub price: Decimal,
    pub source: PriceSource,
    pub confidence: PriceConfidence,
}

/// Price source information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PriceSource {
    #[serde(rename = "jupiter")]
    Jupiter,
    #[serde(rename = "pyth")]
    Pyth,
    #[serde(rename = "exec_obs")]
    ExecutionObserved,
    #[serde(rename = "vwap")]
    Vwap,
    #[serde(rename = "fallback")]
    Fallback,
}

impl PriceSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriceSource::Jupiter => "jupiter",
            PriceSource::Pyth => "pyth",
            PriceSource::ExecutionObserved => "exec_obs",
            PriceSource::Vwap => "vwap",
            PriceSource::Fallback => "fallback",
        }
    }

    /// Returns the reliability score of this price source (0.0 to 1.0)
    pub fn reliability(&self) -> f32 {
        match self {
            PriceSource::Jupiter => 0.95,
            PriceSource::Pyth => 0.90,
            PriceSource::ExecutionObserved => 0.85,
            PriceSource::Vwap => 0.70,
            PriceSource::Fallback => 0.50,
        }
    }
}

/// Price confidence level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PriceConfidence {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
    #[serde(rename = "very_low")]
    VeryLow,
}

impl PriceConfidence {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriceConfidence::High => "high",
            PriceConfidence::Medium => "medium",
            PriceConfidence::Low => "low",
            PriceConfidence::VeryLow => "very_low",
        }
    }

    pub fn from_source_and_age(source: &PriceSource, age_minutes: i64) -> Self {
        match (source, age_minutes) {
            (PriceSource::Jupiter | PriceSource::Pyth, 0..=5) => PriceConfidence::High,
            (PriceSource::Jupiter | PriceSource::Pyth, 6..=30) => PriceConfidence::Medium,
            (PriceSource::ExecutionObserved, 0..=60) => PriceConfidence::Medium,
            (PriceSource::Vwap, 0..=120) => PriceConfidence::Low,
            _ => PriceConfidence::VeryLow,
        }
    }
}

/// Price bucket timeframe
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PriceBucket {
    #[serde(rename = "1m")]
    OneMinute,
    #[serde(rename = "5m")]
    FiveMinutes,
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "1d")]
    OneDay,
}

impl PriceBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            PriceBucket::OneMinute => "1m",
            PriceBucket::FiveMinutes => "5m",
            PriceBucket::OneHour => "1h",
            PriceBucket::OneDay => "1d",
        }
    }

    /// Get the appropriate bucket for a given time range
    pub fn for_time_range(from: OffsetDateTime, to: OffsetDateTime) -> Self {
        let duration = to - from;
        let days = duration.whole_days();

        if days <= 7 {
            PriceBucket::OneMinute
        } else if days <= 180 {
            PriceBucket::FiveMinutes
        } else {
            PriceBucket::OneHour
        }
    }

    /// Get the time duration for this bucket
    pub fn duration(&self) -> time::Duration {
        match self {
            PriceBucket::OneMinute => time::Duration::minutes(1),
            PriceBucket::FiveMinutes => time::Duration::minutes(5),
            PriceBucket::OneHour => time::Duration::hours(1),
            PriceBucket::OneDay => time::Duration::days(1),
        }
    }
}

/// Price range query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceRange {
    pub mint: String,
    pub from: OffsetDateTime,
    pub to: OffsetDateTime,
    pub min_price: Decimal,
    pub min_timestamp: OffsetDateTime,
    pub max_price: Decimal,
    pub max_timestamp: OffsetDateTime,
    pub avg_price: Decimal,
    pub source: PriceSource,
    pub confidence: PriceConfidence,
}

/// OHLCV candle data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candle {
    pub mint: String,
    pub timestamp: OffsetDateTime,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub source: PriceSource,
}

/// Price provider trait for different data sources
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Get price at a specific timestamp (nearest available)
    async fn get_price_at(
        &self,
        mint: &str,
        timestamp: OffsetDateTime,
    ) -> Result<Option<PricePoint>>;

    /// Get maximum price in a time range
    async fn get_max_in_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>>;

    /// Get minimum price in a time range
    async fn get_min_in_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>>;

    /// Get price range data
    async fn get_price_range(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
    ) -> Result<Option<PriceRange>>;

    /// Get candle data for a time range
    async fn get_candles(
        &self,
        mint: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
        bucket: PriceBucket,
    ) -> Result<Vec<Candle>>;

    /// Get current price (most recent available)
    async fn get_current_price(&self, mint: &str) -> Result<Option<PricePoint>>;

    /// Get prices for multiple mints at once
    async fn get_prices_bulk(
        &self,
        mints: &[String],
        timestamp: OffsetDateTime,
    ) -> Result<Vec<PricePoint>>;
}

/// Jupiter API response structures
#[derive(Debug, Deserialize)]
pub struct JupiterPriceResponse {
    pub data: std::collections::HashMap<String, JupiterTokenPrice>,
}

#[derive(Debug, Deserialize)]
pub struct JupiterTokenPrice {
    pub id: String,
    pub price: String,
    #[serde(rename = "vsToken")]
    pub vs_token: String,
    #[serde(rename = "vsTokenSymbol")]
    pub vs_token_symbol: String,
    pub timestamp: i64,
}

/// Pyth price feed data
#[derive(Debug, Deserialize)]
pub struct PythPriceData {
    pub price: String,
    pub conf: String,
    pub timestamp: i64,
    pub status: String,
}

/// Token metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
}

/// Price movement detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceMovement {
    pub mint: String,
    pub from_timestamp: OffsetDateTime,
    pub to_timestamp: OffsetDateTime,
    pub from_price: Decimal,
    pub to_price: Decimal,
    pub change_pct: Decimal,
    pub change_usd: Decimal,
    pub direction: MovementDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MovementDirection {
    Up,
    Down,
    Flat,
}

impl PriceMovement {
    pub fn new(
        mint: String,
        from_timestamp: OffsetDateTime,
        to_timestamp: OffsetDateTime,
        from_price: Decimal,
        to_price: Decimal,
    ) -> Self {
        let change_usd = to_price - from_price;
        let change_pct = if from_price != Decimal::ZERO {
            change_usd / from_price
        } else {
            Decimal::ZERO
        };

        let direction = if change_usd > Decimal::ZERO {
            MovementDirection::Up
        } else if change_usd < Decimal::ZERO {
            MovementDirection::Down
        } else {
            MovementDirection::Flat
        };

        Self {
            mint,
            from_timestamp,
            to_timestamp,
            from_price,
            to_price,
            change_pct,
            change_usd,
            direction,
        }
    }
}
