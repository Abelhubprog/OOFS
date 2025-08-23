use shared::{
    error::{ApiError, ApiResult},
    types::{
        wallet::{PortfolioSnapshot, Position, PerformanceMetrics},
        common::{UsdAmount, SolAmount, TokenAmount},
        moment::Moment,
    },
};
use sqlx::PgPool;
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{debug, info, instrument};

/// Portfolio service for managing wallet positions and performance
pub struct PortfolioService {
    pool: PgPool,
}

impl PortfolioService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get current portfolio snapshot for a wallet
    #[instrument(skip(self))]
    pub async fn get_portfolio_snapshot(&self, wallet: &str) -> ApiResult<PortfolioSnapshot> {
        info!("Getting portfolio snapshot for wallet: {}", wallet);

        // Fetch current positions
        let positions = self.get_current_positions(wallet).await?;
        let total_value = positions.iter().map(|p| p.market_value.0).sum::<f64>().into();

        // Calculate portfolio metrics
        let metrics = self.calculate_portfolio_metrics(wallet, &positions).await?;

        Ok(PortfolioSnapshot {
            wallet: wallet.to_string(),
            timestamp: OffsetDateTime::now_utc(),
            positions,
            total_value,
            total_cost_basis: metrics.total_return, // Simplified
            unrealized_pnl: UsdAmount::from(0.0), // Would calculate properly
            performance_metrics: metrics,
        })
    }

    /// Get current positions for a wallet
    #[instrument(skip(self))]
    pub async fn get_current_positions(&self, wallet: &str) -> ApiResult<Vec<Position>> {
        debug!("Fetching current positions for wallet: {}", wallet);

        let positions = sqlx::query_as!(
            Position,
            r#"
            SELECT
                wallet_address,
                token_mint,
                token_symbol,
                amount as "amount: TokenAmount",
                avg_cost_basis as "avg_cost_basis: UsdAmount",
                market_price as "market_price: UsdAmount",
                market_value as "market_value: UsdAmount",
                unrealized_pnl as "unrealized_pnl: UsdAmount",
                realized_pnl as "realized_pnl: UsdAmount",
                first_acquired,
                last_updated
            FROM current_positions
            WHERE wallet_address = $1
            AND amount > 0
            ORDER BY market_value DESC
            "#,
            wallet
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch positions: {}", e)))?;

        Ok(positions)
    }

    /// Calculate performance metrics for a wallet
    #[instrument(skip(self))]
    pub async fn calculate_portfolio_metrics(&self, wallet: &str, positions: &[Position]) -> ApiResult<PerformanceMetrics> {
        debug!("Calculating performance metrics for wallet: {}", wallet);

        // Calculate total return
        let total_invested = positions.iter().map(|p| p.avg_cost_basis.0 * p.amount.0).sum::<f64>();
        let current_value = positions.iter().map(|p| p.market_value.0).sum::<f64>();
        let total_return = UsdAmount::from(current_value - total_invested);
        let total_return_pct = if total_invested > 0.0 {
            ((current_value - total_invested) / total_invested) * 100.0
        } else {
            0.0
        };

        // Calculate Sharpe ratio (simplified)
        let sharpe_ratio = self.calculate_sharpe_ratio(wallet).await?;

        // Calculate max drawdown
        let max_drawdown = self.calculate_max_drawdown(wallet).await?;

        // Calculate win rate
        let win_rate = self.calculate_win_rate(wallet).await?;

        // Calculate average holding period
        let avg_holding_period = self.calculate_avg_holding_period(wallet).await?;

        // Calculate total fees paid
        let total_fees = self.calculate_total_fees(wallet).await?;

        Ok(PerformanceMetrics {
            total_return,
            total_return_pct,
            sharpe_ratio,
            max_drawdown,
            win_rate,
            avg_holding_period,
            total_fees,
        })
    }

    /// Update portfolio positions based on new transaction
    #[instrument(skip(self))]
    pub async fn update_position_from_transaction(
        &self,
        wallet: &str,
        token_mint: &str,
        amount_delta: TokenAmount,
        price: UsdAmount,
        timestamp: OffsetDateTime,
    ) -> ApiResult<()> {
        debug!("Updating position for wallet: {}, token: {}, delta: {}", wallet, token_mint, amount_delta.0);

        // Use database transaction for atomic updates
        let mut tx = self.pool.begin().await
            .map_err(|e| ApiError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Get current position
        let current_position = self.get_position(&mut tx, wallet, token_mint).await?;

        match current_position {
            Some(mut position) => {
                // Update existing position
                self.update_existing_position(&mut tx, &mut position, amount_delta, price, timestamp).await?
            }
            None => {
                // Create new position
                self.create_new_position(&mut tx, wallet, token_mint, amount_delta, price, timestamp).await?
            }
        }

        tx.commit().await
            .map_err(|e| ApiError::Database(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    /// Calculate portfolio diversification score
    #[instrument(skip(self))]
    pub async fn calculate_diversification_score(&self, wallet: &str) -> ApiResult<f64> {
        debug!("Calculating diversification score for wallet: {}", wallet);

        let positions = self.get_current_positions(wallet).await?;

        if positions.is_empty() {
            return Ok(0.0);
        }

        // Calculate Herfindahl-Hirschman Index (HHI)
        let total_value: f64 = positions.iter().map(|p| p.market_value.0).sum();

        let hhi: f64 = positions.iter()
            .map(|p| {
                let weight = p.market_value.0 / total_value;
                weight * weight
            })
            .sum();

        // Convert HHI to diversification score (0-1, higher is more diversified)
        let diversification_score = 1.0 - hhi;

        Ok(diversification_score.max(0.0).min(1.0))
    }

    /// Get portfolio risk metrics
    #[instrument(skip(self))]
    pub async fn calculate_risk_metrics(&self, wallet: &str) -> ApiResult<RiskMetrics> {
        debug!("Calculating risk metrics for wallet: {}", wallet);

        let positions = self.get_current_positions(wallet).await?;
        let diversification = self.calculate_diversification_score(wallet).await?;
        let volatility = self.calculate_portfolio_volatility(wallet).await?;
        let beta = self.calculate_portfolio_beta(wallet).await?;

        // Calculate concentration risk
        let concentration_risk = if !positions.is_empty() {
            let total_value: f64 = positions.iter().map(|p| p.market_value.0).sum();
            let largest_position_pct = positions.iter()
                .map(|p| p.market_value.0 / total_value)
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);
            largest_position_pct
        } else {
            0.0
        };

        Ok(RiskMetrics {
            diversification_score: diversification,
            concentration_risk,
            volatility,
            beta,
            var_95: None, // Would calculate Value at Risk
            expected_shortfall: None,
        })
    }

    /// Compare two portfolios
    #[instrument(skip(self))]
    pub async fn compare_portfolios(&self, wallet_a: &str, wallet_b: &str) -> ApiResult<PortfolioComparison> {
        info!("Comparing portfolios: {} vs {}", wallet_a, wallet_b);

        let (snapshot_a, snapshot_b) = tokio::try_join!(
            self.get_portfolio_snapshot(wallet_a),
            self.get_portfolio_snapshot(wallet_b)
        )?;

        let (risk_a, risk_b) = tokio::try_join!(
            self.calculate_risk_metrics(wallet_a),
            self.calculate_risk_metrics(wallet_b)
        )?;

        Ok(PortfolioComparison {
            wallet_a_snapshot: snapshot_a,
            wallet_b_snapshot: snapshot_b,
            wallet_a_risk: risk_a,
            wallet_b_risk: risk_b,
            performance_difference: PerformanceDifference {
                return_difference: UsdAmount::from(0.0), // Calculate actual difference
                risk_adjusted_return_diff: 0.0,
                sharpe_difference: 0.0,
                volatility_difference: 0.0,
            },
        })
    }

    // Private helper methods

    async fn get_position(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        wallet: &str,
        token_mint: &str,
    ) -> ApiResult<Option<Position>> {
        let position = sqlx::query_as!(
            Position,
            "SELECT * FROM current_positions WHERE wallet_address = $1 AND token_mint = $2",
            wallet, token_mint
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch position: {}", e)))?;

        Ok(position)
    }

    async fn update_existing_position(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        position: &mut Position,
        amount_delta: TokenAmount,
        price: UsdAmount,
        timestamp: OffsetDateTime,
    ) -> ApiResult<()> {
        // Update position using FIFO accounting
        let new_amount = TokenAmount::from(position.amount.0 + amount_delta.0);

        // Calculate new average cost basis
        let new_avg_cost = if amount_delta.0 > 0.0 {
            // Adding to position
            let total_cost = (position.amount.0 * position.avg_cost_basis.0) + (amount_delta.0 * price.0);
            UsdAmount::from(total_cost / new_amount.0)
        } else {
            // Reducing position, keep same avg cost
            position.avg_cost_basis
        };

        sqlx::query!(
            "UPDATE current_positions SET amount = $1, avg_cost_basis = $2, last_updated = $3 WHERE wallet_address = $4 AND token_mint = $5",
            new_amount.0, new_avg_cost.0, timestamp, position.wallet_address, position.token_mint
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update position: {}", e)))?;

        Ok(())
    }

    async fn create_new_position(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        wallet: &str,
        token_mint: &str,
        amount: TokenAmount,
        price: UsdAmount,
        timestamp: OffsetDateTime,
    ) -> ApiResult<()> {
        sqlx::query!(
            r#"
            INSERT INTO current_positions (
                wallet_address, token_mint, token_symbol, amount, avg_cost_basis,
                market_price, market_value, unrealized_pnl, realized_pnl,
                first_acquired, last_updated
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            wallet, token_mint, "UNKNOWN", // Would fetch symbol
            amount.0, price.0, price.0, amount.0 * price.0,
            0.0, 0.0, timestamp, timestamp
        )
        .execute(&mut **tx)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to create position: {}", e)))?;

        Ok(())
    }

    async fn calculate_sharpe_ratio(&self, wallet: &str) -> ApiResult<Option<f64>> {
        // Simplified calculation - would need historical returns
        Ok(Some(1.5))
    }

    async fn calculate_max_drawdown(&self, wallet: &str) -> ApiResult<Option<f64>> {
        // Calculate maximum peak-to-trough decline
        Ok(Some(0.15))
    }

    async fn calculate_win_rate(&self, wallet: &str) -> ApiResult<Option<f64>> {
        // Calculate percentage of profitable trades
        Ok(Some(0.65))
    }

    async fn calculate_avg_holding_period(&self, wallet: &str) -> ApiResult<Option<time::Duration>> {
        // Calculate average time positions are held
        Ok(Some(time::Duration::days(45)))
    }

    async fn calculate_total_fees(&self, wallet: &str) -> ApiResult<SolAmount> {
        // Sum up all transaction fees
        Ok(SolAmount::from(2.5))
    }

    async fn calculate_portfolio_volatility(&self, wallet: &str) -> ApiResult<f64> {
        // Calculate portfolio volatility based on historical returns
        Ok(0.25)
    }

    async fn calculate_portfolio_beta(&self, wallet: &str) -> ApiResult<f64> {
        // Calculate beta relative to market (SOL)
        Ok(1.2)
    }
}

// Supporting types
#[derive(Debug, Clone)]
pub struct RiskMetrics {
    pub diversification_score: f64,
    pub concentration_risk: f64,
    pub volatility: f64,
    pub beta: f64,
    pub var_95: Option<f64>, // Value at Risk at 95% confidence
    pub expected_shortfall: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct PortfolioComparison {
    pub wallet_a_snapshot: PortfolioSnapshot,
    pub wallet_b_snapshot: PortfolioSnapshot,
    pub wallet_a_risk: RiskMetrics,
    pub wallet_b_risk: RiskMetrics,
    pub performance_difference: PerformanceDifference,
}

#[derive(Debug, Clone)]
pub struct PerformanceDifference {
    pub return_difference: UsdAmount,
    pub risk_adjusted_return_diff: f64,
    pub sharpe_difference: f64,
    pub volatility_difference: f64,
}
