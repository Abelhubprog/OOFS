use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use shared::{
    error::{ApiError, ApiResult},
    observability::{BuildInfo, HealthChecker, HealthReport, HealthStatus},
};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{debug, info, instrument};

use crate::state::AppState;

/// Health check query parameters
#[derive(Debug, Deserialize)]
pub struct HealthQuery {
    /// Include detailed component checks
    #[serde(default)]
    pub detailed: bool,
    /// Include build information
    #[serde(default)]
    pub build_info: bool,
    /// Include metrics summary
    #[serde(default)]
    pub metrics: bool,
}

/// Basic health check response
#[derive(Debug, Serialize)]
pub struct BasicHealthResponse {
    pub status: String,
    pub timestamp: OffsetDateTime,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Detailed health check response
#[derive(Debug, Serialize)]
pub struct DetailedHealthResponse {
    pub status: HealthStatus,
    pub timestamp: OffsetDateTime,
    pub uptime_seconds: u64,
    pub version: String,
    pub components: HashMap<String, ComponentHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_info: Option<BuildInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics_summary: Option<MetricsSummary>,
}

/// Component health status
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_checked: OffsetDateTime,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Metrics summary
#[derive(Debug, Serialize)]
pub struct MetricsSummary {
    pub total_requests: u64,
    pub error_rate: f64,
    pub avg_response_time_ms: f64,
    pub active_connections: u32,
    pub database_connections: u32,
}

/// Simple health check endpoint
#[instrument(skip(state))]
pub async fn health_check(State(state): State<AppState>) -> ApiResult<Json<BasicHealthResponse>> {
    let health_report = state.health_checker.get_health_report().await;

    Ok(Json(BasicHealthResponse {
        status: match health_report.overall_status {
            HealthStatus::Healthy => "healthy".to_string(),
            HealthStatus::Degraded => "degraded".to_string(),
            HealthStatus::Unhealthy => "unhealthy".to_string(),
            HealthStatus::Unknown => "unknown".to_string(),
        },
        timestamp: OffsetDateTime::now_utc(),
        uptime_seconds: health_report.uptime_seconds,
        version: health_report.version,
    }))
}

/// Detailed health check endpoint
#[instrument(skip(state))]
pub async fn detailed_health_check(
    State(state): State<AppState>,
    Query(query): Query<HealthQuery>,
) -> ApiResult<Json<DetailedHealthResponse>> {
    debug!("Detailed health check requested with options: {:?}", query);

    let health_report = state.health_checker.get_health_report().await;

    // Convert health checks to component health
    let components = health_report
        .checks
        .into_iter()
        .map(|check| {
            (
                check.component.clone(),
                ComponentHealth {
                    status: check.status,
                    message: check.message,
                    latency_ms: check.latency_ms,
                    last_checked: check.last_checked,
                    metadata: check.metadata,
                },
            )
        })
        .collect();

    let build_info = if query.build_info {
        Some(health_report.build_info)
    } else {
        None
    };

    let metrics_summary = if query.metrics {
        Some(get_metrics_summary(&state).await?)
    } else {
        None
    };

    Ok(Json(DetailedHealthResponse {
        status: health_report.overall_status,
        timestamp: health_report.timestamp,
        uptime_seconds: health_report.uptime_seconds,
        version: health_report.version,
        components,
        build_info,
        metrics_summary,
    }))
}

/// Readiness probe (Kubernetes compatible)
#[instrument(skip(state))]
pub async fn readiness_check(State(state): State<AppState>) -> ApiResult<StatusCode> {
    let is_ready = check_readiness(&state).await?;

    if is_ready {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Liveness probe (Kubernetes compatible)
#[instrument(skip(state))]
pub async fn liveness_check(State(state): State<AppState>) -> ApiResult<StatusCode> {
    let is_alive = check_liveness(&state).await?;

    if is_alive {
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Database health check
#[instrument(skip(state))]
pub async fn database_health_check(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let db_health = check_database_health(&state).await?;

    Ok(Json(serde_json::json!({
        "database": db_health,
        "timestamp": OffsetDateTime::now_utc()
    })))
}

/// Redis health check
#[instrument(skip(state))]
pub async fn redis_health_check(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let redis_health = check_redis_health(&state).await?;

    Ok(Json(serde_json::json!({
        "redis": redis_health,
        "timestamp": OffsetDateTime::now_utc()
    })))
}

/// External services health check
#[instrument(skip(state))]
pub async fn external_services_health_check(
    State(state): State<AppState>,
) -> ApiResult<Json<serde_json::Value>> {
    let external_health = check_external_services_health(&state).await?;

    Ok(Json(serde_json::json!({
        "external_services": external_health,
        "timestamp": OffsetDateTime::now_utc()
    })))
}

/// System metrics endpoint
#[instrument(skip(state))]
pub async fn metrics_endpoint(State(state): State<AppState>) -> ApiResult<String> {
    // Return Prometheus-formatted metrics
    let metrics = state.metrics_registry.gather();
    Ok(metrics)
}

/// Version information endpoint
#[instrument]
pub async fn version_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "description": env!("CARGO_PKG_DESCRIPTION"),
        "authors": env!("CARGO_PKG_AUTHORS").split(':').collect::<Vec<_>>(),
        "build_time": option_env!("BUILD_TIME").unwrap_or("unknown"),
        "git_hash": option_env!("GIT_HASH").unwrap_or("unknown"),
        "rust_version": option_env!("RUSTC_VERSION").unwrap_or("unknown"),
        "build_profile": if cfg!(debug_assertions) { "debug" } else { "release" },
    }))
}

// Helper functions

async fn check_readiness(state: &AppState) -> ApiResult<bool> {
    // Check if all critical services are ready
    let mut ready = true;

    // Check database connectivity
    if !check_database_connectivity(state).await? {
        ready = false;
    }

    // Check Redis connectivity (if configured)
    if state.redis.is_some() && !check_redis_connectivity(state).await? {
        ready = false;
    }

    // Check if configuration is valid
    if !validate_configuration(state).await? {
        ready = false;
    }

    Ok(ready)
}

async fn check_liveness(state: &AppState) -> ApiResult<bool> {
    // Basic liveness check - ensure the service is responsive
    let health_report = state.health_checker.get_health_report().await;

    // Service is alive if it's not completely unhealthy
    Ok(!matches!(
        health_report.overall_status,
        HealthStatus::Unhealthy
    ))
}

async fn check_database_health(state: &AppState) -> ApiResult<serde_json::Value> {
    let start_time = std::time::Instant::now();

    // Test database connection with a simple query
    let result = sqlx::query!("SELECT 1 as test")
        .fetch_one(&state.database.pool)
        .await;

    let latency_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(_) => {
            // Get additional database stats
            let pool_info = get_database_pool_info(state).await?;

            Ok(serde_json::json!({
                "status": "healthy",
                "latency_ms": latency_ms,
                "pool_info": pool_info,
                "last_checked": OffsetDateTime::now_utc()
            }))
        }
        Err(e) => Ok(serde_json::json!({
            "status": "unhealthy",
            "error": e.to_string(),
            "latency_ms": latency_ms,
            "last_checked": OffsetDateTime::now_utc()
        })),
    }
}

