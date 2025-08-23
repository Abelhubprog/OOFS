#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use serde_json;
    use std::str::FromStr;
    use time::OffsetDateTime;

    #[test]
    fn test_moment_kind_display() {
        assert_eq!(MomentKind::SoldTooEarly.to_string(), "S2E");
        assert_eq!(MomentKind::BagHolderDrawdown.to_string(), "BHD");
        assert_eq!(MomentKind::BadRoute.to_string(), "BadRoute");
        assert_eq!(MomentKind::IdleYield.to_string(), "Idle");
        assert_eq!(MomentKind::Rug.to_string(), "Rug");
    }

    #[test]
    fn test_moment_kind_from_str() {
        assert_eq!(MomentKind::from_str("S2E").unwrap(), MomentKind::SoldTooEarly);
        assert_eq!(MomentKind::from_str("BHD").unwrap(), MomentKind::BagHolderDrawdown);
        assert_eq!(MomentKind::from_str("BadRoute").unwrap(), MomentKind::BadRoute);
        assert_eq!(MomentKind::from_str("Idle").unwrap(), MomentKind::IdleYield);
        assert_eq!(MomentKind::from_str("Rug").unwrap(), MomentKind::Rug);

        // Case insensitive
        assert_eq!(MomentKind::from_str("s2e").unwrap(), MomentKind::SoldTooEarly);
        assert_eq!(MomentKind::from_str("bhd").unwrap(), MomentKind::BagHolderDrawdown);

        // Invalid
        assert!(MomentKind::from_str("InvalidKind").is_err());
        assert!(MomentKind::from_str("").is_err());
    }

    #[test]
    fn test_moment_kind_serialization() {
        let kind = MomentKind::SoldTooEarly;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"S2E\"");

        let deserialized: MomentKind = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, MomentKind::SoldTooEarly);
    }

    #[test]
    fn test_moment_creation() {
        let moment = Moment {
            id: "test_id".to_string(),
            wallet: "test_wallet".to_string(),
            mint: Some("test_mint".to_string()),
            kind: MomentKind::SoldTooEarly,
            t_event: OffsetDateTime::now_utc(),
            pct_dec: Some(Decimal::new(2543, 2)), // 25.43%
            missed_usd_dec: Some(Decimal::new(150000, 2)), // $1,500.00
            severity_dec: Decimal::new(85, 0), // 85
            sig_ref: Some("test_signature".to_string()),
            slot_ref: Some(123456789),
            version: 1,
            explain_json: serde_json::json!({"reason": "Sold too early"}),
            preview_png_url: None,
        };

        assert_eq!(moment.id, "test_id");
        assert_eq!(moment.wallet, "test_wallet");
        assert_eq!(moment.kind, MomentKind::SoldTooEarly);
        assert_eq!(moment.version, 1);
    }

    #[test]
    fn test_moment_context_creation() {
        let context = MomentContext {
            wallet: "test_wallet".to_string(),
            analysis_start: OffsetDateTime::now_utc() - time::Duration::days(30),
            analysis_end: OffsetDateTime::now_utc(),
            total_transactions: 150,
            total_volume_usd: Decimal::new(50000, 2), // $500.00
            unique_tokens: 25,
            metadata: serde_json::json!({"source": "test"}),
        };

        assert_eq!(context.wallet, "test_wallet");
        assert_eq!(context.total_transactions, 150);
        assert_eq!(context.unique_tokens, 25);
    }

    #[test]
    fn test_extreme_entry_comparison() {
        let entry1 = ExtremeEntry {
            moment_id: "moment1".to_string(),
            value_usd_dec: Decimal::new(100000, 2), // $1,000.00
            percentage_dec: Some(Decimal::new(2500, 2)), // 25.00%
            mint: Some("mint1".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        let entry2 = ExtremeEntry {
            moment_id: "moment2".to_string(),
            value_usd_dec: Decimal::new(200000, 2), // $2,000.00
            percentage_dec: Some(Decimal::new(1500, 2)), // 15.00%
            mint: Some("mint2".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        // Higher USD value
        assert!(entry2.value_usd_dec > entry1.value_usd_dec);
        // Higher percentage in entry1
        assert!(entry1.percentage_dec > entry2.percentage_dec);
    }

    #[test]
    fn test_wallet_extremes_initialization() {
        let extremes = WalletExtremes {
            wallet: "test_wallet".to_string(),
            computed_at: OffsetDateTime::now_utc(),
            highest_win: None,
            highest_loss: None,
            top_s2e: None,
            worst_bhd: None,
            worst_bad_route: None,
            largest_idle: None,
        };

        assert_eq!(extremes.wallet, "test_wallet");
        assert!(extremes.highest_win.is_none());
        assert!(extremes.highest_loss.is_none());
        assert!(extremes.top_s2e.is_none());
    }

    #[test]
    fn test_wallet_extremes_with_data() {
        let highest_win = ExtremeEntry {
            moment_id: "win_moment".to_string(),
            value_usd_dec: Decimal::new(500000, 2), // $5,000.00
            percentage_dec: Some(Decimal::new(15000, 2)), // 150.00%
            mint: Some("winning_mint".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        let top_s2e = ExtremeEntry {
            moment_id: "s2e_moment".to_string(),
            value_usd_dec: Decimal::new(200000, 2), // $2,000.00
            percentage_dec: Some(Decimal::new(5000, 2)), // 50.00%
            mint: Some("s2e_mint".to_string()),
            timestamp: OffsetDateTime::now_utc(),
        };

        let extremes = WalletExtremes {
            wallet: "test_wallet".to_string(),
            computed_at: OffsetDateTime::now_utc(),
            highest_win: Some(highest_win.clone()),
            highest_loss: None,
            top_s2e: Some(top_s2e.clone()),
            worst_bhd: None,
            worst_bad_route: None,
            largest_idle: None,
        };

        assert!(extremes.highest_win.is_some());
        assert_eq!(extremes.highest_win.unwrap().value_usd_dec, Decimal::new(500000, 2));
        assert!(extremes.top_s2e.is_some());
        assert_eq!(extremes.top_s2e.unwrap().moment_id, "s2e_moment");
    }

    #[test]
    fn test_moment_serialization_roundtrip() {
        let moment = Moment {
            id: "test_moment_id".to_string(),
            wallet: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM".to_string(),
            mint: Some("So11111111111111111111111111111111111111112".to_string()),
            kind: MomentKind::BagHolderDrawdown,
            t_event: OffsetDateTime::UNIX_EPOCH,
            pct_dec: Some(Decimal::new(-4567, 2)), // -45.67%
            missed_usd_dec: Some(Decimal::new(123456, 2)), // $1,234.56
            severity_dec: Decimal::new(92, 0), // 92
            sig_ref: Some("5w1Z2K3X4Y5N6P7Q8R9S".to_string()),
            slot_ref: Some(987654321),
            version: 2,
            explain_json: serde_json::json!({
                "peak_value": 5000.0,
                "exit_value": 2500.0,
                "drawdown_percentage": 50.0
            }),
            preview_png_url: Some("https://example.com/preview.png".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&moment).unwrap();

        // Deserialize back
        let deserialized: Moment = serde_json::from_str(&json).unwrap();

        // Verify all fields match
        assert_eq!(deserialized.id, moment.id);
        assert_eq!(deserialized.wallet, moment.wallet);
        assert_eq!(deserialized.mint, moment.mint);
        assert_eq!(deserialized.kind, moment.kind);
        assert_eq!(deserialized.pct_dec, moment.pct_dec);
        assert_eq!(deserialized.missed_usd_dec, moment.missed_usd_dec);
        assert_eq!(deserialized.severity_dec, moment.severity_dec);
        assert_eq!(deserialized.sig_ref, moment.sig_ref);
        assert_eq!(deserialized.slot_ref, moment.slot_ref);
        assert_eq!(deserialized.version, moment.version);
        assert_eq!(deserialized.preview_png_url, moment.preview_png_url);
    }

    #[test]
    fn test_extreme_entry_serialization() {
        let entry = ExtremeEntry {
            moment_id: "extreme_moment".to_string(),
            value_usd_dec: Decimal::new(999999, 2), // $9,999.99
            percentage_dec: Some(Decimal::new(-7825, 2)), // -78.25%
            mint: Some("DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string()),
            timestamp: OffsetDateTime::UNIX_EPOCH + time::Duration::days(365),
        };

        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: ExtremeEntry = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.moment_id, entry.moment_id);
        assert_eq!(deserialized.value_usd_dec, entry.value_usd_dec);
        assert_eq!(deserialized.percentage_dec, entry.percentage_dec);
        assert_eq!(deserialized.mint, entry.mint);
        assert_eq!(deserialized.timestamp, entry.timestamp);
    }

    #[test]
    fn test_decimal_precision_in_moments() {
        // Test very precise decimal values
        let moment = Moment {
            id: "precision_test".to_string(),
            wallet: "test_wallet".to_string(),
            mint: None,
            kind: MomentKind::Rug,
            t_event: OffsetDateTime::now_utc(),
            pct_dec: Some(Decimal::new(123456789, 9)), // Very precise percentage
            missed_usd_dec: Some(Decimal::new(987654321, 6)), // Very precise USD amount
            severity_dec: Decimal::new(9999, 2), // 99.99
            sig_ref: None,
            slot_ref: None,
            version: 1,
            explain_json: serde_json::json!({}),
            preview_png_url: None,
        };

        // Ensure precision is maintained through serialization
        let json = serde_json::to_string(&moment).unwrap();
        let deserialized: Moment = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.pct_dec, moment.pct_dec);
        assert_eq!(deserialized.missed_usd_dec, moment.missed_usd_dec);
        assert_eq!(deserialized.severity_dec, moment.severity_dec);
    }
}
