use anyhow::Result;
use shared::{AppConfig, Pg};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::env;
use std::sync::Once;
use testcontainers::{clients::Cli, core::WaitFor, images::postgres::Postgres, Container};
use tracing_subscriber::EnvFilter;

mod test_env;
pub use test_env::{TestEnvironment, MockJwksServer, TestUtils};

static INIT: Once = Once::new();

/// Initialize test logging (call once per test suite)
pub fn init_test_logging() {
    INIT.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive("oof_backend=debug".parse().unwrap()))
            .with_test_writer()
            .try_init();
    });
}

/// Test database configuration
pub struct TestDb {
    pub pool: PgPool,
    pub url: String,
    _container: Container<'static, Postgres>,
}

impl TestDb {
    /// Create a new test database with migrations applied
    pub async fn new() -> Result<Self> {
        init_test_logging();

        let docker = Cli::default();
        let postgres_image = Postgres::default()
            .with_db_name("test_db")
            .with_user("test_user")
            .with_password("test_password");

        let container = docker.run(postgres_image);
        let connection_string = format!(
            "postgres://test_user:test_password@127.0.0.1:{}/test_db",
            container.get_host_port_ipv4(5432)
        );

        // Wait for database to be ready
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        // Run migrations
        sqlx::migrate!("../db/migrations").run(&pool).await?;

        // Insert test data
        Self::seed_test_data(&pool).await?;

        Ok(TestDb {
            pool: pool.clone(),
            url: connection_string,
            _container: container,
        })
    }

    /// Create a Pg wrapper for the test database
    pub fn pg(&self) -> Pg {
        Pg(self.pool.clone())
    }

