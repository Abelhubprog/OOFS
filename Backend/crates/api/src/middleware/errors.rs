use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use shared::{
    error::{ApiError, ErrorCode, ErrorContext},
    observability::MetricsRegistry,
};
use std::{sync::Arc, time::Instant};
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

use crate::state::AppState;

/// Global error handling middleware
#[instrument(skip(state, request, next))]
pub async fn error_handling_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let request_id = Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Add request ID to headers
    let mut request = request;
    request
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap_or_default());

    debug!(
        "Processing request: {} {} (ID: {})",
        method, uri, request_id
    );

    // Process the request
    let response_result = next.run(request).await;
    let duration = start_time.elapsed();

    // Log request completion
    let status = response_result.status();
    info!(
        "Request completed: {} {} {} - {}ms (ID: {})",
        method,
        uri,
        status.as_u16(),
        duration.as_millis(),
        request_id
    );

    // Update metrics
    state
        .metrics_registry
        .http_requests_total
        .with_label_values(&["unknown", method.as_str(), &status.as_u16().to_string()])
        .inc();

    state
        .metrics_registry
        .http_request_duration
        .with_label_values(&[method.as_str(), &status.as_u16().to_string()])
        .observe(duration.as_secs_f64());

    // Add response headers
    let mut response = response_result;
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap_or_default());
    response.headers_mut().insert(
        "x-response-time",
        format!("{}ms", duration.as_millis())
            .parse()
            .unwrap_or_default(),
    );

    response
}

/// Panic recovery middleware
#[instrument(skip(state, request, next))]
pub async fn panic_recovery_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    // Catch any panics and convert them to proper error responses
    let result = std::panic::AssertUnwindSafe(next.run(request))
        .catch_unwind()
        .await;

    match result {
        Ok(response) => response,
        Err(panic_info) => {
            // Log the panic
            let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "Unknown panic".to_string()
            };

            error!(
                "Request panic recovered: {} (ID: {})",
                panic_msg, request_id
            );

            // Update metrics
            state
                .metrics_registry
                .http_requests_total
                .with_label_values(&["unknown", "PANIC", "500"])
                .inc();

            // Return error response
            let error_response = ErrorResponse {
                error: ErrorDetail {
                    code: ErrorCode::InternalServerError,
                    message: "Internal server error".to_string(),
                    details: None,
                    request_id: Some(request_id.to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            };

            (StatusCode::INTERNAL_SERVER_ERROR, Json(error_response)).into_response()
        }
    }
}

/// Timeout middleware
#[instrument(skip(state, request, next))]
pub async fn timeout_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let timeout_duration = state.config.api.request_timeout;

    match tokio::time::timeout(timeout_duration, next.run(request)).await {
        Ok(response) => response,
        Err(_) => {
            warn!("Request timed out after {:?}", timeout_duration);

            // Update metrics
            state
                .metrics_registry
                .http_requests_total
                .with_label_values(&["unknown", "TIMEOUT", "408"])
                .inc();

            let error_response = ErrorResponse {
                error: ErrorDetail {
                    code: ErrorCode::RequestTimeout,
                    message: "Request timed out".to_string(),
                    details: Some(json!({
                        "timeout_seconds": timeout_duration.as_secs()
                    })),
                    request_id: None,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                },
            };

            (StatusCode::REQUEST_TIMEOUT, Json(error_response)).into_response()
        }
    }
}

/// Request validation middleware
#[instrument(skip(request, next))]
pub async fn validation_middleware(request: Request, next: Next) -> Response {
    // Validate request size
    if let Some(content_length) = request.headers().get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024; // 10MB
                if length > MAX_REQUEST_SIZE {
                    let error_response = ErrorResponse {
                        error: ErrorDetail {
                            code: ErrorCode::PayloadTooLarge,
                            message: "Request payload too large".to_string(),
                            details: Some(json!({
                                "max_size_bytes": MAX_REQUEST_SIZE,
                                "actual_size_bytes": length
                            })),
                            request_id: None,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        },
                    };

                    return (StatusCode::PAYLOAD_TOO_LARGE, Json(error_response)).into_response();
                }
            }
        }
    }

    // Validate content type for POST/PUT requests
    let method = request.method();
    if method == "POST" || method == "PUT" {
        if let Some(content_type) = request.headers().get("content-type") {
            if let Ok(ct_str) = content_type.to_str() {
                if !ct_str.starts_with("application/json")
                    && !ct_str.starts_with("multipart/form-data")
                {
                    let error_response = ErrorResponse {
                        error: ErrorDetail {
                            code: ErrorCode::UnsupportedMediaType,
                            message: "Unsupported content type".to_string(),
                            details: Some(json!({
                                "supported_types": ["application/json", "multipart/form-data"],
                                "provided_type": ct_str
                            })),
                            request_id: None,
                            timestamp: chrono::Utc::now().to_rfc3339(),
                        },
                    };

                    return (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(error_response))
                        .into_response();
                }
            }
        }
    }

    next.run(request).await
}

