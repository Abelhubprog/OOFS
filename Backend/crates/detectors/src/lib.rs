use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::{ChainEvent, Moment, MomentContext, MomentKind, PriceProvider};
use time::{Duration, OffsetDateTime};

pub mod position;
pub mod prices;

/// Context for detector processing
#[derive(Clone)]
pub struct DetectorContext {
    pub pool: sqlx::PgPool,
    pub price_provider: std::sync::Arc<dyn PriceProvider + Send + Sync>,
    pub redis: shared::MaybeRedis,
}

/// Trait for OOF moment detectors
#[async_trait]
pub trait Detector: Send + Sync {
    fn name(&self) -> &'static str;
    fn version(&self) -> u8;

    /// Process a chain event and optionally emit a moment
    async fn process(
        &self,
        event: &ChainEvent,
        context: &DetectorContext,
    ) -> Result<Option<Moment>>;

    /// Check if this detector should process the given event
    fn should_process(&self, event: &ChainEvent) -> bool;
}

/// S2E (Sold Too Early) detector
pub struct SoldTooEarlyDetector {
    min_percentage: Decimal,
    min_usd_amount: Decimal,
    window_days: i64,
}

impl Default for SoldTooEarlyDetector {
    fn default() -> Self {
        Self {
            min_percentage: shared::constants::thresholds::s2e_min_pct(),
            min_usd_amount: shared::constants::thresholds::s2e_min_usd(),
            window_days: 7,
        }
    }
}

#[async_trait]
impl Detector for SoldTooEarlyDetector {
    fn name(&self) -> &'static str {
        "S2E"
    }

    fn version(&self) -> u8 {
        1
    }

    fn should_process(&self, event: &ChainEvent) -> bool {
        matches!(event.kind, shared::EventKind::Sell) && event.mint.is_some()
    }

    async fn process(
        &self,
        event: &ChainEvent,
        context: &DetectorContext,
    ) -> Result<Option<Moment>> {
        let mint = match &event.mint {
            Some(m) => m,
            None => return Ok(None),
        };

        let (qty_sold, exit_price) = match (event.amount, event.price_usd) {
            (Some(q), Some(p)) => (q, p),
            _ => return Ok(None),
        };

        // Look for peak price in the next 7 days
        let window_start = event.timestamp;
        let window_end = event.timestamp + Duration::days(self.window_days);

        let price_range = context
            .price_provider
            .get_max_in_range(mint, window_start, window_end)
            .await?;

        if let Some(range) = price_range {
            let peak_price = range.max_price;
            let missed_pct = (peak_price - exit_price) / exit_price;
            let missed_usd = qty_sold * (peak_price - exit_price);

            // Check thresholds
            if missed_pct >= self.min_percentage && missed_usd >= self.min_usd_amount {
                let severity = shared::constants::severity::s2e_severity(missed_pct);

                let mut moment = Moment::new(
                    event.wallet.clone(),
                    Some(mint.clone()),
                    MomentKind::SoldTooEarly,
                    event.timestamp,
                );

                moment.window = Some(Duration::days(self.window_days));
                moment.pct_dec = Some(missed_pct);
                moment.missed_usd_dec = Some(missed_usd);
                moment.severity_dec = Some(severity);
                moment.sig_ref = Some(event.signature.clone());
                moment.slot_ref = Some(event.slot);
                moment.version = self.version().to_string();

                moment.explain_json = serde_json::json!({
                    "exit_price": exit_price,
                    "peak_price": peak_price,
                    "peak_timestamp": range.max_timestamp,
                    "qty_sold": qty_sold,
                    "window_days": self.window_days,
                    "price_source": range.source,
                    "confidence": range.confidence
                });

                return Ok(Some(moment));
            }
        }

        Ok(None)
    }
}

/// BHD (Bag Holder Drawdown) detector
pub struct BagHolderDrawdownDetector {
    min_drawdown_pct: Decimal,
    window_days: i64,
}

impl Default for BagHolderDrawdownDetector {
    fn default() -> Self {
        Self {
            min_drawdown_pct: shared::constants::thresholds::bhd_min_pct(),
            window_days: 7,
        }
    }
}

