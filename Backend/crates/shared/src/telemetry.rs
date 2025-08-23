use std::env;
use tracing::{Level, Subscriber};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Initialize telemetry for the application
pub fn init_telemetry() {
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .json();

    let subscriber = Registry::default().with(env_filter).with(fmt_layer);

    if let Err(e) = subscriber.try_init() {
        eprintln!("Failed to initialize telemetry: {}", e);
    }
}

/// Get the current service name from environment or default
pub fn service_name() -> String {
    env::var("SERVICE_NAME").unwrap_or_else(|_| "oof-backend".to_string())
}

/// Get the current service version from environment or default
pub fn service_version() -> String {
    env::var("SERVICE_VERSION").unwrap_or_else(|_| "0.1.0".to_string())
}

/// Create a span for request tracing
#[macro_export]
macro_rules! request_span {
    ($request_id:expr, $method:expr, $path:expr) => {
        tracing::info_span!(
            "request",
            request_id = %$request_id,
            method = %$method,
            path = %$path,
            service = %crate::telemetry::service_name(),
            version = %crate::telemetry::service_version()
        )
    };
}

/// Create a span for job processing
#[macro_export]
macro_rules! job_span {
    ($job_id:expr, $job_kind:expr) => {
        tracing::info_span!(
            "job",
            job_id = %$job_id,
            job_kind = %$job_kind,
            service = %crate::telemetry::service_name(),
            version = %crate::telemetry::service_version()
        )
    };
}
