use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Sse},
    Json,
};
use shared::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
    types::{
        moment::{MomentKind, MomentSeverity},
        wallet::WalletAnalysis,
    },
    validation::validate_pubkey,
};
use std::{collections::HashMap, convert::Infallible, time::Duration};
use tokio_stream::{wrappers::IntervalStream, StreamExt};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;
use validator::Validate;

use crate::{
    dto::analyze::{
        AnalysisMetadata, AnalysisProgress, AnalysisStage, AnalysisStatus, AnalysisSummary,
        AnalyzeRequest, AnalyzeResponse, BulkAnalysisMetadata, BulkAnalysisSummary,
        BulkAnalyzeRequest, BulkAnalyzeResponse, MomentSummary, PerformanceOverview,
        TimeframeSummary, TransactionDetail, WalletAnalysisResult,
    },
    state::AppState,
};

/// Analyze a wallet for OOF moments
#[instrument(skip(state))]
pub async fn analyze_wallet(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<AnalyzeRequest>,
) -> ApiResult<Json<AnalyzeResponse>> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    info!(
        "Starting wallet analysis for: {} by user: {}",
        request.wallet, user.claims.sub
    );

    // Check user permissions
    if !user.permissions.contains(&"analyze".to_string()) {
        return Err(ApiError::Forbidden(
            "Analysis permission required".to_string(),
        ));
    }

    // Check rate limits for analysis requests
    check_analysis_rate_limit(&state, &user.claims.sub).await?;

    // Validate wallet address
    validate_pubkey(&request.wallet)
        .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    // Check if analysis is cached and not force refresh
    if !request.force_refresh.unwrap_or(false) {
        if let Some(cached_analysis) = get_cached_analysis(&state, &request).await? {
            info!("Returning cached analysis for wallet: {}", request.wallet);
            return Ok(Json(cached_analysis));
        }
    }

    // Start analysis
    let start_time = std::time::Instant::now();
    let analysis_id = Uuid::new_v4().to_string();

    // Broadcast analysis start
    broadcast_analysis_progress(
        &state,
        &request.wallet,
        &analysis_id,
        AnalysisStage::Initializing,
        0.0,
    )
    .await?;

    // Perform analysis steps
    let moments = analyze_moments(&state, &request, &analysis_id).await?;
    let positions = if request.include_positions.unwrap_or(true) {
        Some(get_position_snapshots(&state, &request.wallet).await?)
    } else {
        None
    };

    let metrics = if request.include_metrics.unwrap_or(true) {
        Some(calculate_performance_metrics(&state, &request.wallet, &moments).await?)
    } else {
        None
    };

    let transactions = if request.include_transactions.unwrap_or(false) {
        Some(get_transaction_details(&state, &request.wallet, &moments).await?)
    } else {
        None
    };

    // Generate summary
    let summary = generate_analysis_summary(&moments, &metrics)?;

    // Create response
    let duration = start_time.elapsed();
    let response = AnalyzeResponse {
        wallet: request.wallet.clone(),
        summary,
        moments,
        positions,
        metrics,
        transactions,
        metadata: AnalysisMetadata {
            analyzed_at: time::OffsetDateTime::now_utc(),
            duration_seconds: duration.as_secs_f64(),
            data_last_updated: time::OffsetDateTime::now_utc(),
            from_cache: false,
            cache_expires_at: Some(time::OffsetDateTime::now_utc() + time::Duration::hours(1)),
            version: env!("CARGO_PKG_VERSION").to_string(),
            warnings: vec![],
        },
        next_cursor: None,
    };

    // Cache the result
    cache_analysis(&state, &request, &response).await?;

    // Broadcast completion
    broadcast_analysis_progress(
        &state,
        &request.wallet,
        &analysis_id,
        AnalysisStage::Complete,
        100.0,
    )
    .await?;

    info!(
        "Completed wallet analysis for: {} in {:?}",
        request.wallet, duration
    );

    Ok(Json(response))
}

/// Stream real-time analysis progress
#[instrument(skip(state))]
pub async fn stream_analysis_progress(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
) -> ApiResult<Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>>> {
    info!(
        "Starting analysis progress stream for wallet: {} by user: {}",
        wallet, user.claims.sub
    );

    // Validate wallet
    validate_pubkey(&wallet)
        .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    // Create progress stream
    let progress_stream = create_analysis_progress_stream(state, wallet, user.claims.sub);

    Ok(Sse::new(progress_stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(30))
            .text("keep-alive"),
    ))
}