#[async_trait]
impl Detector for BagHolderDrawdownDetector {
    fn name(&self) -> &'static str {
        "BHD"
    }

    fn version(&self) -> u8 {
        1
    }

    fn should_process(&self, event: &ChainEvent) -> bool {
        matches!(event.kind, shared::EventKind::Buy) && event.mint.is_some()
    }

    async fn process(
        &self,
        event: &ChainEvent,
        context: &DetectorContext,
    ) -> Result<Option<Moment>> {
        let mint = match &event.mint {
            Some(m) => m,
            None => return Ok(None),
        };

        let (qty_bought, entry_price) = match (event.amount, event.price_usd) {
            (Some(q), Some(p)) => (q, p),
            _ => return Ok(None),
        };

        // Look for trough price in the next 7 days
        let window_start = event.timestamp;
        let window_end = event.timestamp + Duration::days(self.window_days);

        let price_range = context
            .price_provider
            .get_min_in_range(mint, window_start, window_end)
            .await?;

        if let Some(range) = price_range {
            let trough_price = range.min_price;
            let drawdown_pct = (trough_price - entry_price) / entry_price;

            // Check if drawdown is significant enough
            if drawdown_pct <= self.min_drawdown_pct {
                let severity = shared::constants::severity::bhd_severity(drawdown_pct);
                let unrealized_loss = qty_bought * (trough_price - entry_price);

                let mut moment = Moment::new(
                    event.wallet.clone(),
                    Some(mint.clone()),
                    MomentKind::BagHolderDrawdown,
                    event.timestamp,
                );

                moment.window = Some(Duration::days(self.window_days));
                moment.pct_dec = Some(drawdown_pct);
                moment.missed_usd_dec = Some(unrealized_loss.abs());
                moment.severity_dec = Some(severity);
                moment.sig_ref = Some(event.signature.clone());
                moment.slot_ref = Some(event.slot);
                moment.version = self.version().to_string();

                moment.explain_json = serde_json::json!({
                    "entry_price": entry_price,
                    "trough_price": trough_price,
                    "trough_timestamp": range.min_timestamp,
                    "qty_bought": qty_bought,
                    "drawdown_pct": drawdown_pct,
                    "unrealized_loss_usd": unrealized_loss,
                    "window_days": self.window_days,
                    "price_source": range.source,
                    "confidence": range.confidence
                });

                return Ok(Some(moment));
            }
        }

        Ok(None)
    }
}

/// Bad Route detector
pub struct BadRouteDetector {
    min_worse_pct: Decimal,
}

impl Default for BadRouteDetector {
    fn default() -> Self {
        Self {
            min_worse_pct: shared::constants::thresholds::bad_route_min_pct(),
        }
    }
}

#[async_trait]
impl Detector for BadRouteDetector {
    fn name(&self) -> &'static str {
        "BAD_ROUTE"
    }

    fn version(&self) -> u8 {
        1
    }

    fn should_process(&self, event: &ChainEvent) -> bool {
        matches!(event.kind, shared::EventKind::Swap) && event.mint.is_some()
    }

    async fn process(
        &self,
        event: &ChainEvent,
        context: &DetectorContext,
    ) -> Result<Option<Moment>> {
        let mint = match &event.mint {
            Some(m) => m,
            None => return Ok(None),
        };

        let executed_price = match event.price_usd {
            Some(p) => p,
            None => return Ok(None),
        };

        // Get the best available price at the time of execution
        // We use a 1-minute window around the execution time
        let window_start = event.timestamp - Duration::minutes(1);
        let window_end = event.timestamp + Duration::minutes(1);

        let best_price_point = context
            .price_provider
            .get_price_at(mint, event.timestamp)
            .await?;

        if let Some(best_price_info) = best_price_point {
            let best_price = best_price_info.price;
            let worse_pct = (executed_price - best_price) / best_price;

            if worse_pct >= self.min_worse_pct {
                let amount_lost =
                    event.amount.unwrap_or(Decimal::ZERO) * (executed_price - best_price);
                let severity = shared::constants::severity::bad_route_severity(worse_pct);

                let mut moment = Moment::new(
                    event.wallet.clone(),
                    Some(mint.clone()),
                    MomentKind::BadRoute,
                    event.timestamp,
                );

                moment.pct_dec = Some(worse_pct);
                moment.missed_usd_dec = Some(amount_lost);
                moment.severity_dec = Some(severity);
                moment.sig_ref = Some(event.signature.clone());
                moment.slot_ref = Some(event.slot);
                moment.version = self.version().to_string();

                moment.explain_json = serde_json::json!({
                    "executed_price": executed_price,
                    "best_available_price": best_price,
                    "price_difference_pct": worse_pct,
                    "amount_lost_usd": amount_lost,
                    "route_used": event.route,
                    "execution_time": event.timestamp,
                    "price_source": best_price_info.source,
                    "confidence": best_price_info.confidence
                });

                return Ok(Some(moment));
            }
        }

        Ok(None)
    }
}

