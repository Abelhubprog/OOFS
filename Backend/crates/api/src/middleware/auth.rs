use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use shared::{
    auth::{AuthUser, Claims, JwtService},
    error::{ApiError, ApiResult},
    observability::MetricsRegistry,
};
use std::sync::Arc;
use time::OffsetDateTime;
use tracing::{debug, error, info, instrument, warn};

use crate::state::AppState;

/// Authentication middleware that validates JWT tokens
#[instrument(skip(state, request, next))]
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            header.trim_start_matches("Bearer ").trim()
        }
        _ => {
            debug!("Missing or invalid authorization header");
            state
                .metrics_registry
                .http_requests_total
                .with_label_values(&["unknown", "GET", "401"])
                .inc();
            return Err(ApiError::Unauthorized(
                "Missing or invalid authorization header".to_string(),
            ));
        }
    };

    // Validate JWT token using Dynamic.xyz
    let jwt_service = JwtService::new(
        &state.config.dynamic_environment_id,
        &state.config.dynamic_jwks_url,
        &state.config.app_secret,
    )?;

    let claims = match jwt_service.validate_token(token).await {
        Ok(claims) => claims,
        Err(e) => {
            warn!("Dynamic.xyz JWT validation failed: {}", e);
            state
                .metrics_registry
                .http_requests_total
                .with_label_values(&["unknown", "GET", "401"])
                .inc();
            return Err(ApiError::Unauthorized(format!("Invalid token: {}", e)));
        }
    };

    // Check token expiration (already handled by JWT validation)
    let now = OffsetDateTime::now_utc().unix_timestamp();
    if claims.exp < now {
        debug!("Token expired: exp={}, now={}", claims.exp, now);
        state
            .metrics_registry
            .http_requests_total
            .with_label_values(&[&claims.sub, "GET", "401"])
            .inc();
        return Err(ApiError::Unauthorized("Token expired".to_string()));
    }

    // Validate Dynamic.xyz environment
    if let Some(env_id) = &claims.environment_id {
        if env_id != &state.config.dynamic_environment_id {
            warn!(
                "Invalid environment ID: expected={}, got={}",
                state.config.dynamic_environment_id, env_id
            );
            return Err(ApiError::Unauthorized(
                "Invalid token environment".to_string(),
            ));
        }
    }

    // Check if user is active (if we have user management)
    if let Some(user_status) = check_user_status(&state, &claims.sub).await? {
        if !user_status.is_active {
            warn!("Inactive user attempted access: {}", claims.sub);
            return Err(ApiError::Forbidden("User account is inactive".to_string()));
        }

        if user_status.is_suspended {
            warn!("Suspended user attempted access: {}", claims.sub);
            return Err(ApiError::Forbidden("User account is suspended".to_string()));
        }
    }

    // Check rate limits for this user
    if let Err(e) = check_user_rate_limit(&state, &claims.sub).await {
        warn!("Rate limit exceeded for user: {}", claims.sub);
        return Err(e);
    }

    // Create authenticated user
    let auth_user = AuthUser {
        claims: claims.clone(),
        permissions: get_user_permissions(&state, &claims).await?,
        metadata: get_user_metadata(&state, &claims).await?,
    };

    // Add user to request extensions
    request.extensions_mut().insert(auth_user);

    // Add user ID to headers for downstream services
    request.headers_mut().insert(
        "x-user-id",
        claims
            .sub
            .parse()
            .map_err(|_| ApiError::Internal("Invalid user ID".to_string()))?,
    );

    // Log successful authentication
    debug!("User authenticated successfully: {}", claims.sub);
    state
        .metrics_registry
        .http_requests_total
        .with_label_values(&[&claims.sub, "GET", "200"])
        .inc();

    // Update last seen timestamp
    update_user_last_seen(&state, &claims.sub).await?;

    // Continue to next middleware/handler
    Ok(next.run(request).await)
}

/// Optional authentication middleware (doesn't fail if no auth provided)
#[instrument(skip(state, request, next))]
pub async fn optional_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract authorization header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    if let Some(header) = auth_header {
        if header.starts_with("Bearer ") {
            let token = header.trim_start_matches("Bearer ").trim();

            // Try to validate JWT token using Dynamic.xyz
            let jwt_service = JwtService::new(
                &state.config.dynamic_environment_id,
                &state.config.dynamic_jwks_url,
                &state.config.app_secret,
            )?;

            if let Ok(claims) = jwt_service.validate_token(token).await {
                // Check token expiration
                let now = OffsetDateTime::now_utc().unix_timestamp();
                if claims.exp >= now {
                    // Create authenticated user
                    if let Ok(auth_user) = create_auth_user(&state, claims).await {
                        request.extensions_mut().insert(auth_user);
                        debug!("Optional auth: User authenticated");
                    }
                }
            }
        }
    }

    // Continue regardless of auth status
    Ok(next.run(request).await)
}

/// Admin-only authentication middleware
#[instrument(skip(state, request, next))]
pub async fn admin_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // First run standard auth
    let auth_result = auth_middleware(State(state.clone()), request, next.clone()).await;

    // Check if user has admin role
    if let Ok(response) = &auth_result {
        if let Some(auth_user) = request.extensions().get::<AuthUser>() {
            if !auth_user.permissions.contains(&"admin".to_string()) {
                warn!(
                    "Non-admin user attempted admin access: {}",
                    auth_user.claims.sub
                );
                return Err(ApiError::Forbidden("Admin access required".to_string()));
            }
        }
    }

    auth_result
}

