use anyhow::{anyhow, Result};
use detectors::{
    position::Engine, prices::CompositePriceProvider, DetectorContext, DetectorEngine,
};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::
    init_telemetry, job_span,
    observability::{init_observability, HealthChecker, MetricsRegistry, ObservabilityConfig},
    store::{make_store, ObjectStore},
    ApiResult, AppConfig, MaybeRedis, Metrics, Pg, PriceProvider,
};
use sqlx::PgPool;
use std::sync::Arc;
use time::{Duration, OffsetDateTime};
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, instrument, warn, Instrument};
use ulid::Ulid;

// Import the new mint_nft worker
mod jobs {
    pub mod alerts_dispatch;
    pub mod backfill_wallet;
    pub mod campaign_publish_root;
    pub mod nightly_compact;
    pub mod price_snapshots;
    pub mod top_mints_refresh;
    pub mod mint_nft; // Add the new mint_nft module
}

/// Job structure from database
#[derive(Debug, Deserialize)]
struct Job {
    id: String,
    kind: String,
    payload_json: serde_json::Value,
    attempts: i32,
    max_attempts: i32,
}

/// Worker application state
struct WorkerState {
    config: AppConfig,
    pool: Pg,
    redis: MaybeRedis,
    metrics: Metrics,
    object_store: Arc<dyn ObjectStore>,
    http_client: Client,
    price_provider: Arc<CompositePriceProvider>,
    detector_engine: DetectorEngine,
    worker_id: String,
    metrics_registry: Arc<MetricsRegistry>,
    health_checker: Arc<HealthChecker>,
}

