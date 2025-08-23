#[cfg(test)]
mod tests {
    use super::*;
    use mockall::predicate::*;
    use rust_decimal::Decimal;
    use shared::types::{
        moment::{Moment, MomentKind},
        price::{PricePoint, PriceSource},
    };
    use std::collections::HashMap;
    use time::OffsetDateTime;

    // Mock structures for testing
    struct MockPriceProvider {
        prices: HashMap<(String, OffsetDateTime), Decimal>,
    }

    impl MockPriceProvider {
        fn new() -> Self {
            Self {
                prices: HashMap::new(),
            }
        }

        fn add_price(&mut self, mint: &str, timestamp: OffsetDateTime, price: Decimal) {
            self.prices.insert((mint.to_string(), timestamp), price);
        }
    }

    #[async_trait::async_trait]
    impl PriceProvider for MockPriceProvider {
        async fn get_price_at(&self, mint: &str, timestamp: OffsetDateTime) -> Result<Option<Decimal>, anyhow::Error> {
            Ok(self.prices.get(&(mint.to_string(), timestamp)).copied())
        }

        async fn get_price_range(&self, _mint: &str, _start: OffsetDateTime, _end: OffsetDateTime) -> Result<Vec<PricePoint>, anyhow::Error> {
            // Not used in S2E detector tests
            Ok(vec![])
        }
    }

    fn create_test_trade(
        wallet: &str,
        mint: &str,
        quantity: Decimal,
        sale_price: Decimal,
        sale_time: OffsetDateTime,
        acquisition_sig: &str,
    ) -> RealizedTrade {
        RealizedTrade {
            id: ulid::Ulid::new().to_string(),
            wallet: wallet.to_string(),
            mint: mint.to_string(),
            quantity_dec: quantity,
            cost_basis_usd_dec: quantity * Decimal::new(1, 0), // $1.00 cost basis
            sale_proceeds_usd_dec: quantity * sale_price,
            realized_pnl_usd_dec: quantity * (sale_price - Decimal::new(1, 0)),
            avg_cost_per_token: Decimal::new(1, 0), // $1.00
            sale_price_per_token: sale_price,
            acquisition_time: sale_time - time::Duration::hours(24), // Bought 24 hours ago
            sale_time,
            acquisition_sig: acquisition_sig.to_string(),
            sale_sig: format!("sell_sig_{}", ulid::Ulid::new()),
            episode_id: Some("episode_1".to_string()),
        }
    }

    #[tokio::test]
    async fn test_s2e_detector_basic_scenario() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: User sold at $1.50, price went to $2.00 within 7 days
        let sale_price = Decimal::new(150, 2); // $1.50
        let peak_price = Decimal::new(200, 2); // $2.00
        let peak_time = sale_time + time::Duration::days(3);

        price_provider.add_price(mint, peak_time, peak_price);

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        // Mock price provider to return increasing prices
        for i in 1..=7 {
            let day_price = sale_price + Decimal::new(i * 10, 2); // Increasing by $0.10 per day
            let day_time = sale_time + time::Duration::days(i);
            price_provider.add_price(mint, day_time, day_price);
        }

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        assert!(result.is_some());
        let moment = result.unwrap();

        assert_eq!(moment.kind, MomentKind::SoldTooEarly);
        assert_eq!(moment.wallet, wallet);
        assert_eq!(moment.mint, Some(mint.to_string()));

        // Should have detected the missed gains
        assert!(moment.pct_dec.is_some());
        assert!(moment.missed_usd_dec.is_some());

        // Verify the percentage calculation
        // Sold at $1.50, peak at $2.20, so missed 46.67% gain
        let expected_pct = ((Decimal::new(220, 2) - sale_price) / sale_price) * Decimal::new(100, 0);
        assert!((moment.pct_dec.unwrap() - expected_pct).abs() < Decimal::new(1, 0)); // Within 1%