/// Idle Yield detector
pub struct IdleYieldDetector {
    min_missed_usd: Decimal,
    oof_token_mint: String,
    annual_yield_rate: Decimal,
}

impl IdleYieldDetector {
    pub fn new(oof_token_mint: String) -> Self {
        Self {
            min_missed_usd: shared::constants::thresholds::idle_min_usd(),
            oof_token_mint,
            annual_yield_rate: shared::constants::thresholds::oof_apr(),
        }
    }
}

#[async_trait]
impl Detector for IdleYieldDetector {
    fn name(&self) -> &'static str {
        "IDLE_YIELD"
    }

    fn version(&self) -> u8 {
        1
    }

    fn should_process(&self, event: &ChainEvent) -> bool {
        // Process any event involving OOF token
        event.mint.as_ref() == Some(&self.oof_token_mint)
    }

    async fn process(
        &self,
        event: &ChainEvent,
        context: &DetectorContext,
    ) -> Result<Option<Moment>> {
        // For idle yield detection, we need to analyze periods of inactivity
        // This is more complex and would typically be triggered by a periodic job
        // rather than individual events. For now, we'll implement a simplified version.

        // Check if wallet has been holding OOF tokens for an extended period
        let lookback_days = 30;
        let lookback_start = event.timestamp - Duration::days(lookback_days);

        // Get the average balance over the period
        let avg_balance = self
            .calculate_average_oof_balance(&event.wallet, lookback_start, event.timestamp, context)
            .await?;

        if avg_balance > Decimal::ZERO {
            // Get average OOF price over the period
            let avg_price = self
                .calculate_average_oof_price(lookback_start, event.timestamp, context)
                .await?;

            if let Some(price) = avg_price {
                let days_idle = Decimal::from(lookback_days);
                let missed_yield_tokens =
                    avg_balance * self.annual_yield_rate * days_idle / Decimal::from(365);
                let missed_yield_usd = missed_yield_tokens * price;

                if missed_yield_usd >= self.min_missed_usd {
                    let mut moment = Moment::new(
                        event.wallet.clone(),
                        Some(self.oof_token_mint.clone()),
                        MomentKind::IdleYield,
                        event.timestamp,
                    );

                    moment.missed_usd_dec = Some(missed_yield_usd);
                    moment.sig_ref = Some(event.signature.clone());
                    moment.slot_ref = Some(event.slot);
                    moment.version = self.version().to_string();

                    moment.explain_json = serde_json::json!({
                        "avg_balance": avg_balance,
                        "idle_days": lookback_days,
                        "apr_rate": self.annual_yield_rate,
                        "missed_yield_tokens": missed_yield_tokens,
                        "missed_yield_usd": missed_yield_usd,
                        "avg_token_price": price,
                        "lookback_period": {
                            "start": lookback_start,
                            "end": event.timestamp
                        }
                    });

                    return Ok(Some(moment));
                }
            }
        }

        Ok(None)
    }
}

impl IdleYieldDetector {
    async fn calculate_average_oof_balance(
        &self,
        wallet: &str,
        from: OffsetDateTime,
        to: OffsetDateTime,
        context: &DetectorContext,
    ) -> Result<Decimal> {
        // This would require tracking balance changes over time
        // For now, return a placeholder
        Ok(Decimal::ZERO)
    }

    async fn calculate_average_oof_price(
        &self,
        from: OffsetDateTime,
        to: OffsetDateTime,
        context: &DetectorContext,
    ) -> Result<Option<Decimal>> {
        // Get average price from price provider
        if let Some(range) = context
            .price_provider
            .get_price_range(&self.oof_token_mint, from, to)
            .await?
        {
            Ok(Some(range.avg_price))
        } else {
            Ok(None)
        }
    }
}

/// Rug detector (optional, more complex)
pub struct RugDetector {
    min_collapse_pct: Decimal,
    persistence_days: i64,
}

impl Default for RugDetector {
    fn default() -> Self {
        Self {
            min_collapse_pct: Decimal::from_str_exact("-0.80").unwrap(), // 80% drop
            persistence_days: 7,
        }
    }
}

#[async_trait]
impl Detector for RugDetector {
    fn name(&self) -> &'static str {
        "RUG"
    }

    fn version(&self) -> u8 {
        1
    }

    fn should_process(&self, event: &ChainEvent) -> bool {
        // This detector would typically be triggered by price analysis jobs
        // rather than individual transaction events
        false // Disabled for now
    }

    async fn process(
        &self,
        _event: &ChainEvent,
        _context: &DetectorContext,
    ) -> Result<Option<Moment>> {
        // Rug detection would analyze:
        // 1. Token ATH vs current price collapse
        // 2. Liquidity removal patterns
        // 3. Developer token movements
        // 4. Holder distribution changes
        // This is complex and would be implemented as a separate analysis job
        Ok(None)
    }
}