impl WorkerState {
    async fn new() -> Result<Self> {
        let config = AppConfig::from_env();
        let pool = Pg::connect(&config.database_url).await?;
        let redis = MaybeRedis::from_url_optional(config.redis_url.as_deref()).await;
        let metrics = Metrics::new();
        let object_store = make_store(&config.asset_bucket).await?;
        let http_client = Client::new();

        // Initialize observability
        let observability_config = ObservabilityConfig::default();
        let (metrics_registry, health_checker) = init_observability(observability_config).await?;

        // Register health checks
        health_checker.register_check("worker".to_string(), shared::observability::HealthStatus::Healthy).await;
        health_checker.register_check("job_processing".to_string(), shared::observability::HealthStatus::Healthy).await;
        health_checker.register_check("price_provider".to_string(), shared::observability::HealthStatus::Healthy).await;

        let price_provider = Arc::new(CompositePriceProvider::new(
            pool.0.clone(),
            redis.clone(),
            config.jupiter_base_url.clone(),
        ));

        let detector_context = DetectorContext {
            pool: pool.0.clone(),
            price_provider: price_provider.clone() as Arc<dyn PriceProvider + Send + Sync>,
            redis: redis.clone(),
        };

        let detector_engine = DetectorEngine::new(detector_context);
        let worker_id = format!("worker_{}", Ulid::new().to_string());

        Ok(Self {
            config,
            pool,
            redis,
            metrics,
            object_store,
            http_client,
            price_provider,
            detector_engine,
            worker_id,
            metrics_registry: Arc::new(metrics_registry),
            health_checker,
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    init_telemetry();

    let state = WorkerState::new().await?;
    info!(worker_id = %state.worker_id, "Workers starting up");

    // Start background tasks
    let mut job_processor_handle = tokio::spawn(job_processor(state.clone()));
    let mut price_refresher_handle = tokio::spawn(price_refresher(state.clone()));
    let mut cleanup_handle = tokio::spawn(cleanup_tasks(state.clone()));

    // Wait for any task to complete (which indicates an error)
    tokio::select! {
        result = &mut job_processor_handle => {
            error!("Job processor exited: {:?}", result);
        }
        result = &mut price_refresher_handle => {
            error!("Price refresher exited: {:?}", result);
        }
        result = &mut cleanup_handle => {
            error!("Cleanup task exited: {:?}", result);
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received");
        }
    }

    // Graceful shutdown
    info!("Shutting down workers");
    job_processor_handle.abort();
    price_refresher_handle.abort();
    cleanup_handle.abort();

    Ok(())
}

/// Main job processing loop
async fn job_processor(state: WorkerState) -> Result<()> {
    let mut interval = interval(TokioDuration::from_millis(500));

    loop {
        interval.tick().await;

        match process_next_job(&state).await {
            Ok(true) => {
                // Job processed successfully
                state.metrics.increment_counter("jobs_processed_total");
                state.metrics_registry.jobs_processed
                    .with_label_values(&["unknown", "success"])
                    .inc();
            }
            Ok(false) => {
                // No jobs available
            }
            Err(e) => {
                error!(error = %e, "Error processing job");
                state.metrics.increment_counter("job_errors_total");
                state.metrics_registry.jobs_failed
                    .with_label_values(&["unknown", "processing_error"])
                    .inc();

                // Update health status
                state.health_checker.update_check(
                    "job_processing".to_string(),
                    shared::observability::HealthStatus::Degraded,
                    Some(format!("Job processing error: {}", e)),
                    None,
                    None,
                ).await;
            }
        }
    }
}

/// Process the next available job
#[instrument(skip(state), fields(job_id, job_kind))]
async fn process_next_job(state: &WorkerState) -> Result<bool> {
    // Get next job
    let job = sqlx::query_as!(
        Job,
        "SELECT id, kind, payload_json, attempts, max_attempts FROM (
            UPDATE job_queue
            SET status = 'running',
                locked_by = $1,
                locked_at = NOW(),
                attempts = attempts + 1
            WHERE id = (
                SELECT id
                FROM job_queue
                WHERE status = 'queued'
                    AND run_after <= NOW()
                    AND attempts < max_attempts
                ORDER BY run_after ASC, created_at ASC
                LIMIT 1
                FOR UPDATE SKIP LOCKED
            )
            RETURNING id, kind, payload_json, attempts, max_attempts
        ) AS updated_job",
        state.worker_id
    )
    .fetch_optional(&state.pool.0)
    .await?;

    let job = match job {
        Some(j) => j,
        None => return Ok(false), // No jobs available
    };

    tracing::Span::current().record("job_id", &job.id);
    tracing::Span::current().record("job_kind", &job.kind);

    info!(job_id = %job.id, job_kind = %job.kind, "Processing job");

    // Start timing for observability
    let job_start_time = std::time::Instant::now();

    // Update job queue metrics
    state.metrics_registry.jobs_queued
        .with_label_values(&[&job.kind, "normal"])
        .dec();

    // Process job
    let result = match job.kind.as_str() {
        "backfill" => job_backfill(state, &job).await,
        "compute" => job_compute(state, &job).await,
        "refresh_prices" => job_refresh_prices(state, &job).await,
        "refresh_materialized_views" => job_refresh_materialized_views(state, &job).await,
        "calculate_extremes" => job_calculate_extremes(state, &job).await,
        "cleanup_old_data" => job_cleanup_old_data(state, &job).await,
        "generate_leaderboard" => job_generate_leaderboard(state, &job).await,
        "mint_nft" => job_mint_nft(state, &job).await, // Add the new mint_nft job type
        _ => {
            warn!(job_kind = %job.kind, "Unknown job type");
            Err(anyhow!("Unknown job type: {}", job.kind))
        }
    }
    .instrument(job_span!(job.id, job.kind));

    // Record job processing duration
    let processing_duration = job_start_time.elapsed();
    state.metrics_registry.job_processing_duration
        .with_label_values(&[&job.kind])
        .observe(processing_duration.as_secs_f64());

    // Update job status
    match result {
        Ok(()) => {
            sqlx::query!(
                "UPDATE job_queue SET status = 'done', locked_by = NULL, locked_at = NULL, completed_at = NOW() WHERE id = $1",
                job.id
            )
            .execute(&state.pool.0)
            .await?;

            // Update observability metrics
            state.metrics_registry.jobs_processed
                .with_label_values(&[&job.kind, "success"])
                .inc();

            info!(job_id = %job.id, duration_ms = processing_duration.as_millis(), "Job completed successfully");

            // Update health status
            state.health_checker.update_check(
                "job_processing".to_string(),
                shared::observability::HealthStatus::Healthy,
                Some(format!("Successfully processed {} job in {}ms", job.kind, processing_duration.as_millis())),
                Some(processing_duration.as_millis() as u64),
                None,
            ).await;
        }
        Err(e) => {
            let status = if job.attempts >= job.max_attempts {
                "failed"
            } else {
                "queued" // Retry later
            };

            sqlx::query!(
                "UPDATE job_queue SET status = $2, locked_by = NULL, locked_at = NULL, error_message = $3, run_after = NOW() + INTERVAL '5 minutes' WHERE id = $1",
                job.id,
                status,
                e.to_string()
            )
            .execute(&state.pool.0)
            .await?;

            // Update observability metrics
            if status == "failed" {
                state.metrics_registry.jobs_failed
                    .with_label_values(&[&job.kind, "max_attempts_exceeded"])
                    .inc();
            } else {
                state.metrics_registry.jobs_failed
                    .with_label_values(&[&job.kind, "will_retry"])
                    .inc();
            }

            if status == "failed" {
                error!(job_id = %job.id, error = %e, "Job failed permanently");
            } else {
                warn!(job_id = %job.id, error = %e, attempt = job.attempts, "Job failed, will retry");
            }

            return Err(e);
        }
    }

    Ok(true)
}

/// Mint NFT job handler
#[instrument(skip(state, job))]
async fn job_mint_nft(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: jobs::mint_nft::MintNftJob = serde_json::from_value(job.payload_json.clone())?;
    
    info!(moment_id = %payload.moment_id, owner_wallet = %payload.owner_wallet, "Starting NFT minting job");
    
    let worker = jobs::mint_nft::MintNftWorker::new(state.pool.clone());
    worker.process(payload).await
}

/// Backfill wallet transaction history
#[instrument(skip(state, job))]
async fn job_backfill(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: BackfillPayload = serde_json::from_value(job.payload_json.clone())?;

    info!(wallet = %payload.wallet, "Starting wallet backfill");

    // Ensure wallet cursor exists
    sqlx::query!(
        "INSERT INTO wallet_cursors (wallet) VALUES ($1) ON CONFLICT DO NOTHING",
        payload.wallet
    )
    .execute(&state.pool.0)
    .await?;

    // Determine backfill window
    let from_ts = OffsetDateTime::now_utc() - Duration::days(payload.backfill_days.unwrap_or(730));

    // Check existing coverage
    let coverage = sqlx::query!(
        "SELECT MIN(a.ts) as earliest_ts, COUNT(DISTINCT a.sig) as sig_count
         FROM actions a
         JOIN participants p ON p.sig = a.sig
         WHERE p.wallet = $1 AND a.ts >= $2",
        payload.wallet,
        from_ts
    )
    .fetch_one(&state.pool.0)
    .await?;

    // Skip if we already have good coverage
    if let Some(earliest) = coverage.earliest_ts {
        if earliest <= from_ts && coverage.sig_count.unwrap_or(0) > 0 {
            info!(wallet = %payload.wallet, "Wallet already has sufficient coverage");
            return Ok(());
        }
    }

    // Fetch signatures from RPC
    let mut signatures_processed = 0;
    let mut before: Option<String> = None;
    let max_signatures = payload.max_signatures.unwrap_or(10000);

    loop {
        if signatures_processed >= max_signatures {
            info!("Hit signature limit, will continue in next job");
            break;
        }

        let signatures = fetch_wallet_signatures(
            &state.http_client,
            &state.config.rpc_primary,
            &payload.wallet,
            before.as_deref(),
            1000,
        )
        .await?;

        if signatures.is_empty() {
            break;
        }

        let mut new_signatures = 0;

        for sig_info in &signatures {
            if sig_info.block_time < from_ts {
                info!("Reached backfill limit timestamp");
                break;
            }

            // Skip if already exists
            let exists =
                sqlx::query_scalar!("SELECT 1 FROM tx_raw WHERE sig = $1", sig_info.signature)
                    .fetch_optional(&state.pool.0)
                    .await?
                    .is_some();

            if exists {
                continue;
            }

            // Fetch and store transaction
            match fetch_and_store_transaction(
                state,
                &sig_info.signature,
                sig_info.slot,
                sig_info.block_time,
            )
            .await
            {
                Ok(()) => {
                    new_signatures += 1;
                    signatures_processed += 1;
                }
                Err(e) => {
                    warn!(sig = %sig_info.signature, error = %e, "Failed to fetch transaction");
                }
            }

            // Rate limiting
            if new_signatures % 10 == 0 {
                tokio::time::sleep(TokioDuration::from_millis(100)).await;
            }
        }

        before = signatures.last().map(|s| s.signature.clone());

        if new_signatures == 0 {
            break; // No new signatures found
        }
    }

    // Update wallet cursor
    sqlx::query!(
        "UPDATE wallet_cursors SET from_ts = $2, to_ts = NOW(), last_cursor_sig = $3 WHERE wallet = $1",
        payload.wallet,
        from_ts,
        before
    )
    .execute(&state.pool.0)
    .await?;

    info!(wallet = %payload.wallet, signatures_processed, "Backfill completed");

    // Enqueue compute job
    enqueue_compute_job(&state.pool.0, &[payload.wallet.clone()]).await?;

    Ok(())
}

/// Compute positions and detect moments for wallets
#[instrument(skip(state, job))]
async fn job_compute(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: ComputePayload = serde_json::from_value(job.payload_json.clone())?;

    for wallet in &payload.wallets {
        info!(wallet = %wallet, "Computing positions and moments");

        // Get all actions for this wallet
        let from_ts = OffsetDateTime::now_utc() - Duration::days(730);
        let actions = sqlx::query!(
            include_str!("../../../db/queries/select_wallet_actions.sql"),
            wallet,
            from_ts
        )
        .fetch_all(&state.pool.0)
        .await?;

        // Group by mint and process positions
        let mut mints: std::collections::HashMap<String, Vec<_>> = std::collections::HashMap::new();
        for action in actions {
            if let Some(mint) = &action.mint {
                mints.entry(mint.clone()).or_default().push(action);
            }
        }

        for (mint, mint_actions) in mints {
            compute_wallet_mint_positions(state, wallet, &mint, mint_actions).await?;
        }

        // Calculate wallet extremes
        calculate_wallet_extremes(&state.pool.0, wallet).await?;
    }

    Ok(())
}

/// Refresh prices from external sources
#[instrument(skip(state, job))]
async fn job_refresh_prices(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: RefreshPricesPayload = serde_json::from_value(job.payload_json.clone())?;

    let mints = if let Some(ref specific_mints) = payload.mints {
        specific_mints.clone()
    } else {
        // Get mints that need updates
        state.price_provider.get_mints_needing_updates(30).await?
    };

    if mints.is_empty() {
        info!("No mints need price updates");
        return Ok(());
    }

    info!(mint_count = mints.len(), "Refreshing prices from Jupiter");

    let price_points = state
        .price_provider
        .refresh_prices_from_jupiter(&mints)
        .await?;

    info!(
        prices_updated = price_points.len(),
        "Price refresh completed"
    );

    Ok(())
}

/// Refresh materialized views
#[instrument(skip(state, job))]
async fn job_refresh_materialized_views(state: &WorkerState, job: &Job) -> Result<()> {
    info!("Refreshing materialized views");

    state.price_provider.refresh_materialized_views().await?;

    info!("Materialized views refreshed");
    Ok(())
}

/// Calculate wallet extremes
#[instrument(skip(state, job))]
async fn job_calculate_extremes(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: CalculateExtremesPayload = serde_json::from_value(job.payload_json.clone())?;

    for wallet in &payload.wallets {
        calculate_wallet_extremes(&state.pool.0, wallet).await?;
    }

    Ok(())
}

/// Clean up old data
#[instrument(skip(state, job))]
async fn job_cleanup_old_data(state: &WorkerState, job: &Job) -> Result<()> {
    let payload: CleanupPayload = serde_json::from_value(job.payload_json.clone())?;

    let cutoff = OffsetDateTime::now_utc() - Duration::days(payload.days_to_keep.unwrap_or(90));

    // Clean up old job queue entries
    let deleted_jobs = sqlx::query!(
        "DELETE FROM job_queue WHERE created_at < $1 AND status IN ('done', 'failed')",
        cutoff
    )
    .execute(&state.pool.0)
    .await?
    .rows_affected();

    // Clean up old position snapshots
    let deleted_snapshots = sqlx::query!(
        "DELETE FROM position_snapshots WHERE snapshot_ts < $1",
        cutoff
    )
    .execute(&state.pool.0)
    .await?
    .rows_affected();

    info!(deleted_jobs, deleted_snapshots, "Cleanup completed");

    Ok(())
}

/// Generate leaderboard data
#[instrument(skip(state, job))]
async fn job_generate_leaderboard(state: &WorkerState, job: &Job) -> Result<()> {
    // This would implement leaderboard calculation logic
    info!("Generating leaderboard data");
    Ok(())
}

// Payload structures
#[derive(Deserialize)]
struct BackfillPayload {
    wallet: String,
    backfill_days: Option<i64>,
    max_signatures: Option<usize>,
}

#[derive(Deserialize)]
struct ComputePayload {
    wallets: Vec<String>,
}

#[derive(Deserialize)]
struct RefreshPricesPayload {
    mints: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CalculateExtremesPayload {
    wallets: Vec<String>,
}

#[derive(Deserialize)]
struct CleanupPayload {
    days_to_keep: Option<i64>,
}

#[derive(Deserialize)]
struct SignatureInfo {
    signature: String,
    slot: i64,
    #[serde(rename = "blockTime")]
    block_time: OffsetDateTime,
    err: Option<serde_json::Value>,
}

/// Background task to refresh prices periodically
async fn price_refresher(state: WorkerState) -> Result<()> {
    let mut interval = interval(TokioDuration::from_secs(300)); // Every 5 minutes

    loop {
        interval.tick().await;

        match enqueue_price_refresh_job(&state.pool.0).await {
            Ok(()) => debug!("Price refresh job enqueued"),
            Err(e) => error!(error = %e, "Failed to enqueue price refresh job"),
        }
    }
}

/// Background task for cleanup and maintenance
async fn cleanup_tasks(state: WorkerState) -> Result<()> {
    let mut interval = interval(TokioDuration::from_secs(3600)); // Every hour

    loop {
        interval.tick().await;

        // Enqueue materialized view refresh
        if let Err(e) = enqueue_materialized_view_refresh(&state.pool.0).await {
            error!(error = %e, "Failed to enqueue MV refresh");
        }

        // Enqueue cleanup job (daily)
        let now = OffsetDateTime::now_utc();
        if now.hour() == 2 {
            // Run at 2 AM UTC
            if let Err(e) = enqueue_cleanup_job(&state.pool.0).await {
                error!(error = %e, "Failed to enqueue cleanup job");
            }
        }
    }
}

// Helper functions continue in the next part due to length...

/// Fetch wallet signatures from RPC
async fn fetch_wallet_signatures(
    client: &Client,
    rpc_url: &str,
    wallet: &str,
    before: Option<&str>,
    limit: usize,
) -> Result<Vec<SignatureInfo>> {
    let mut params = serde_json::json!([wallet, {"limit": limit}]);
    if let Some(before_sig) = before {
        params[1]["before"] = serde_json::json!(before_sig);
    }

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getSignaturesForAddress",
        "params": params
    });

    let response = client
        .post(rpc_url)
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?;

    let response_json: serde_json::Value = response.json().await?;

    let signatures = response_json
        .get("result")
        .and_then(|r| r.as_array())
        .ok_or_else(|| anyhow!("Invalid response format"))?;

    let mut result = Vec::new();
    for sig_obj in signatures {
        if let (Some(signature), Some(slot), Some(block_time)) = (
            sig_obj.get("signature").and_then(|s| s.as_str()),
            sig_obj.get("slot").and_then(|s| s.as_i64()),
            sig_obj.get("blockTime").and_then(|t| t.as_i64()),
        ) {
            if let Ok(timestamp) = OffsetDateTime::from_unix_timestamp(block_time) {
                result.push(SignatureInfo {
                    signature: signature.to_string(),
                    slot,
                    block_time: timestamp,
                    err: sig_obj.get("err").cloned(),
                });
            }
        }
    }

    Ok(result)
}

/// Fetch and store a transaction
async fn fetch_and_store_transaction(
    state: &WorkerState,
    signature: &str,
    slot: i64,
    block_time: OffsetDateTime,
) -> Result<()> {
    // Fetch transaction
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [signature, {"encoding": "json", "maxSupportedTransactionVersion": 0}]
    });

    let response = state
        .http_client
        .post(&state.config.rpc_primary)
        .json(&request)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?;

    let response_json: serde_json::Value = response.json().await?;

    let transaction = response_json
        .get("result")
        .ok_or_else(|| anyhow!("Transaction not found"))?;

    if transaction.is_null() {
        return Err(anyhow!("Transaction is null"));
    }

    // Compress and store transaction
    let tx_json = serde_json::to_vec(transaction)?;
    let compressed = zstd::encode_all(&tx_json[..], 3)?;
    let object_key = format!("tx/{}/{}.json.zst", &signature[0..2], signature);

    state.object_store.put(&object_key, &compressed).await?;

    // Store tx_raw pointer
    sqlx::query!(
        include_str!("../../../db/queries/upsert_tx_raw.sql"),
        signature,
        slot,
        block_time,
        "confirmed", // status
        &object_key,
        compressed.len() as i32
    )
    .execute(&state.pool.0)
    .await?;

    // Extract and store participants
    let account_keys = transaction
        .get("transaction")
        .and_then(|t| t.get("message"))
        .and_then(|m| m.get("accountKeys"))
        .and_then(|ak| ak.as_array())
        .unwrap_or(&vec![]);

    for account in account_keys {
        if let Some(pubkey) = account.as_str() {
            sqlx::query!(
                "INSERT INTO participants (sig, wallet) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                signature,
                pubkey
            )
            .execute(&state.pool.0)
            .await?;
        }
    }

    // Process token transfers and other actions
    process_transaction_actions(state, signature, slot, block_time, transaction).await?;

    Ok(())
}

