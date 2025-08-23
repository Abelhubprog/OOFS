use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use shared::{
    auth::AuthUser,
    error::{ApiError, ApiResult},
};
use std::collections::HashMap;
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::state::AppState;

/// Card generation query parameters
#[derive(Debug, Deserialize)]
pub struct CardQuery {
    /// Card theme (dark, light, gradient)
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Card size (small, medium, large)
    #[serde(default = "default_size")]
    pub size: String,
    /// Output format (png, svg, webp)
    #[serde(default = "default_format")]
    pub format: String,
    /// Quality for raster formats (1-100)
    #[serde(default = "default_quality")]
    pub quality: u8,
    /// Force regeneration
    #[serde(default)]
    pub refresh: bool,
}

fn default_theme() -> String { "dark".to_string() }
fn default_size() -> String { "medium".to_string() }
fn default_format() -> String { "png".to_string() }
fn default_quality() -> u8 { 85 }

/// Generate a moment card
#[instrument(skip(state))]
pub async fn generate_moment_card(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    Path(moment_id): Path<String>,
    Query(query): Query<CardQuery>,
) -> ApiResult<Response> {
    info!("Generating moment card for: {} with theme: {}", moment_id, query.theme);

    // Validate parameters
    validate_card_params(&query)?;

    // Check if card is cached
    if !query.refresh {
        if let Some(cached_card) = get_cached_card(&state, &moment_id, &query).await? {
            return serve_card_response(cached_card, &query.format);
        }
    }

    // Fetch moment data
    let moment = fetch_moment_for_card(&state, &moment_id).await?
        .ok_or_else(|| ApiError::NotFound("Moment not found".to_string()))?;

    // Generate card
    let card_data = generate_card_image(&state, &moment, &query).await?;

    // Cache the result
    cache_card(&state, &moment_id, &query, &card_data).await?;

    serve_card_response(card_data, &query.format)
}

/// Generate a wallet summary card
#[instrument(skip(state))]
pub async fn generate_wallet_card(
    State(state): State<AppState>,
    user: AuthUser,
    Path(wallet): Path<String>,
    Query(query): Query<CardQuery>,
) -> ApiResult<Response> {
    info!("Generating wallet card for: {} by user: {}", wallet, user.claims.sub);

    // Validate wallet and permissions
    shared::validation::validate_pubkey(&wallet)
        .map_err(|_| ApiError::BadRequest("Invalid wallet address".to_string()))?;

    if !user.permissions.contains(&"read".to_string()) {
        return Err(ApiError::Forbidden("Read permission required".to_string()));
    }

    validate_card_params(&query)?;

    // Check cache
    let cache_key = format!("{}-wallet", wallet);
    if !query.refresh {
        if let Some(cached_card) = get_cached_card(&state, &cache_key, &query).await? {
            return serve_card_response(cached_card, &query.format);
        }
    }

    // Fetch wallet data
    let wallet_summary = fetch_wallet_summary_for_card(&state, &wallet).await?;

    // Generate card
    let card_data = generate_wallet_card_image(&state, &wallet_summary, &query).await?;

    // Cache the result
    cache_card(&state, &cache_key, &query, &card_data).await?;

    serve_card_response(card_data, &query.format)
}

/// Generate a leaderboard card
#[instrument(skip(state))]
pub async fn generate_leaderboard_card(
    State(state): State<AppState>,
    Path(leaderboard_type): Path<String>,
    Query(query): Query<CardQuery>,
) -> ApiResult<Response> {
    info!("Generating leaderboard card for type: {}", leaderboard_type);

    validate_card_params(&query)?;

    // Validate leaderboard type
    if !["worst-oof", "best-performance", "most-active"].contains(&leaderboard_type.as_str()) {
        return Err(ApiError::BadRequest("Invalid leaderboard type".to_string()));
    }

    // Check cache
    let cache_key = format!("leaderboard-{}", leaderboard_type);
    if !query.refresh {
        if let Some(cached_card) = get_cached_card(&state, &cache_key, &query).await? {
            return serve_card_response(cached_card, &query.format);
        }
    }

    // Fetch leaderboard data
    let leaderboard_data = fetch_leaderboard_for_card(&state, &leaderboard_type).await?;

    // Generate card
    let card_data = generate_leaderboard_card_image(&state, &leaderboard_data, &query).await?;

    // Cache with shorter TTL for leaderboards
    cache_card_with_ttl(&state, &cache_key, &query, &card_data, 300).await?; // 5 minutes

    serve_card_response(card_data, &query.format)
}

