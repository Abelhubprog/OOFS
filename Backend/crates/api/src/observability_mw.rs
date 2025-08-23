use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use shared::observability::MetricsRegistry;
use std::{sync::Arc, time::Instant};
use tracing::{error, info, instrument};

/// Middleware to track HTTP request metrics
#[instrument(skip_all, fields(method = %req.method(), path = %req.uri().path()))]
pub async fn track_request_metrics(
    State(metrics): State<Arc<MetricsRegistry>>,
    req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    // Normalize path for metrics (remove dynamic segments)
    let normalized_path = normalize_path(&path);

    // Increment in-flight requests
    metrics
        .http_requests_in_flight
        .with_label_values(&[&method, &normalized_path])
        .inc();

    // Process the request
    let response = next.run(req).await;

    // Record request completion
    let status = response.status().as_u16().to_string();
    let duration = start_time.elapsed();

    // Update metrics
    metrics
        .http_requests_total
        .with_label_values(&[&method, &normalized_path, &status])
        .inc();

    metrics
        .http_request_duration
        .with_label_values(&[&method, &normalized_path])
        .observe(duration.as_secs_f64());

    metrics
        .http_requests_in_flight
        .with_label_values(&[&method, &normalized_path])
        .dec();

    // Log request completion
    let status_code = response.status();
    if status_code.is_server_error() {
        error!(
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = duration.as_millis(),
            "HTTP request failed"
        );
    } else if status_code.is_client_error() {
        info!(
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = duration.as_millis(),
            "HTTP request completed with client error"
        );
    } else {
        info!(
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = duration.as_millis(),
            "HTTP request completed successfully"
        );
    }

    response
}

/// Normalize paths for metrics collection by removing dynamic segments
fn normalize_path(path: &str) -> String {
    // Replace common dynamic segments with placeholders
    let mut normalized = path.to_string();

    // Replace UUIDs and ULIDs (26 chars)
    if let Some(re) = regex::Regex::new(r"/[0-9a-zA-Z]{26}").ok() {
        normalized = re.replace_all(&normalized, "/:id").to_string();
    }

    // Replace other common ID patterns
    if let Some(re) = regex::Regex::new(r"/[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}").ok() {
        normalized = re.replace_all(&normalized, "/:uuid").to_string();
    }

    // Replace wallet addresses (32-44 chars of base58)
    if let Some(re) = regex::Regex::new(r"/[1-9A-HJ-NP-Za-km-z]{32,44}").ok() {
        normalized = re.replace_all(&normalized, "/:wallet").to_string();
    }

    // Replace numeric IDs
    if let Some(re) = regex::Regex::new(r"/\d+").ok() {
        normalized = re.replace_all(&normalized, "/:id").to_string();
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("/v1/wallets"), "/v1/wallets");
        assert_eq!(
            normalize_path("/v1/wallets/01J1K2L3M4N5P6Q7R8S9T0"),
            "/v1/wallets/:id"
        );
        assert_eq!(
            normalize_path("/v1/wallets/9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"),
            "/v1/wallets/:wallet"
        );
        assert_eq!(normalize_path("/v1/moments/123"), "/v1/moments/:id");
        assert_eq!(
            normalize_path("/v1/users/550e8400-e29b-41d4-a716-446655440000"),
            "/v1/users/:uuid"
        );
    }
}