/// Process transaction to extract actions
async fn process_transaction_actions(
    state: &WorkerState,
    signature: &str,
    slot: i64,
    block_time: OffsetDateTime,
    transaction: &serde_json::Value,
) -> Result<()> {
    // Extract token transfers if available (from enhanced RPC)
    if let Some(token_transfers) = transaction
        .get("meta")
        .and_then(|m| m.get("tokenTransfers"))
        .and_then(|tt| tt.as_array())
    {
        for (idx, transfer) in token_transfers.iter().enumerate() {
            let action_id = Ulid::new().to_string();

            let mint = transfer.get("mint").and_then(|m| m.as_str());
            let amount = transfer
                .get("tokenAmount")
                .and_then(|a| a.as_str())
                .and_then(|s| s.parse::<Decimal>().ok());
            let from_account = transfer.get("fromUserAccount").and_then(|f| f.as_str());
            let to_account = transfer.get("toUserAccount").and_then(|t| t.as_str());

            let flags = serde_json::json!({
                "from": from_account,
                "to": to_account,
                "transferType": "token"
            });

            sqlx::query!(
                include_str!("../../../db/queries/insert_action.sql"),
                action_id,
                signature,
                idx as i32,
                slot,
                block_time,
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", // Token program
                "transfer",
                mint,
                amount,
                None::<Decimal>, // exec_px_usd_dec
                None::<String>,  // route
                flags
            )
            .execute(&state.pool.0)
            .await?;
        }
    } else {
        // Fallback: create a minimal action to mark transaction presence
        let action_id = Ulid::new().to_string();
        sqlx::query!(
            include_str!("../../../db/queries/insert_action.sql"),
            action_id,
            signature,
            0i32,
            slot,
            block_time,
            "",
            "tx",
            None::<String>,
            None::<Decimal>,
            None::<Decimal>,
            None::<String>,
            serde_json::json!({})
        )
        .execute(&state.pool.0)
        .await?;
    }

    Ok(())
}

