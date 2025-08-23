use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec, Registry,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use time::OffsetDateTime;
use tokio::sync::RwLock;
use tracing::{error, info, instrument, warn};

/// Application health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health check result for individual components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub component: String,
    pub status: HealthStatus,
    pub message: Option<String>,
    pub latency_ms: Option<u64>,
    pub last_checked: OffsetDateTime,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Overall system health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub uptime_seconds: u64,
    pub version: String,
    pub build_info: BuildInfo,
    pub timestamp: OffsetDateTime,
}

/// Build and deployment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildInfo {
    pub version: String,
    pub commit_hash: String,
    pub build_time: String,
    pub rust_version: String,
    pub build_profile: String,
}

/// Application metrics registry
pub struct MetricsRegistry {
    registry: Registry,

    // HTTP metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration: HistogramVec,
    pub http_requests_in_flight: IntGaugeVec,

    // Database metrics
    pub db_connections_active: IntGauge,
    pub db_connections_total: IntCounter,
    pub db_query_duration: HistogramVec,
    pub db_queries_total: IntCounterVec,

    // Webhook metrics
    pub webhook_events_received: IntCounterVec,
    pub webhook_processing_duration: HistogramVec,
    pub webhook_events_failed: IntCounterVec,

    // Job processing metrics
    pub jobs_queued: IntGaugeVec,
    pub jobs_processed: IntCounterVec,
    pub job_processing_duration: HistogramVec,
    pub jobs_failed: IntCounterVec,

    // OOF moment metrics
    pub moments_detected: IntCounterVec,
    pub moments_total: IntGaugeVec,
    pub moment_detection_duration: HistogramVec,

    // Position tracking metrics
    pub positions_tracked: IntGauge,
    pub position_updates: IntCounter,
    pub position_calculation_duration: Histogram,

    // Price provider metrics
    pub price_requests: IntCounterVec,
    pub price_cache_hits: IntCounter,
    pub price_cache_misses: IntCounter,
    pub price_fetch_duration: HistogramVec,

    // Card rendering metrics
    pub cards_rendered: IntCounterVec,
    pub card_render_duration: HistogramVec,
    pub card_render_failures: IntCounterVec,

    // System metrics
    pub memory_usage: Gauge,
    pub cpu_usage: Gauge,
    pub disk_usage: GaugeVec,
    pub network_bytes: CounterVec,
}

impl MetricsRegistry {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        // HTTP metrics
        let http_requests_total = IntCounterVec::new(
            prometheus::Opts::new("http_requests_total", "Total HTTP requests"),
            &["method", "endpoint", "status"],
        )?;

