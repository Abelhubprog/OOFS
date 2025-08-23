#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockPriceProvider;
    use mockall::predicate::*;
    use rust_decimal::Decimal;
    use shared::types::chain::{ChainEvent, EventKind};
    use sqlx::PgPool;
    use time::OffsetDateTime;
    use tokio_test;

    // Helper function to create test pool (mock)
    async fn create_test_pool() -> PgPool {
        // In real tests, this would use testcontainers
        // For now, we'll use mock implementations where possible
        panic!("Not implemented for unit tests - use integration tests for database operations")
    }

    fn create_test_buy_event(
        wallet: &str,
        mint: &str,
        amount: Decimal,
        price: Decimal,
        timestamp: OffsetDateTime,
    ) -> ChainEvent {
        ChainEvent {
            id: ulid::Ulid::new().to_string(),
            signature: format!("test_sig_{}", ulid::Ulid::new()),
            wallet: wallet.to_string(),
            kind: EventKind::Buy,
            slot: 123456789,
            timestamp,
            mint: Some(mint.to_string()),
            amount_dec: Some(amount),
            sol_amount_dec: Some(amount * price),
            counterparty: None,
            flags: serde_json::json!({
                "direction": "buy",
                "price": price
            }),
            program_id: "test_program".to_string(),
            version: 1,
        }
    }

    fn create_test_sell_event(
        wallet: &str,
        mint: &str,
        amount: Decimal,
        price: Decimal,
        timestamp: OffsetDateTime,
    ) -> ChainEvent {
        ChainEvent {
            id: ulid::Ulid::new().to_string(),
            signature: format!("test_sig_{}", ulid::Ulid::new()),
            wallet: wallet.to_string(),
            kind: EventKind::Sell,
            slot: 123456789,
            timestamp,
            mint: Some(mint.to_string()),
            amount_dec: Some(amount),
            sol_amount_dec: Some(amount * price),
            counterparty: None,
            flags: serde_json::json!({
                "direction": "sell",
                "price": price
            }),
            program_id: "test_program".to_string(),
            version: 1,
        }
    }

    #[test]
    fn test_position_state_new() {
        let state = PositionState::new("test_wallet".to_string());
        assert_eq!(state.wallet, "test_wallet");
        assert!(state.holdings.is_empty());
        assert!(state.lot_storage.is_empty());
        assert!(state.episodes.is_empty());
        assert_eq!(state.total_realized_pnl, Decimal::ZERO);
    }

    #[test]
    fn test_lot_creation() {
        let now = OffsetDateTime::now_utc();
        let lot = Lot {
            id: "test_lot".to_string(),
            wallet: "test_wallet".to_string(),
            mint: "test_mint".to_string(),
            quantity_dec: Decimal::new(1000, 0), // 1000 tokens
            cost_basis_usd_dec: Decimal::new(50000, 2), // $500.00
            acquired_at: now,
            episode_id: Some("episode_1".to_string()),
            acquisition_sig: "test_sig".to_string(),
            avg_cost_per_token: Decimal::new(5, 1), // $0.50 per token
        };

        assert_eq!(lot.quantity_dec, Decimal::new(1000, 0));
        assert_eq!(lot.cost_basis_usd_dec, Decimal::new(50000, 2));
        assert_eq!(lot.avg_cost_per_token, Decimal::new(5, 1));
    }

    #[test]
    fn test_position_state_add_holding() {
        let mut state = PositionState::new("test_wallet".to_string());
        let amount = Decimal::new(1000, 0);

        state.add_holding("test_mint", amount);

        assert_eq!(state.holdings.get("test_mint"), Some(&amount));
    }

    #[test]
    fn test_position_state_reduce_holding() {
        let mut state = PositionState::new("test_wallet".to_string());
        let initial_amount = Decimal::new(1000, 0);
        let reduce_amount = Decimal::new(300, 0);

        state.add_holding("test_mint", initial_amount);
        let success = state.reduce_holding("test_mint", reduce_amount);

        assert!(success);
        assert_eq!(
            state.holdings.get("test_mint"),
            Some(&(initial_amount - reduce_amount))
        );
    }

    #[test]
    fn test_position_state_reduce_holding_insufficient() {
        let mut state = PositionState::new("test_wallet".to_string());
        let initial_amount = Decimal::new(100, 0);
        let reduce_amount = Decimal::new(200, 0);

        state.add_holding("test_mint", initial_amount);
        let success = state.reduce_holding("test_mint", reduce_amount);

        assert!(!success);
        assert_eq!(state.holdings.get("test_mint"), Some(&initial_amount));
    }

    #[test]
    fn test_position_state_fifo_lot_matching() {
        let mut state = PositionState::new("test_wallet".to_string());
        let mint = "test_mint";

        // Create two lots acquired at different times and prices
        let now = OffsetDateTime::now_utc();
        let lot1 = Lot {
            id: "lot1".to_string(),
            wallet: "test_wallet".to_string(),
            mint: mint.to_string(),
            quantity_dec: Decimal::new(1000, 0), // 1000 tokens
            cost_basis_usd_dec: Decimal::new(100000, 2), // $1000.00
            acquired_at: now - time::Duration::hours(2),
            episode_id: Some("episode_1".to_string()),
            acquisition_sig: "sig1".to_string(),
            avg_cost_per_token: Decimal::new(1, 0), // $1.00 per token
        };

        let lot2 = Lot {
            id: "lot2".to_string(),
            wallet: "test_wallet".to_string(),
            mint: mint.to_string(),
            quantity_dec: Decimal::new(500, 0), // 500 tokens
            cost_basis_usd_dec: Decimal::new(100000, 2), // $1000.00
            acquired_at: now - time::Duration::hours(1),
            episode_id: Some("episode_1".to_string()),
            acquisition_sig: "sig2".to_string(),
            avg_cost_per_token: Decimal::new(2, 0), // $2.00 per token
        };

        state
            .lot_storage
            .entry(mint.to_string())
            .or_default()
            .push(lot1.clone());
        state
            .lot_storage
            .entry(mint.to_string())
            .or_default()
            .push(lot2.clone());

        // Sell 800 tokens - should consume all of lot1 (1000) and none of lot2
        let consumed_lots = state.consume_lots_fifo(mint, Decimal::new(800, 0));

        assert_eq!(consumed_lots.len(), 1);
        assert_eq!(consumed_lots[0].lot.id, "lot1");
        assert_eq!(consumed_lots[0].quantity_consumed, Decimal::new(800, 0));
        assert_eq!(consumed_lots[0].cost_basis_consumed, Decimal::new(80000, 2)); // $800.00

        // Check remaining lots
        let remaining_lots = &state.lot_storage[mint];
        assert_eq!(remaining_lots.len(), 2);
        assert_eq!(remaining_lots[0].quantity_dec, Decimal::new(200, 0)); // 200 remaining from lot1
        assert_eq!(remaining_lots[1].quantity_dec, Decimal::new(500, 0)); // lot2 unchanged
    }

    #[test]
    fn test_position_state_fifo_multiple_lots() {
        let mut state = PositionState::new("test_wallet".to_string());
        let mint = "test_mint";
        let now = OffsetDateTime::now_utc();

        // Create three lots
        let lots = vec![
            Lot {
                id: "lot1".to_string(),
                wallet: "test_wallet".to_string(),
                mint: mint.to_string(),
                quantity_dec: Decimal::new(500, 0),
                cost_basis_usd_dec: Decimal::new(50000, 2), // $500.00
                acquired_at: now - time::Duration::hours(3),
                episode_id: Some("episode_1".to_string()),
                acquisition_sig: "sig1".to_string(),
                avg_cost_per_token: Decimal::new(1, 0), // $1.00
            },
            Lot {
                id: "lot2".to_string(),
                wallet: "test_wallet".to_string(),
                mint: mint.to_string(),
                quantity_dec: Decimal::new(300, 0),
                cost_basis_usd_dec: Decimal::new(60000, 2), // $600.00
                acquired_at: now - time::Duration::hours(2),
                episode_id: Some("episode_1".to_string()),
                acquisition_sig: "sig2".to_string(),
                avg_cost_per_token: Decimal::new(2, 0), // $2.00
            },
            Lot {
                id: "lot3".to_string(),
                wallet: "test_wallet".to_string(),
                mint: mint.to_string(),
                quantity_dec: Decimal::new(200, 0),
                cost_basis_usd_dec: Decimal::new(60000, 2), // $600.00
                acquired_at: now - time::Duration::hours(1),
                episode_id: Some("episode_1".to_string()),
                acquisition_sig: "sig3".to_string(),
                avg_cost_per_token: Decimal::new(3, 0), // $3.00
            },
        ];

        for lot in lots {
            state
                .lot_storage
                .entry(mint.to_string())
                .or_default()
                .push(lot);
        }

        // Sell 700 tokens - should consume lot1 (500) + lot2 (200 out of 300)
        let consumed_lots = state.consume_lots_fifo(mint, Decimal::new(700, 0));

        assert_eq!(consumed_lots.len(), 2);

        // First lot completely consumed
        assert_eq!(consumed_lots[0].lot.id, "lot1");
        assert_eq!(consumed_lots[0].quantity_consumed, Decimal::new(500, 0));
        assert_eq!(consumed_lots[0].cost_basis_consumed, Decimal::new(50000, 2));

        // Second lot partially consumed
        assert_eq!(consumed_lots[1].lot.id, "lot2");
        assert_eq!(consumed_lots[1].quantity_consumed, Decimal::new(200, 0));
        assert_eq!(consumed_lots[1].cost_basis_consumed, Decimal::new(40000, 2)); // $400.00 (200 * $2.00)

        // Check remaining lots
        let remaining_lots = &state.lot_storage[mint];
        assert_eq!(remaining_lots.len(), 2); // lot1 removed, lot2 reduced, lot3 unchanged
        assert_eq!(remaining_lots[0].quantity_dec, Decimal::new(100, 0)); // 100 remaining from lot2
        assert_eq!(remaining_lots[1].quantity_dec, Decimal::new(200, 0)); // lot3 unchanged
    }

    #[test]
    fn test_realized_trade_calculation() {
        let consumed_lot = ConsumedLot {
            lot: Lot {
                id: "test_lot".to_string(),
                wallet: "test_wallet".to_string(),
                mint: "test_mint".to_string(),
                quantity_dec: Decimal::new(1000, 0),
                cost_basis_usd_dec: Decimal::new(100000, 2), // $1000.00
                acquired_at: OffsetDateTime::now_utc() - time::Duration::hours(1),
                episode_id: Some("episode_1".to_string()),
                acquisition_sig: "buy_sig".to_string(),
                avg_cost_per_token: Decimal::new(1, 0), // $1.00 per token
            },
            quantity_consumed: Decimal::new(500, 0), // Sell 500 tokens
            cost_basis_consumed: Decimal::new(50000, 2), // $500.00 cost basis
        };

        let sell_price_per_token = Decimal::new(15, 1); // $1.50 per token
        let realized_trade = RealizedTrade::from_consumed_lot(
            consumed_lot,
            sell_price_per_token,
            "sell_sig".to_string(),
            OffsetDateTime::now_utc(),
        );

        assert_eq!(realized_trade.quantity_dec, Decimal::new(500, 0));
        assert_eq!(realized_trade.cost_basis_usd_dec, Decimal::new(50000, 2));
        assert_eq!(realized_trade.sale_proceeds_usd_dec, Decimal::new(75000, 2)); // 500 * $1.50 = $750.00
        assert_eq!(realized_trade.realized_pnl_usd_dec, Decimal::new(25000, 2)); // $750.00 - $500.00 = $250.00
        assert_eq!(realized_trade.avg_cost_per_token, Decimal::new(1, 0));
        assert_eq!(realized_trade.sale_price_per_token, Decimal::new(15, 1));
        assert_eq!(realized_trade.acquisition_sig, "buy_sig");
        assert_eq!(realized_trade.sale_sig, "sell_sig");
    }

    #[test]
    fn test_realized_trade_loss() {
        let consumed_lot = ConsumedLot {
            lot: Lot {
                id: "test_lot".to_string(),
                wallet: "test_wallet".to_string(),
                mint: "test_mint".to_string(),
                quantity_dec: Decimal::new(1000, 0),
                cost_basis_usd_dec: Decimal::new(200000, 2), // $2000.00
                acquired_at: OffsetDateTime::now_utc() - time::Duration::hours(1),
                episode_id: Some("episode_1".to_string()),
                acquisition_sig: "buy_sig".to_string(),
                avg_cost_per_token: Decimal::new(2, 0), // $2.00 per token
            },
            quantity_consumed: Decimal::new(1000, 0), // Sell all 1000 tokens
            cost_basis_consumed: Decimal::new(200000, 2), // $2000.00 cost basis
        };

        let sell_price_per_token = Decimal::new(15, 1); // $1.50 per token (loss)
        let realized_trade = RealizedTrade::from_consumed_lot(
            consumed_lot,
            sell_price_per_token,
            "sell_sig".to_string(),
            OffsetDateTime::now_utc(),
        );

        assert_eq!(
            realized_trade.sale_proceeds_usd_dec,
            Decimal::new(150000, 2)
        ); // 1000 * $1.50 = $1500.00
        assert_eq!(realized_trade.realized_pnl_usd_dec, Decimal::new(-50000, 2));
        // $1500.00 - $2000.00 = -$500.00
    }

    #[test]
    fn test_episode_creation() {
        let episode = Episode::new(
            "test_wallet".to_string(),
            "test_mint".to_string(),
            OffsetDateTime::now_utc(),
        );

        assert!(!episode.id.is_empty());
        assert_eq!(episode.wallet, "test_wallet");
        assert_eq!(episode.mint, "test_mint");
        assert!(episode.is_active);
        assert_eq!(episode.total_bought_dec, Decimal::ZERO);
        assert_eq!(episode.total_sold_dec, Decimal::ZERO);
        assert_eq!(episode.realized_pnl_dec, Decimal::ZERO);
        assert!(episode.first_buy_sig.is_none());
        assert!(episode.last_sell_sig.is_none());
        assert!(episode.closed_at.is_none());
    }

    #[test]
    fn test_episode_add_buy() {
        let mut episode = Episode::new(
            "test_wallet".to_string(),
            "test_mint".to_string(),
            OffsetDateTime::now_utc(),
        );

        let amount = Decimal::new(1000, 0);
        let cost = Decimal::new(100000, 2);

        episode.add_buy(amount, cost, "buy_sig".to_string());

        assert_eq!(episode.total_bought_dec, amount);
        assert_eq!(episode.total_cost_basis_dec, cost);
        assert_eq!(episode.first_buy_sig, Some("buy_sig".to_string()));
        assert!(episode.last_sell_sig.is_none());
    }

    #[test]
    fn test_episode_add_sell() {
        let mut episode = Episode::new(
            "test_wallet".to_string(),
            "test_mint".to_string(),
            OffsetDateTime::now_utc(),
        );

        // First add a buy
        episode.add_buy(
            Decimal::new(1000, 0),
            Decimal::new(100000, 2),
            "buy_sig".to_string(),
        );

        // Then add a sell
        let sold_amount = Decimal::new(500, 0);
        let proceeds = Decimal::new(75000, 2);
        let pnl = Decimal::new(25000, 2);

        episode.add_sell(sold_amount, proceeds, pnl, "sell_sig".to_string());

        assert_eq!(episode.total_sold_dec, sold_amount);
        assert_eq!(episode.total_proceeds_dec, proceeds);
        assert_eq!(episode.realized_pnl_dec, pnl);
        assert_eq!(episode.last_sell_sig, Some("sell_sig".to_string()));
    }

    #[test]
    fn test_episode_close() {
        let mut episode = Episode::new(
            "test_wallet".to_string(),
            "test_mint".to_string(),
            OffsetDateTime::now_utc(),
        );

        assert!(episode.is_active);
        assert!(episode.closed_at.is_none());

        let close_time = OffsetDateTime::now_utc();
        episode.close(close_time);

        assert!(!episode.is_active);
        assert_eq!(episode.closed_at, Some(close_time));
    }

    #[test]
    fn test_episode_multiple_transactions() {
        let mut episode = Episode::new(
            "test_wallet".to_string(),
            "test_mint".to_string(),
            OffsetDateTime::now_utc(),
        );

        // Multiple buys
        episode.add_buy(
            Decimal::new(1000, 0),
            Decimal::new(100000, 2),
            "buy1".to_string(),
        );
        episode.add_buy(
            Decimal::new(500, 0),
            Decimal::new(75000, 2),
            "buy2".to_string(),
        );

        // Multiple sells
        episode.add_sell(
            Decimal::new(300, 0),
            Decimal::new(45000, 2),
            Decimal::new(15000, 2),
            "sell1".to_string(),
        );
        episode.add_sell(
            Decimal::new(200, 0),
            Decimal::new(40000, 2),
            Decimal::new(20000, 2),
            "sell2".to_string(),
        );

        assert_eq!(episode.total_bought_dec, Decimal::new(1500, 0)); // 1000 + 500
        assert_eq!(episode.total_cost_basis_dec, Decimal::new(175000, 2)); // $1000 + $750
        assert_eq!(episode.total_sold_dec, Decimal::new(500, 0)); // 300 + 200
        assert_eq!(episode.total_proceeds_dec, Decimal::new(85000, 2)); // $450 + $400
        assert_eq!(episode.realized_pnl_dec, Decimal::new(35000, 2)); // $150 + $200
        assert_eq!(episode.first_buy_sig, Some("buy1".to_string()));
        assert_eq!(episode.last_sell_sig, Some("sell2".to_string()));
    }
}
