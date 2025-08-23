use shared::{
    observability::{HealthChecker, MetricsRegistry},
    store::ObjectStore,
    AppConfig, MaybeRedis, Pg, PolicyService,
};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Application state shared across all request handlers
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub database: Pg,
    pub redis: MaybeRedis,
    pub object_store: Arc<dyn ObjectStore>,
    pub policy_service: PolicyService,
    pub metrics_registry: Arc<MetricsRegistry>,
    pub health_checker: Arc<HealthChecker>,
    pub broadcast_tx: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(
        config: AppConfig,
        database: Pg,
        redis: MaybeRedis,
        object_store: Arc<dyn ObjectStore>,
        metrics_registry: MetricsRegistry,
        health_checker: Arc<HealthChecker>,
    ) -> Self {
        let policy_service = PolicyService::new(database.0.clone());
        let (broadcast_tx, _) = broadcast::channel(1000);

        Self {
            config,
            database,
            redis,
            object_store,
            policy_service,
            metrics_registry: Arc::new(metrics_registry),
            health_checker,
            broadcast_tx,
        }
    }

    /// Get database connection pool
    pub fn db(&self) -> &Pg {
        &self.database
    }

    /// Get Redis client if available
    pub fn redis(&self) -> &MaybeRedis {
        &self.redis
    }

    /// Get object store
    pub fn store(&self) -> &Arc<dyn ObjectStore> {
        &self.object_store
    }

    /// Get policy service
    pub fn policy(&self) -> &PolicyService {
        &self.policy_service
    }
}