        let http_request_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["method", "endpoint"],
        )?;

        let http_requests_in_flight = IntGaugeVec::new(
            prometheus::Opts::new("http_requests_in_flight", "In-flight HTTP requests"),
            &["method", "endpoint"],
        )?;

        // Database metrics
        let db_connections_active =
            IntGauge::new("db_connections_active", "Active database connections")?;

        let db_connections_total =
            IntCounter::new("db_connections_total", "Total database connections created")?;

        let db_query_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("db_query_duration_seconds", "Database query duration")
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            &["query_type", "table"],
        )?;

        let db_queries_total = IntCounterVec::new(
            prometheus::Opts::new("db_queries_total", "Total database queries"),
            &["query_type", "table", "status"],
        )?;

        // Webhook metrics
        let webhook_events_received = IntCounterVec::new(
            prometheus::Opts::new("webhook_events_received_total", "Webhook events received"),
            &["source", "event_type"],
        )?;

        let webhook_processing_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "webhook_processing_duration_seconds",
                "Webhook processing duration",
            )
            .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
            &["source", "event_type"],
        )?;

        let webhook_events_failed = IntCounterVec::new(
            prometheus::Opts::new("webhook_events_failed_total", "Failed webhook events"),
            &["source", "event_type", "error_type"],
        )?;

        // Job processing metrics
        let jobs_queued = IntGaugeVec::new(
            prometheus::Opts::new("jobs_queued", "Jobs in queue"),
            &["job_type", "priority"],
        )?;

        let jobs_processed = IntCounterVec::new(
            prometheus::Opts::new("jobs_processed_total", "Jobs processed"),
            &["job_type", "status"],
        )?;

        let job_processing_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "job_processing_duration_seconds",
                "Job processing duration",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 300.0, 600.0]),
            &["job_type"],
        )?;

        let jobs_failed = IntCounterVec::new(
            prometheus::Opts::new("jobs_failed_total", "Failed jobs"),
            &["job_type", "error_type"],
        )?;

        // OOF moment metrics
        let moments_detected = IntCounterVec::new(
            prometheus::Opts::new("moments_detected_total", "OOF moments detected"),
            &["moment_type", "severity"],
        )?;

        let moments_total = IntGaugeVec::new(
            prometheus::Opts::new("moments_total", "Total OOF moments"),
            &["moment_type", "status"],
        )?;

        let moment_detection_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "moment_detection_duration_seconds",
                "Moment detection duration",
            )
            .buckets(vec![0.1, 0.5, 1.0, 5.0, 10.0, 30.0]),
            &["moment_type"],
        )?;

        // Position tracking metrics
        let positions_tracked =
            IntGauge::new("positions_tracked", "Number of positions being tracked")?;

        let position_updates = IntCounter::new("position_updates_total", "Total position updates")?;

        let position_calculation_duration = Histogram::new(
            prometheus::HistogramOpts::new(
                "position_calculation_duration_seconds",
                "Position calculation duration",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5]),
        )?;

        // Price provider metrics
        let price_requests = IntCounterVec::new(
            prometheus::Opts::new("price_requests_total", "Price requests"),
            &["provider", "mint", "status"],
        )?;

        let price_cache_hits = IntCounter::new("price_cache_hits_total", "Price cache hits")?;

        let price_cache_misses = IntCounter::new("price_cache_misses_total", "Price cache misses")?;

        let price_fetch_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("price_fetch_duration_seconds", "Price fetch duration")
                .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["provider"],
        )?;

        // Card rendering metrics
        let cards_rendered = IntCounterVec::new(
            prometheus::Opts::new("cards_rendered_total", "Cards rendered"),
            &["card_type", "theme", "size"],
        )?;

        let card_render_duration = HistogramVec::new(
            prometheus::HistogramOpts::new("card_render_duration_seconds", "Card render duration")
                .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0]),
            &["card_type"],
        )?;

        let card_render_failures = IntCounterVec::new(
            prometheus::Opts::new("card_render_failures_total", "Card render failures"),
            &["card_type", "error_type"],
        )?;

        // System metrics
        let memory_usage = Gauge::new("memory_usage_bytes", "Memory usage in bytes")?;

        let cpu_usage = Gauge::new("cpu_usage_percent", "CPU usage percentage")?;

        let disk_usage = GaugeVec::new(
            prometheus::Opts::new("disk_usage_bytes", "Disk usage in bytes"),
            &["mount_point", "device"],
        )?;

        let network_bytes = CounterVec::new(
            prometheus::Opts::new("network_bytes_total", "Network bytes transferred"),
            &["interface", "direction"],
        )?;

        // Register all metrics
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration.clone()))?;
        registry.register(Box::new(http_requests_in_flight.clone()))?;
        registry.register(Box::new(db_connections_active.clone()))?;
        registry.register(Box::new(db_connections_total.clone()))?;
        registry.register(Box::new(db_query_duration.clone()))?;
        registry.register(Box::new(db_queries_total.clone()))?;
        registry.register(Box::new(webhook_events_received.clone()))?;
        registry.register(Box::new(webhook_processing_duration.clone()))?;
        registry.register(Box::new(webhook_events_failed.clone()))?;
        registry.register(Box::new(jobs_queued.clone()))?;
        registry.register(Box::new(jobs_processed.clone()))?;
        registry.register(Box::new(job_processing_duration.clone()))?;
        registry.register(Box::new(jobs_failed.clone()))?;
        registry.register(Box::new(moments_detected.clone()))?;
        registry.register(Box::new(moments_total.clone()))?;
        registry.register(Box::new(moment_detection_duration.clone()))?;
        registry.register(Box::new(positions_tracked.clone()))?;
        registry.register(Box::new(position_updates.clone()))?;
        registry.register(Box::new(position_calculation_duration.clone()))?;
        registry.register(Box::new(price_requests.clone()))?;
        registry.register(Box::new(price_cache_hits.clone()))?;
        registry.register(Box::new(price_cache_misses.clone()))?;
        registry.register(Box::new(price_fetch_duration.clone()))?;
        registry.register(Box::new(cards_rendered.clone()))?;
        registry.register(Box::new(card_render_duration.clone()))?;
        registry.register(Box::new(card_render_failures.clone()))?;
        registry.register(Box::new(memory_usage.clone()))?;
        registry.register(Box::new(cpu_usage.clone()))?;
        registry.register(Box::new(disk_usage.clone()))?;
        registry.register(Box::new(network_bytes.clone()))?;

        Ok(Self {
            registry,
            http_requests_total,
            http_request_duration,
            http_requests_in_flight,
            db_connections_active,
            db_connections_total,
            db_query_duration,
            db_queries_total,
            webhook_events_received,
            webhook_processing_duration,
            webhook_events_failed,
            jobs_queued,
            jobs_processed,
            job_processing_duration,
            jobs_failed,
            moments_detected,
            moments_total,
            moment_detection_duration,
            positions_tracked,
            position_updates,
            position_calculation_duration,
            price_requests,
            price_cache_hits,
            price_cache_misses,
            price_fetch_duration,
            cards_rendered,
            card_render_duration,
            card_render_failures,
            memory_usage,
            cpu_usage,
            disk_usage,
            network_bytes,
        })
    }

    /// Get the Prometheus registry
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Collect all metrics as a Prometheus-formatted string
    pub fn gather(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder
            .encode_to_string(&metric_families)
            .unwrap_or_default()
    }
}

