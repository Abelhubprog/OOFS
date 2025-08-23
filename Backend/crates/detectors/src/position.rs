use anyhow::Result;
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::{ChainEvent, EventKind};
use sqlx::PgPool;
use std::collections::VecDeque;
use time::OffsetDateTime;
use ulid::Ulid;

/// Individual lot representing a buy position
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lot {
    pub lot_id: String,
    pub entry_ts: OffsetDateTime,
    pub qty_initial: Decimal,
    pub qty_remaining: Decimal,
    pub entry_px: Decimal,
}

impl Lot {
    pub fn new(entry_ts: OffsetDateTime, qty: Decimal, entry_px: Decimal) -> Self {
        Self {
            lot_id: Ulid::new().to_string(),
            entry_ts,
            qty_initial: qty,
            qty_remaining: qty,
            entry_px,
        }
    }

    /// Calculate the cost basis for this lot
    pub fn cost_basis(&self) -> Decimal {
        self.qty_initial * self.entry_px
    }

    /// Calculate unrealized PnL at current price
    pub fn unrealized_pnl(&self, current_px: Decimal) -> Decimal {
        (current_px - self.entry_px) * self.qty_remaining
    }
}

/// Episode representing a complete entry/exit cycle
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Episode {
    pub episode_id: String,
    pub wallet: String,
    pub mint: String,
    pub start_ts: OffsetDateTime,
    pub end_ts: Option<OffsetDateTime>,
    pub basis_usd: Decimal,
    pub realized_pnl_usd: Decimal,
    pub roi_pct: Option<Decimal>,
    pub is_active: bool,
}

impl Episode {
    pub fn new(wallet: String, mint: String, start_ts: OffsetDateTime) -> Self {
        Self {
            episode_id: Ulid::new().to_string(),
            wallet,
            mint,
            start_ts,
            end_ts: None,
            basis_usd: Decimal::ZERO,
            realized_pnl_usd: Decimal::ZERO,
            roi_pct: None,
            is_active: true,
        }
    }

    pub fn close(&mut self, end_ts: OffsetDateTime) {
        self.end_ts = Some(end_ts);
        self.is_active = false;
        if self.basis_usd > Decimal::ZERO {
            self.roi_pct = Some(self.realized_pnl_usd / self.basis_usd);
        }
    }
}

/// Realized trade record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RealizedTrade {
    pub exit_id: String,
    pub wallet: String,
    pub mint: String,
    pub episode_id: String,
    pub ts: OffsetDateTime,
    pub qty: Decimal,
    pub vwavg_exit_px: Decimal,
    pub realized_pnl_usd: Decimal,
    pub sig: String,
}

impl RealizedTrade {
    pub fn new(
        wallet: String,
        mint: String,
        episode_id: String,
        ts: OffsetDateTime,
        qty: Decimal,
        vwavg_exit_px: Decimal,
        realized_pnl_usd: Decimal,
        sig: String,
    ) -> Self {
        Self {
            exit_id: Ulid::new().to_string(),
            wallet,
            mint,
            episode_id,
            ts,
            qty,
            vwavg_exit_px,
            realized_pnl_usd,
            sig,
        }
    }
}

/// Position state for a wallet/mint pair
#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct PositionState {
    pub wallet: String,
    pub mint: String,
    pub lots: VecDeque<Lot>,
    pub exposure: Decimal,
    pub current_episode: Option<Episode>,
    pub total_realized_pnl: Decimal,
    pub last_event_ts: Option<OffsetDateTime>,
    pub snapshot_counter: usize,
}

impl PositionState {
    pub fn new(wallet: String, mint: String) -> Self {
        Self {
            wallet,
            mint,
            lots: VecDeque::new(),
            exposure: Decimal::ZERO,
            current_episode: None,
            total_realized_pnl: Decimal::ZERO,
            last_event_ts: None,
            snapshot_counter: 0,
        }
    }

    /// Get current cost basis
    pub fn cost_basis(&self) -> Decimal {
        self.lots.iter().map(|lot| lot.cost_basis()).sum()
    }

    /// Calculate average entry price
    pub fn average_entry_price(&self) -> Option<Decimal> {
        if self.exposure == Decimal::ZERO {
            return None;
        }
        Some(self.cost_basis() / self.exposure)
    }

    /// Calculate unrealized PnL at current price
    pub fn unrealized_pnl(&self, current_px: Decimal) -> Decimal {
        self.lots
            .iter()
            .map(|lot| lot.unrealized_pnl(current_px))
            .sum()
    }

