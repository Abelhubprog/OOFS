use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use shared::{auth::AuthUser, error::ApiError, observability::MetricsRegistry};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;
use tracing::{debug, info, instrument, warn};

use crate::state::AppState;

/// Rate limiting configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Requests per window for anonymous users
    pub anonymous_requests_per_window: u32,
    /// Requests per window for authenticated users
    pub authenticated_requests_per_window: u32,
    /// Requests per window for premium users
    pub premium_requests_per_window: u32,
    /// Time window in seconds
    pub window_seconds: u64,
    /// Burst allowance (additional requests allowed temporarily)
    pub burst_allowance: u32,
    /// Whether to use sliding window (true) or fixed window (false)
    pub sliding_window: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            anonymous_requests_per_window: 100,
            authenticated_requests_per_window: 1000,
            premium_requests_per_window: 5000,
            window_seconds: 3600, // 1 hour
            burst_allowance: 10,
            sliding_window: true,
        }
    }
}

/// Rate limit state
#[derive(Debug, Clone)]
struct RateLimitState {
    count: u32,
    window_start: u64,
    requests: Vec<u64>, // For sliding window
}

/// In-memory rate limit store (for development/testing)
type RateLimitStore = Arc<RwLock<HashMap<String, RateLimitState>>>;

/// Rate limiting middleware
#[instrument(skip(state, request, next))]
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = get_rate_limit_config(&state).await;

    // Determine rate limit key and limits
    let (limit_key, max_requests) = get_rate_limit_params(&request, &addr, &config);

    // Check rate limit
    let rate_limit_result = check_rate_limit(&state, &limit_key, max_requests, &config).await?;

    // Add rate limit headers to response
    let mut response = next.run(request).await;
    add_rate_limit_headers(&mut response, &rate_limit_result, &config);

    // Log rate limit metrics
    record_rate_limit_metrics(&state, &rate_limit_result);

    Ok(response)
}

/// Strict rate limiting middleware (fails fast)
#[instrument(skip(state, request, next))]
pub async fn strict_rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = RateLimitConfig {
        burst_allowance: 0, // No burst allowed
        ..RateLimitConfig::default()
    };

    let (limit_key, max_requests) = get_rate_limit_params(&request, &addr, &config);

    let rate_limit_result = check_rate_limit(&state, &limit_key, max_requests, &config).await?;

    if rate_limit_result.exceeded {
        warn!("Rate limit exceeded for key: {}", limit_key);
        return Err(ApiError::RateLimitExceeded {
            retry_after: rate_limit_result.retry_after,
            limit: max_requests,
            remaining: 0,
        });
    }

    let mut response = next.run(request).await;
    add_rate_limit_headers(&mut response, &rate_limit_result, &config);

    Ok(response)
}

/// Premium user rate limiting (higher limits)
#[instrument(skip(state, request, next))]
pub async fn premium_rate_limit_middleware(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let config = RateLimitConfig {
        authenticated_requests_per_window: 10000, // Higher limit for premium
        premium_requests_per_window: 50000,
        burst_allowance: 100,
        ..RateLimitConfig::default()
    };

    let (limit_key, max_requests) = get_rate_limit_params(&request, &addr, &config);

    let rate_limit_result = check_rate_limit(&state, &limit_key, max_requests, &config).await?;

    if rate_limit_result.exceeded {
        return Err(ApiError::RateLimitExceeded {
            retry_after: rate_limit_result.retry_after,
            limit: max_requests,
            remaining: 0,
        });
    }

    let mut response = next.run(request).await;
    add_rate_limit_headers(&mut response, &rate_limit_result, &config);

    Ok(response)
}