/// Detector registry and processing engine
pub struct DetectorEngine {
    detectors: Vec<Box<dyn Detector>>,
    context: DetectorContext,
}

impl DetectorEngine {
    pub fn new(context: DetectorContext) -> Self {
        let mut detectors: Vec<Box<dyn Detector>> = vec![
            Box::new(SoldTooEarlyDetector::default()),
            Box::new(BagHolderDrawdownDetector::default()),
            Box::new(BadRouteDetector::default()),
        ];

        // Add idle yield detector if OOF token mint is configured
        if let Ok(oof_mint) = std::env::var("OOF_TOKEN_MINT") {
            detectors.push(Box::new(IdleYieldDetector::new(oof_mint)));
        }

        // Rug detector is disabled by default
        // detectors.push(Box::new(RugDetector::default()));

        Self { detectors, context }
    }

    /// Process a chain event through all applicable detectors
    pub async fn process_event(&self, event: &ChainEvent) -> Result<Vec<Moment>> {
        let mut moments = Vec::new();

        for detector in &self.detectors {
            if detector.should_process(event) {
                if let Some(moment) = detector.process(event, &self.context).await? {
                    // Store moment in database
                    self.persist_moment(&moment).await?;

                    // Publish to SSE/WebSocket
                    self.publish_moment(&moment).await?;

                    moments.push(moment);
                }
            }
        }

        Ok(moments)
    }

    /// Persist moment to database
    async fn persist_moment(&self, moment: &Moment) -> Result<()> {
        sqlx::query(include_str!("../../../db/queries/insert_moment.sql"))
            .bind(&moment.id)
            .bind(&moment.wallet)
            .bind(&moment.mint)
            .bind(moment.kind.as_str())
            .bind(moment.t_event)
            .bind(moment.window.map(|d| d.whole_seconds() as i32))
            .bind(moment.pct_dec)
            .bind(moment.missed_usd_dec)
            .bind(moment.severity_dec)
            .bind(&moment.sig_ref)
            .bind(moment.slot_ref)
            .bind(&moment.version)
            .bind(&moment.explain_json)
            .bind(&moment.preview_png_url)
            .execute(&self.context.pool)
            .await?;

        Ok(())
    }

    /// Publish moment to Redis for SSE/WebSocket distribution
    async fn publish_moment(&self, moment: &Moment) -> Result<()> {
        let moment_json = serde_json::to_string(moment)?;

        // Publish to general moment stream
        let _ = self
            .context
            .redis
            .publish("new_oof_moment", &moment_json)
            .await;

        // Publish to wallet-specific stream
        let wallet_channel = format!("wallet_moments:{}", moment.wallet);
        let _ = self
            .context
            .redis
            .publish(wallet_channel, &moment_json)
            .await;

        Ok(())
    }

    /// Get list of active detectors
    pub fn get_detector_info(&self) -> Vec<(String, u8)> {
        self.detectors
            .iter()
            .map(|d| (d.name().to_string(), d.version()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::datetime;

    #[test]
    fn test_detector_creation() {
        let s2e = SoldTooEarlyDetector::default();
        assert_eq!(s2e.name(), "S2E");
        assert_eq!(s2e.version(), 1);

        let bhd = BagHolderDrawdownDetector::default();
        assert_eq!(bhd.name(), "BHD");
        assert_eq!(bhd.version(), 1);
    }

    #[test]
    fn test_event_filtering() {
        let s2e = SoldTooEarlyDetector::default();

        let sell_event = ChainEvent {
            id: "test".to_string(),
            signature: "sig".to_string(),
            log_idx: 0,
            slot: 1000,
            timestamp: datetime!(2024-01-01 0:00 UTC),
            wallet: "wallet".to_string(),
            mint: Some("mint".to_string()),
            program_id: "program".to_string(),
            kind: shared::EventKind::Sell,
            amount: Some(dec!(100)),
            price_usd: Some(dec!(1.5)),
            route: None,
            metadata: serde_json::json!({}),
        };

        assert!(s2e.should_process(&sell_event));

        let mut buy_event = sell_event.clone();
        buy_event.kind = shared::EventKind::Buy;
        assert!(!s2e.should_process(&buy_event));
    }
}