    /// Check if position should be snapshotted
    pub fn should_snapshot(&self) -> bool {
        self.snapshot_counter % shared::constants::system::POSITION_SNAPSHOT_INTERVAL == 0
    }
}

/// Position engine for processing chain events
pub struct Engine {
    pool: PgPool,
}

impl Engine {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process a chain event and update position state
    pub async fn process_event(
        &self,
        state: &mut PositionState,
        event: &ChainEvent,
    ) -> Result<Vec<RealizedTrade>> {
        let mut trades = Vec::new();

        match event.kind {
            EventKind::Buy => {
                if let (Some(qty), Some(px)) = (event.amount, event.price_usd) {
                    self.on_buy(state, event.timestamp, qty, px).await?;
                }
            }
            EventKind::Sell => {
                if let (Some(qty), Some(px)) = (event.amount, event.price_usd) {
                    let realized_trades = self
                        .on_sell(state, event.timestamp, qty, px, &event.signature)
                        .await?;
                    trades.extend(realized_trades);
                }
            }
            EventKind::Transfer => {
                // Handle transfers (could be self-transfer or to CEX)
                if let Some(qty) = event.amount {
                    self.on_transfer(state, event.timestamp, qty, &event.metadata)
                        .await?;
                }
            }
            _ => {
                // Other event types don't affect positions directly
            }
        }

        state.last_event_ts = Some(event.timestamp);
        state.snapshot_counter += 1;

        // Persist snapshots periodically
        if state.should_snapshot() {
            self.persist_snapshot(state).await?;
        }

        Ok(trades)
    }

    /// Handle buy events (enter position)
    async fn on_buy(
        &self,
        state: &mut PositionState,
        ts: OffsetDateTime,
        qty: Decimal,
        px: Decimal,
    ) -> Result<()> {
        // Start new episode if no exposure
        if state.exposure == Decimal::ZERO {
            let episode = Episode::new(state.wallet.clone(), state.mint.clone(), ts);
            state.current_episode = Some(episode);
        }

        // Add new lot
        let lot = Lot::new(ts, qty, px);

        // Persist lot to database
        let episode_id = state.current_episode.as_ref().unwrap().episode_id.clone();
        sqlx::query(include_str!("../../../../db/queries/upsert_lot.sql"))
            .bind(&lot.lot_id)
            .bind(&state.wallet)
            .bind(&state.mint)
            .bind(&episode_id)
            .bind(lot.entry_ts)
            .bind(lot.qty_initial)
            .bind(lot.qty_remaining)
            .bind(lot.entry_px)
            .execute(&self.pool)
            .await?;

        state.lots.push_back(lot);
        state.exposure += qty;

        // Update episode basis
        if let Some(episode) = &mut state.current_episode {
            episode.basis_usd += qty * px;
        }

        Ok(())
    }

