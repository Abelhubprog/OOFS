use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Internal server error")]
    Internal(#[from] anyhow::Error),
    #[error("Database error")]
    Database(#[from] sqlx::Error),
    #[error("Invalid wallet address")]
    InvalidWallet,
    #[error("Plan quota exceeded")]
    QuotaExceeded,
    #[error("Job not found")]
    JobNotFound,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limited".to_string()),
            ApiError::InvalidWallet => (
                StatusCode::BAD_REQUEST,
                "Invalid wallet address".to_string(),
            ),
            ApiError::QuotaExceeded => (
                StatusCode::PAYMENT_REQUIRED,
                "Plan quota exceeded".to_string(),
            ),
            ApiError::JobNotFound => (StatusCode::NOT_FOUND, "Job not found".to_string()),
            ApiError::Internal(_) | ApiError::Database(_) => {
                tracing::error!("Internal error: {}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
        };

        let body = Json(json!({
            "error": message,
            "code": status.as_u16()
        }));

        (status, body).into_response()
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;