/// Health checker that monitors various system components
pub struct HealthChecker {
    checks: Arc<RwLock<HashMap<String, HealthCheck>>>,
    start_time: Instant,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
            start_time: Instant::now(),
        }
    }

    /// Register a health check
    pub async fn register_check(&self, component: String, initial_status: HealthStatus) {
        let check = HealthCheck {
            component: component.clone(),
            status: initial_status,
            message: None,
            latency_ms: None,
            last_checked: OffsetDateTime::now_utc(),
            metadata: HashMap::new(),
        };

        self.checks.write().await.insert(component, check);
    }

    /// Update health check status
    #[instrument(skip(self))]
    pub async fn update_check(
        &self,
        component: String,
        status: HealthStatus,
        message: Option<String>,
        latency_ms: Option<u64>,
        metadata: Option<HashMap<String, serde_json::Value>>,
    ) {
        let mut checks = self.checks.write().await;
        if let Some(check) = checks.get_mut(&component) {
            check.status = status;
            check.message = message;
            check.latency_ms = latency_ms;
            check.last_checked = OffsetDateTime::now_utc();
            if let Some(meta) = metadata {
                check.metadata = meta;
            }
        }
    }

    /// Get current health report
    pub async fn get_health_report(&self) -> HealthReport {
        let checks_map = self.checks.read().await;
        let checks: Vec<HealthCheck> = checks_map.values().cloned().collect();

        // Determine overall status
        let overall_status = if checks.is_empty() {
            HealthStatus::Unknown
        } else if checks.iter().any(|c| c.status == HealthStatus::Unhealthy) {
            HealthStatus::Unhealthy
        } else if checks.iter().any(|c| c.status == HealthStatus::Degraded) {
            HealthStatus::Degraded
        } else if checks.iter().all(|c| c.status == HealthStatus::Healthy) {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unknown
        };

        HealthReport {
            overall_status,
            checks,
            uptime_seconds: self.start_time.elapsed().as_secs(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_info: BuildInfo {
                version: env!("CARGO_PKG_VERSION").to_string(),
                commit_hash: option_env!("GIT_HASH").unwrap_or("unknown").to_string(),
                build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
                rust_version: option_env!("RUSTC_VERSION")
                    .unwrap_or("unknown")
                    .to_string(),
                build_profile: if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                }
                .to_string(),
            },
            timestamp: OffsetDateTime::now_utc(),
        }
    }

    /// Check if system is healthy
    pub async fn is_healthy(&self) -> bool {
        let report = self.get_health_report().await;
        matches!(
            report.overall_status,
            HealthStatus::Healthy | HealthStatus::Degraded
        )
    }

    /// Start background health monitoring
    pub async fn start_monitoring(self: Arc<Self>) {
        let checker = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Update system metrics
                checker.update_system_metrics().await;

                // Check for stale health checks
                checker.check_stale_checks().await;
            }
        });
    }

    /// Update system-level metrics
    async fn update_system_metrics(&self) {
        // Memory usage
        if let Ok(memory) = self.get_memory_usage().await {
            self.update_check(
                "memory".to_string(),
                if memory > 0.9 {
                    HealthStatus::Unhealthy
                } else if memory > 0.8 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                },
                Some(format!("Memory usage: {:.1}%", memory * 100.0)),
                None,
                Some(HashMap::from([(
                    "usage_percent".to_string(),
                    serde_json::Value::Number(
                        serde_json::Number::from_f64(memory * 100.0).unwrap(),
                    ),
                )])),
            )
            .await;
        }

        // CPU usage
        if let Ok(cpu) = self.get_cpu_usage().await {
            self.update_check(
                "cpu".to_string(),
                if cpu > 0.95 {
                    HealthStatus::Unhealthy
                } else if cpu > 0.85 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                },
                Some(format!("CPU usage: {:.1}%", cpu * 100.0)),
                None,
                Some(HashMap::from([(
                    "usage_percent".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(cpu * 100.0).unwrap()),
                )])),
            )
            .await;
        }

        // Disk usage
        if let Ok(disk) = self.get_disk_usage().await {
            self.update_check(
                "disk".to_string(),
                if disk > 0.95 {
                    HealthStatus::Unhealthy
                } else if disk > 0.85 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                },
                Some(format!("Disk usage: {:.1}%", disk * 100.0)),
                None,
                Some(HashMap::from([(
                    "usage_percent".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(disk * 100.0).unwrap()),
                )])),
            )
            .await;
        }
    }

    /// Check for stale health checks (not updated recently)
    async fn check_stale_checks(&self) {
        let mut checks = self.checks.write().await;
        let now = OffsetDateTime::now_utc();
        let stale_threshold = Duration::from_secs(300); // 5 minutes

        for (component, check) in checks.iter_mut() {
            let duration_since_update = (now - check.last_checked).unsigned_abs();
            if duration_since_update > stale_threshold {
                warn!(
                    "Health check for {} is stale (last updated {} seconds ago)",
                    component,
                    duration_since_update.as_secs()
                );
                check.status = HealthStatus::Unknown;
                check.message = Some(format!(
                    "Stale check (last updated {} seconds ago)",
                    duration_since_update.as_secs()
                ));
            }
        }
    }

    /// Get memory usage (placeholder implementation)
    async fn get_memory_usage(&self) -> anyhow::Result<f64> {
        // In a real implementation, this would read from /proc/meminfo or use a system crate
        Ok(0.5) // 50% memory usage
    }

    /// Get CPU usage (placeholder implementation)
    async fn get_cpu_usage(&self) -> anyhow::Result<f64> {
        // In a real implementation, this would read from /proc/stat or use a system crate
        Ok(0.3) // 30% CPU usage
    }

    /// Get disk usage (placeholder implementation)
    async fn get_disk_usage(&self) -> anyhow::Result<f64> {
        // In a real implementation, this would read disk usage from the filesystem
        Ok(0.6) // 60% disk usage
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilityConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Metrics endpoint path
    pub metrics_endpoint: String,
    /// Health check endpoint path
    pub health_endpoint: String,
    /// Health check interval in seconds
    pub health_check_interval: u64,
    /// Log level
    pub log_level: String,
    /// Enable structured logging
    pub structured_logging: bool,
    /// Enable distributed tracing
    pub tracing_enabled: bool,
    /// Jaeger endpoint for distributed tracing
    pub jaeger_endpoint: Option<String>,
}

impl Default for ObservabilityConfig {
    fn default() -> Self {
        Self {
            metrics_enabled: true,
            metrics_endpoint: "/metrics".to_string(),
            health_endpoint: "/health".to_string(),
            health_check_interval: 30,
            log_level: "info".to_string(),
            structured_logging: true,
            tracing_enabled: false,
            jaeger_endpoint: None,
        }
    }
}

/// Initialize observability stack
pub async fn init_observability(
    config: ObservabilityConfig,
) -> anyhow::Result<(MetricsRegistry, Arc<HealthChecker>)> {
    // Initialize metrics
    let metrics = MetricsRegistry::new()?;

    // Initialize health checker
    let health_checker = Arc::new(HealthChecker::new());

    // Register basic health checks
    health_checker
        .register_check("application".to_string(), HealthStatus::Healthy)
        .await;
    health_checker
        .register_check("memory".to_string(), HealthStatus::Unknown)
        .await;
    health_checker
        .register_check("cpu".to_string(), HealthStatus::Unknown)
        .await;
    health_checker
        .register_check("disk".to_string(), HealthStatus::Unknown)
        .await;

    // Start background monitoring
    health_checker.clone().start_monitoring().await;

    info!("Observability stack initialized");

    Ok((metrics, health_checker))
}
