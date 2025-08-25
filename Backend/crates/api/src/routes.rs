use crate::auth_mw::AuthUser;
use async_stream::stream;
use axum::{
    extract::{ws::WebSocket, Path, Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive},
        IntoResponse, Response, Sse,
    },
    Json,
};
use base64::{engine::general_purpose::URL_SAFE as BASE64_URL_SAFE, Engine};
use futures::{stream::StreamExt as _, SinkExt, Stream, StreamExt};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use shared::{
    constants::api::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE},
    observability::{HealthChecker, MetricsRegistry},
    store::ObjectStore,
    utils::{new_id, new_request_id, truncate_wallet},
    validation::{validate_moment_kinds, validate_pagination, validate_wallet_address},
    ApiError, ApiResult, AppConfig, AuthMethod, MaybeRedis, Metrics, Pg, PolicyService,
    UserContext,
};
use sqlx::Row;
use std::{collections::HashMap, sync::Arc};
use time::OffsetDateTime;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::{error, info, instrument, warn};
use ulid::Ulid;

pub mod tokens;
pub mod campaigns;

#[derive(Clone)]
pub struct AppState {
    pub cfg: AppConfig,
    pub pg: Pg,
    pub metrics: Metrics,
    pub redis: MaybeRedis,
    pub store: Arc<dyn ObjectStore>,
    pub policy_service: PolicyService,
    pub sse_broadcast: broadcast::Sender<String>,
    pub metrics_registry: Arc<MetricsRegistry>,
    pub health_checker: Arc<HealthChecker>,
}

impl AppState {
    pub fn new(
        cfg: AppConfig,
        pg: Pg,
        metrics: Metrics,
        redis: MaybeRedis,
        store: Arc<dyn ObjectStore>,
        metrics_registry: MetricsRegistry,
        health_checker: Arc<HealthChecker>,
    ) -> Self {
        let policy_service = PolicyService::new(pg.0.clone());
        let (sse_broadcast, _) = broadcast::channel(1000);

        Self {
            cfg,
            pg,
            metrics,
            redis,
            store,
            policy_service,
            sse_broadcast,
            metrics_registry: Arc::new(metrics_registry),
            health_checker,
        }
    }
}

// Auth verification request/response
#[derive(Deserialize)]
pub struct AuthVerifyRequest {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthVerifyResponse {
    pub user_id: String,
    pub wallet_address: Option<String>,
    pub email: Option<String>,
}

// NFT minting request/response
#[derive(Deserialize)]
pub struct MintNftRequest {
    pub moment_id: String,
}

#[derive(Serialize)]
pub struct MintNftResponse {
    pub job_id: String,
    pub status: String,
    pub message: String,
}

/// POST /auth/verify - Verify Dynamic.xyz JWT token
#[instrument(skip(state))]
pub async fn auth_verify(
    State(state): State<AppState>,
    Json(payload): Json<AuthVerifyRequest>,
) -> ApiResult<Json<AuthVerifyResponse>> {
    // Use the shared auth module to verify the token
    let claims = shared::auth::verify_dynamic_jwt(
        &payload.token,
        &state.cfg.dynamic_jwks_url,
        &state.cfg.dynamic_environment_id,
    ).await.map_err(|_| ApiError::Unauthorized)?;

    Ok(Json(AuthVerifyResponse {
        user_id: claims.sub,
        wallet_address: claims.wallet_public_key,
        email: claims.email,
    }))
}

/// POST /v1/moments/:id/mint - Mint an NFT for a moment
#[instrument(skip(state, user))]
pub async fn mint_moment_nft(
    State(state): State<AppState>,
    user: AuthUser,
    Path(moment_id): Path<String>,
    Json(_payload): Json<MintNftRequest>,
) -> ApiResult<Json<MintNftResponse>> {
    // Verify the moment exists and belongs to the user
    let moment_exists: bool = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM oof_moments WHERE id = $1 AND wallet = $2)",
        moment_id,
        user.wallet_address.as_ref().unwrap_or(&"".to_string())
    )
    .fetch_one(&state.pg.0)
    .await
    .map_err(|_| ApiError::InternalServerError)?
    .unwrap_or(false);

    if !moment_exists {
        return Err(ApiError::NotFound("Moment not found or not owned by user".to_string()));
    }

    // Create a job to mint the NFT
    let job_id = new_id();
    let payload = serde_json::json!({
        "moment_id": moment_id,
        "owner_wallet": user.wallet_address
    });

    sqlx::query!(
        "INSERT INTO job_queue (id, kind, payload_json, max_attempts, run_after, created_at) VALUES ($1, $2, $3, $4, $5, $6)",
        job_id,
        "mint_nft",
        payload,
        3,
        OffsetDateTime::now_utc(),
        OffsetDateTime::now_utc()
    )
    .execute(&state.pg.0)
    .await
    .map_err(|_| ApiError::InternalServerError)?;

    Ok(Json(MintNftResponse {
        job_id,
        status: "queued".to_string(),
        message: "NFT minting job queued successfully".to_string(),
    }))
}