/// API key rate limiting
#[instrument(skip(state, request, next))]
pub async fn api_key_rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract API key
    let api_key = request
        .headers()
        .get("x-api-key")
        .and_then(|h| h.to_str().ok());

    if let Some(key) = api_key {
        let config = get_api_key_rate_limit_config(&state, key).await?;
        let limit_key = format!("api_key:{}", key);

        let rate_limit_result = check_rate_limit(
            &state,
            &limit_key,
            config.authenticated_requests_per_window,
            &config,
        )
        .await?;

        if rate_limit_result.exceeded {
            return Err(ApiError::RateLimitExceeded {
                retry_after: rate_limit_result.retry_after,
                limit: config.authenticated_requests_per_window,
                remaining: 0,
            });
        }

        let mut response = next.run(request).await;
        add_rate_limit_headers(&mut response, &rate_limit_result, &config);

        Ok(response)
    } else {
        // No API key, continue with regular rate limiting
        Ok(next.run(request).await)
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
struct RateLimitResult {
    exceeded: bool,
    remaining: u32,
    reset_time: u64,
    retry_after: u64,
    current_count: u32,
    limit: u32,
}

/// Get rate limit configuration from app state
async fn get_rate_limit_config(state: &AppState) -> RateLimitConfig {
    // Load configuration based on environment
    match state.config.environment.as_str() {
        "production" => RateLimitConfig {
            anonymous_requests_per_window: 60, // More restrictive in production
            authenticated_requests_per_window: 1000,
            premium_requests_per_window: 5000,
            window_seconds: 3600, // 1 hour
            burst_allowance: 5,   // Lower burst in production
            sliding_window: true,
        },
        "development" => RateLimitConfig {
            anonymous_requests_per_window: 1000, // More lenient in development
            authenticated_requests_per_window: 5000,
            premium_requests_per_window: 10000,
            window_seconds: 3600,
            burst_allowance: 50, // Higher burst for development
            sliding_window: true,
        },
        _ => RateLimitConfig::default(),
    }
}

/// Get rate limit parameters based on request context
fn get_rate_limit_params(
    request: &Request,
    addr: &SocketAddr,
    config: &RateLimitConfig,
) -> (String, u32) {
    // Check if user is authenticated
    if let Some(auth_user) = request.extensions().get::<AuthUser>() {
        let user_id = &auth_user.claims.sub;

        // Check if user is premium
        if auth_user.permissions.contains(&"premium".to_string()) {
            return (
                format!("premium_user:{}", user_id),
                config.premium_requests_per_window,
            );
        }

        // Regular authenticated user
        return (
            format!("user:{}", user_id),
            config.authenticated_requests_per_window,
        );
    }

    // Anonymous user - use IP address
    (
        format!("ip:{}", addr.ip()),
        config.anonymous_requests_per_window,
    )
}

/// Check rate limit for a given key
async fn check_rate_limit(
    state: &AppState,
    key: &str,
    max_requests: u32,
    config: &RateLimitConfig,
) -> Result<RateLimitResult, ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Try Redis first, fall back to in-memory
    if let Some(redis) = &state.redis {
        check_rate_limit_redis(redis, key, max_requests, config, now).await
    } else {
        check_rate_limit_memory(key, max_requests, config, now).await
    }
}

/// Check rate limit using Redis
async fn check_rate_limit_redis(
    redis: &redis::Client,
    key: &str,
    max_requests: u32,
    config: &RateLimitConfig,
    now: u64,
) -> Result<RateLimitResult, ApiError> {
    let mut conn = redis
        .get_async_connection()
        .await
        .map_err(|e| ApiError::Internal(format!("Redis connection failed: {}", e)))?;

    if config.sliding_window {
        check_sliding_window_redis(&mut conn, key, max_requests, config, now).await
    } else {
        check_fixed_window_redis(&mut conn, key, max_requests, config, now).await
    }
}

/// Check sliding window rate limit in Redis
async fn check_sliding_window_redis(
    conn: &mut redis::aio::Connection,
    key: &str,
    max_requests: u32,
    config: &RateLimitConfig,
    now: u64,
) -> Result<RateLimitResult, ApiError> {
    use redis::AsyncCommands;

    let window_start = now - config.window_seconds;
    let requests_key = format!("rate_limit:sliding:{}", key);

    // Remove old entries
    let _: () = conn
        .zremrangebyscore(&requests_key, 0, window_start)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis ZREMRANGEBYSCORE failed: {}", e)))?;

    // Count current requests
    let current_count: u64 = conn
        .zcard(&requests_key)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis ZCARD failed: {}", e)))?;

    let allowed_requests = max_requests + config.burst_allowance;
    let exceeded = current_count >= allowed_requests as u64;

    if !exceeded {
        // Add current request
        let _: () = conn
            .zadd(&requests_key, now, now)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis ZADD failed: {}", e)))?;

        // Set expiration
        let _: () = conn
            .expire(&requests_key, config.window_seconds as usize)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis EXPIRE failed: {}", e)))?;
    }

    let remaining = if current_count >= allowed_requests as u64 {
        0
    } else {
        allowed_requests - current_count as u32
    };

    Ok(RateLimitResult {
        exceeded,
        remaining,
        reset_time: now + config.window_seconds,
        retry_after: if exceeded { config.window_seconds } else { 0 },
        current_count: current_count as u32,
        limit: max_requests,
    })
}

/// Check fixed window rate limit in Redis
async fn check_fixed_window_redis(
    conn: &mut redis::aio::Connection,
    key: &str,
    max_requests: u32,
    config: &RateLimitConfig,
    now: u64,
) -> Result<RateLimitResult, ApiError> {
    use redis::AsyncCommands;

    let window_start = (now / config.window_seconds) * config.window_seconds;
    let count_key = format!("rate_limit:fixed:{}:{}", key, window_start);

    // Increment counter
    let current_count: u64 = conn
        .incr(&count_key, 1)
        .await
        .map_err(|e| ApiError::Internal(format!("Redis INCR failed: {}", e)))?;

    // Set expiration on first increment
    if current_count == 1 {
        let _: () = conn
            .expire(&count_key, config.window_seconds as usize)
            .await
            .map_err(|e| ApiError::Internal(format!("Redis EXPIRE failed: {}", e)))?;
    }

    let allowed_requests = max_requests + config.burst_allowance;
    let exceeded = current_count > allowed_requests as u64;

    let remaining = if current_count >= allowed_requests as u64 {
        0
    } else {
        allowed_requests - current_count as u32
    };

    let reset_time = window_start + config.window_seconds;

    Ok(RateLimitResult {
        exceeded,
        remaining,
        reset_time,
        retry_after: if exceeded { reset_time - now } else { 0 },
        current_count: current_count as u32,
        limit: max_requests,
    })
}