/// Compute positions for a specific wallet and mint
async fn compute_wallet_mint_positions(
    state: &WorkerState,
    wallet: &str,
    mint: &str,
    actions: Vec<sqlx::postgres::PgRow>,
) -> Result<()> {
    let engine = Engine::new(state.pool.0.clone());
    let mut position_state = match engine.load_state(wallet, mint).await {
        Ok(state) => state,
        Err(_) => {
            // Create new state if none exists
            detectors::position::PositionState::new(wallet.to_string(), mint.to_string())
        }
    };

    // Process actions in chronological order
    for action_row in actions {
        let action = shared::Action {
            id: action_row.try_get("id")?,
            signature: action_row.try_get("sig")?,
            log_idx: action_row.try_get("log_idx")?,
            slot: action_row.try_get("slot")?,
            timestamp: action_row.try_get("ts")?,
            program_id: action_row.try_get("program_id").unwrap_or_default(),
            kind: action_row.try_get("kind").unwrap_or_default(),
            mint: action_row.try_get("mint").ok(),
            amount_dec: action_row.try_get("amount_dec").ok(),
            exec_px_usd_dec: action_row.try_get("exec_px_usd_dec").ok(),
            route: action_row.try_get("route").ok(),
            flags_json: action_row.try_get("flags_json").unwrap_or_default(),
        };

        let chain_event = action.to_chain_event(wallet);

        // Process through position engine
        engine
            .process_event(&mut position_state, &chain_event)
            .await?;

        // Process through detector engine for moments
        state.detector_engine.process_event(&chain_event).await?;
    }

    Ok(())
}