/// GET /v1/moments/:id/nft - Get NFT details for a moment
#[instrument(skip(state))]
pub async fn get_moment_nft(
    State(state): State<AppState>,
    Path(moment_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let row = sqlx::query!(
        "SELECT id, moment_id, nft_mint_address, metadata_uri, owner_wallet, created_at FROM minted_cards WHERE moment_id = $1",
        moment_id
    )
    .fetch_optional(&state.pg.0)
    .await
    .map_err(|_| ApiError::InternalServerError)?;

    match row {
        Some(record) => {
            let nft_details = serde_json::json!({
                "id": record.id,
                "moment_id": record.moment_id,
                "nft_mint_address": record.nft_mint_address,
                "metadata_uri": record.metadata_uri,
                "owner_wallet": record.owner_wallet,
                "created_at": record.created_at
            });
            Ok(Json(nft_details))
        }
        None => Err(ApiError::NotFound("NFT not found for this moment".to_string())),
    }
}

fn compute_display_meta(kind: &str, severity_dec: Option<&str>) -> DisplayMeta {
    let sev = severity_dec.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let rarity = if sev > 0.8 {
        "legendary"
    } else if sev > 0.4 {
        "epic"
    } else {
        "rare"
    };

    let (emoji, from, to) = match kind {
        k if k.contains("sold") || k.contains("s2e") => ("💎", "from-green-600/40", "to-emerald-700/40"),
        k if k.contains("bag") || k.contains("bhd") => ("📄", "from-rose-600/40", "to-orange-600/40"),
        _ => ("🧹", "from-slate-600/40", "to-gray-700/40"),
    };

    DisplayMeta {
        emoji: emoji.to_string(),
        gradient_from: from.to_string(),
        gradient_to: to.to_string(),
        rarity: rarity.to_string(),
    }
}

/// GET /v1/openapi.json - Serve static OpenAPI spec for v1
pub async fn openapi_spec() -> Response {
    let body = include_str!("../../../schemas/http/openapi.v1.json");
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body,
    )
        .into_response()
}

/// GET /ready - Readiness probe: verify core dependencies
#[instrument(skip(state))]
pub async fn ready(State(state): State<AppState>) -> Response {
    // Check DB
    let db_ok = sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pg.0)
        .await
        .map(|v| v == 1)
        .unwrap_or(false);

    // Optionally check Redis
    let redis_ok = match &state.redis.0 {
        Some(client) => client.exists("oof:ready:probe").await.unwrap_or(false) || true,
        None => true, // optional dep
    };

    // Optionally attempt a light object-store write (best-effort)
    let store_ok = match state
        .store
        .put("health/ready.txt", b"ok")
        .await
    {
        Ok(_) => true,
        Err(_) => state.cfg.environment != "production", // allow fail in non-prod
    };

    let ok = db_ok && redis_ok && store_ok;
    if ok {
        (axum::http::StatusCode::OK, "ready").into_response()
    } else {
        (axum::http::StatusCode::SERVICE_UNAVAILABLE, "not ready").into_response()
    }
}

// Request/Response DTOs
#[derive(Deserialize)]
pub struct AnalyzeRequest {
    pub wallets: Vec<String>,
    #[serde(rename = "planCode")]
    pub plan_code: Option<String>,
}

#[derive(Serialize)]
pub struct AnalyzeResponse {
    #[serde(rename = "jobId")]
    pub job_id: String,
    pub status: String,
    pub wallets: Vec<String>,
    #[serde(rename = "estimatedTimeSeconds")]
    pub estimated_time_seconds: u32,
}