/// Error response structure
#[derive(Debug, serde::Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

/// Error detail structure
#[derive(Debug, serde::Serialize)]
struct ErrorDetail {
    code: ErrorCode,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
    timestamp: String,
}

/// Convert ApiError to HTTP response
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status_code, error_code, message, details) = match &self {
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorCode::BadRequest,
                msg.clone(),
                None,
            ),
            ApiError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                ErrorCode::Unauthorized,
                msg.clone(),
                None,
            ),
            ApiError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorCode::Forbidden,
                msg.clone(),
                None,
            ),
            ApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorCode::NotFound,
                msg.clone(),
                None,
            ),
            ApiError::Conflict(msg) => {
                (StatusCode::CONFLICT, ErrorCode::Conflict, msg.clone(), None)
            }
            ApiError::ValidationError(errors) => (
                StatusCode::BAD_REQUEST,
                ErrorCode::ValidationError,
                "Validation failed".to_string(),
                Some(json!({
                    "validation_errors": errors
                })),
            ),
            ApiError::RateLimitExceeded { retry_after, .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                ErrorCode::RateLimitExceeded,
                "Rate limit exceeded".to_string(),
                Some(json!({
                    "retry_after_seconds": retry_after
                })),
            ),
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                ErrorCode::ServiceUnavailable,
                msg.clone(),
                None,
            ),
            ApiError::Internal(msg) => {
                // Log internal errors
                error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::InternalServerError,
                    "Internal server error".to_string(),
                    Some(json!({
                        "internal_message": msg
                    })),
                )
            }
            ApiError::Database(msg) => {
                error!("Database error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorCode::DatabaseError,
                    "Database error".to_string(),
                    None,
                )
            }
            ApiError::ExternalService { service, error } => {
                warn!("External service error ({}): {}", service, error);
                (
                    StatusCode::BAD_GATEWAY,
                    ErrorCode::ExternalServiceError,
                    format!("External service error: {}", service),
                    Some(json!({
                        "service": service,
                        "error": error
                    })),
                )
            }
        };

        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: error_code,
                message,
                details,
                request_id: None, // This would be set by the error handling middleware
                timestamp: chrono::Utc::now().to_rfc3339(),
            },
        };

        // Add retry-after header for rate limit errors
        let mut response = (status_code, Json(error_response)).into_response();
        if let ApiError::RateLimitExceeded { retry_after, .. } = &self {
            response.headers_mut().insert(
                "retry-after",
                retry_after.to_string().parse().unwrap_or_default(),
            );
        }

        response
    }
}

/// Health check for error handling middleware
pub fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "middleware": "error_handling",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

/// Create a standardized error response
pub fn create_error_response(
    status: StatusCode,
    code: ErrorCode,
    message: String,
    details: Option<serde_json::Value>,
    request_id: Option<String>,
) -> Response {
    let error_response = ErrorResponse {
        error: ErrorDetail {
            code,
            message,
            details,
            request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        },
    };

    (status, Json(error_response)).into_response()
}

/// Extract request ID from headers
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

/// Add error context to logs
pub fn log_error_with_context(error: &ApiError, context: &ErrorContext, request_id: Option<&str>) {
    match error {
        ApiError::Internal(_) | ApiError::Database(_) => {
            error!(
                "Error: {} | Context: {:?} | Request ID: {:?}",
                error, context, request_id
            );
        }
        ApiError::ExternalService { .. } => {
            warn!(
                "External service error: {} | Context: {:?} | Request ID: {:?}",
                error, context, request_id
            );
        }
        _ => {
            debug!(
                "Client error: {} | Context: {:?} | Request ID: {:?}",
                error, context, request_id
            );
        }
    }
}