    /// Seed the database with test data
    async fn seed_test_data(pool: &PgPool) -> Result<()> {
        // Insert test token facts
        sqlx::query!(
            "INSERT INTO token_facts (mint, symbol, name, decimals, verified) VALUES
             ('So11111111111111111111111111111111111111112', 'WSOL', 'Wrapped SOL', 9, true),
             ('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 'USD Coin', 6, true),
             ('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 'USDT', 'Tether USD', 6, true),
             ('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263', 'BONK', 'Bonk', 5, true),
             ('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs', 'WIF', 'dogwifhat', 6, true)"
        )
        .execute(pool)
        .await?;

        // Insert test price data
        let now = time::OffsetDateTime::now_utc();
        sqlx::query!(
            "INSERT INTO token_prices (mint, ts, price, source, confidence) VALUES
             ('So11111111111111111111111111111111111111112', $1, 100.50, 'jupiter', 0.95),
             ('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', $1, 1.00, 'jupiter', 0.99),
             ('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', $1, 1.00, 'jupiter', 0.99),
             ('DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263', $1, 0.000025, 'jupiter', 0.85),
             ('7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs', $1, 2.45, 'jupiter', 0.90)",
            now
        )
        .execute(pool)
        .await?;

        // Insert test wallets
        sqlx::query!(
            "INSERT INTO wallet_cursors (wallet) VALUES
             ('9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM'),
             ('5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1'),
             ('6RVB2B1Xpb22KqQ5vVqxf9VzN2H4s5uMp8QN5XKp9tGd')"
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Clean up test data between tests
    pub async fn cleanup(&self) -> Result<()> {
        // Clean up in reverse dependency order
        sqlx::query!("TRUNCATE oof_moments CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE lots CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE holdings CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE actions CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE participants CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE tx_raw CASCADE").execute(&self.pool).await?;
        sqlx::query!("TRUNCATE job_queue CASCADE").execute(&self.pool).await?;

        // Reset sequences
        sqlx::query!("ALTER SEQUENCE job_queue_id_seq RESTART WITH 1").execute(&self.pool).await?;

        Ok(())
    }
}

/// Test configuration for services
pub struct TestConfig {
    pub db_url: String,
    pub redis_url: Option<String>,
    pub test_bucket: String,
}

impl TestConfig {
    pub fn new(db_url: String) -> Self {
        Self {
            db_url,
            redis_url: None, // Use in-memory for tests unless specifically testing Redis
            test_bucket: "s3://test-bucket".to_string(),
        }
    }

    /// Create AppConfig for testing
    pub fn app_config(&self) -> AppConfig {
        AppConfig {
            database_url: self.db_url.clone(),
            redis_url: self.redis_url.clone(),
            asset_bucket: self.test_bucket.clone(),
            api_bind: "127.0.0.1:0".to_string(), // Random port
            indexer_bind: "127.0.0.1:0".to_string(),
            cdn_base: "http://localhost:9000/test-bucket".to_string(),
            cors_allow_origin: Some("http://localhost:3000".to_string()),
            jwks_url: "https://auth.dynamic.xyz/.well-known/jwks".to_string(),
            jwt_issuer: "https://auth.dynamic.xyz".to_string(),
            jwt_audience: "oof-backend-test".to_string(),
            rate_limit_requests_per_minute: 1000,
            rpc_primary: "https://api.mainnet-beta.solana.com".to_string(),
            rpc_backup: Some("https://solana-api.projectserum.com".to_string()),
            jupiter_base_url: "https://price.jup.ag/v3".to_string(),
            helius_webhook_secret: "test-secret".to_string(),
        }
    }
}

/// Test helpers for creating mock data
pub mod fixtures {
    use rust_decimal::Decimal;
    use shared::types::chain::{Action, ChainEvent, EventKind, Participant, TxContext};
    use time::OffsetDateTime;
    use ulid::Ulid;

    /// Create a test chain event
    pub fn create_test_chain_event(
        wallet: &str,
        mint: &str,
        amount: Decimal,
        event_kind: EventKind,
    ) -> ChainEvent {
        ChainEvent {
            id: Ulid::new().to_string(),
            signature: format!("test_sig_{}", Ulid::new()),
            wallet: wallet.to_string(),
            kind: event_kind,
            slot: 123456789,
            timestamp: OffsetDateTime::now_utc(),
            mint: Some(mint.to_string()),
            amount_dec: Some(amount),
            sol_amount_dec: None,
            counterparty: None,
            flags: serde_json::json!({}),
            program_id: "test_program".to_string(),
            version: 1,
        }
    }

    /// Create a test participant
    pub fn create_test_participant(sig: &str, wallet: &str) -> Participant {
        Participant {
            sig: sig.to_string(),
            wallet: wallet.to_string(),
        }
    }

    /// Create a test transaction context
    pub fn create_test_tx_context(signature: &str) -> TxContext {
        TxContext {
            signature: signature.to_string(),
            slot: 123456789,
            block_time: OffsetDateTime::now_utc(),
            status: "confirmed".to_string(),
            compression_url: None,
            raw_size: 1024,
        }
    }

    /// Create a buy action
    pub fn create_buy_action(wallet: &str, mint: &str, amount: Decimal, price: Decimal) -> Action {
        Action {
            id: Ulid::new().to_string(),
            sig: format!("test_sig_{}", Ulid::new()),
            ix_index: 0,
            slot: 123456789,
            ts: OffsetDateTime::now_utc(),
            program_id: "test_program".to_string(),
            action_type: "swap".to_string(),
            mint: Some(mint.to_string()),
            amount_dec: Some(amount),
            sol_amount_dec: Some(amount * price),
            counterparty: None,
            flags: serde_json::json!({
                "direction": "buy",
                "price": price
            }),
        }
    }

    /// Create a sell action
    pub fn create_sell_action(wallet: &str, mint: &str, amount: Decimal, price: Decimal) -> Action {
        Action {
            id: Ulid::new().to_string(),
            sig: format!("test_sig_{}", Ulid::new()),
            ix_index: 0,
            slot: 123456789,
            ts: OffsetDateTime::now_utc(),
            program_id: "test_program".to_string(),
            action_type: "swap".to_string(),
            mint: Some(mint.to_string()),
            amount_dec: Some(amount),
            sol_amount_dec: Some(amount * price),
            counterparty: None,
            flags: serde_json::json!({
                "direction": "sell",
                "price": price
            }),
        }
    }
}

/// Assertion helpers for tests
pub mod assertions {
    use rust_decimal::Decimal;
    use shared::types::moment::{Moment, MomentKind};

    /// Assert that a moment is of the expected type with minimum severity
    pub fn assert_moment_type_and_severity(
        moment: &Moment,
        expected_kind: MomentKind,
        min_severity: Decimal,
    ) {
        assert_eq!(moment.kind, expected_kind);
        assert!(
            moment.severity_dec >= min_severity,
            "Expected severity >= {}, got {}",
            min_severity,
            moment.severity_dec
        );
    }

    /// Assert that a decimal is approximately equal (within tolerance)
    pub fn assert_decimal_approx_eq(actual: Decimal, expected: Decimal, tolerance: Decimal) {
        let diff = (actual - expected).abs();
        assert!(
            diff <= tolerance,
            "Expected ~{} (±{}), got {}. Difference: {}",
            expected,
            tolerance,
            actual,
            diff
        );
    }
}

/// Async test helper macros
#[macro_export]
macro_rules! async_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> anyhow::Result<()> {
            $crate::common::init_test_logging();
            $body
        }
    };
}

/// Integration test helper that sets up database
#[macro_export]
macro_rules! integration_test {
    ($name:ident, $body:expr) => {
        #[tokio::test]
        async fn $name() -> anyhow::Result<()> {
            use $crate::common::TestDb;

            let test_db = TestDb::new().await?;
            let result = $body(&test_db).await;
            test_db.cleanup().await?;
            result
        }
    };
}