    /// Handle sell events (exit position)
    async fn on_sell(
        &self,
        state: &mut PositionState,
        ts: OffsetDateTime,
        mut qty_to_sell: Decimal,
        exit_px: Decimal,
        sig: &str,
    ) -> Result<Vec<RealizedTrade>> {
        let mut trades = Vec::new();
        let mut total_realized = Decimal::ZERO;
        let mut total_qty_sold = Decimal::ZERO;
        let mut weighted_entry_px = Decimal::ZERO;

        // Process FIFO lot matching
        while qty_to_sell > Decimal::ZERO && !state.lots.is_empty() {
            let mut lot = state.lots.pop_front().unwrap();
            let qty_from_lot = qty_to_sell.min(lot.qty_remaining);

            // Calculate realized PnL for this partial sell
            let realized_pnl = (exit_px - lot.entry_px) * qty_from_lot;
            total_realized += realized_pnl;
            total_qty_sold += qty_from_lot;
            weighted_entry_px += lot.entry_px * qty_from_lot;

            // Update lot
            lot.qty_remaining -= qty_from_lot;
            qty_to_sell -= qty_from_lot;

            // Update lot in database or remove if depleted
            if lot.qty_remaining > Decimal::ZERO {
                sqlx::query(include_str!("../../../../db/queries/upsert_lot.sql"))
                    .bind(&lot.lot_id)
                    .bind(&state.wallet)
                    .bind(&state.mint)
                    .bind(state.current_episode.as_ref().unwrap().episode_id.clone())
                    .bind(lot.entry_ts)
                    .bind(lot.qty_initial)
                    .bind(lot.qty_remaining)
                    .bind(lot.entry_px)
                    .execute(&self.pool)
                    .await?;

                state.lots.push_front(lot);
            } else {
                // Lot fully consumed, remove from database
                sqlx::query!("DELETE FROM lots WHERE lot_id = $1", lot.lot_id)
                    .execute(&self.pool)
                    .await?;
            }
        }

        // Update state
        state.exposure = state.lots.iter().map(|l| l.qty_remaining).sum();
        state.total_realized_pnl += total_realized;

        // Create realized trade record
        if total_qty_sold > Decimal::ZERO {
            let vwavg_entry_px = weighted_entry_px / total_qty_sold;
            let episode_id = state.current_episode.as_ref().unwrap().episode_id.clone();

            let trade = RealizedTrade::new(
                state.wallet.clone(),
                state.mint.clone(),
                episode_id.clone(),
                ts,
                total_qty_sold,
                exit_px,
                total_realized,
                sig.to_string(),
            );

            // Persist trade to database
            sqlx::query(include_str!(
                "../../../../db/queries/insert_realized_trade.sql"
            ))
            .bind(&trade.exit_id)
            .bind(&trade.wallet)
            .bind(&trade.mint)
            .bind(&trade.episode_id)
            .bind(trade.ts)
            .bind(trade.qty)
            .bind(trade.vwavg_exit_px)
            .bind(trade.realized_pnl_usd)
            .bind(&trade.sig)
            .execute(&self.pool)
            .await?;

            trades.push(trade);
        }

        // Close episode if position fully exited
        if state.exposure == Decimal::ZERO {
            if let Some(mut episode) = state.current_episode.take() {
                episode.realized_pnl_usd = state.total_realized_pnl;
                episode.close(ts);

                // Persist episode to database
                sqlx::query(include_str!("../../../../db/queries/upsert_episode.sql"))
                    .bind(&episode.episode_id)
                    .bind(&episode.wallet)
                    .bind(&episode.mint)
                    .bind(episode.start_ts)
                    .bind(episode.end_ts)
                    .bind(episode.basis_usd)
                    .bind(episode.realized_pnl_usd)
                    .bind(episode.roi_pct)
                    .execute(&self.pool)
                    .await?;
            }
        }

        Ok(trades)
    }

    /// Handle transfer events
    async fn on_transfer(
        &self,
        state: &mut PositionState,
        _ts: OffsetDateTime,
        _qty: Decimal,
        metadata: &serde_json::Value,
    ) -> Result<()> {
        // Check if transfer is to a known CEX address
        let to_address = metadata.get("to").and_then(|v| v.as_str());

        if let Some(to) = to_address {
            // Check against known CEX addresses (this would be configurable)
            let is_cex_transfer = self.is_cex_address(to).await?;

            if is_cex_transfer {
                // Treat as realization at current market price
                // This would require price lookup
                tracing::info!("CEX transfer detected for {}: {}", state.wallet, to);
            }
        }

        // For now, transfers don't affect position (only moves tokens, not basis)
        Ok(())
    }

    /// Check if address is a known CEX address
    async fn is_cex_address(&self, _address: &str) -> Result<bool> {
        // This would check against a database of known CEX addresses
        // For now, return false
        Ok(false)
    }

