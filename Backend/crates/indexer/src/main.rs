use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{get, post},
    Router,
};
use shared::{
    metrics_router,
    observability::{init_observability, HealthChecker, MetricsRegistry, ObservabilityConfig},
    security::helius_hmac::{get_helius_sig_from_headers, verify_webhook_signature},
    store::{make_store, ObjectStore},
    AppConfig, MaybeRedis, Metrics, Pg,
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::broadcast, time::interval};
use tracing::{error, info, instrument, warn};
use ulid::Ulid;

#[derive(Clone)]
struct AppState {
    cfg: AppConfig,
    pg: Pg,
    metrics: Metrics,
    store: Arc<dyn ObjectStore>,
    redis: MaybeRedis,
    price_update_tx: broadcast::Sender<String>,
    metrics_registry: Arc<MetricsRegistry>,
    health_checker: Arc<HealthChecker>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct HeliusTransactionNotification {
    signature: String,
    slot: i64,
    timestamp: i64,
    #[serde(rename = "accountKeys")]
    account_keys: Option<Vec<String>>,
    #[serde(rename = "tokenTransfers")]
    token_transfers: Option<Vec<HeliusTokenTransfer>>,
    #[serde(rename = "nativeTransfers")]
    native_transfers: Option<Vec<HeliusNativeTransfer>>,
    meta: Option<serde_json::Value>,
    description: Option<String>,
    #[serde(rename = "type")]
    tx_type: Option<String>,
    source: Option<String>,
    fee: Option<i64>,
    #[serde(rename = "feePayer")]
    fee_payer: Option<String>,
    instructions: Option<Vec<serde_json::Value>>,
    events: Option<serde_json::Value>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct HeliusTokenTransfer {
    #[serde(rename = "fromUserAccount")]
    from_user_account: Option<String>,
    #[serde(rename = "toUserAccount")]
    to_user_account: Option<String>,
    #[serde(rename = "fromTokenAccount")]
    from_token_account: Option<String>,
    #[serde(rename = "toTokenAccount")]
    to_token_account: Option<String>,
    mint: String,
    #[serde(rename = "tokenAmount")]
    token_amount: Option<String>,
    #[serde(rename = "tokenStandard")]
    token_standard: Option<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct HeliusNativeTransfer {
    #[serde(rename = "fromUserAccount")]
    from_user_account: Option<String>,
    #[serde(rename = "toUserAccount")]
    to_user_account: Option<String>,
    amount: i64,
}

#[derive(serde::Serialize)]
struct WebhookStats {
    processed_count: u64,
    error_count: u64,
    last_processed: Option<String>,
    avg_processing_time_ms: f64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cfg = AppConfig::from_env();
    let pg = Pg::connect(&cfg.database_url).await?;
    let metrics = Metrics::new();
    let store = make_store(&cfg.asset_bucket).await?;
    let redis = MaybeRedis::connect(cfg.redis_url.as_deref()).await;

    // Initialize observability
    let observability_config = ObservabilityConfig::default();
    let (metrics_registry, health_checker) = init_observability(observability_config).await?;

    // Register health checks
    health_checker.register_check("indexer".to_string(), shared::observability::HealthStatus::Healthy).await;
    health_checker.register_check("webhook_processing".to_string(), shared::observability::HealthStatus::Healthy).await;

    let (price_update_tx, _) = broadcast::channel(1000);

    let state = AppState {
        cfg: cfg.clone(),
        pg,
        metrics,
        store,
        redis,
        price_update_tx,
        metrics_registry: Arc::new(metrics_registry),
        health_checker,
    };

    // Start background price refresh worker
    let price_worker_state = state.clone();
    tokio::spawn(async move {
        price_refresh_worker(price_worker_state).await;
    });

    // Start metrics cleanup worker
    let cleanup_worker_state = state.clone();
    tokio::spawn(async move {
        metrics_cleanup_worker(cleanup_worker_state).await;
    });

    let app = Router::new()
        .route("/webhooks/helius", post(helius_webhook))
        .route("/health", get(health_check))
        .route("/stats", get(webhook_stats))
        .route("/price-refresh/:mint", post(manual_price_refresh))
        .merge(metrics_router())
        .with_state(state);

    let addr: std::net::SocketAddr = cfg.indexer_bind.parse()?;
    info!("Indexer listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

/// Main webhook handler for Helius transaction notifications
#[instrument(skip(state, body), fields(webhook_size = body.len()))]
async fn helius_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> StatusCode {
    let start_time = std::time::Instant::now();
    state
        .metrics
        .increment_counter("helius_webhooks_received_total");

    // Update observability metrics
    state.metrics_registry.webhook_events_received
        .with_label_values(&["helius", "transaction"])
        .inc();

    // Verify HMAC signature
    let signature = get_helius_sig_from_headers(&headers);
    if !verify_webhook_signature(
        &state.cfg.helius_webhook_secret,
        &body,
        signature.as_deref(),
    ) {
        warn!("Invalid webhook signature");
        state
            .metrics
            .increment_counter("helius_webhooks_invalid_signature_total");
        state.metrics_registry.webhook_events_failed
            .with_label_values(&["helius", "transaction", "invalid_signature"])
            .inc();
        return StatusCode::UNAUTHORIZED;
    }

    // Parse webhook payload
    let notifications: Vec<HeliusTransactionNotification> = match serde_json::from_slice(&body) {
        Ok(notifications) => notifications,
        Err(e) => {
            error!("Failed to parse webhook payload: {}", e);
            state
                .metrics
                .increment_counter("helius_webhooks_parse_errors_total");
            return StatusCode::BAD_REQUEST;
        }
    };

    info!(
        "Processing {} transaction notifications",
        notifications.len()
    );

    let mut processed_count = 0;
    let mut error_count = 0;
    let mut unique_mints = std::collections::HashSet::new();

    for notification in notifications {
        match process_transaction_notification(&state, &notification).await {
            Ok(mints) => {
                processed_count += 1;
                unique_mints.extend(mints);
            }
            Err(e) => {
                error!(
                    "Failed to process transaction {}: {}",
                    notification.signature, e
                );
                error_count += 1;
            }
        }
    }

    // Trigger price updates for discovered tokens
    for mint in unique_mints {
        let _ = state.price_update_tx.send(mint);
    }

    let processing_time = start_time.elapsed();

    // Update observability metrics
    state.metrics_registry.webhook_processing_duration
        .with_label_values(&["helius", "transaction"])
        .observe(processing_time.as_secs_f64());

    if error_count > 0 {
        state.metrics_registry.webhook_events_failed
            .with_label_values(&["helius", "transaction", "processing_error"])
            .inc_by(error_count);
    }

    // Update health status based on error rate
    let error_rate = error_count as f64 / (processed_count + error_count) as f64;
    let health_status = if error_rate > 0.5 {
        shared::observability::HealthStatus::Unhealthy
    } else if error_rate > 0.1 {
        shared::observability::HealthStatus::Degraded
    } else {
        shared::observability::HealthStatus::Healthy
    };

    state.health_checker.update_check(
        "webhook_processing".to_string(),
        health_status,
        Some(format!("Processed {}/{} transactions, error rate: {:.1}%", processed_count, processed_count + error_count, error_rate * 100.0)),
        Some(processing_time.as_millis() as u64),
        None,
    ).await;

    state.metrics.observe_histogram(
        "helius_webhook_processing_seconds",
        processing_time.as_secs_f64(),
    );
    state
        .metrics
        .increment_counter_by("helius_transactions_processed_total", processed_count);

    if error_count > 0 {
        state
            .metrics
            .increment_counter_by("helius_transactions_errors_total", error_count);
    }

    info!(
        "Processed {}/{} transactions in {:?}",
        processed_count,
        processed_count + error_count,
        processing_time
    );

    StatusCode::OK
}

/// Process a single transaction notification
async fn process_transaction_notification(
    state: &AppState,
    notification: &HeliusTransactionNotification,
) -> anyhow::Result<Vec<String>> {
    let mut discovered_mints = Vec::new();

    // Store raw compressed transaction data
    let storage_key = format!(
        "tx/{}/{}.json.zst",
        &notification.signature[0..2],
        notification.signature
    );
    let raw_data = serde_json::to_vec(notification)?;
    let compressed_data = zstd::encode_all(&raw_data[..], 3)?;

    state.store.put(&storage_key, &compressed_data).await?;

    let timestamp = time::OffsetDateTime::from_unix_timestamp(notification.timestamp)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);

    // Upsert transaction record
    sqlx::query!(
        include_str!("../../../db/queries/upsert_tx_raw.sql"),
        notification.signature,
        notification.slot,
        timestamp,
        "confirmed", // Default status
        storage_key,
        compressed_data.len() as i32
    )
    .execute(&state.pg.0)
    .await?;

    // Process participants (wallet addresses involved)
    if let Some(account_keys) = &notification.account_keys {
        for account in account_keys {
            sqlx::query!(
                "INSERT INTO participants (sig, wallet) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                notification.signature,
                account
            )
            .execute(&state.pg.0)
            .await?;
        }
    }

    // Process token transfers
    if let Some(transfers) = &notification.token_transfers {
        for (idx, transfer) in transfers.iter().enumerate() {
            let action_id = Ulid::new().to_string();
            let amount = transfer
                .token_amount
                .as_ref()
                .and_then(|s| rust_decimal::Decimal::from_str_exact(s).ok());

            let action_type = determine_action_type(transfer);
            let flags = create_transfer_flags(transfer);

            discovered_mints.push(transfer.mint.clone());

            sqlx::query!(
                include_str!("../../../db/queries/insert_action.sql"),
                action_id,
                notification.signature,
                idx as i32,
                notification.slot,
                timestamp,
                "", // program_id - will be filled by backfill worker
                action_type,
                transfer.mint,
                amount,
                None::<rust_decimal::Decimal>, // sol_amount
                None::<String>,                // counterparty
                flags
            )
            .execute(&state.pg.0)
            .await?;
        }
    }

    // Process native SOL transfers
    if let Some(native_transfers) = &notification.native_transfers {
        for (idx, transfer) in native_transfers.iter().enumerate() {
            let action_id = Ulid::new().to_string();
            let sol_amount = rust_decimal::Decimal::new(transfer.amount, 9); // Convert lamports to SOL

            let flags = serde_json::json!({
                "from": transfer.from_user_account,
                "to": transfer.to_user_account,
                "amount_lamports": transfer.amount
            });

            let token_transfer_count = notification
                .token_transfers
                .as_ref()
                .map(|t| t.len())
                .unwrap_or(0);

            sqlx::query!(
                include_str!("../../../db/queries/insert_action.sql"),
                action_id,
                notification.signature,
                (token_transfer_count + idx) as i32, // Offset by token transfers
                notification.slot,
                timestamp,
                "", // program_id
                "sol_transfer",
                None::<String>,                // mint
                None::<rust_decimal::Decimal>, // token_amount
                sol_amount,
                transfer.to_user_account.clone(),
                flags
            )
            .execute(&state.pg.0)
            .await?;
        }
    }

    // If no token or native transfers, create a placeholder action
    if notification.token_transfers.is_none() && notification.native_transfers.is_none() {
        let action_id = Ulid::new().to_string();
        let flags = serde_json::json!({
            "description": notification.description,
            "type": notification.tx_type,
            "source": notification.source
        });

        sqlx::query!(
            include_str!("../../../db/queries/insert_action.sql"),
            action_id,
            notification.signature,
            0i32,
            notification.slot,
            timestamp,
            "",
            "unknown",
            None::<String>,
            None::<rust_decimal::Decimal>,
            None::<rust_decimal::Decimal>,
            None::<String>,
            flags
        )
        .execute(&state.pg.0)
        .await?;
    }

    Ok(discovered_mints)
}

/// Determine the action type based on transfer details
fn determine_action_type(transfer: &HeliusTokenTransfer) -> &'static str {
    match (&transfer.from_user_account, &transfer.to_user_account) {
        (Some(_), Some(_)) => "transfer",
        (None, Some(_)) => "mint",
        (Some(_), None) => "burn",
        (None, None) => "unknown",
    }
}

/// Create flags JSON for transfer metadata
fn create_transfer_flags(transfer: &HeliusTokenTransfer) -> serde_json::Value {
    serde_json::json!({
        "from_user": transfer.from_user_account,
        "to_user": transfer.to_user_account,
        "from_token_account": transfer.from_token_account,
        "to_token_account": transfer.to_token_account,
        "token_standard": transfer.token_standard
    })
}

/// Background worker for price refreshing
async fn price_refresh_worker(state: AppState) {
    let mut rx = state.price_update_tx.subscribe();
    let mut interval = interval(Duration::from_secs(300)); // 5 minutes

    loop {
        tokio::select! {
            // Handle explicit price update requests
            mint_result = rx.recv() => {
                if let Ok(mint) = mint_result {
                    if let Err(e) = refresh_token_price(&state, &mint).await {
                        error!("Failed to refresh price for {}: {}", mint, e);
                    }
                }
            }
            // Periodic price refresh for active tokens
            _ = interval.tick() => {
                if let Err(e) = refresh_active_token_prices(&state).await {
                    error!("Failed to refresh active token prices: {}", e);
                }
            }
        }
    }
}

/// Refresh price for a specific token
async fn refresh_token_price(state: &AppState, mint: &str) -> anyhow::Result<()> {
    info!("Refreshing price for token: {}", mint);

    // In a full implementation, this would:
    // 1. Query Jupiter API for current price
    // 2. Store price in token_prices table
    // 3. Update price materialized views

    Ok(())
}

/// Refresh prices for recently active tokens
async fn refresh_active_token_prices(state: &AppState) -> anyhow::Result<()> {
    // Get tokens that have been active in the last hour
    let active_mints = sqlx::query_scalar!(
        "SELECT DISTINCT mint FROM actions
         WHERE ts >= NOW() - INTERVAL '1 hour'
         AND mint IS NOT NULL
         LIMIT 100"
    )
    .fetch_all(&state.pg.0)
    .await?;

    info!("Refreshing prices for {} active tokens", active_mints.len());

    for mint in active_mints {
        if let Some(mint) = mint {
            let _ = refresh_token_price(state, &mint).await;
            // Small delay to avoid overwhelming APIs
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    Ok(())
}

/// Background worker for metrics cleanup
async fn metrics_cleanup_worker(state: AppState) {
    let mut interval = interval(Duration::from_secs(3600)); // 1 hour

    loop {
        interval.tick().await;

        // Clean up old transaction records (keep last 30 days)
        let cleanup_result =
            sqlx::query!("DELETE FROM tx_raw WHERE ts < NOW() - INTERVAL '30 days'")
                .execute(&state.pg.0)
                .await;

        match cleanup_result {
            Ok(result) => {
                let deleted = result.rows_affected();
                if deleted > 0 {
                    info!("Cleaned up {} old transaction records", deleted);
                }
            }
            Err(e) => error!("Failed to cleanup old transactions: {}", e),
        }
    }
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    let db_ok = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pg.0)
        .await
        .is_ok();

    let store_ok = state
        .store
        .put("health/indexer-check.txt", b"ok")
        .await
        .is_ok();

    Json(serde_json::json!({
        "status": if db_ok && store_ok { "healthy" } else { "unhealthy" },
        "database": db_ok,
        "object_store": store_ok,
        "timestamp": time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339).unwrap_or_default()
    }))
}

/// Webhook statistics endpoint
async fn webhook_stats(State(state): State<AppState>) -> Json<WebhookStats> {
    // Get basic stats from the last 24 hours
    let processed_count =
        sqlx::query_scalar!("SELECT COUNT(*) FROM tx_raw WHERE ts >= NOW() - INTERVAL '24 hours'")
            .fetch_one(&state.pg.0)
            .await
            .unwrap_or(0) as u64;

    let last_processed = sqlx::query_scalar!("SELECT ts FROM tx_raw ORDER BY ts DESC LIMIT 1")
        .fetch_optional(&state.pg.0)
        .await
        .ok()
        .flatten()
        .map(|ts| {
            ts.format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default()
        });

    Json(WebhookStats {
        processed_count,
        error_count: 0, // Would track in production
        last_processed,
        avg_processing_time_ms: 0.0, // Would calculate in production
    })
}

/// Manual price refresh endpoint
async fn manual_price_refresh(
    State(state): State<AppState>,
    Path(mint): Path<String>,
) -> StatusCode {
    match refresh_token_price(&state, &mint).await {
        Ok(_) => {
            info!("Manually triggered price refresh for {}", mint);
            StatusCode::OK
        }
        Err(e) => {
            error!("Failed to refresh price for {}: {}", mint, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}
