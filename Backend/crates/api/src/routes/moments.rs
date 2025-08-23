use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use shared::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    types::{
        moment::{Moment, MomentKind, MomentSeverity},
        common::{UsdAmount, TokenAmount},
    },
    validation::validate_pubkey,
};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::moment::{
        MomentsListRequest, MomentsListResponse, GetMomentRequest, GetMomentResponse,
        MomentStatsRequest, MomentStatsResponse, MomentDetail, DetailedContext,
        TransactionInfo, MomentAnalysis, PaginationInfo, MomentsSummary, AppliedFilters,
        SortInfo, DateRange, SimilarMoment, SharingOptions, OverallStats, GroupStats,
        TrendingMoment, StatsPeriod, PositionInfo, MarketConditions, PortfolioContext,
        AlternativeAction, HistoricalParallel, TransactionRole,
    },
    state::AppState,
};

/// List moments for a wallet with filtering and pagination
#[instrument(skip(state))]
pub async fn list_wallet_moments(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
    Query(request): Query<MomentsListRequest>,
) -> ApiResult<Json<MomentsListResponse>> {
    // Validate request
    let mut request = request;
    request.wallet = wallet.clone();
    request.validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    info!("Listing moments for wallet: {} by user: {}", wallet, user.claims.sub);

    // Validate wallet address
    validate_pubkey(&wallet)
        .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    // Check user permissions
    if !user.permissions.contains(&"read".to_string()) {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    // Get moments from database with filters
    let moments = fetch_wallet_moments(&state, &request).await?;

    // Get total count for pagination
    let total_count = count_wallet_moments(&state, &request).await?;

    // Generate summary
    let summary = generate_moments_summary(&moments, total_count).await?;

    // Create pagination info
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(50);
    let pagination = PaginationInfo {
        offset,
        limit,
        total: total_count,
        has_more: offset + moments.len() as u32 < total_count,
        next_offset: if offset + moments.len() as u32 < total_count {
            Some(offset + limit)
        } else {
            None
        },
    };

    // Create applied filters info
    let filters = AppliedFilters {
        moment_types: request.moment_types,
        severity_range: if request.min_severity.is_some() || request.max_severity.is_some() {
            Some((request.min_severity.unwrap_or(MomentSeverity::Low),
                  request.max_severity.unwrap_or(MomentSeverity::Critical)))
        } else {
            None
        },
        date_range: if request.start_date.is_some() || request.end_date.is_some() {
            Some((request.start_date.unwrap_or_else(|| OffsetDateTime::now_utc() - time::Duration::days(365)),
                  request.end_date.unwrap_or_else(OffsetDateTime::now_utc)))
        } else {
            None
        },
        min_value_lost: request.min_value_lost,
        token_symbol: request.token_symbol,
        sort: SortInfo {
            field: request.sort_by.unwrap_or_default(),
            order: request.sort_order.unwrap_or_default(),
        },
    };

    Ok(Json(MomentsListResponse {
        wallet,
        moments,
        pagination,
        summary,
        filters,
    }))
}

/// Get detailed information about a specific moment
#[instrument(skip(state))]
pub async fn get_moment_details(
    State(state): State<AppState>,
    user: AuthUser,
    Path(moment_id): Path<String>,
    Query(request): Query<GetMomentRequest>,
) -> ApiResult<Json<GetMomentResponse>> {
    info!("Getting moment details for: {} by user: {}", moment_id, user.claims.sub);

    // Check user permissions
    if !user.permissions.contains(&"read".to_string()) {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    // Fetch moment from database
    let moment = fetch_moment_by_id(&state, &moment_id).await?
        .ok_or_else(|| ApiError::NotFound("Moment not found".to_string()))?;

    // Build detailed moment info
    let moment_detail = build_moment_detail(&state, moment, &request).await?;

    // Get similar moments if requested
    let similar_moments = if request.include_similar.unwrap_or(false) {
        Some(find_similar_moments(&state, &moment_detail).await?)
    } else {
        None
    };

    // Generate sharing options
    let sharing = generate_sharing_options(&moment_detail);

    Ok(Json(GetMomentResponse {
        moment: moment_detail,
        similar_moments,
        sharing,
    }))
}

/// Get moment statistics
#[instrument(skip(state))]
pub async fn get_moment_statistics(
    State(state): State<AppState>,
    user: AuthUser,
    Query(request): Query<MomentStatsRequest>,
) -> ApiResult<Json<MomentStatsResponse>> {
    info!("Getting moment statistics by user: {}", user.claims.sub);

    // Validate request if wallet is provided
    if let Some(wallet) = &request.wallet {
        validate_pubkey(wallet)
            .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;
    }

    // Check user permissions
    if !user.permissions.contains(&"read".to_string()) {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    // Calculate statistics
    let overall = calculate_overall_stats(&state, &request).await?;
    let groups = if request.group_by.is_some() {
        Some(calculate_grouped_stats(&state, &request).await?)
    } else {
        None
    };
    let trending = calculate_trending_moments(&state, &request).await?;
    let period = parse_stats_period(&request);

    Ok(Json(MomentStatsResponse {
        overall,
        groups,
        trending,
        period,
    }))
}

/// Share a moment (create shareable link)
#[instrument(skip(state))]
pub async fn share_moment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(moment_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    info!("Sharing moment: {} by user: {}", moment_id, user.claims.sub);

    // Verify moment exists and user has access
    let moment = fetch_moment_by_id(&state, &moment_id).await?
        .ok_or_else(|| ApiError::NotFound("Moment not found".to_string()))?;

    // Generate share token
    let share_token = generate_share_token(&moment_id);

    // Store share info in database/cache
    store_share_info(&state, &moment_id, &share_token, &user.claims.sub).await?;

    // Generate card image URL
    let card_url = format!(
        "{}/api/v1/cards/moments/{}.png",
        state.config.api.base_url,
        moment_id
    );

    // Generate sharing URLs
    let web_url = format!(
        "{}/moments/{}?share={}",
        state.config.app.frontend_url,
        moment_id,
        share_token
    );

    let twitter_url = format!(
        "https://twitter.com/intent/tweet?text=Check%20out%20this%20OOF%20moment!&url={}&hashtags=OOF,Solana,Crypto",
        urlencoding::encode(&web_url)
    );

    Ok(Json(serde_json::json!({
        "share_token": share_token,
        "web_url": web_url,
        "card_url": card_url,
        "twitter_url": twitter_url,
        "expires_at": OffsetDateTime::now_utc() + time::Duration::days(30)
    })))
}

/// Get top moments (leaderboard style)
#[instrument(skip(state))]
pub async fn get_top_moments(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    info!("Getting top moments by user: {}", user.claims.sub);

    // Parse query parameters
    let period = params.get("period").unwrap_or(&"7d".to_string()).clone();
    let moment_type = params.get("type")
        .and_then(|t| serde_json::from_str::<MomentKind>(&format!("\"{}\"", t)).ok());
    let limit = params.get("limit")
        .and_then(|l| l.parse::<u32>().ok())
        .unwrap_or(10)
        .min(100);

    // Fetch top moments
    let top_moments = fetch_top_moments(&state, &period, moment_type, limit).await?;

    Ok(Json(serde_json::json!({
        "period": period,
        "moment_type": moment_type,
        "limit": limit,
        "moments": top_moments,
        "generated_at": OffsetDateTime::now_utc()
    })))
}

/// Export moments data
#[instrument(skip(state))]
pub async fn export_moments(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> ApiResult<Json<serde_json::Value>> {
    info!("Exporting moments for wallet: {} by user: {}", wallet, user.claims.sub);

    // Validate wallet
    validate_pubkey(&wallet)
        .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    // Check premium permissions for export
    if !user.permissions.contains(&"premium".to_string()) {
        return Err(ApiError::Forbidden("Premium subscription required for exports".to_string()));
    }

    let format = params.get("format").unwrap_or(&"json".to_string()).clone();
    let include_transactions = params.get("include_transactions")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    // Generate export
    let export_id = start_moments_export(&state, &wallet, &format, include_transactions, &user.claims.sub).await?;

    Ok(Json(serde_json::json!({
        "export_id": export_id,
        "status": "started",
        "format": format,
        "estimated_completion": OffsetDateTime::now_utc() + time::Duration::minutes(5)
    })))
}

// Helper functions

async fn fetch_wallet_moments(
    state: &AppState,
    request: &MomentsListRequest,
) -> ApiResult<Vec<MomentDetail>> {
    // This would query the database for moments
    // Mock implementation for now
    debug!("Fetching moments for wallet: {}", request.wallet);

    // In real implementation, this would:
    // 1. Build SQL query with filters
    // 2. Apply sorting and pagination
    // 3. Fetch moments with optional context/transactions
    // 4. Convert to MomentDetail objects

    Ok(vec![])
}

async fn count_wallet_moments(
    state: &AppState,
    request: &MomentsListRequest,
) -> ApiResult<u32> {
    // Count total moments matching filters
    debug!("Counting moments for wallet: {}", request.wallet);
    Ok(0)
}

async fn generate_moments_summary(
    moments: &[MomentDetail],
    total_count: u32,
) -> ApiResult<MomentsSummary> {
    let moments_by_type = moments.iter()
        .fold(HashMap::new(), |mut acc, moment| {
            *acc.entry(moment.moment.moment_type.clone()).or_insert(0) += 1;
            acc
        });

    let moments_by_severity = moments.iter()
        .fold(HashMap::new(), |mut acc, moment| {
            *acc.entry(moment.moment.severity.clone()).or_insert(0) += 1;
            acc
        });

    let total_value_lost = moments.iter()
        .map(|m| m.moment.value_lost_usd.0)
        .sum::<f64>()
        .into();

    let avg_value_lost = if total_count > 0 {
        UsdAmount::from(total_value_lost.0 / total_count as f64)
    } else {
        UsdAmount::from(0.0)
    };

    let most_common_type = moments_by_type.iter()
        .max_by_key(|(_, count)| *count)
        .map(|(kind, _)| kind.clone())
        .unwrap_or(MomentKind::SoldTooEarly);

    // Calculate date range
    let earliest = moments.iter()
        .map(|m| m.moment.timestamp)
        .min()
        .unwrap_or_else(OffsetDateTime::now_utc);

    let latest = moments.iter()
        .map(|m| m.moment.timestamp)
        .max()
        .unwrap_or_else(OffsetDateTime::now_utc);

    let days_spanned = (latest - earliest).whole_days().max(0) as u32;

    Ok(MomentsSummary {
        total_moments: total_count,
        by_type: moments_by_type,
        by_severity: moments_by_severity,
        total_value_lost,
        avg_value_lost,
        most_common_type,
        date_range: DateRange {
            earliest,
            latest,
            days_spanned,
        },
    })
}

async fn fetch_moment_by_id(
    state: &AppState,
    moment_id: &str,
) -> ApiResult<Option<Moment>> {
    debug!("Fetching moment by ID: {}", moment_id);

    // Query database for moment
    // Mock implementation
    Ok(None)
}

async fn build_moment_detail(
    state: &AppState,
    moment: Moment,
    request: &GetMomentRequest,
) -> ApiResult<MomentDetail> {
    let context = if request.include_context.unwrap_or(true) {
        Some(build_detailed_context(state, &moment).await?)
    } else {
        None
    };

    let transactions = if request.include_transactions.unwrap_or(true) {
        Some(fetch_moment_transactions(state, &moment).await?)
    } else {
        None
    };

    let card_url = format!(
        "{}/api/v1/cards/moments/{}.png",
        state.config.api.base_url,
        moment.id
    );

    let analysis = generate_moment_analysis(&moment);

    Ok(MomentDetail {
        id: moment.id.clone(),
        moment,
        context,
        transactions,
        card_url,
        analysis,
    })
}

async fn build_detailed_context(
    state: &AppState,
    moment: &Moment,
) -> ApiResult<DetailedContext> {
    // Fetch position information before/after the moment
    let position_before = get_position_at_time(state, &moment.wallet_address, moment.timestamp - time::Duration::hours(1)).await?;
    let position_after = get_position_at_time(state, &moment.wallet_address, moment.timestamp + time::Duration::hours(1)).await?;

    // Get market conditions
    let market_conditions = get_market_conditions(state, &moment.token_mint, moment.timestamp).await?;

    // Get portfolio context
    let portfolio_context = get_portfolio_context(state, &moment.wallet_address, moment.timestamp).await?;

    // Generate alternative action suggestion
    let alternative_action = generate_alternative_action(moment, &market_conditions);

    Ok(DetailedContext {
        position_before,
        position_after,
        market_conditions,
        portfolio_context,
        alternative_action,
    })
}

async fn fetch_moment_transactions(
    state: &AppState,
    moment: &Moment,
) -> ApiResult<Vec<TransactionInfo>> {
    debug!("Fetching transactions for moment: {}", moment.id);

    // Fetch related transactions from database
    // Mock implementation
    Ok(vec![])
}

fn generate_moment_analysis(moment: &Moment) -> MomentAnalysis {
    let what_happened = format!(
        "You {} {} {} at ${:.2}, but the price later reached ${:.2}",
        if matches!(moment.moment_type, MomentKind::SoldTooEarly) { "sold" } else { "bought" },
        moment.amount.0,
        moment.token_symbol,
        moment.price_at_time.0,
        moment.price_at_time.0 * 2.0 // Mock calculation
    );

    let why_oof = match moment.moment_type {
        MomentKind::SoldTooEarly => "The token price continued to rise significantly after your sale".to_string(),
        MomentKind::BagHolderDrawdown => "You held while the token price declined substantially".to_string(),
        MomentKind::BadRoute => "A better trading route was available that would have saved fees".to_string(),
        MomentKind::IdleYield => "Your tokens could have been earning yield through staking or lending".to_string(),
        _ => "Market conditions changed unfavorably after your transaction".to_string(),
    };

    let contributing_factors = vec![
        "Market volatility".to_string(),
        "Timing of transaction".to_string(),
        "Lack of market analysis".to_string(),
    ];

    let lessons = vec![
        "Consider dollar-cost averaging for large positions".to_string(),
        "Set stop-losses to limit downside risk".to_string(),
        "Research market conditions before trading".to_string(),
    ];

    let emotional_impact = match moment.severity {
        MomentSeverity::Low => 3.0,
        MomentSeverity::Medium => 5.5,
        MomentSeverity::High => 7.5,
        MomentSeverity::Critical => 9.0,
    };

    MomentAnalysis {
        what_happened,
        why_oof,
        contributing_factors,
        lessons,
        emotional_impact,
        historical_parallels: vec![],
    }
}

async fn find_similar_moments(
    state: &AppState,
    moment: &MomentDetail,
) -> ApiResult<Vec<SimilarMoment>> {
    debug!("Finding similar moments to: {}", moment.id);

    // Query database for similar moments based on:
    // - Same moment type
    // - Similar token
    // - Similar value lost
    // - Similar time period

    // Mock implementation
    Ok(vec![])
}

fn generate_sharing_options(moment: &MomentDetail) -> SharingOptions {
    let card_url = moment.card_url.clone();
    let web_url = format!("https://oof.com/moments/{}", moment.id);

    let tweet_text = format!(
        "Just discovered my {} OOF moment! Lost ${:.0} on {}. Check out the details:",
        moment.moment.moment_type.to_string().replace("_", " "),
        moment.moment.value_lost_usd.0,
        moment.moment.token_symbol
    );

    let twitter_url = format!(
        "https://twitter.com/intent/tweet?text={}&url={}&hashtags=OOF,Solana,Crypto",
        urlencoding::encode(&tweet_text),
        urlencoding::encode(&web_url)
    );

    SharingOptions {
        twitter_url,
        discord_url: format!("https://discord.com/channels/@me"), // Placeholder
        card_image_url: card_url,
        web_url,
    }
}

// Additional helper functions would be implemented here...
// These would handle:
// - calculate_overall_stats
// - calculate_grouped_stats
// - calculate_trending_moments
// - parse_stats_period
// - generate_share_token
// - store_share_info
// - fetch_top_moments
// - start_moments_export
// - get_position_at_time
// - get_market_conditions
// - get_portfolio_context
// - generate_alternative_action

// Placeholder implementations for now
async fn calculate_overall_stats(state: &AppState, request: &MomentStatsRequest) -> ApiResult<OverallStats> {
    Ok(OverallStats {
        total_moments: 50000,
        total_value_lost: UsdAmount::from(10000000.0),
        avg_value_lost: UsdAmount::from(200.0),
        most_common_type: MomentKind::SoldTooEarly,
        growth_pct: 15.5,
    })
}

async fn calculate_grouped_stats(state: &AppState, request: &MomentStatsRequest) -> ApiResult<HashMap<String, GroupStats>> {
    let mut groups = HashMap::new();
    groups.insert("SoldTooEarly".to_string(), GroupStats {
        count: 150,
        total_value_lost: UsdAmount::from(75000.0),
        avg_value_lost: UsdAmount::from(500.0),
        percentage: 25.5,
    });
    Ok(groups)
}

async fn calculate_trending_moments(state: &AppState, request: &MomentStatsRequest) -> ApiResult<Vec<TrendingMoment>> {
    Ok(vec![
        TrendingMoment {
            moment_type: MomentKind::SoldTooEarly,
            recent_count: 45,
            growth_pct: 125.5,
            avg_value_lost: UsdAmount::from(850.0),
        }
    ])
}

fn parse_stats_period(request: &MomentStatsRequest) -> StatsPeriod {
    let period = request.period.as_deref().unwrap_or("30d");
    let (start, description) = match period {
        "7d" => (OffsetDateTime::now_utc() - time::Duration::days(7), "Last 7 days"),
        "30d" => (OffsetDateTime::now_utc() - time::Duration::days(30), "Last 30 days"),
        "90d" => (OffsetDateTime::now_utc() - time::Duration::days(90), "Last 90 days"),
        _ => (OffsetDateTime::now_utc() - time::Duration::days(30), "Last 30 days"),
    };

    StatsPeriod {
        start,
        end: OffsetDateTime::now_utc(),
        description: description.to_string(),
    }
}

fn generate_share_token(moment_id: &str) -> String {
    format!("share_{}_{}", moment_id, Uuid::new_v4().to_string()[..8].to_string())
}

async fn store_share_info(state: &AppState, moment_id: &str, share_token: &str, user_id: &str) -> ApiResult<()> {
    debug!("Storing share info for moment: {} with token: {}", moment_id, share_token);
    Ok(())
}

async fn fetch_top_moments(state: &AppState, period: &str, moment_type: Option<MomentKind>, limit: u32) -> ApiResult<Vec<serde_json::Value>> {
    debug!("Fetching top {} moments for period: {:?}", limit, period);
    Ok(vec![])
}

async fn start_moments_export(state: &AppState, wallet: &str, format: &str, include_transactions: bool, user_id: &str) -> ApiResult<String> {
    let export_id = Uuid::new_v4().to_string();
    debug!("Starting export {} for wallet: {} in format: {}", export_id, wallet, format);
    Ok(export_id)
}

async fn get_position_at_time(state: &AppState, wallet: &str, timestamp: OffsetDateTime) -> ApiResult<Option<PositionInfo>> {
    Ok(None)
}

async fn get_market_conditions(state: &AppState, token_mint: &str, timestamp: OffsetDateTime) -> ApiResult<MarketConditions> {
    Ok(MarketConditions {
        price_at_moment: UsdAmount::from(50.0),
        price_1h_later: Some(UsdAmount::from(52.0)),
        price_24h_later: Some(UsdAmount::from(65.0)),
        price_7d_later: Some(UsdAmount::from(85.0)),
        volume_24h: Some(UsdAmount::from(10000000.0)),
        market_cap: Some(UsdAmount::from(1000000000.0)),
        volatility: Some(0.15),
    })
}

async fn get_portfolio_context(state: &AppState, wallet: &str, timestamp: OffsetDateTime) -> ApiResult<PortfolioContext> {
    Ok(PortfolioContext {
        total_value: UsdAmount::from(50000.0),
        portfolio_pct: 15.0,
        other_positions: 12,
        diversification: 0.7,
    })
}

fn generate_alternative_action(moment: &Moment, market_conditions: &MarketConditions) -> Option<AlternativeAction> {
    Some(AlternativeAction {
        action: "Hold for 30 more days".to_string(),
        potential_value: UsdAmount::from(15000.0),
        confidence: 0.85,
        reasoning: "Strong technical indicators suggested continued uptrend".to_string(),
    })
}