    /// Persist position state snapshot
    async fn persist_snapshot(&self, state: &PositionState) -> Result<()> {
        let snapshot_data = serde_json::to_value(state)?;

        sqlx::query!(
            "INSERT INTO position_snapshots (wallet, mint, snapshot_ts, snapshot_data) VALUES ($1, $2, NOW(), $3)",
            state.wallet,
            state.mint,
            snapshot_data
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load position state from database
    pub async fn load_state(&self, wallet: &str, mint: &str) -> Result<PositionState> {
        // Try to load from most recent snapshot first
        let snapshot = sqlx::query!(
            "SELECT snapshot_data FROM position_snapshots WHERE wallet = $1 AND mint = $2 ORDER BY snapshot_ts DESC LIMIT 1",
            wallet,
            mint
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(snap) = snapshot {
            if let Ok(state) = serde_json::from_value::<PositionState>(snap.snapshot_data) {
                return Ok(state);
            }
        }

        // Fallback: reconstruct from lots
        let lots = sqlx::query!("SELECT lot_id, episode_id, entry_ts, qty_initial, qty_remaining, entry_px_usd_dec FROM lots WHERE wallet = $1 AND mint = $2 ORDER BY entry_ts ASC", wallet, mint)
            .fetch_all(&self.pool)
            .await?;

        let mut state = PositionState::new(wallet.to_string(), mint.to_string());

        for lot_row in lots {
            let lot = Lot {
                lot_id: lot_row.lot_id,
                entry_ts: lot_row.entry_ts,
                qty_initial: lot_row.qty_initial,
                qty_remaining: lot_row.qty_remaining,
                entry_px: lot_row.entry_px_usd_dec,
            };

            state.lots.push_back(lot);
            state.exposure += lot_row.qty_remaining;
        }

        // Check for active episode
        let active_episode = sqlx::query!(
            "SELECT episode_id, start_ts, basis_usd_dec, realized_pnl_usd_dec FROM episodes WHERE wallet = $1 AND mint = $2 AND end_ts IS NULL",
            wallet,
            mint
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(ep) = active_episode {
            let episode = Episode {
                episode_id: ep.episode_id,
                wallet: wallet.to_string(),
                mint: mint.to_string(),
                start_ts: ep.start_ts,
                end_ts: None,
                basis_usd: ep.basis_usd_dec,
                realized_pnl_usd: ep.realized_pnl_usd_dec,
                roi_pct: None,
                is_active: true,
            };
            state.current_episode = Some(episode);
        }

        Ok(state)
    }

    /// Replay events from a specific timestamp to rebuild state
    pub async fn replay_from_timestamp(
        &self,
        wallet: &str,
        mint: &str,
        from_ts: OffsetDateTime,
    ) -> Result<PositionState> {
        let mut state = PositionState::new(wallet.to_string(), mint.to_string());

        // Get all actions for this wallet/mint from timestamp
        let actions = sqlx::query!(
            include_str!("../../../../db/queries/select_wallet_actions.sql"),
            wallet,
            from_ts
        )
        .fetch_all(&self.pool)
        .await?;

        for action in actions {
            if action.mint.as_deref() == Some(mint) {
                let event = shared::Action {
                    id: action.id,
                    signature: action.sig,
                    log_idx: action.log_idx,
                    slot: action.slot,
                    timestamp: action.ts,
                    program_id: action.program_id,
                    kind: action.kind,
                    mint: action.mint,
                    amount_dec: action.amount_dec,
                    exec_px_usd_dec: action.exec_px_usd_dec,
                    route: action.route,
                    flags_json: action.flags_json,
                }
                .to_chain_event(wallet);

                self.process_event(&mut state, &event).await?;
            }
        }

        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::macros::datetime;

    #[test]
    fn test_lot_creation() {
        let lot = Lot::new(datetime!(2024-01-01 0:00 UTC), dec!(10), dec!(1.5));
        assert_eq!(lot.qty_initial, dec!(10));
        assert_eq!(lot.qty_remaining, dec!(10));
        assert_eq!(lot.entry_px, dec!(1.5));
        assert_eq!(lot.cost_basis(), dec!(15));
    }

    #[test]
    fn test_position_state() {
        let mut state = PositionState::new("wallet123".to_string(), "mint456".to_string());
        assert_eq!(state.exposure, Decimal::ZERO);
        assert_eq!(state.cost_basis(), Decimal::ZERO);
        assert!(state.average_entry_price().is_none());

        let lot = Lot::new(datetime!(2024-01-01 0:00 UTC), dec!(10), dec!(1.5));
        state.lots.push_back(lot);
        state.exposure = dec!(10);

        assert_eq!(state.cost_basis(), dec!(15));
        assert_eq!(state.average_entry_price(), Some(dec!(1.5)));
        assert_eq!(state.unrealized_pnl(dec!(2.0)), dec!(5.0)); // (2.0 - 1.5) * 10
    }

    #[test]
    fn test_episode_lifecycle() {
        let mut episode = Episode::new(
            "wallet".to_string(),
            "mint".to_string(),
            datetime!(2024-01-01 0:00 UTC),
        );
        assert!(episode.is_active);
        assert!(episode.end_ts.is_none());

        episode.basis_usd = dec!(100);
        episode.realized_pnl_usd = dec!(20);
        episode.close(datetime!(2024-01-02 0:00 UTC));

        assert!(!episode.is_active);
        assert!(episode.end_ts.is_some());
        assert_eq!(episode.roi_pct, Some(dec!(0.2))); // 20/100 = 0.2
    }
}

#[cfg(test)]
mod position_tests;