/// Check rate limit using in-memory store
async fn check_rate_limit_memory(
    key: &str,
    max_requests: u32,
    config: &RateLimitConfig,
    now: u64,
) -> Result<RateLimitResult, ApiError> {
    // This is a simple in-memory implementation for development
    // In production, you should use Redis

    static MEMORY_STORE: once_cell::sync::Lazy<RateLimitStore> =
        once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

    let mut store = MEMORY_STORE.write().await;

    let window_start = if config.sliding_window {
        now - config.window_seconds
    } else {
        (now / config.window_seconds) * config.window_seconds
    };

    let entry = store
        .entry(key.to_string())
        .or_insert_with(|| RateLimitState {
            count: 0,
            window_start,
            requests: Vec::new(),
        });

    if config.sliding_window {
        // Remove old requests
        entry.requests.retain(|&time| time >= window_start);

        let current_count = entry.requests.len() as u32;
        let allowed_requests = max_requests + config.burst_allowance;
        let exceeded = current_count >= allowed_requests;

        if !exceeded {
            entry.requests.push(now);
        }

        let remaining = if current_count >= allowed_requests {
            0
        } else {
            allowed_requests - current_count
        };

        Ok(RateLimitResult {
            exceeded,
            remaining,
            reset_time: now + config.window_seconds,
            retry_after: if exceeded { config.window_seconds } else { 0 },
            current_count,
            limit: max_requests,
        })
    } else {
        // Fixed window
        if entry.window_start != window_start {
            entry.count = 0;
            entry.window_start = window_start;
        }

        entry.count += 1;
        let allowed_requests = max_requests + config.burst_allowance;
        let exceeded = entry.count > allowed_requests;

        let remaining = if entry.count >= allowed_requests {
            0
        } else {
            allowed_requests - entry.count
        };

        let reset_time = window_start + config.window_seconds;

        Ok(RateLimitResult {
            exceeded,
            remaining,
            reset_time,
            retry_after: if exceeded { reset_time - now } else { 0 },
            current_count: entry.count,
            limit: max_requests,
        })
    }
}

/// Add rate limit headers to response
fn add_rate_limit_headers(
    response: &mut Response,
    result: &RateLimitResult,
    config: &RateLimitConfig,
) {
    let headers = response.headers_mut();

    headers.insert(
        "x-ratelimit-limit",
        result.limit.to_string().parse().unwrap_or_default(),
    );
    headers.insert(
        "x-ratelimit-remaining",
        result.remaining.to_string().parse().unwrap_or_default(),
    );
    headers.insert(
        "x-ratelimit-reset",
        result.reset_time.to_string().parse().unwrap_or_default(),
    );
    headers.insert(
        "x-ratelimit-window",
        config
            .window_seconds
            .to_string()
            .parse()
            .unwrap_or_default(),
    );

    if result.exceeded {
        headers.insert(
            "retry-after",
            result.retry_after.to_string().parse().unwrap_or_default(),
        );
    }
}

/// Record rate limit metrics
fn record_rate_limit_metrics(state: &AppState, result: &RateLimitResult) {
    // Record metrics
    if result.exceeded {
        debug!(
            "Rate limit exceeded: current={}, limit={}",
            result.current_count, result.limit
        );
    } else {
        debug!(
            "Rate limit check: current={}, limit={}, remaining={}",
            result.current_count, result.limit, result.remaining
        );
    }
}

/// Get API key specific rate limit configuration
async fn get_api_key_rate_limit_config(
    state: &AppState,
    api_key: &str,
) -> Result<RateLimitConfig, ApiError> {
    // In a real implementation, this would look up API key settings in database
    // For now, return default config
    Ok(RateLimitConfig::default())
}

/// Rate limit status endpoint
pub async fn rate_limit_status(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
) -> impl IntoResponse {
    let config = RateLimitConfig::default();
    let (limit_key, max_requests) = get_rate_limit_params(&request, &addr, &config);

    match check_rate_limit(&state, &limit_key, max_requests, &config).await {
        Ok(result) => Json(json!({
            "limit": result.limit,
            "remaining": result.remaining,
            "current": result.current_count,
            "reset_time": result.reset_time,
            "window_seconds": config.window_seconds,
            "exceeded": result.exceeded
        })),
        Err(e) => Json(json!({
            "error": e.to_string()
        })),
    }
}