#[derive(Deserialize)]
pub struct MomentsQuery {
    pub wallet: Option<String>,
    pub kinds: Option<String>,
    pub since: Option<String>,
    pub min_usd: Option<String>,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

#[derive(Serialize)]
pub struct DisplayMeta {
    pub emoji: String,
    #[serde(rename = "gradientFrom")]
    pub gradient_from: String,
    #[serde(rename = "gradientTo")]
    pub gradient_to: String,
    pub rarity: String,
}

#[derive(Serialize)]
pub struct MomentDto {
    pub id: String,
    pub wallet: String,
    pub mint: Option<String>,
    pub kind: String,
    #[serde(rename = "tEvent")]
    pub t_event: String,
    #[serde(rename = "pctDec")]
    pub pct_dec: Option<String>,
    #[serde(rename = "missedUsdDec")]
    pub missed_usd_dec: Option<String>,
    #[serde(rename = "severityDec")]
    pub severity_dec: Option<String>,
    #[serde(rename = "sigRef")]
    pub sig_ref: Option<String>,
    #[serde(rename = "slotRef")]
    pub slot_ref: Option<i64>,
    pub version: Option<String>,
    #[serde(rename = "explainJson")]
    pub explain_json: serde_json::Value,
    #[serde(rename = "previewPngUrl")]
    pub preview_png_url: Option<String>,
    #[serde(rename = "cardUrl")]
    pub card_url: String,
    #[serde(rename = "display", skip_serializing_if = "Option::is_none")]
    pub display: Option<DisplayMeta>,
    #[serde(rename = "tokenSymbol", skip_serializing_if = "Option::is_none")]
    pub token_symbol: Option<String>,
    #[serde(rename = "tokenLogoUrl", skip_serializing_if = "Option::is_none")]
    pub token_logo_url: Option<String>,
}

#[derive(Serialize)]
pub struct MomentsListResponse {
    pub data: Vec<MomentDto>,
    pub pagination: PaginationInfo,
    pub total_count: Option<i64>,
}

#[derive(Serialize)]
pub struct PaginationInfo {
    pub limit: usize,
    pub cursor: Option<String>,
    #[serde(rename = "hasMore")]
    pub has_more: bool,
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

#[derive(Serialize)]
pub struct WalletSummaryResponse {
    pub wallet: String,
    pub holdings: Vec<HoldingDto>,
    #[serde(rename = "realizedPnlUsd")]
    pub realized_pnl_usd: String,
    pub counts: MomentCounts,
    #[serde(rename = "lastAnalyzed")]
    pub last_analyzed: Option<String>,
    #[serde(rename = "analysisRange")]
    pub analysis_range: Option<AnalysisRange>,
}

#[derive(Serialize)]
pub struct HoldingDto {
    pub mint: String,
    pub symbol: Option<String>,
    pub balance: String,
    #[serde(rename = "valueUsd")]
    pub value_usd: String,
    #[serde(rename = "unrealizedPnlUsd")]
    pub unrealized_pnl_usd: String,
}

#[derive(Serialize)]
pub struct MomentCounts {
    pub s2e: i64,
    pub bhd: i64,
    pub bad_route: i64,
    pub idle: i64,
    pub rug: i64,
    pub total: i64,
}

#[derive(Serialize)]
pub struct AnalysisRange {
    pub from: String,
    pub to: String,
    #[serde(rename = "daysAnalyzed")]
    pub days_analyzed: i64,
}

#[derive(Serialize)]
pub struct WalletExtremesResponse {
    pub wallet: String,
    #[serde(rename = "computedAt")]
    pub computed_at: String,
    #[serde(rename = "highestWin")]
    pub highest_win: Option<ExtremeDto>,
    #[serde(rename = "highestLoss")]
    pub highest_loss: Option<ExtremeDto>,
    #[serde(rename = "topS2E")]
    pub top_s2e: Option<ExtremeDto>,
    #[serde(rename = "worstBHD")]
    pub worst_bhd: Option<ExtremeDto>,
    #[serde(rename = "worstBadRoute")]
    pub worst_bad_route: Option<ExtremeDto>,
    #[serde(rename = "largestIdle")]
    pub largest_idle: Option<ExtremeDto>,
}

#[derive(Serialize)]
pub struct ExtremeDto {
    pub id: String,
    pub mint: Option<String>,
    #[serde(rename = "valueUsd")]
    pub value_usd: String,
    #[serde(rename = "valuePct")]
    pub value_pct: Option<String>,
    pub timestamp: String,
    #[serde(rename = "cardUrl")]
    pub card_url: String,
}

#[derive(Deserialize)]
pub struct PriceQuery {
    pub tf: Option<String>, // timeframe
    pub since: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Serialize)]
pub struct PricePoint {
    pub timestamp: String,
    pub price: String,
    pub source: String,
}

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    pub period: Option<String>, // 7d, 30d, 90d
    pub metric: Option<String>, // highest_gains, most_s2e, worst_bhd
    pub limit: Option<usize>,
}

// API Route Handlers

/// POST /v1/analyze - Start wallet analysis
#[instrument(skip(state), fields(user_id, wallet_count))]
pub async fn analyze(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<AnalyzeRequest>,
) -> ApiResult<Json<AnalyzeResponse>> {
    let request_id = new_request_id();
    tracing::Span::current().record("user_id", &user.user_id);
    tracing::Span::current().record("wallet_count", req.wallets.len());

    // Validate wallets
    for wallet in &req.wallets {
        validate_wallet_address(wallet)?;
    }

    if req.wallets.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one wallet required".to_string(),
        ));
    }

    if req.wallets.len() > 25 {
        return Err(ApiError::BadRequest(
            "Maximum 25 wallets per request".to_string(),
        ));
    }

    // Check user plan and quota
    let user_context = state.policy_service.get_user_context(&user.user_id).await?;
    if !user_context.can_analyze_wallet() {
        return Err(ApiError::QuotaExceeded);
    }

    // Consume quota
    state
        .policy_service
        .consume_analysis_quota(&user.user_id, req.wallets.len() as i32)
        .await?;

    // Enqueue backfill jobs
    let job_id = new_id();
    for wallet in &req.wallets {
        let backfill_payload = serde_json::json!({
            "wallet": wallet,
            "backfill_days": user_context.plan.backfill_days,
            "max_signatures": user_context.plan.max_signatures_per_run
        });

        sqlx::query!(
            include_str!("../../../db/queries/enqueue_job.sql"),
            new_id(),
            "backfill",
            backfill_payload,
            OffsetDateTime::now_utc(),
            5i32
        )
        .execute(&state.pg.0)
        .await?;
    }

    // Enqueue compute job
    let compute_payload = serde_json::json!({
        "wallets": req.wallets
    });

    sqlx::query!(
        include_str!("../../../db/queries/enqueue_job.sql"),
        job_id.clone(),
        "compute",
        compute_payload,
        OffsetDateTime::now_utc(),
        5i32
    )
    .execute(&state.pg.0)
    .await?;

    // Estimate completion time based on plan
    let estimated_time = match user_context.plan.perks.priority_queue {
        true => 30,   // Priority queue
        false => 120, // Regular queue
    };

    state.metrics.increment_counter("analysis_requests_total");

    info!(
        user_id = %user.user_id,
        job_id = %job_id,
        wallet_count = req.wallets.len(),
        "Analysis job enqueued"
    );

    Ok(Json(AnalyzeResponse {
        job_id,
        status: "queued".to_string(),
        wallets: req.wallets,
        estimated_time_seconds: estimated_time,
    }))
}