async fn check_redis_health(state: &AppState) -> ApiResult<serde_json::Value> {
    if let Some(redis) = &state.redis {
        let start_time = std::time::Instant::now();

        // Test Redis connection with a ping
        let result = redis.get_async_connection().await.and_then(|mut conn| {
            async move {
                use redis::AsyncCommands;
                conn.ping().await
            }
            .boxed()
        });

        let latency_ms = start_time.elapsed().as_millis() as u64;

        match result.await {
            Ok(_) => Ok(serde_json::json!({
                "status": "healthy",
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })),
            Err(e) => Ok(serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })),
        }
    } else {
        Ok(serde_json::json!({
            "status": "not_configured",
            "message": "Redis is not configured",
            "last_checked": OffsetDateTime::now_utc()
        }))
    }
}

async fn check_external_services_health(
    state: &AppState,
) -> ApiResult<HashMap<String, serde_json::Value>> {
    let mut services = HashMap::new();

    // Check Jupiter API
    let jupiter_health = check_jupiter_api_health().await;
    services.insert("jupiter_api".to_string(), jupiter_health);

    // Check Helius RPC
    let helius_health = check_helius_rpc_health(&state.config.solana.rpc_url).await;
    services.insert("helius_rpc".to_string(), helius_health);

    // Check object storage (S3/R2)
    let storage_health = check_object_storage_health(state).await;
    services.insert("object_storage".to_string(), storage_health);

    Ok(services)
}