/// Get available card themes
#[instrument]
pub async fn get_card_themes() -> Json<serde_json::Value> {
    use axum::response::Json;

    Json(serde_json::json!({
        "themes": [
            {
                "id": "dark",
                "name": "Dark Mode",
                "description": "Dark theme with orange accents",
                "preview_url": "/api/v1/cards/themes/dark/preview.png"
            },
            {
                "id": "light",
                "name": "Light Mode",
                "description": "Clean light theme",
                "preview_url": "/api/v1/cards/themes/light/preview.png"
            },
            {
                "id": "gradient",
                "name": "Gradient",
                "description": "Colorful gradient background",
                "preview_url": "/api/v1/cards/themes/gradient/preview.png"
            }
        ],
        "sizes": ["small", "medium", "large"],
        "formats": ["png", "svg", "webp"]
    }))
}

/// Batch generate cards
#[instrument(skip(state))]
pub async fn batch_generate_cards(
    State(state): State<AppState>,
    user: AuthUser,
    axum::Json(request): axum::Json<BatchCardRequest>,
) -> ApiResult<axum::Json<BatchCardResponse>> {
    info!("Batch generating {} cards by user: {}", request.cards.len(), user.claims.sub);

    // Check premium permissions for batch generation
    if !user.permissions.contains(&"premium".to_string()) {
        return Err(ApiError::Forbidden("Premium subscription required for batch card generation".to_string()));
    }

    // Limit batch size
    if request.cards.len() > 50 {
        return Err(ApiError::BadRequest("Maximum 50 cards per batch".to_string()));
    }

    let batch_id = Uuid::new_v4().to_string();
    let mut results = Vec::new();

    for card_request in request.cards {
        match generate_single_card(&state, &card_request).await {
            Ok(url) => {
                results.push(BatchCardResult {
                    id: card_request.id,
                    status: "success".to_string(),
                    url: Some(url),
                    error: None,
                });
            }
            Err(e) => {
                results.push(BatchCardResult {
                    id: card_request.id,
                    status: "failed".to_string(),
                    url: None,
                    error: Some(e.to_string()),
                });
            }
        }
    }

    Ok(axum::Json(BatchCardResponse {
        batch_id,
        results,
        total: request.cards.len(),
        successful: results.iter().filter(|r| r.status == "success").count(),
        failed: results.iter().filter(|r| r.status == "failed").count(),
    }))
}

// Supporting types
#[derive(Debug, serde::Deserialize)]
struct BatchCardRequest {
    cards: Vec<SingleCardRequest>,
}