/// GET /v1/analyze/:job/stream - SSE stream for job progress
#[instrument(skip(state))]
pub async fn analyze_stream(State(state): State<AppState>, Path(job_id): Path<String>) -> Response {
    let stream = stream! {
        // Send initial status
        yield Ok(Event::default().event("progress").data("Checking job status..."));

        let mut last_status = String::new();
        let mut progress_count = 0;

        loop {
            // Check job status
            let status = sqlx::query_scalar!(
                "SELECT status FROM job_queue WHERE id = $1",
                job_id
            )
            .fetch_optional(&state.pg.0)
            .await
            .ok()
            .flatten();

            match status.as_deref() {
                Some("queued") => {
                    if last_status != "queued" {
                        yield Ok(Event::default().event("progress").data("Queued for processing..."));
                        last_status = "queued".to_string();
                    }
                }
                Some("running") => {
                    if last_status != "running" {
                        yield Ok(Event::default().event("progress").data("Processing wallet data..."));
                        last_status = "running".to_string();
                    }

                    progress_count += 1;
                    if progress_count % 10 == 0 {
                        yield Ok(Event::default().event("progress").data("Still processing..."));
                    }
                }
                Some("done") => {
                    yield Ok(Event::default().event("progress").data("Analysis complete!"));
                    yield Ok(Event::default().event("done").data(&job_id));
                    break;
                }
                Some("failed") => {
                    yield Ok(Event::default().event("error").data("Analysis failed"));
                    break;
                }
                None => {
                    yield Ok(Event::default().event("error").data("Job not found"));
                    break;
                }
                _ => {}
            }

            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
    };

    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

/// GET /v1/moments - List moments with filtering and pagination
#[instrument(skip(state))]
pub async fn moments_list(
    State(state): State<AppState>,
    Query(query): Query<MomentsQuery>,
) -> ApiResult<Json<MomentsListResponse>> {
    let (limit, cursor) = validate_pagination(query.limit, query.cursor.as_deref())?;
    let moment_kinds = validate_moment_kinds(query.kinds.as_deref())?;

    // Validate wallet if provided
    if let Some(ref wallet) = query.wallet {
        validate_wallet_address(wallet)?;
    }

    let mut params: Vec<&dyn sqlx::postgres::PgHasArrayType> = Vec::new();
    let mut param_idx = 1;
    let mut where_clauses = Vec::new();

    // Build query dynamically
    if let Some(ref wallet) = query.wallet {
        where_clauses.push(format!("wallet = ${}", param_idx));
        param_idx += 1;
    }

    if !moment_kinds.is_empty() {
        let kinds_str: Vec<String> = moment_kinds
            .iter()
            .map(|k| k.as_str().to_string())
            .collect();
        where_clauses.push(format!("kind = ANY(${}))", param_idx));
        param_idx += 1;
    }

    if let Some(ref since) = query.since {
        if let Ok(since_ts) =
            time::OffsetDateTime::parse(since, &time::format_description::well_known::Rfc3339)
        {
            where_clauses.push(format!("t_event >= ${}", param_idx));
            param_idx += 1;
        }
    }

    if let Some(ref min_usd) = query.min_usd {
        if let Ok(min_amount) = min_usd.parse::<Decimal>() {
            where_clauses.push(format!("missed_usd_dec >= ${}", param_idx));
            param_idx += 1;
        }
    }

    // Cursor pagination
    if let Some(ref cursor) = cursor {
        if let Ok(decoded) = BASE64_URL_SAFE.decode(cursor) {
            if let Ok(cursor_str) = String::from_utf8(decoded) {
                if let Some((ts_str, id_str)) = cursor_str.split_once('|') {
                    where_clauses.push(format!(
                        "(t_event, id) < (${}, ${})",
                        param_idx,
                        param_idx + 1
                    ));
                    param_idx += 2;
                }
            }
        }
    }

    let where_clause = if where_clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_clauses.join(" AND "))
    };

    let sql = format!(
        "SELECT m.id, m.wallet, m.mint, m.kind, m.t_event, m.pct_dec, m.missed_usd_dec,
               m.severity_dec, m.sig_ref, m.slot_ref, m.version, m.explain_json, m.preview_png_url,
               tf.symbol as token_symbol, tf.logo_url as token_logo_url
         FROM oof_moments m
         LEFT JOIN token_facts tf ON tf.mint = m.mint
         {}
         ORDER BY m.t_event DESC, m.id DESC
         LIMIT {}",
        where_clause,
        limit + 1 // +1 to check if there are more results
    );

    // Execute query with proper parameter binding
    let mut query_builder = sqlx::query(&sql);

    if let Some(ref wallet) = query.wallet {
        query_builder = query_builder.bind(wallet);
    }

    let rows = query_builder.fetch_all(&state.pg.0).await?;

    let has_more = rows.len() > limit;
    let data_rows = if has_more { &rows[..limit] } else { &rows };

    let data: Vec<MomentDto> = data_rows
        .iter()
        .map(|row| {
            let id: String = row.get("id");
            let t_event: OffsetDateTime = row.get("t_event");

            {
                let kind: String = row.get("kind");
                let severity_str = row
                    .try_get::<Option<Decimal>, _>("severity_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string());
                let display = compute_display_meta(&kind, severity_str.as_deref());

                MomentDto {
                id: id.clone(),
                wallet: row.get("wallet"),
                mint: row.try_get("mint").ok(),
                kind,
                t_event: t_event
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                pct_dec: row
                    .try_get::<Option<Decimal>, _>("pct_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string()),
                missed_usd_dec: row
                    .try_get::<Option<Decimal>, _>("missed_usd_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string()),
                severity_dec: row
                    .try_get::<Option<Decimal>, _>("severity_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string()),
                sig_ref: row.try_get("sig_ref").ok(),
                slot_ref: row.try_get("slot_ref").ok(),
                version: row.try_get("version").ok(),
                explain_json: row
                    .try_get::<serde_json::Value, _>("explain_json")
                    .unwrap_or(serde_json::json!({})),
                preview_png_url: row.try_get("preview_png_url").ok(),
                card_url: format!(
                    "{}/v1/cards/moment/{}.png",
                    state.cfg.cdn_base.trim_end_matches('/'),
                    id
                ),
                display: Some(display),
                token_symbol: row.try_get("token_symbol").ok(),
                token_logo_url: row.try_get("token_logo_url").ok(),
            }
        })
        .collect();

    let next_cursor = if has_more && !data.is_empty() {
        let last_item = data.last().unwrap();
        let cursor_data = format!("{}|{}", last_item.t_event, last_item.id);
        Some(BASE64_URL_SAFE.encode(&cursor_data))
    } else {
        None
    };

    state
        .metrics
        .increment_counter("moments_list_requests_total");

    Ok(Json(MomentsListResponse {
        data,
        pagination: PaginationInfo {
            limit,
            cursor,
            has_more,
            next_cursor,
        },
        total_count: None, // Expensive to compute, omit for performance
    }))
}

/// GET /v1/moments/:id - Get moment details by ID
#[instrument(skip(state))]
pub async fn moment_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<MomentDto>> {
    // Validate ID format (ULID)
    if id.len() != 26 {
        return Err(ApiError::BadRequest("Invalid moment ID format".to_string()));
    }

    let row = sqlx::query(
        "SELECT m.id, m.wallet, m.mint, m.kind, m.t_event, m.pct_dec, m.missed_usd_dec,
                m.severity_dec, m.sig_ref, m.slot_ref, m.version, m.explain_json, m.preview_png_url,
                tf.symbol as token_symbol, tf.logo_url as token_logo_url
         FROM oof_moments m
         LEFT JOIN token_facts tf ON tf.mint = m.mint
         WHERE m.id = $1"
    )
    .bind(&id)
    .fetch_optional(&state.pg.0)
    .await?;

    match row {
        Some(row) => {
            let kind: String = row.get("kind");
            let severity_str: Option<String> = row
                .try_get::<Option<Decimal>, _>("severity_dec")
                .ok()
                .flatten()
                .map(|d| d.to_string());
            let display = compute_display_meta(&kind, severity_str.as_deref());

            let t_event: OffsetDateTime = row.get("t_event");
            let moment_dto = MomentDto {
                id: row.get("id"),
                wallet: row.get("wallet"),
                mint: row.try_get("mint").ok(),
                kind,
                t_event: t_event
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                pct_dec: row
                    .try_get::<Option<Decimal>, _>("pct_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string()),
                missed_usd_dec: row
                    .try_get::<Option<Decimal>, _>("missed_usd_dec")
                    .ok()
                    .flatten()
                    .map(|d| d.to_string()),
                severity_dec: severity_str,
                sig_ref: row.try_get("sig_ref").ok(),
                slot_ref: row.try_get("slot_ref").ok(),
                version: row.try_get("version").ok(),
                explain_json: row
                    .try_get::<serde_json::Value, _>("explain_json")
                    .unwrap_or(serde_json::json!({})),
                preview_png_url: row.try_get("preview_png_url").ok(),
                card_url: format!(
                    "{}/v1/cards/moment/{}.png",
                    state.cfg.cdn_base.trim_end_matches('/'),
                    id
                ),
                display: Some(display),
                token_symbol: row.try_get("token_symbol").ok(),
                token_logo_url: row.try_get("token_logo_url").ok(),
            };

            state.metrics.increment_counter("moment_detail_requests_total");
            Ok(Json(moment_dto))
        }
        None => Err(ApiError::NotFound("Moment not found".to_string())),
    }
}

/// GET /v1/wallets/:wallet/summary - Get wallet holdings and summary
#[instrument(skip(state))]
pub async fn wallet_summary(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> ApiResult<Json<WalletSummaryResponse>> {
    validate_wallet_address(&wallet)?;

    // Get current holdings
    let holdings_rows = sqlx::query!(
        include_str!("../../../db/queries/select_wallet_holdings.sql"),
        wallet
    )
    .fetch_all(&state.pg.0)
    .await?;

    let holdings: Vec<HoldingDto> = holdings_rows
        .into_iter()
        .map(|row| HoldingDto {
            mint: row.mint,
            symbol: row.symbol,
            balance: row.balance_dec.to_string(),
            value_usd: row
                .value_usd_dec
                .map(|v| v.to_string())
                .unwrap_or("0".to_string()),
            unrealized_pnl_usd: row
                .unrealized_pnl_dec
                .map(|p| p.to_string())
                .unwrap_or("0".to_string()),
        })
        .collect();

    // Get realized P&L
    let realized_pnl = sqlx::query_scalar!(
        "SELECT COALESCE(SUM(realized_pnl_usd_dec), 0) FROM realized_trades WHERE wallet = $1",
        wallet
    )
    .fetch_one(&state.pg.0)
    .await?
    .unwrap_or_default();

    // Get moment counts
    let counts_row = sqlx::query!(
        "SELECT kind, COUNT(*) as count FROM oof_moments WHERE wallet = $1 GROUP BY kind",
        wallet
    )
    .fetch_all(&state.pg.0)
    .await?;

    let mut counts = MomentCounts {
        s2e: 0,
        bhd: 0,
        bad_route: 0,
        idle: 0,
        rug: 0,
        total: 0,
    };

    for row in counts_row {
        let count = row.count.unwrap_or(0);
        counts.total += count;
        match row.kind.as_str() {
            "sold_too_early" => counts.s2e = count,
            "bag_holder_drawdown" => counts.bhd = count,
            "bad_route" => counts.bad_route = count,
            "idle_yield" => counts.idle = count,
            "rug" => counts.rug = count,
            _ => {}
        }
    }

    // Get analysis metadata
    let analysis_info = sqlx::query!(
        "SELECT MIN(t_start) as earliest, MAX(t_end) as latest,
                MAX(updated_at) as last_analyzed
         FROM wallet_coverage WHERE wallet = $1",
        wallet
    )
    .fetch_optional(&state.pg.0)
    .await?;

    let (last_analyzed, analysis_range) = match analysis_info {
        Some(info) => {
            let last_analyzed = info.last_analyzed.map(|dt| {
                dt.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            });

            let analysis_range = match (info.earliest, info.latest) {
                (Some(start), Some(end)) => {
                    let days = (end - start).whole_days();
                    Some(AnalysisRange {
                        from: start
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default(),
                        to: end
                            .format(&time::format_description::well_known::Rfc3339)
                            .unwrap_or_default(),
                        days_analyzed: days,
                    })
                }
                _ => None,
            };

            (last_analyzed, analysis_range)
        }
        None => (None, None),
    };

    state
        .metrics
        .increment_counter("wallet_summary_requests_total");

    Ok(Json(WalletSummaryResponse {
        wallet,
        holdings,
        realized_pnl_usd: realized_pnl.to_string(),
        counts,
        last_analyzed,
        analysis_range,
    }))
}

/// GET /v1/wallets/:wallet/extremes - Get wallet's extreme moments and trades
#[instrument(skip(state))]
pub async fn wallet_extremes(
    State(state): State<AppState>,
    Path(wallet): Path<String>,
) -> ApiResult<Json<WalletExtremesResponse>> {
    validate_wallet_address(&wallet)?;

    let extremes_row = sqlx::query!(
        include_str!("../../../db/queries/calculate_wallet_extremes.sql"),
        wallet
    )
    .fetch_optional(&state.pg.0)
    .await?;

    let computed_at = OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default();

    let (highest_win, highest_loss, top_s2e, worst_bhd, worst_bad_route, largest_idle) =
        match extremes_row {
            Some(row) => {
                // Parse the extremes JSON from the query result
                let extremes_json: serde_json::Value =
                    row.extremes_json.unwrap_or(serde_json::json!({}));

                let highest_win = extremes_json
                    .get("highest_win")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                let highest_loss = extremes_json
                    .get("highest_loss")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                let top_s2e = extremes_json
                    .get("top_s2e")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                let worst_bhd = extremes_json
                    .get("worst_bhd")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                let worst_bad_route = extremes_json
                    .get("worst_bad_route")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                let largest_idle = extremes_json
                    .get("largest_idle")
                    .and_then(|v| serde_json::from_value::<ExtremeDto>(v.clone()).ok());

                (
                    highest_win,
                    highest_loss,
                    top_s2e,
                    worst_bhd,
                    worst_bad_route,
                    largest_idle,
                )
            }
            None => (None, None, None, None, None, None),
        };

    state
        .metrics
        .increment_counter("wallet_extremes_requests_total");

    Ok(Json(WalletExtremesResponse {
        wallet,
        computed_at,
        highest_win,
        highest_loss,
        top_s2e,
        worst_bhd,
        worst_bad_route,
        largest_idle,
    }))
}

/// GET /health - Health check endpoint
#[instrument(skip(state))]
pub async fn health(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    // Update database health check
    let db_start = std::time::Instant::now();
    let db_result = sqlx::query_scalar::<_, i64>("SELECT 1")
        .fetch_one(&state.pg.0)
        .await;
    let db_latency = db_start.elapsed().as_millis();
    let db_status = if db_result.is_ok() {
        shared::observability::HealthStatus::Healthy
    } else {
        shared::observability::HealthStatus::Unhealthy
    };

    state
        .health_checker
        .update_check(
            "database".to_string(),
            db_status,
            Some(format!("Latency: {}ms", db_latency)),
            Some(db_latency as u64),
            None,
        )
        .await;

    // Update object store health check
    let store_start = std::time::Instant::now();
    let store_result = state.store.put("health/check.txt", b"ok").await;
    let store_latency = store_start.elapsed().as_millis();
    let store_status = if store_result.is_ok() {
        shared::observability::HealthStatus::Healthy
    } else {
        shared::observability::HealthStatus::Unhealthy
    };

    state
        .health_checker
        .update_check(
            "object_store".to_string(),
            store_status,
            Some(format!("Latency: {}ms", store_latency)),
            Some(store_latency as u64),
            None,
        )
        .await;

    // Update Redis health check (if configured)
    if let Some(ref redis) = state.redis {
        let redis_start = std::time::Instant::now();
        let redis_result = redis.ping().await;
        let redis_latency = redis_start.elapsed().as_millis();
        let redis_status = if redis_result.is_ok() {
            shared::observability::HealthStatus::Healthy
        } else {
            shared::observability::HealthStatus::Unhealthy
        };

        state
            .health_checker
            .update_check(
                "redis".to_string(),
                redis_status,
                Some(format!("Latency: {}ms", redis_latency)),
                Some(redis_latency as u64),
                None,
            )
            .await;
    }

    // Get comprehensive health report
    let health_report = state.health_checker.get_health_report().await;

    // Update metrics
    state
        .metrics
        .increment_counter("health_check_requests_total");
    state
        .metrics_registry
        .http_requests_total
        .with_label_values(&["GET", "/health", "200"])
        .inc();

    Ok(Json(serde_json::to_value(health_report)?))
}

/// GET /metrics - Prometheus metrics endpoint
#[instrument(skip(state))]
pub async fn metrics(State(state): State<AppState>) -> Result<String, StatusCode> {
    let metrics_data = state.metrics_registry.gather();
    Ok(metrics_data)
}

pub async fn card_png(State(st): State<AppState>, Path(id): Path<String>) -> impl IntoResponse {
    // Load moment for context
    let r = sqlx::query("SELECT wallet, kind, mint, t_event FROM oof_moments WHERE id=$1")
        .bind(&id)
        .fetch_optional(&st.pg.0)
        .await
        .ok()
        .flatten();
    let kind_v: String = r
        .as_ref()
        .map(|row| row.get::<String, _>("kind"))
        .unwrap_or_else(|| "OOF".into());
    let title = format!("{}", kind_v);
    let subtitle = r
        .as_ref()
        .map(|row| {
            let wallet: String = row.get("wallet");
            let mint: Option<String> = row.try_get("mint").ok();
            format!("{} {}", wallet, mint.unwrap_or_default())
        })
        .unwrap_or_default();
    let card = renderer::MomentCard {
        title,
        subtitle,
        kind: kind_v,
        primary: "".into(),
    };
    match renderer::render_moment_card(&card, "dark", "1200x630") {
        Ok(png) => {
            let key = format!("cards/moments/{}.png", id);
            let _ = st.store.put(&key, &png).await;
            let url = format!("{}/{}", st.cfg.cdn_base.trim_end_matches('/'), key);
            let _ = sqlx::query("UPDATE oof_moments SET preview_png_url=$2 WHERE id=$1")
                .bind(&id)
                .bind(&url)
                .execute(&st.pg.0)
                .await;
            (
                axum::http::StatusCode::OK,
                [("Content-Type", "image/png")],
                png,
            )
        }
        Err(_) => (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            [("Content-Type", "text/plain")],
            b"render error".to_vec(),
        ),
    }
}

/// GET /v1/tokens/:mint/prices - Get historical price data for a token
#[instrument(skip(state))]
pub async fn token_prices(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(query): Query<PriceQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    // Validate mint address format (basic length check)
    if mint.len() < 32 || mint.len() > 44 {
        return Err(ApiError::BadRequest(
            "Invalid mint address format".to_string(),
        ));
    }

    let since = query.since.as_deref().unwrap_or("1970-01-01T00:00:00Z");
    let limit = query.limit.unwrap_or(500).min(2000); // Cap at 2000 points

    let sql = match query.tf.as_deref() {
        Some("1m") => {
            "SELECT ts, price FROM mv_price_1m WHERE mint = $1 AND ts >= $2 ORDER BY ts ASC LIMIT $3"
        }
        Some("5m") => {
            "SELECT ts, price FROM mv_price_5m WHERE mint = $1 AND ts >= $2 ORDER BY ts ASC LIMIT $3"
        }
        Some("1h") => {
            "SELECT ts, price FROM mv_price_1h WHERE mint = $1 AND ts >= $2 ORDER BY ts ASC LIMIT $3"
        }
        _ => "SELECT ts, price FROM token_prices WHERE mint = $1 AND ts >= $2 ORDER BY ts ASC LIMIT $3",
    };

    let since_ts =
        time::OffsetDateTime::parse(since, &time::format_description::well_known::Rfc3339)
            .map_err(|_| ApiError::BadRequest("Invalid since timestamp format".to_string()))?;

    let rows = sqlx::query(sql)
        .bind(&mint)
        .bind(since_ts)
        .bind(limit as i64)
        .fetch_all(&state.pg.0)
        .await?;

    let data: Vec<PricePoint> = rows
        .into_iter()
        .map(|row| {
            let ts: OffsetDateTime = row.get("ts");
            let price: Decimal = row.get("price");
            PricePoint {
                timestamp: ts
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                price: price.to_string(),
                source: "jupiter".to_string(),
            }
        })
        .collect();

    state
        .metrics
        .increment_counter("token_prices_requests_total");

    Ok(Json(serde_json::json!({
        "mint": mint,
        "timeframe": query.tf.unwrap_or("raw".to_string()),
        "count": data.len(),
        "data": data
    })))
}

/// GET /v1/leaderboard - Get performance leaderboard
#[instrument(skip(state))]
pub async fn leaderboard(
    State(state): State<AppState>,
    Query(query): Query<LeaderboardQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let period = match query.period.as_deref() {
        Some("7d") => "7 days",
        Some("30d") => "30 days",
        Some("90d") => "90 days",
        _ => "7 days",
    };

    let metric = query.metric.as_deref().unwrap_or("highest_gains");
    let limit = query.limit.unwrap_or(50).min(100); // Cap at 100 entries

    let (sql, metric_name) = match metric {
        "highest_gains" => (
            format!(
                "SELECT wallet, SUM(realized_pnl_usd_dec) AS metric_value
                 FROM realized_trades
                 WHERE ts >= NOW() - INTERVAL '{}' AND realized_pnl_usd_dec > 0
                 GROUP BY wallet
                 ORDER BY metric_value DESC
                 LIMIT ${}",
                period
            ),
            "total_gains_usd",
        ),
        "most_s2e" => (
            format!(
                "SELECT wallet, COUNT(*) AS metric_value
                 FROM oof_moments
                 WHERE t_event >= NOW() - INTERVAL '{}' AND kind = 'sold_too_early'
                 GROUP BY wallet
                 ORDER BY metric_value DESC
                 LIMIT ${}",
                period
            ),
            "s2e_count",
        ),
        "worst_bhd" => (
            format!(
                "SELECT wallet, COUNT(*) AS metric_value
                 FROM oof_moments
                 WHERE t_event >= NOW() - INTERVAL '{}' AND kind = 'bag_holder_drawdown'
                 GROUP BY wallet
                 ORDER BY metric_value DESC
                 LIMIT ${}",
                period
            ),
            "bhd_count",
        ),
        _ => return Err(ApiError::BadRequest("Invalid metric type".to_string())),
    };

    let rows = sqlx::query(&sql)
        .bind(limit as i64)
        .fetch_all(&state.pg.0)
        .await?;

    let data: Vec<serde_json::Value> = rows
        .into_iter()
        .enumerate()
        .map(|(idx, row)| {
            let wallet: String = row.get("wallet");
            let metric_value: i64 = row
                .try_get("metric_value")
                .or_else(|_| {
                    row.try_get::<Decimal, _>("metric_value")
                        .map(|d| d.to_string().parse().unwrap_or(0))
                })
                .unwrap_or(0);

            serde_json::json!({
                "rank": idx + 1,
                "wallet": truncate_wallet(&wallet),
                "wallet_full": wallet,
                metric_name: metric_value.to_string()
            })
        })
        .collect();

    state
        .metrics
        .increment_counter("leaderboard_requests_total");

    Ok(Json(serde_json::json!({
        "period": query.period.unwrap_or("7d".to_string()),
        "metric": metric,
        "count": data.len(),
        "data": data
    })))
}