/// Service account authentication middleware
#[instrument(skip(state, request, next))]
pub async fn service_auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Extract service token from header
    let service_token = request
        .headers()
        .get("x-service-token")
        .and_then(|header| header.to_str().ok());

    let token = match service_token {
        Some(token) => token,
        None => {
            debug!("Missing service token header");
            return Err(ApiError::Unauthorized("Missing service token".to_string()));
        }
    };

    // Validate service token
    if !is_valid_service_token(&state, token).await? {
        warn!("Invalid service token provided");
        return Err(ApiError::Unauthorized("Invalid service token".to_string()));
    }

    // Add service identity to request
    let service_id = get_service_id_from_token(&state, token).await?;
    request.headers_mut().insert(
        "x-service-id",
        service_id
            .parse()
            .map_err(|_| ApiError::Internal("Invalid service ID".to_string()))?,
    );

    debug!("Service authenticated: {}", service_id);

    Ok(next.run(request).await)
}

/// User status information
#[derive(Debug, Clone)]
struct UserStatus {
    is_active: bool,
    is_suspended: bool,
    last_seen: Option<OffsetDateTime>,
}

/// Check user status in database
async fn check_user_status(state: &AppState, user_id: &str) -> ApiResult<Option<UserStatus>> {
    // In a real implementation, this would query the database
    // For now, return None to skip user status checks
    Ok(None)
}

/// Check user-specific rate limits
async fn check_user_rate_limit(state: &AppState, user_id: &str) -> ApiResult<()> {
    // In a real implementation, this would check Redis for rate limit counters
    // For now, always allow
    Ok(())
}

/// Get user permissions
async fn get_user_permissions(state: &AppState, claims: &Claims) -> ApiResult<Vec<String>> {
    // Extract permissions from JWT claims or database
    let permissions = claims.permissions.clone().unwrap_or_default();

    // Add default permissions based on user type
    let mut all_permissions = permissions;
    all_permissions.push("read".to_string());

    // Check if user is admin
    if claims.roles.contains(&"admin".to_string()) {
        all_permissions.push("admin".to_string());
        all_permissions.push("write".to_string());
        all_permissions.push("delete".to_string());
    }

    // Check if user is premium
    if claims.roles.contains(&"premium".to_string()) {
        all_permissions.push("premium".to_string());
        all_permissions.push("bulk_analyze".to_string());
    }

    Ok(all_permissions)
}

/// Get user metadata
async fn get_user_metadata(state: &AppState, claims: &Claims) -> ApiResult<serde_json::Value> {
    // Return basic metadata from claims
    Ok(serde_json::json!({
        "user_id": claims.sub,
        "email": claims.email,
        "roles": claims.roles,
        "issued_at": claims.iat,
        "expires_at": claims.exp
    }))
}

/// Update user's last seen timestamp
async fn update_user_last_seen(state: &AppState, user_id: &str) -> ApiResult<()> {
    // In a real implementation, this would update the database
    // For now, just log
    debug!("Updated last seen for user: {}", user_id);
    Ok(())
}

/// Create authenticated user from claims
async fn create_auth_user(state: &AppState, claims: Claims) -> ApiResult<AuthUser> {
    let permissions = get_user_permissions(state, &claims).await?;
    let metadata = get_user_metadata(state, &claims).await?;

    Ok(AuthUser {
        claims,
        permissions,
        metadata,
    })
}

/// Validate service token against configuration using HMAC
async fn is_valid_service_token(state: &AppState, token: &str) -> ApiResult<bool> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    // Extract timestamp and signature from token (format: service_id.timestamp.signature)
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return Ok(false);
    }

    let service_id = parts[0];
    let timestamp = parts[1];
    let provided_signature = parts[2];

    // Check timestamp validity (tokens expire after 1 hour)
    if let Ok(ts) = timestamp.parse::<i64>() {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if now - ts > 3600 {
            // 1 hour expiry
            return Ok(false);
        }
    } else {
        return Ok(false);
    }

    // Create expected signature using HMAC
    let message = format!("{}.{}", service_id, timestamp);
    let mut mac = Hmac::<Sha256>::new_from_slice(state.config.app_secret.as_bytes())
        .map_err(|_| ApiError::Internal("Invalid HMAC key".to_string()))?;
    mac.update(message.as_bytes());
    let expected_signature = hex::encode(mac.finalize().into_bytes());

    // Compare signatures securely using constant-time comparison
    use subtle::ConstantTimeEq;
    Ok(expected_signature
        .as_bytes()
        .ct_eq(provided_signature.as_bytes())
        .into())
}

/// Get service ID from token
async fn get_service_id_from_token(state: &AppState, token: &str) -> ApiResult<String> {
    // In a real implementation, this would look up the service ID
    // For now, return a hash of the token
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    token.hash(&mut hasher);
    Ok(format!("service_{}", hasher.finish()))
}

/// CORS middleware for auth endpoints
#[instrument(skip(request, next))]
pub async fn cors_middleware(request: Request, next: Next) -> Result<Response, ApiError> {
    let mut response = next.run(request).await;

    // Add CORS headers
    let headers = response.headers_mut();
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET, POST, PUT, DELETE, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Authorization, Content-Type, x-service-token"
            .parse()
            .unwrap(),
    );
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());

    Ok(response)
}

/// Security headers middleware
#[instrument(skip(request, next))]
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let mut response = next.run(request).await;

    // Add security headers
    let headers = response.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap(),
    );
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    Ok(response)
}