        // Verify USD amount: 1000 tokens * ($2.20 - $1.50) = $700
        let expected_usd = Decimal::new(1000, 0) * (Decimal::new(220, 2) - sale_price);
        assert!((moment.missed_usd_dec.unwrap() - expected_usd).abs() < Decimal::new(100, 2)); // Within $1
    }

    #[tokio::test]
    async fn test_s2e_detector_no_significant_gain() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: User sold at $1.50, price only went to $1.60 (6.67% gain, below 20% threshold)
        let sale_price = Decimal::new(150, 2); // $1.50

        // Add prices that don't meet the threshold
        for i in 1..=7 {
            let day_price = sale_price + Decimal::new(i, 2); // Increasing by $0.01 per day
            let day_time = sale_time + time::Duration::days(i);
            price_provider.add_price(mint, day_time, day_price);
        }

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should not detect S2E moment because gain is below threshold
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_s2e_detector_insufficient_usd_amount() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 0), // $100.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: High percentage gain but low USD amount
        let sale_price = Decimal::new(100, 2); // $1.00
        let peak_price = Decimal::new(200, 2); // $2.00 (100% gain)

        // Add the peak price
        let peak_time = sale_time + time::Duration::days(3);
        price_provider.add_price(mint, peak_time, peak_price);

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(50, 0), // Only 50 tokens, so missed gain = 50 * $1.00 = $50 (below $100 threshold)
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should not detect S2E moment because USD amount is below threshold
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_s2e_detector_peak_outside_window() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: Price peaks outside the 7-day window
        let sale_price = Decimal::new(150, 2); // $1.50

        // Add prices within window (no significant gain)
        for i in 1..=7 {
            let day_price = sale_price + Decimal::new(i, 2); // Small increases
            let day_time = sale_time + time::Duration::days(i);
            price_provider.add_price(mint, day_time, day_price);
        }

        // Add peak price outside window (day 10)
        let peak_price = Decimal::new(300, 2); // $3.00
        let peak_time = sale_time + time::Duration::days(10);
        price_provider.add_price(mint, peak_time, peak_price);

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should not detect S2E moment because peak is outside the time window
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_s2e_detector_no_price_data() {
        let price_provider = MockPriceProvider::new(); // Empty price provider
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();
        let sale_price = Decimal::new(150, 2); // $1.50

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should not detect S2E moment because there's no price data
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_s2e_detector_price_decline() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: Price declines after sale (good timing, not S2E)
        let sale_price = Decimal::new(150, 2); // $1.50

        // Add declining prices
        for i in 1..=7 {
            let day_price = sale_price - Decimal::new(i * 5, 2); // Declining by $0.05 per day
            let day_time = sale_time + time::Duration::days(i);
            price_provider.add_price(mint, day_time, day_price);
        }

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should not detect S2E moment because price declined (good sale timing)
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_s2e_detector_exact_threshold() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 0), // $100.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: Exactly 20% gain and exactly $100 missed
        let sale_price = Decimal::new(100, 2); // $1.00
        let peak_price = Decimal::new(120, 2); // $1.20 (exactly 20% gain)

        let peak_time = sale_time + time::Duration::days(3);
        price_provider.add_price(mint, peak_time, peak_price);

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(500, 0), // 500 tokens * $0.20 missed = $100 exactly
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        // Should detect S2E moment because thresholds are met exactly
        assert!(result.is_some());
        let moment = result.unwrap();
        assert_eq!(moment.kind, MomentKind::SoldTooEarly);
    }

    #[tokio::test]
    async fn test_s2e_detector_multiple_peaks() {
        let mut price_provider = MockPriceProvider::new();
        let detector = SoldTooEarlyDetector::new(
            Decimal::new(20, 0), // 20% minimum gain threshold
            Decimal::new(100, 2), // $1.00 minimum USD amount
            7, // 7 days window
        );

        let mint = "test_mint";
        let wallet = "test_wallet";
        let sale_time = OffsetDateTime::now_utc();

        // Setup: Multiple peaks within the window
        let sale_price = Decimal::new(100, 2); // $1.00

        // First peak
        let peak1_price = Decimal::new(130, 2); // $1.30 (30% gain)
        let peak1_time = sale_time + time::Duration::days(2);
        price_provider.add_price(mint, peak1_time, peak1_price);

        // Second, higher peak
        let peak2_price = Decimal::new(150, 2); // $1.50 (50% gain)
        let peak2_time = sale_time + time::Duration::days(5);
        price_provider.add_price(mint, peak2_time, peak2_price);

        // Lower price after second peak
        let later_price = Decimal::new(110, 2); // $1.10
        let later_time = sale_time + time::Duration::days(7);
        price_provider.add_price(mint, later_time, later_price);

        let trade = create_test_trade(
            wallet,
            mint,
            Decimal::new(1000, 0), // 1000 tokens
            sale_price,
            sale_time,
            "buy_sig_123",
        );

        let context = DetectorContext {
            pool: todo!(), // Not used in this test
            price_provider: Arc::new(price_provider),
            redis: shared::MaybeRedis::None,
        };

        let result = detector.detect(&context, &trade).await.unwrap();

        assert!(result.is_some());
        let moment = result.unwrap();

        // Should use the highest peak for calculation
        let expected_pct = Decimal::new(50, 0); // 50% gain from $1.00 to $1.50
        let expected_usd = Decimal::new(1000, 0) * (peak2_price - sale_price); // 1000 * $0.50 = $500

        assert!((moment.pct_dec.unwrap() - expected_pct).abs() < Decimal::new(1, 0));
        assert!((moment.missed_usd_dec.unwrap() - expected_usd).abs() < Decimal::new(100, 2));
    }
}
