mod auth_mw;
mod observability_mw;
mod rate_limit_mw;
mod routes;

use axum::{
    routing::{get, post},
    Router,
};
use shared::{
    metrics_router,
    observability::{init_observability, HealthChecker, MetricsRegistry, ObservabilityConfig},
    store::{make_store_from_config, ObjectStore},
    AppConfig, MaybeRedis, Metrics, Pg,
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // Load and validate configuration
    let cfg = AppConfig::from_env().map_err(|e| anyhow::anyhow!("Configuration error: {}", e))?;
    cfg.validate()
        .map_err(|e| anyhow::anyhow!("Configuration validation failed: {}", e))?;
    let pg = Pg::connect(&cfg.database_url).await?;
    let metrics = Metrics::new();

    // Initialize object store using new config-based approach
    let store = make_store_from_config(&cfg, Some("oof-assets")).await?;
    let redis = MaybeRedis::connect(cfg.redis_url.as_deref()).await;

    // Initialize observability
    let observability_config = ObservabilityConfig::default();
    let (metrics_registry, health_checker) = init_observability(observability_config).await?;

    // Register health checks
    health_checker
        .register_check(
            "database".to_string(),
            shared::observability::HealthStatus::Healthy,
        )
        .await;

    health_checker
        .register_check(
            "object_store".to_string(),
            shared::observability::HealthStatus::Healthy,
        )
        .await;

    // Test Dynamic.xyz configuration
    if cfg.environment == "production" {
        info!("Production environment detected - validating Dynamic.xyz configuration");
        if cfg.dynamic_environment_id.is_empty() || cfg.dynamic_api_key.is_empty() {
            anyhow::bail!("Dynamic.xyz configuration is required in production");
        }
    }

    let cors = if let Some(origin) = &cfg.cors_allow_origin {
        CorsLayer::new()
            .allow_origin(AllowOrigin::exact(origin.parse().unwrap()))
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        CorsLayer::very_permissive()
    };

    let state = routes::AppState::new(
        cfg.clone(),
        pg,
        metrics.clone(),
        redis,
        store,
        metrics_registry,
        health_checker.clone(),
    );

    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/metrics", get(routes::metrics))
        .route(
            "/v1/analyze",
            post(routes::analyze).route_layer(axum::middleware::from_fn_with_state(
                state.clone(),
                auth_mw::require_auth,
            )),
        )
        .route("/v1/analyze/:job/stream", get(routes::analyze_stream))
        .route(
            "/v1/moments",
            get(routes::moments_list)
                .route_layer(axum::middleware::from_fn(rate_limit_mw::per_ip_limit)),
        )
        .route("/v1/moments/:id", get(routes::moment_detail))
        .route("/v1/wallets/:wallet/summary", get(routes::wallet_summary))
        .route(
            "/v1/wallets/:wallet/extremes",
            get(routes::wallet_extremes)
                .route_layer(axum::middleware::from_fn(rate_limit_mw::per_ip_limit)),
        )
        .route("/v1/cards/moment/:id.png", get(routes::card_png))
        .route("/v1/tokens/:mint/prices", get(routes::token_prices))
        .route("/v1/leaderboard", get(routes::leaderboard))
        .merge(metrics_router())
        .layer(cors)
        .layer(axum::middleware::from_fn_with_state(
            state.metrics_registry.clone(),
            observability_mw::track_request_metrics,
        ))
        .with_state(state);

    let addr: SocketAddr = cfg.api_bind.parse().expect("Invalid bind address");
    info!("API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