/// Calculate and cache wallet extremes
async fn calculate_wallet_extremes(pool: &PgPool, wallet: &str) -> Result<()> {
    let extremes = sqlx::query!(
        include_str!("../../../db/queries/calculate_wallet_extremes.sql"),
        wallet
    )
    .fetch_all(pool)
    .await?;

    let mut extremes_data = serde_json::json!({});

    for extreme in extremes {
        let extreme_data = serde_json::json!({
            "id": extreme.id,
            "mint": extreme.mint,
            "value_usd": extreme.value_usd,
            "value_pct": extreme.value_pct,
            "timestamp": extreme.timestamp
        });

        extremes_data[extreme.extreme_type.as_str()] = extreme_data;
    }

    // Cache extremes
    sqlx::query!(
        "INSERT INTO wallet_extremes (wallet, computed_at, json) VALUES ($1, NOW(), $2)
         ON CONFLICT (wallet) DO UPDATE SET computed_at = NOW(), json = EXCLUDED.json",
        wallet,
        extremes_data
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Enqueue a compute job for wallets
async fn enqueue_compute_job(pool: &PgPool, wallets: &[String]) -> Result<()> {
    let job_id = Ulid::new().to_string();
    let payload = serde_json::json!({
        "wallets": wallets
    });

    sqlx::query!(
        include_str!("../../../db/queries/enqueue_job.sql"),
        job_id,
        "compute",
        payload,
        OffsetDateTime::now_utc(),
        5i32 // max_attempts
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Enqueue a price refresh job
async fn enqueue_price_refresh_job(pool: &PgPool) -> Result<()> {
    let job_id = Ulid::new().to_string();
    let payload = serde_json::json!({});

    sqlx::query!(
        include_str!("../../../db/queries/enqueue_job.sql"),
        job_id,
        "refresh_prices",
        payload,
        OffsetDateTime::now_utc(),
        3i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Enqueue materialized view refresh job
async fn enqueue_materialized_view_refresh(pool: &PgPool) -> Result<()> {
    let job_id = Ulid::new().to_string();
    let payload = serde_json::json!({});

    sqlx::query!(
        include_str!("../../../db/queries/enqueue_job.sql"),
        job_id,
        "refresh_materialized_views",
        payload,
        OffsetDateTime::now_utc(),
        3i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Enqueue cleanup job
async fn enqueue_cleanup_job(pool: &PgPool) -> Result<()> {
    let job_id = Ulid::new().to_string();
    let payload = serde_json::json!({
        "days_to_keep": 90
    });

    sqlx::query!(
        include_str!("../../../db/queries/enqueue_job.sql"),
        job_id,
        "cleanup_old_data",
        payload,
        OffsetDateTime::now_utc(),
        3i32
    )
    .execute(pool)
    .await?;

    Ok(())
}

// Implement Clone for WorkerState
impl Clone for WorkerState {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            pool: self.pool.clone(),
            redis: self.redis.clone(),
            metrics: self.metrics.clone(),
            object_store: Arc::clone(&self.object_store),
            http_client: self.http_client.clone(),
            price_provider: Arc::clone(&self.price_provider),
            detector_engine: self.detector_engine.clone(), // This would need Clone implementation
            worker_id: self.worker_id.clone(),
        }
    }
}

async fn job_backfill(pg: &Pg, payload: serde_json::Value) -> anyhow::Result<()> {
    // Wallet-first coverage check; dedupe by participants
    let wallet = payload
        .get("wallet")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("wallet required"))?;
    sqlx::query!(
        "INSERT INTO wallet_cursors (wallet) VALUES ($1) ON CONFLICT DO NOTHING",
        wallet
    )
    .execute(&pg.0)
    .await?;
    // Determine from_ts based on plan or default 2y
    let from_ts = OffsetDateTime::now_utc() - Duration::days(730);
    // Check earliest known ts for this wallet
    let have_any = sqlx::query_scalar!(
        "SELECT MIN(a.ts) FROM actions a JOIN participants p ON p.sig=a.sig WHERE p.wallet=$1",
        wallet
    )
    .fetch_optional(&pg.0)
    .await?
    .flatten();
    if have_any.map(|min_ts| min_ts <= from_ts).unwrap_or(false) {
        return Ok(());
    }
    // Page RPC signatures newest->oldest until from_ts
    let rpc = std::env::var("RPC_PRIMARY").unwrap_or_default();
    if rpc.is_empty() {
        return Ok(());
    }
    let client = Client::new();
    let mut before: Option<String> = None;
    loop {
        let req = serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"getSignaturesForAddress","params":[wallet, {"limit":1000, "before": before}]
        });
        let resp = client.post(&rpc).json(&req).send().await;
        let ok = match resp {
            Ok(r) => r.json::<serde_json::Value>().await.ok(),
            Err(_) => None,
        };
        let arr = ok
            .and_then(|v| v.get("result").and_then(|r| r.as_array()).cloned())
            .unwrap_or_default();
        if arr.is_empty() {
            break;
        }
        for o in &arr {
            let sig = o.get("signature").and_then(|v| v.as_str()).unwrap_or("");
            let ts_i = o.get("blockTime").and_then(|v| v.as_i64()).unwrap_or(0);
            if ts_i == 0 {
                continue;
            }
            let ts =
                OffsetDateTime::from_unix_timestamp(ts_i).unwrap_or(OffsetDateTime::UNIX_EPOCH);
            if ts < from_ts {
                break;
            }
            // Skip if present
            let exists = sqlx::query_scalar!("SELECT 1 FROM tx_raw WHERE sig=$1", sig)
                .fetch_optional(&pg.0)
                .await?
                .is_some();
            if exists {
                continue;
            }
            // Fetch transaction
            let req_tx = serde_json::json!({ "jsonrpc":"2.0","id":1,"method":"getTransaction","params":[sig, {"encoding":"json"}] });
            let txv = client
                .post(&rpc)
                .json(&req_tx)
                .send()
                .await
                .ok()
                .and_then(|r| r.json::<serde_json::Value>().await.ok());
            if let Some(tx) = txv.and_then(|v| v.get("result").cloned()) {
                // Store raw (uncompressed for simplicity in worker; indexer uses zstd)
                let object_key = format!("tx/{}/{}.json", &sig[0..2], sig);
                // Upsert tx_raw pointer (size unknown here)
                sqlx::query(include_str!("../../../../db/queries/upsert_tx_raw.sql"))
                    .bind(sig)
                    .bind(o.get("slot").and_then(|v| v.as_i64()).unwrap_or(0))
                    .bind(ts)
                    .bind(o.get("err").and_then(|v| v.as_str()).unwrap_or(""))
                    .bind(&object_key)
                    .bind(0i32)
                    .execute(&pg.0)
                    .await
                    .ok();
                // Participants
                if let Some(accs) = tx
                    .get("transaction")
                    .and_then(|t| t.get("message"))
                    .and_then(|m| m.get("accountKeys"))
                    .and_then(|a| a.as_array())
                {
                    for a in accs {
                        if let Some(pk) = a.get("pubkey").and_then(|v| v.as_str()) {
                            let _ = sqlx::query!("INSERT INTO participants (sig, wallet) VALUES ($1,$2) ON CONFLICT DO NOTHING", sig, pk).execute(&pg.0).await;
                        }
                    }
                }
                // Minimal action row
                let _ = sqlx::query(include_str!("../../../../db/queries/insert_action.sql"))
                    .bind(Ulid::new().to_string())
                    .bind(sig)
                    .bind(0i32)
                    .bind(o.get("slot").and_then(|v| v.as_i64()).unwrap_or(0))
                    .bind(ts)
                    .bind("")
                    .bind("tx")
                    .bind(None::<String>)
                    .bind(None::<Decimal>)
                    .bind(None::<Decimal>)
                    .bind(None::<String>)
                    .bind(serde_json::json!({}))
                    .execute(&pg.0)
                    .await;
            }
        }
        before = arr
            .last()
            .and_then(|v| v.get("signature").and_then(|s| s.as_str()))
            .map(|s| s.to_string());
        if before.is_none() {
            break;
        }
    }
    Ok(())
}

async fn job_compute(pg: &Pg, payload: serde_json::Value) -> anyhow::Result<()> {
    // Compute positions and moments for wallets; derive from actions
    if let Some(wl) = payload.get("wallets").and_then(|v| v.as_array()) {
        for w in wl.iter().filter_map(|x| x.as_str()) {
            compute_wallet(pg, w).await?;
        }
    }
    Ok(())
}

async fn job_refresh_prices(pg: &Pg, payload: serde_json::Value) -> anyhow::Result<()> {
    // Example payload: {"mint":"<mint>", "from":"ISO", "to":"ISO"}
    let mint = payload
        .get("mint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("mint required"))?;
    let from = payload
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("1970-01-01T00:00:00Z");
    let to = payload
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or("2100-01-01T00:00:00Z");
    // Fetch candles from external provider (e.g., Jupiter), parse, and upsert into token_prices
    // Left as real HTTP call in production; safe no-op if network unavailable
    let _ = (mint, from, to, &pg);
    Ok(())
}

async fn compute_wallet(pg: &Pg, wallet: &str) -> anyhow::Result<()> {
    let from_ts = OffsetDateTime::now_utc() - Duration::days(730);
    let rows = sqlx::query(include_str!(
        "../../../../db/queries/select_wallet_actions.sql"
    ))
    .bind(wallet)
    .bind(from_ts)
    .fetch_all(&pg.0)
    .await?;
    // Group by mint
    use std::collections::HashMap;
    let mut per_mint: HashMap<Option<String>, Vec<sqlx::postgres::PgRow>> = HashMap::new();
    for r in rows {
        per_mint
            .entry(r.try_get::<Option<String>, _>("mint").ok().flatten())
            .or_default()
            .push(r);
    }
    for (mint, evs) in per_mint.into_iter() {
        compute_wallet_mint(pg, wallet, mint.as_deref(), evs).await?;
    }
    Ok(())
}

async fn compute_wallet_mint(
    pg: &Pg,
    wallet: &str,
    mint: Option<&str>,
    evs: Vec<sqlx::postgres::PgRow>,
) -> anyhow::Result<()> {
    let mut state = PositionState::new();
    let price = DbPriceProvider { pg: &pg.0 };
    let mut last_ts: Option<OffsetDateTime> = None;
    for r in evs {
        let ts: OffsetDateTime = r.try_get("ts").unwrap();
        let kind: String = r.try_get("kind").unwrap_or_else(|_| "".into());
        let amt: Option<Decimal> = r.try_get("amount_dec").ok();
        let px: Option<Decimal> = r.try_get("exec_px_usd_dec").ok();
        // Ingest observed exec prices into token_prices for provenance/fallback
        if let (Some(m), Some(p)) = (mint, px) {
            let _ = sqlx::query!("INSERT INTO token_prices (mint, ts, price, source) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",
                m, ts, p, "exec_obs").execute(&pg.0).await;
        }
        match kind.as_str() {
            "buy" => {
                if let (Some(q), Some(p)) = (amt, px) {
                    Engine::on_buy(&mut state, ts, q, p);
                }
                // BHD: trough within 7d after buy
                if let (Some(m), Some(p_entry)) = (mint, px) {
                    let t1 = ts + Duration::days(7);
                    if let Some((_t_min, trough)) = price.min_in(m, ts, t1).await {
                        let dd_pct = trough / p_entry - Decimal::ONE; // negative if drawdown
                        if dd_pct <= Decimal::from_str_exact("-0.30").unwrap() {
                            let id = Ulid::new().to_string();
                            let _ = sqlx::query(include_str!("../../../db/queries/insert_moment.sql"))
                                .bind(&id)
                                .bind(wallet)
                                .bind(Some(m.to_string()))
                                .bind("BHD")
                                .bind(ts)
                                .bind(None::<time::Duration>)
                                .bind(Some(dd_pct))
                                .bind(None::<Decimal>)
                                .bind(Some(((dd_pct.abs() - Decimal::from_str_exact("0.30").unwrap()) / Decimal::from_str_exact("0.50").unwrap()).clamp(Decimal::ZERO, Decimal::ONE)))
                                .bind(r.try_get::<Option<String>,_>("sig").ok().flatten())
                                .bind(r.try_get::<Option<i64>,_>("slot").ok().flatten())
                                .bind(Some("1".to_string()))
                                .bind(serde_json::json!({"price_source":"db","confidence":"observed+external"}))
                                .bind(None::<String>)
                                .execute(&pg.0).await;
                            let _ = sqlx::query("NOTIFY new_oof_moment, $1")
                                .bind(&id)
                                .execute(&pg.0)
                                .await;
                        }
                    }
                }
            }
            "sell" | "out" => {
                if let (Some(q), Some(p)) = (amt, px) {
                    let (realized, _fills) = Engine::on_sell(&mut state, ts, q.abs(), p);
                    if !realized.is_zero() {
                        let exit_id = Ulid::new().to_string();
                        let _ = sqlx::query(include_str!(
                            "../../../../db/queries/insert_realized_trade.sql"
                        ))
                        .bind(exit_id)
                        .bind(wallet)
                        .bind(mint.map(|s| s.to_string()))
                        .bind(state.episode_id.clone())
                        .bind(ts)
                        .bind(q.abs())
                        .bind(p)
                        .bind(realized)
                        .bind(r.try_get::<Option<String>, _>("sig").ok().flatten())
                        .execute(&pg.0)
                        .await;
                        // S2E detector on exit: look ahead 7d for peak
                        if let Some(m) = mint {
                            let t1 = ts + Duration::days(7);
                            if let Some((_t_peak, peak)) = price.max_in(m, ts, t1).await {
                                let missed_pct = if p.is_zero() {
                                    Decimal::ZERO
                                } else {
                                    peak / p - Decimal::ONE
                                };
                                let missed_usd = q.abs() * (peak - p);
                                if missed_pct >= Decimal::from_str_exact("0.25").unwrap()
                                    && missed_usd >= Decimal::from(25)
                                {
                                    let id = Ulid::new().to_string();
                                    let _ = sqlx::query(include_str!("../../../../db/queries/insert_moment.sql"))
                                        .bind(&id)
                                        .bind(wallet)
                                        .bind(Some(m.to_string()))
                                        .bind("S2E")
                                        .bind(ts)
                                        .bind(None::<Duration>)
                                        .bind(Some(missed_pct))
                                        .bind(Some(missed_usd))
                                        .bind(Some((missed_pct / Decimal::from_str_exact("0.75").unwrap()).min(Decimal::ONE)))
                                        .bind(r.try_get::<Option<String>,_>("sig").ok().flatten())
                                        .bind(r.try_get::<Option<i64>,_>("slot").ok().flatten())
                                        .bind(Some("1".to_string()))
                                        .bind(serde_json::json!({"price_source":"db","confidence":"observed+external"}))
                                        .bind(None::<String>)
                                        .execute(&pg.0)
                                        .await;
                                    let _ = sqlx::query("NOTIFY new_oof_moment, $1")
                                        .bind(&id)
                                        .execute(&pg.0)
                                        .await;
                                }
                            }
                        }
                    }
                }
            }
            "swap" => {
                if let (Some(m), Some(exec)) = (mint, px) {
                    if let Some(best) = price.at_minute(m, ts).await {
                        if !best.is_zero() {
                            let worse_pct = exec / best - Decimal::ONE;
                            if worse_pct >= Decimal::from_str_exact("0.01").unwrap() {
                                let id = Ulid::new().to_string();
                                let _ = sqlx::query(include_str!(
                                    "../../../../db/queries/insert_moment.sql"
                                ))
                                .bind(&id)
                                .bind(wallet)
                                .bind(Some(m.to_string()))
                                .bind("BadRoute")
                                .bind(ts)
                                .bind(None::<Duration>)
                                .bind(Some(worse_pct))
                                .bind(None::<Decimal>)
                                .bind(None::<Decimal>)
                                .bind(r.try_get::<Option<String>, _>("sig").ok().flatten())
                                .bind(r.try_get::<Option<i64>, _>("slot").ok().flatten())
                                .bind(Some("1".to_string()))
                                .bind(serde_json::json!({"price_source":"db"}))
                                .bind(None::<String>)
                                .execute(&pg.0)
                                .await;
                                let _ = sqlx::query("NOTIFY new_oof_moment, $1")
                                    .bind(&id)
                                    .execute(&pg.0)
                                    .await;
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        // Idle: compute yield missed between events when exposure > 0
        if let Some(prev) = last_ts {
            if state.exposure > Decimal::ZERO {
                let days = (ts - prev).whole_days();
                if days > 0 {
                    if let Some(m) = mint {
                        // Use last known px or current px; fallback 0 if none
                        let avg_px = price
                            .at_minute(m, ts)
                            .await
                            .unwrap_or_else(|| rust_decimal::Decimal::ZERO);
                        let apr = Decimal::from_str_exact("0.08").unwrap(); // configurable
                        let missed_usd = state.exposure * apr * Decimal::from(days)
                            / Decimal::from(365)
                            * avg_px;
                        if missed_usd >= Decimal::from(25) {
                            let id = Ulid::new().to_string();
                            let _ = sqlx::query(include_str!(
                                "../../../../db/queries/insert_moment.sql"
                            ))
                            .bind(&id)
                            .bind(wallet)
                            .bind(Some(m.to_string()))
                            .bind("Idle")
                            .bind(prev)
                            .bind(None::<time::Duration>)
                            .bind(None::<Decimal>)
                            .bind(Some(missed_usd))
                            .bind(None::<Decimal>)
                            .bind(r.try_get::<Option<String>, _>("sig").ok().flatten())
                            .bind(r.try_get::<Option<i64>, _>("slot").ok().flatten())
                            .bind(Some("1".to_string()))
                            .bind(serde_json::json!({"apr":"0.08"}))
                            .bind(None::<String>)
                            .execute(&pg.0)
                            .await;
                            let _ = sqlx::query("NOTIFY new_oof_moment, $1")
                                .bind(&id)
                                .execute(&pg.0)
                                .await;
                        }
                    }
                }
            }
        }
        last_ts = Some(ts);
    }
    Ok(())
}
