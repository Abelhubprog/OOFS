use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use shared::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    validation::validate_pubkey,
};
use tracing::{debug, info, instrument};
use validator::Validate;

use crate::{
    dto::wallet::{
        GetWalletRequest, GetWalletResponse, CompareWalletsRequest, CompareWalletsResponse,
        WalletMetadata, PortfolioSummary, PositionDetail, ActivityItem, OofSummary,
        HoldingSummary, WalletComparisonSummary, ComparisonMetrics, WinnerAnalysis,
    },
    state::AppState,
};

/// Get detailed wallet information
#[instrument(skip(state))]
pub async fn get_wallet_info(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
    Query(request): Query<GetWalletRequest>,
) -> ApiResult<Json<GetWalletResponse>> {
    let mut request = request;
    request.wallet = wallet.clone();
    request.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;

    info!("Getting wallet info for: {} by user: {}", wallet, user.claims.sub);

    validate_pubkey(&wallet).map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    if !user.permissions.contains(&"read".to_string()) {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    let metadata = fetch_wallet_metadata(&state, &wallet).await?;
    let portfolio = fetch_portfolio_summary(&state, &wallet).await?;

    let positions = if request.include_positions.unwrap_or(true) {
        Some(fetch_wallet_positions(&state, &wallet).await?)
    } else { None };

    let metrics = if request.include_metrics.unwrap_or(true) {
        Some(calculate_wallet_metrics(&state, &wallet).await?)
    } else { None };

    let recent_activity = if request.include_activity.unwrap_or(true) {
        Some(fetch_recent_activity(&state, &wallet).await?)
    } else { None };

    let oof_summary = if request.include_oof_summary.unwrap_or(true) {
        Some(calculate_oof_summary(&state, &wallet).await?)
    } else { None };

    Ok(Json(GetWalletResponse {
        wallet,
        metadata,
        portfolio,
        positions,
        metrics,
        recent_activity,
        oof_summary,
    }))
}

/// Compare two wallets
#[instrument(skip(state))]
pub async fn compare_wallets(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<CompareWalletsRequest>,
) -> ApiResult<Json<CompareWalletsResponse>> {
    request.validate().map_err(|e| ApiError::ValidationError(e.to_string()))?;

    info!("Comparing wallets: {} vs {} by user: {}", request.wallet_a, request.wallet_b, user.claims.sub);

    if !user.permissions.contains(&"premium".to_string()) {
        return Err(ApiError::Forbidden("Premium subscription required for wallet comparison".to_string()));
    }

    let wallet_a_summary = build_wallet_comparison_summary(&state, &request.wallet_a).await?;
    let wallet_b_summary = build_wallet_comparison_summary(&state, &request.wallet_b).await?;

    let comparison = calculate_comparison_metrics(&wallet_a_summary, &wallet_b_summary);
    let winner_analysis = determine_winner(&wallet_a_summary, &wallet_b_summary, &comparison);

    let detailed_comparison = if request.include_details.unwrap_or(false) {
        Some(build_detailed_comparison(&state, &request.wallet_a, &request.wallet_b).await?)
    } else { None };

    Ok(Json(CompareWalletsResponse {
        wallet_a: wallet_a_summary,
        wallet_b: wallet_b_summary,
        comparison,
        winner_analysis,
        detailed_comparison,
    }))
}

// Helper functions with simplified implementations
async fn fetch_wallet_metadata(state: &AppState, wallet: &str) -> ApiResult<WalletMetadata> {
    debug!("Fetching metadata for wallet: {}", wallet);
    Ok(WalletMetadata {
        is_tracked: true,
        first_transaction: Some(time::OffsetDateTime::now_utc() - time::Duration::days(365)),
        last_transaction: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(2)),
        transaction_count: 1250,
        unique_tokens: 45,
        last_analyzed: Some(time::OffsetDateTime::now_utc() - time::Duration::hours(1)),
        data_freshness: 0.95,
        labels: vec![],
    })
}

async fn fetch_portfolio_summary(state: &AppState, wallet: &str) -> ApiResult<PortfolioSummary> {
    debug!("Fetching portfolio summary for wallet: {}", wallet);
    Ok(PortfolioSummary {
        total_value: shared::types::common::UsdAmount::from(75000.0),
        change_24h: shared::types::common::UsdAmount::from(1250.0),
        change_24h_pct: 1.7,
        active_positions: 8,
        largest_position_pct: 35.5,
        diversification: 0.7,
        sol_balance: shared::types::common::SolAmount::from(12.5),
        top_holdings: vec![
            HoldingSummary {
                symbol: "SOL".to_string(),
                name: "Solana".to_string(),
                mint: "So11111111111111111111111111111111111111112".to_string(),
                amount: shared::types::common::TokenAmount::from(100.0),
                price: shared::types::common::UsdAmount::from(150.0),
                value: shared::types::common::UsdAmount::from(15000.0),
                portfolio_pct: 20.0,
                change_24h_pct: 2.5,
            }
        ],
    })
}

async fn fetch_wallet_positions(state: &AppState, wallet: &str) -> ApiResult<Vec<PositionDetail>> {
    debug!("Fetching positions for wallet: {}", wallet);
    Ok(vec![])
}

async fn calculate_wallet_metrics(state: &AppState, wallet: &str) -> ApiResult<shared::types::wallet::PerformanceMetrics> {
    debug!("Calculating metrics for wallet: {}", wallet);
    Ok(shared::types::wallet::PerformanceMetrics {
        total_return: shared::types::common::UsdAmount::from(25000),
        total_return_pct: 150.0,
        sharpe_ratio: Some(1.8),
        max_drawdown: Some(0.25),
        win_rate: Some(0.65),
        avg_holding_period: Some(time::Duration::days(45)),
        total_fees: shared::types::common::SolAmount::from(2.5),
    })
}