#[derive(Debug, serde::Deserialize)]
struct SingleCardRequest {
    id: String,
    card_type: String, // "moment", "wallet", "leaderboard"
    target_id: String, // moment_id, wallet_address, or leaderboard_type
    theme: Option<String>,
    size: Option<String>,
    format: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct BatchCardResponse {
    batch_id: String,
    results: Vec<BatchCardResult>,
    total: usize,
    successful: usize,
    failed: usize,
}

#[derive(Debug, serde::Serialize)]
struct BatchCardResult {
    id: String,
    status: String,
    url: Option<String>,
    error: Option<String>,
}

// Helper functions
fn validate_card_params(query: &CardQuery) -> ApiResult<()> {
    // Validate theme
    if !["dark", "light", "gradient"].contains(&query.theme.as_str()) {
        return Err(ApiError::BadRequest("Invalid theme".to_string()));
    }

    // Validate size
    if !["small", "medium", "large"].contains(&query.size.as_str()) {
        return Err(ApiError::BadRequest("Invalid size".to_string()));
    }

    // Validate format
    if !["png", "svg", "webp"].contains(&query.format.as_str()) {
        return Err(ApiError::BadRequest("Invalid format".to_string()));
    }

    // Validate quality
    if query.quality == 0 || query.quality > 100 {
        return Err(ApiError::BadRequest("Quality must be between 1 and 100".to_string()));
    }

    Ok(())
}

async fn get_cached_card(state: &AppState, key: &str, query: &CardQuery) -> ApiResult<Option<Vec<u8>>> {
    if let Some(redis) = &state.redis {
        let cache_key = format!("card:{}:{}:{}:{}", key, query.theme, query.size, query.format);
        debug!("Checking card cache for key: {}", cache_key);

        // Try to get from Redis cache
        // Implementation would go here
    }
    Ok(None)
}

async fn cache_card(state: &AppState, key: &str, query: &CardQuery, data: &[u8]) -> ApiResult<()> {
    cache_card_with_ttl(state, key, query, data, 3600).await // 1 hour default
}

async fn cache_card_with_ttl(state: &AppState, key: &str, query: &CardQuery, data: &[u8], ttl: u64) -> ApiResult<()> {
    if let Some(redis) = &state.redis {
        let cache_key = format!("card:{}:{}:{}:{}", key, query.theme, query.size, query.format);
        debug!("Caching card with key: {} (TTL: {}s)", cache_key, ttl);

        // Store in Redis with TTL
        // Implementation would store the binary data
    }
    Ok(())
}

fn serve_card_response(data: Vec<u8>, format: &str) -> ApiResult<Response> {
    let content_type = match format {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    };

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "public, max-age=3600") // 1 hour cache
        .header(header::CONTENT_LENGTH, data.len())
        .body(data.into())
        .map_err(|e| ApiError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}

// Mock implementation functions
async fn fetch_moment_for_card(state: &AppState, moment_id: &str) -> ApiResult<Option<serde_json::Value>> {
    debug!("Fetching moment {} for card generation", moment_id);
    // Would fetch from database
    Ok(Some(serde_json::json!({
        "id": moment_id,
        "type": "SoldTooEarly",
        "value_lost": 15000.0,
        "token_symbol": "SOL"
    })))
}

async fn fetch_wallet_summary_for_card(state: &AppState, wallet: &str) -> ApiResult<serde_json::Value> {
    debug!("Fetching wallet summary for card: {}", wallet);
    Ok(serde_json::json!({
        "wallet": wallet,
        "oof_score": 75.5,
        "total_value_lost": 50000.0,
        "portfolio_value": 150000.0
    }))
}

async fn fetch_leaderboard_for_card(state: &AppState, leaderboard_type: &str) -> ApiResult<serde_json::Value> {
    debug!("Fetching leaderboard data for: {}", leaderboard_type);
    Ok(serde_json::json!({
        "type": leaderboard_type,
        "entries": []
    }))
}

async fn generate_card_image(state: &AppState, moment: &serde_json::Value, query: &CardQuery) -> ApiResult<Vec<u8>> {
    debug!("Generating card image with theme: {}, size: {}", query.theme, query.size);

    // This would integrate with the renderer crate
    // For now, return a placeholder
    Ok(b"PNG_PLACEHOLDER".to_vec())
}

async fn generate_wallet_card_image(state: &AppState, wallet_data: &serde_json::Value, query: &CardQuery) -> ApiResult<Vec<u8>> {
    debug!("Generating wallet card image");
    Ok(b"WALLET_PNG_PLACEHOLDER".to_vec())
}

async fn generate_leaderboard_card_image(state: &AppState, leaderboard_data: &serde_json::Value, query: &CardQuery) -> ApiResult<Vec<u8>> {
    debug!("Generating leaderboard card image");
    Ok(b"LEADERBOARD_PNG_PLACEHOLDER".to_vec())
}

async fn generate_single_card(state: &AppState, request: &SingleCardRequest) -> ApiResult<String> {
    let query = CardQuery {
        theme: request.theme.clone().unwrap_or_else(default_theme),
        size: request.size.clone().unwrap_or_else(default_size),
        format: request.format.clone().unwrap_or_else(default_format),
        quality: default_quality(),
        refresh: false,
    };

    match request.card_type.as_str() {
        "moment" => {
            let _data = generate_card_image(state, &serde_json::json!({"id": request.target_id}), &query).await?;
            Ok(format!("/api/v1/cards/moments/{}.{}", request.target_id, query.format))
        }
        "wallet" => {
            let _data = generate_wallet_card_image(state, &serde_json::json!({"wallet": request.target_id}), &query).await?;
            Ok(format!("/api/v1/cards/wallets/{}.{}", request.target_id, query.format))
        }
        "leaderboard" => {
            let _data = generate_leaderboard_card_image(state, &serde_json::json!({"type": request.target_id}), &query).await?;
            Ok(format!("/api/v1/cards/leaderboards/{}.{}", request.target_id, query.format))
        }
        _ => Err(ApiError::BadRequest("Invalid card type".to_string()))
    }
}