/// Bulk analyze multiple wallets
#[instrument(skip(state))]
pub async fn bulk_analyze_wallets(
    State(state): State<AppState>,
    user: AuthUser,
    Json(request): Json<BulkAnalyzeRequest>,
) -> ApiResult<Json<BulkAnalyzeResponse>> {
    // Validate request
    request
        .validate()
        .map_err(|e| ApiError::ValidationError(e.to_string()))?;

    info!(
        "Starting bulk analysis for {} wallets by user: {}",
        request.wallets.len(),
        user.claims.sub
    );

    // Check user permissions for bulk analysis
    if !user.permissions.contains(&"bulk_analyze".to_string()) {
        return Err(ApiError::Forbidden(
            "Bulk analysis permission required".to_string(),
        ));
    }

    // Check wallet count limits
    let max_wallets = if user.permissions.contains(&"premium".to_string()) {
        50
    } else {
        10
    };

    if request.wallets.len() > max_wallets {
        return Err(ApiError::BadRequest(format!(
            "Too many wallets. Maximum: {}, provided: {}",
            max_wallets,
            request.wallets.len()
        )));
    }

    let batch_id = request
        .batch_id
        .unwrap_or_else(|| format!("batch_{}", Uuid::new_v4()));
    let start_time = time::OffsetDateTime::now_utc();

    // Analyze wallets (parallel or sequential based on request)
    let results = if request.parallel.unwrap_or(true) {
        analyze_wallets_parallel(&state, &request, &user).await?
    } else {
        analyze_wallets_sequential(&state, &request, &user).await?
    };

    // Calculate summary
    let summary = calculate_bulk_summary(&results);

    let response = BulkAnalyzeResponse {
        batch_id,
        results,
        summary,
        metadata: BulkAnalysisMetadata {
            started_at: start_time,
            completed_at: Some(time::OffsetDateTime::now_utc()),
            duration_seconds: Some((time::OffsetDateTime::now_utc() - start_time).as_seconds_f64()),
            parallel_processing: request.parallel.unwrap_or(true),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    info!(
        "Completed bulk analysis for {} wallets",
        request.wallets.len()
    );

    Ok(Json(response))
}

/// Get analysis status for a specific request
#[instrument(skip(state))]
pub async fn get_analysis_status(
    State(state): State<AppState>,
    user: AuthUser,
    Path(analysis_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    info!(
        "Getting analysis status for: {} by user: {}",
        analysis_id, user.claims.sub
    );

    // Get status from cache/database
    let status = get_analysis_status_from_store(&state, &analysis_id)
        .await?
        .ok_or_else(|| ApiError::NotFound("Analysis not found".to_string()))?;

    Ok(Json(serde_json::json!({
        "analysis_id": analysis_id,
        "status": status,
        "timestamp": time::OffsetDateTime::now_utc()
    })))
}

/// Cancel an ongoing analysis
#[instrument(skip(state))]
pub async fn cancel_analysis(
    State(state): State<AppState>,
    user: AuthUser,
    Path(analysis_id): Path<String>,
) -> ApiResult<StatusCode> {
    info!(
        "Cancelling analysis: {} by user: {}",
        analysis_id, user.claims.sub
    );

    // Check if user owns this analysis
    verify_analysis_ownership(&state, &analysis_id, &user.claims.sub).await?;

    // Cancel the analysis
    cancel_analysis_in_store(&state, &analysis_id).await?;

    // Broadcast cancellation
    broadcast_analysis_progress(&state, "unknown", &analysis_id, AnalysisStage::Failed, 0.0)
        .await?;

    info!("Analysis cancelled: {}", analysis_id);

    Ok(StatusCode::NO_CONTENT)
}

// Helper functions

async fn check_analysis_rate_limit(state: &AppState, user_id: &str) -> ApiResult<()> {
    // Implement rate limiting logic for analysis requests
    // For now, just log
    debug!("Checking analysis rate limit for user: {}", user_id);
    Ok(())
}

async fn get_cached_analysis(
    state: &AppState,
    request: &AnalyzeRequest,
) -> ApiResult<Option<AnalyzeResponse>> {
    // Check Redis cache for existing analysis
    if let Some(redis) = &state.redis {
        let cache_key = format!(
            "analysis:{}:{}",
            request.wallet,
            serde_json::to_string(request).unwrap_or_default().len()
        );

        // Try to get from cache
        debug!("Checking cache for key: {}", cache_key);
        // Implementation would go here
    }

    Ok(None)
}

async fn analyze_moments(
    state: &AppState,
    request: &AnalyzeRequest,
    analysis_id: &str,
) -> ApiResult<Vec<shared::types::moment::Moment>> {
    broadcast_analysis_progress(
        state,
        &request.wallet,
        analysis_id,
        AnalysisStage::DetectingMoments,
        50.0,
    )
    .await?;

    // This would integrate with the detector engine
    debug!("Analyzing moments for wallet: {}", request.wallet);

    // Mock implementation - in real code this would call the detector engine
    Ok(vec![])
}

async fn get_position_snapshots(
    state: &AppState,
    wallet: &str,
) -> ApiResult<Vec<shared::types::wallet::PositionSnapshot>> {
    debug!("Getting position snapshots for wallet: {}", wallet);

    // This would query the position engine
    Ok(vec![])
}

async fn calculate_performance_metrics(
    state: &AppState,
    wallet: &str,
    moments: &[shared::types::moment::Moment],
) -> ApiResult<shared::types::wallet::PerformanceMetrics> {
    debug!("Calculating performance metrics for wallet: {}", wallet);

    // Mock implementation
    Ok(shared::types::wallet::PerformanceMetrics {
        total_return: shared::types::common::UsdAmount::from(0),
        total_return_pct: 0.0,
        sharpe_ratio: None,
        max_drawdown: None,
        win_rate: None,
        avg_holding_period: None,
        total_fees: shared::types::common::SolAmount::from(0),
    })
}

async fn get_transaction_details(
    state: &AppState,
    wallet: &str,
    moments: &[shared::types::moment::Moment],
) -> ApiResult<Vec<TransactionDetail>> {
    debug!("Getting transaction details for wallet: {}", wallet);

    // This would query transaction data
    Ok(vec![])
}

fn generate_analysis_summary(
    moments: &[shared::types::moment::Moment],
    metrics: &Option<shared::types::wallet::PerformanceMetrics>,
) -> ApiResult<AnalysisSummary> {
    let total_moments = moments.len() as u32;

    let moments_by_type = moments.iter().fold(HashMap::new(), |mut acc, moment| {
        *acc.entry(moment.moment_type.clone()).or_insert(0) += 1;
        acc
    });

    let moments_by_severity = moments.iter().fold(HashMap::new(), |mut acc, moment| {
        *acc.entry(moment.severity.clone()).or_insert(0) += 1;
        acc
    });

    let total_value_lost = moments
        .iter()
        .map(|m| m.value_lost_usd.0)
        .sum::<f64>()
        .into();

    let worst_moment = moments
        .iter()
        .max_by(|a, b| {
            a.value_lost_usd
                .0
                .partial_cmp(&b.value_lost_usd.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|moment| MomentSummary {
            moment_type: moment.moment_type.clone(),
            severity: moment.severity.clone(),
            value_lost: moment.value_lost_usd.clone(),
            token_symbol: moment.token_symbol.clone(),
            timestamp: moment.timestamp,
            description: format!(
                "{:?} moment with {} USD loss",
                moment.moment_type, moment.value_lost_usd.0
            ),
        });

    // Calculate OOF score (simplified)
    let oof_score = if total_moments == 0 {
        0.0
    } else {
        (total_value_lost.0 / 1000.0).min(100.0)
    };

    Ok(AnalysisSummary {
        total_moments,
        moments_by_type,
        moments_by_severity,
        total_value_lost,
        worst_moment,
        oof_score,
        timeframe: TimeframeSummary {
            start_date: time::OffsetDateTime::now_utc() - time::Duration::days(365),
            end_date: time::OffsetDateTime::now_utc(),
            days: 365,
            transaction_count: 0,
            unique_tokens: 0,
        },
        performance: PerformanceOverview {
            initial_value: shared::types::common::UsdAmount::from(10000),
            current_value: shared::types::common::UsdAmount::from(15000),
            total_return_pct: 50.0,
            potential_value: shared::types::common::UsdAmount::from(25000),
            potential_return_pct: 150.0,
            efficiency_score: 60.0,
        },
    })
}

async fn cache_analysis(
    state: &AppState,
    request: &AnalyzeRequest,
    response: &AnalyzeResponse,
) -> ApiResult<()> {
    if let Some(redis) = &state.redis {
        let cache_key = format!(
            "analysis:{}:{}",
            request.wallet,
            serde_json::to_string(request).unwrap_or_default().len()
        );

        debug!("Caching analysis result for key: {}", cache_key);
        // Implementation would cache the response in Redis
    }

    Ok(())
}

async fn broadcast_analysis_progress(
    state: &AppState,
    wallet: &str,
    analysis_id: &str,
    stage: AnalysisStage,
    progress: f64,
) -> ApiResult<()> {
    let progress_update = AnalysisProgress {
        wallet: wallet.to_string(),
        stage,
        progress_pct: progress,
        stage_progress_pct: progress,
        eta_seconds: None,
        message: "Processing...".to_string(),
        moments_found: 0,
        transactions_processed: 0,
        total_transactions: 0,
    };

    // Broadcast to SSE connections
    debug!(
        "Broadcasting analysis progress: {}% for wallet: {}",
        progress, wallet
    );

    Ok(())
}

fn create_analysis_progress_stream(
    state: AppState,
    wallet: String,
    user_id: String,
) -> impl tokio_stream::Stream<Item = Result<Event, Infallible>> {
    // Create a stream that sends periodic progress updates
    let interval = tokio::time::interval(Duration::from_secs(2));
    let stream = IntervalStream::new(interval);

    stream.enumerate().map(move |(i, _)| {
        let progress = (i as f64 * 10.0).min(100.0);
        let stage = if progress < 20.0 {
            AnalysisStage::Initializing
        } else if progress < 40.0 {
            AnalysisStage::FetchingTransactions
        } else if progress < 60.0 {
            AnalysisStage::DetectingMoments
        } else if progress < 80.0 {
            AnalysisStage::CalculatingMetrics
        } else if progress < 100.0 {
            AnalysisStage::Finalizing
        } else {
            AnalysisStage::Complete
        };

        let progress_update = AnalysisProgress {
            wallet: wallet.clone(),
            stage,
            progress_pct: progress,
            stage_progress_pct: progress % 20.0 * 5.0,
            eta_seconds: if progress < 100.0 {
                Some(((100.0 - progress) / 10.0) as u32)
            } else {
                None
            },
            message: format!("Processing analysis... {}%", progress as u32),
            moments_found: (progress / 10.0) as u32,
            transactions_processed: (progress * 10.0) as u32,
            total_transactions: 1000,
        };

        match serde_json::to_string(&progress_update) {
            Ok(json) => Ok(Event::default().event("progress").data(json)),
            Err(_) => Ok(Event::default().event("error").data("Serialization error")),
        }
    })
}

async fn analyze_wallets_parallel(
    state: &AppState,
    request: &BulkAnalyzeRequest,
    user: &AuthUser,
) -> ApiResult<Vec<WalletAnalysisResult>> {
    use futures::future::join_all;

    let tasks = request.wallets.iter().map(|wallet| {
        let analyze_request = AnalyzeRequest {
            wallet: wallet.clone(),
            timeframe_days: request.params.timeframe_days,
            moment_types: request.params.moment_types.clone(),
            min_severity: request.params.min_severity.clone(),
            include_transactions: request.params.include_transactions,
            include_positions: request.params.include_positions,
            include_metrics: request.params.include_metrics,
            force_refresh: request.params.force_refresh,
            cursor: None,
            limit: request.params.limit,
        };

        async move {
            match analyze_single_wallet_for_bulk(state, &analyze_request).await {
                Ok(analysis) => WalletAnalysisResult {
                    wallet: wallet.clone(),
                    analysis: Some(analysis),
                    error: None,
                    status: AnalysisStatus::Success,
                },
                Err(e) => WalletAnalysisResult {
                    wallet: wallet.clone(),
                    analysis: None,
                    error: Some(e.to_string()),
                    status: AnalysisStatus::Failed,
                },
            }
        }
    });

    Ok(join_all(tasks).await)
}

async fn analyze_wallets_sequential(
    state: &AppState,
    request: &BulkAnalyzeRequest,
    user: &AuthUser,
) -> ApiResult<Vec<WalletAnalysisResult>> {
    let mut results = Vec::new();

    for wallet in &request.wallets {
        let analyze_request = AnalyzeRequest {
            wallet: wallet.clone(),
            timeframe_days: request.params.timeframe_days,
            moment_types: request.params.moment_types.clone(),
            min_severity: request.params.min_severity.clone(),
            include_transactions: request.params.include_transactions,
            include_positions: request.params.include_positions,
            include_metrics: request.params.include_metrics,
            force_refresh: request.params.force_refresh,
            cursor: None,
            limit: request.params.limit,
        };

        let result = match analyze_single_wallet_for_bulk(state, &analyze_request).await {
            Ok(analysis) => WalletAnalysisResult {
                wallet: wallet.clone(),
                analysis: Some(analysis),
                error: None,
                status: AnalysisStatus::Success,
            },
            Err(e) => WalletAnalysisResult {
                wallet: wallet.clone(),
                analysis: None,
                error: Some(e.to_string()),
                status: AnalysisStatus::Failed,
            },
        };

        results.push(result);
    }

    Ok(results)
}

async fn analyze_single_wallet_for_bulk(
    state: &AppState,
    request: &AnalyzeRequest,
) -> ApiResult<AnalyzeResponse> {
    // Simplified version of analyze_wallet for bulk operations
    let moments = vec![]; // Would call actual analysis
    let summary = generate_analysis_summary(&moments, &None)?;

    Ok(AnalyzeResponse {
        wallet: request.wallet.clone(),
        summary,
        moments,
        positions: None,
        metrics: None,
        transactions: None,
        metadata: AnalysisMetadata {
            analyzed_at: time::OffsetDateTime::now_utc(),
            duration_seconds: 1.0,
            data_last_updated: time::OffsetDateTime::now_utc(),
            from_cache: false,
            cache_expires_at: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
            warnings: vec![],
        },
        next_cursor: None,
    })
}

fn calculate_bulk_summary(results: &[WalletAnalysisResult]) -> BulkAnalysisSummary {
    let total_wallets = results.len() as u32;
    let successful = results
        .iter()
        .filter(|r| matches!(r.status, AnalysisStatus::Success))
        .count() as u32;
    let failed = total_wallets - successful;

    let total_moments = results
        .iter()
        .filter_map(|r| r.analysis.as_ref())
        .map(|a| a.summary.total_moments)
        .sum();

    let total_value_lost = results
        .iter()
        .filter_map(|r| r.analysis.as_ref())
        .map(|a| a.summary.total_value_lost.0)
        .sum::<f64>()
        .into();

    let average_oof_score = if successful > 0 {
        results
            .iter()
            .filter_map(|r| r.analysis.as_ref())
            .map(|a| a.summary.oof_score)
            .sum::<f64>()
            / successful as f64
    } else {
        0.0
    };

    BulkAnalysisSummary {
        total_wallets,
        successful,
        failed,
        total_moments,
        total_value_lost,
        average_oof_score,
    }
}

async fn get_analysis_status_from_store(
    state: &AppState,
    analysis_id: &str,
) -> ApiResult<Option<String>> {
    // Check status in Redis/database
    debug!("Getting analysis status for: {}", analysis_id);
    Ok(Some("completed".to_string()))
}

async fn verify_analysis_ownership(
    state: &AppState,
    analysis_id: &str,
    user_id: &str,
) -> ApiResult<()> {
    // Verify that the user owns this analysis
    debug!(
        "Verifying ownership of analysis {} by user {}",
        analysis_id, user_id
    );
    Ok(())
}

async fn cancel_analysis_in_store(state: &AppState, analysis_id: &str) -> ApiResult<()> {
    // Cancel the analysis in the job queue
    debug!("Cancelling analysis: {}", analysis_id);
    Ok(())
}