async fn fetch_recent_activity(state: &AppState, wallet: &str) -> ApiResult<Vec<ActivityItem>> {
    debug!("Fetching recent activity for wallet: {}", wallet);
    Ok(vec![])
}

async fn calculate_oof_summary(state: &AppState, wallet: &str) -> ApiResult<OofSummary> {
    use crate::dto::wallet::{
        ImprovementSuggestion, SuggestionCategory, Priority,
    };
    use std::collections::HashMap;

    debug!("Calculating OOF summary for wallet: {}", wallet);
    Ok(OofSummary {
        oof_score: 75.5,
        oof_rank: "Top 25%".to_string(),
        total_moments: 15,
        total_value_lost: shared::types::common::UsdAmount::from(50000.0),
        most_common_type: shared::types::moment::MomentKind::SoldTooEarly,
        recent_moments: 3,
        moments_by_type: HashMap::new(),
        moments_by_severity: HashMap::new(),
        efficiency_score: 33.3,
        suggestions: vec![
            ImprovementSuggestion {
                category: SuggestionCategory::Timing,
                priority: Priority::High,
                title: "Hold longer during volatility".to_string(),
                description: "Consider holding positions longer during market volatility.".to_string(),
                potential_value_saved: Some(shared::types::common::UsdAmount::from(15000.0)),
                addresses_moment_types: vec![shared::types::moment::MomentKind::SoldTooEarly],
            }
        ],
    })
}

async fn build_wallet_comparison_summary(state: &AppState, wallet: &str) -> ApiResult<WalletComparisonSummary> {
    debug!("Building comparison summary for wallet: {}", wallet);
    Ok(WalletComparisonSummary {
        wallet: wallet.to_string(),
        portfolio_value: shared::types::common::UsdAmount::from(75000.0),
        total_return_pct: 150.0,
        oof_score: 65.5,
        oof_moments: 12,
        value_lost: shared::types::common::UsdAmount::from(25000.0),
        efficiency_score: 45.2,
    })
}

fn calculate_comparison_metrics(wallet_a: &WalletComparisonSummary, wallet_b: &WalletComparisonSummary) -> ComparisonMetrics {
    use crate::dto::wallet::{
        PerformanceComparison, OofComparison, RiskComparison, StrategyComparison,
    };

    ComparisonMetrics {
        performance: PerformanceComparison {
            return_difference_pct: wallet_a.total_return_pct - wallet_b.total_return_pct,
            value_difference: shared::types::common::UsdAmount::from(
                wallet_a.portfolio_value.0 - wallet_b.portfolio_value.0
            ),
            winner: if wallet_a.total_return_pct > wallet_b.total_return_pct {
                wallet_a.wallet.clone()
            } else {
                wallet_b.wallet.clone()
            },
            explanation: "Better timing on major trades".to_string(),
        },
        oof_comparison: OofComparison {
            score_difference: wallet_a.oof_score - wallet_b.oof_score,
            moments_difference: wallet_a.oof_moments as i32 - wallet_b.oof_moments as i32,
            value_lost_difference: shared::types::common::UsdAmount::from(
                wallet_a.value_lost.0 - wallet_b.value_lost.0
            ),
            better_wallet: if wallet_a.oof_score < wallet_b.oof_score {
                wallet_a.wallet.clone()
            } else {
                wallet_b.wallet.clone()
            },
            key_differences: vec!["Different trading frequency".to_string()],
        },
        risk_comparison: RiskComparison {
            diversification_difference: 0.15,
            volatility_difference: -0.25,
            more_conservative: wallet_b.wallet.clone(),
            analysis: "Wallet B shows better risk management".to_string(),
        },
        strategy_comparison: StrategyComparison {
            trading_frequency: "Wallet A trades 3x more frequently".to_string(),
            holding_periods: "Wallet B holds positions 2x longer on average".to_string(),
            token_preferences: vec!["Different risk appetites".to_string()],
            insights: vec!["Contrasting investment styles".to_string()],
        },
    }
}

fn determine_winner(wallet_a: &WalletComparisonSummary, wallet_b: &WalletComparisonSummary, comparison: &ComparisonMetrics) -> WinnerAnalysis {
    WinnerAnalysis {
        overall_winner: comparison.performance.winner.clone(),
        winning_margin: format!("{:.1}% better returns", comparison.performance.return_difference_pct.abs()),
        success_factors: vec!["Better timing".to_string(), "Lower fees".to_string()],
        lessons_for_loser: vec!["Improve entry timing".to_string()],
        confidence: 0.85,
    }
}

async fn build_detailed_comparison(state: &AppState, wallet_a: &str, wallet_b: &str) -> ApiResult<crate::dto::wallet::DetailedComparison> {
    use crate::dto::wallet::{
        DetailedComparison, MonthlyComparison, TokenComparison, OofTimelineItem,
        StatisticalAnalysis,
    };
    use std::collections::HashMap;

    debug!("Building detailed comparison between {} and {}", wallet_a, wallet_b);
    Ok(DetailedComparison {
        monthly_performance: vec![
            MonthlyComparison {
                month: "2024-01".to_string(),
                wallet_a_return: 15.5,
                wallet_b_return: 12.3,
                difference: 3.2,
                winner: wallet_a.to_string(),
            }
        ],
        token_comparison: HashMap::new(),
        oof_timeline: vec![],
        statistical_analysis: StatisticalAnalysis {
            correlation: 0.65,
            p_value: 0.025,
            confidence_interval: (-2.5, 8.5),
            sample_size: 365,
            reliability: "High".to_string(),
        },
    })
}