async fn check_database_connectivity(state: &AppState) -> ApiResult<bool> {
    let result = sqlx::query!("SELECT 1")
        .fetch_one(&state.database.pool)
        .await;

    Ok(result.is_ok())
}

async fn check_redis_connectivity(state: &AppState) -> ApiResult<bool> {
    if let Some(redis) = &state.redis {
        let result = redis
            .get_async_connection()
            .await
            .and_then(|mut conn| {
                async move {
                    use redis::AsyncCommands;
                    conn.ping().await
                }
                .boxed()
            })
            .await;
        Ok(result.is_ok())
    } else {
        Ok(true) // Redis not configured, so consider it "connected"
    }
}

async fn validate_configuration(state: &AppState) -> ApiResult<bool> {
    // Validate critical configuration
    let mut valid = true;

    // Check if required environment variables are set
    if state.config.database.url.is_empty() {
        valid = false;
    }

    if state.config.auth.jwt_secret.is_empty() {
        valid = false;
    }

    Ok(valid)
}

async fn get_database_pool_info(state: &AppState) -> ApiResult<serde_json::Value> {
    let pool = &state.database.pool;

    Ok(serde_json::json!({
        "active_connections": pool.num_idle(),
        "idle_connections": pool.num_idle(),
        "max_connections": pool.options().get_max_connections(),
        "min_connections": pool.options().get_min_connections()
    }))
}

async fn get_metrics_summary(state: &AppState) -> ApiResult<MetricsSummary> {
    // This would typically gather metrics from the metrics registry
    // For now, return mock data
    Ok(MetricsSummary {
        total_requests: 1000,
        error_rate: 0.01,
        avg_response_time_ms: 150.0,
        active_connections: 25,
        database_connections: 5,
    })
}

async fn check_jupiter_api_health() -> serde_json::Value {
    let start_time = std::time::Instant::now();

    // Test Jupiter API with a simple price check
    let client = reqwest::Client::new();
    let result = client
        .get("https://price.jup.ag/v4/price?ids=So11111111111111111111111111111111111111112")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    let latency_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(response) if response.status().is_success() => {
            serde_json::json!({
                "status": "healthy",
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
        Ok(response) => {
            serde_json::json!({
                "status": "unhealthy",
                "error": format!("HTTP {}", response.status()),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
        Err(e) => {
            serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
    }
}

async fn check_helius_rpc_health(rpc_url: &str) -> serde_json::Value {
    let start_time = std::time::Instant::now();

    // Test Helius RPC with getHealth method
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getHealth"
    });

    let result = client
        .post(rpc_url)
        .json(&request_body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;

    let latency_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(response) if response.status().is_success() => {
            serde_json::json!({
                "status": "healthy",
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
        Ok(response) => {
            serde_json::json!({
                "status": "unhealthy",
                "error": format!("HTTP {}", response.status()),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
        Err(e) => {
            serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
    }
}

async fn check_object_storage_health(state: &AppState) -> serde_json::Value {
    let start_time = std::time::Instant::now();

    // Test object storage with a simple list operation
    use object_store::ListResult;

    let result = state
        .object_store
        .list(Some(&object_store::path::Path::from("/")))
        .try_collect::<Vec<_>>()
        .await;

    let latency_ms = start_time.elapsed().as_millis() as u64;

    match result {
        Ok(_) => {
            serde_json::json!({
                "status": "healthy",
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
        Err(e) => {
            serde_json::json!({
                "status": "unhealthy",
                "error": e.to_string(),
                "latency_ms": latency_ms,
                "last_checked": OffsetDateTime::now_utc()
            })
        }
    }
}
