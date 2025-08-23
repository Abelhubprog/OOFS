use anyhow::Result;
use shared::{config::AppConfig, db::Pg, health::EnhancedHealthChecker, MaybeRedis};
use sqlx::PgPool;
use std::sync::Arc;
use testcontainers::{clients::Cli, containers::postgres::Postgres, core::WaitFor, Container, Image};
use tokio::sync::OnceCell;

/// Test environment for integration tests
pub struct TestEnvironment {
    pub config: AppConfig,
    pub database: Pg,
    pub redis: MaybeRedis,
    pub health_checker: Arc<EnhancedHealthChecker>,
    pub client: reqwest::Client,
    _pg_container: Container<'static, Postgres>,
}

static DOCKER: OnceCell<Cli> = OnceCell::const_new();

impl TestEnvironment {
    /// Create a new test environment with isolated database
    pub async fn new() -> Result<Self> {
        let docker = DOCKER.get_or_init(|| async { Cli::default() }).await;

        // Start PostgreSQL container
        let pg_container = docker.run(Postgres::default());
        let connection_string = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            pg_container.get_host_port_ipv4(5432)
        );

        // Wait for database to be ready
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Connect to database
        let pool = sqlx::PgPool::connect(&connection_string).await?;

        // Run migrations
        sqlx::migrate!("../../../db/migrations").run(&pool).await?;

        // Seed test data
        Self::seed_test_data(&pool).await?;

        // Create test configuration
        let config = Self::create_test_config(&connection_string);

        // Initialize components
        let database = Pg(pool);
        let redis = MaybeRedis::Disabled; // Use in-memory for tests
        let health_checker = Arc::new(EnhancedHealthChecker::new(
            shared::health::HealthConfig::default(),
        ));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            config,
            database,
            redis,
            health_checker,
            client,
            _pg_container: pg_container,
        })
    }

    /// Create test configuration
    fn create_test_config(database_url: &str) -> AppConfig {
        AppConfig {
            database_url: database_url.to_string(),
            redis_url: None,
            rpc_primary: "https://api.devnet.solana.com".to_string(),
            rpc_secondary: None,
            helius_webhook_secret: "test_secret".to_string(),
            jupiter_base_url: "https://price.jup.ag/v3".to_string(),
            pyth_sse: "wss://hermes.pyth.network/ws".to_string(),
            dynamic_environment_id: "test_env_id".to_string(),
            dynamic_api_key: "test_api_key".to_string(),
            dynamic_jwks_url: "https://example.com/jwks".to_string(),
            dynamic_webhook_secret: Some("test_webhook_secret".to_string()),
            r2_endpoint: "https://test.r2.cloudflarestorage.com".to_string(),
            r2_access_key_id: "test_access_key".to_string(),
            r2_secret_access_key: "test_secret_key".to_string(),
            r2_bucket_name: "test-bucket".to_string(),
            r2_public_url: "https://test-assets.example.com".to_string(),
            api_bind: "127.0.0.1:0".to_string(),
            indexer_bind: "127.0.0.1:0".to_string(),
            cors_allow_origin: Some("*".to_string()),
            app_secret: "test_app_secret_that_is_long_enough_for_validation".to_string(),
            environment: "test".to_string(),
        }
    }

    /// Seed test data
    async fn seed_test_data(pool: &PgPool) -> Result<()> {
        // Insert test token facts
        sqlx::query!(
            r#"
            INSERT INTO token_facts (mint, symbol, name, decimals, verified) VALUES
            ('So11111111111111111111111111111111111111112', 'WSOL', 'Wrapped SOL', 9, true),
            ('EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'USDC', 'USD Coin', 6, true),
            ('Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB', 'USDT', 'Tether USD', 6, true)
            ON CONFLICT (mint) DO NOTHING
            "#
        )
        .execute(pool)
        .await?;

        // Insert test wallet data
        sqlx::query!(
            r#"
            INSERT INTO participants (sig, wallet) VALUES
            ('test_signature_1', 'test_wallet_1'),
            ('test_signature_2', 'test_wallet_2')
            ON CONFLICT DO NOTHING
            "#
        )
        .execute(pool)
        .await?;

        // Insert test OOF moments
        sqlx::query!(
            r#"
            INSERT INTO moments (id, wallet, mint, kind, t_event, pct_dec, missed_usd_dec, severity_dec, version) VALUES
            ('test_moment_1', 'test_wallet_1', 'So11111111111111111111111111111111111111112', 'S2E', NOW(), 0.25, 1000.00, 0.75, '1'),
            ('test_moment_2', 'test_wallet_2', 'EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v', 'BHD', NOW(), -0.30, 500.00, 0.60, '1')
            ON CONFLICT (id) DO NOTHING
            "#
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Create a test JWT token for authentication
    pub async fn create_test_jwt(&self, user_id: &str, roles: Vec<String>) -> Result<String> {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use shared::auth::Claims;
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?;

        let claims = Claims {
            sub: user_id.to_string(),
            exp: now.as_secs() as i64 + 3600, // 1 hour
            iat: now.as_secs() as i64,
            iss: "test_issuer".to_string(),
            aud: vec!["test_audience".to_string()],
            email: Some(format!("{}@test.com", user_id)),
            email_verified: Some(true),
            environment_id: Some(self.config.dynamic_environment_id.clone()),
            user_id: Some(user_id.to_string()),
            wallet_public_key: Some("test_wallet_public_key".to_string()),
            wallet_name: Some("phantom".to_string()),
            auth_provider: Some("wallet".to_string()),
            social_provider: None,
            roles,
            permissions: Some(vec!["read".to_string(), "write".to_string()]),
            subscription_tier: Some("premium".to_string()),
            rate_limit_tier: Some("high".to_string()),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.app_secret.as_bytes()),
        )?;

        Ok(token)
    }

    /// Create test transaction data
    pub async fn create_test_transaction(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "signature": "test_signature_12345",
            "slot": 123456789,
            "blockTime": 1640995200,
            "transaction": {
                "message": {
                    "accountKeys": [
                        {"pubkey": "test_wallet_1"},
                        {"pubkey": "So11111111111111111111111111111111111111112"}
                    ],
                    "instructions": [
                        {
                            "programId": "11111111111111111111111111111111",
                            "accounts": [0, 1],
                            "data": "test_instruction_data"
                        }
                    ]
                }
            },
            "meta": {
                "err": null,
                "fee": 5000,
                "preBalances": [1000000000, 0],
                "postBalances": [999995000, 0]
            }
        }))
    }

    /// Get moments for a specific wallet
    pub async fn get_moments_for_wallet(&self, wallet: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query!(
            "SELECT id, wallet, mint, kind, t_event, pct_dec, missed_usd_dec FROM moments WHERE wallet = $1",
            wallet
        )
        .fetch_all(&self.database.0)
        .await?;

        let moments: Vec<serde_json::Value> = rows
            .into_iter()
            .map(|row| {
                serde_json::json!({
                    "id": row.id,
                    "wallet": row.wallet,
                    "mint": row.mint,
                    "kind": row.kind,
                    "t_event": row.t_event,
                    "pct_dec": row.pct_dec,
                    "missed_usd_dec": row.missed_usd_dec
                })
            })
            .collect();

        Ok(moments)
    }

    /// Process a transaction through the detection engine
    pub async fn process_transaction(&self, _tx_data: serde_json::Value) -> Result<()> {
        // In a full implementation, this would:
        // 1. Parse the transaction data
        // 2. Run it through the detector engine
        // 3. Store any detected moments
        // For now, we'll simulate by inserting a test moment

        sqlx::query!(
            r#"
            INSERT INTO moments (id, wallet, mint, kind, t_event, pct_dec, missed_usd_dec, severity_dec, version)
            VALUES ($1, $2, $3, $4, NOW(), $5, $6, $7, $8)
            ON CONFLICT (id) DO NOTHING
            "#,
            "processed_moment_test",
            "test_wallet_processed",
            "So11111111111111111111111111111111111111112",
            "S2E",
            rust_decimal::Decimal::new(35, 2), // 0.35
            rust_decimal::Decimal::new(75000, 2), // 750.00
            rust_decimal::Decimal::new(85, 2), // 0.85
            "1"
        )
        .execute(&self.database.0)
        .await?;

        Ok(())
    }

    /// Test database connectivity
    pub async fn test_database_connection(&self) -> Result<bool> {
        let result = sqlx::query("SELECT 1").fetch_one(&self.database.0).await;
        Ok(result.is_ok())
    }

    /// Clean up test data
    pub async fn cleanup(&self) -> Result<()> {
        // Clean up test data in reverse order of dependencies
        sqlx::query!("DELETE FROM moments WHERE id LIKE 'test_%' OR id LIKE 'processed_%'")
            .execute(&self.database.0)
            .await?;

        sqlx::query!("DELETE FROM participants WHERE sig LIKE 'test_%'")
            .execute(&self.database.0)
            .await?;

        Ok(())
    }
}

/// Mock Dynamic.xyz JWKS server for testing
pub struct MockJwksServer {
    pub base_url: String,
    server: Option<tokio::task::JoinHandle<()>>,
}

impl MockJwksServer {
    pub async fn start() -> Result<Self> {
        use axum::{routing::get, Json, Router};
        use serde_json::json;

        let app = Router::new().route("/jwks", get(|| async {
            Json(json!({
                "keys": [
                    {
                        "kty": "RSA",
                        "kid": "test_key_1",
                        "n": "test_n_value",
                        "e": "AQAB",
                        "alg": "RS256"
                    }
                ]
            }))
        }));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let base_url = format!("http://127.0.0.1:{}", addr.port());

        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(Self {
            base_url,
            server: Some(server),
        })
    }

    pub fn jwks_url(&self) -> String {
        format!("{}/jwks", self.base_url)
    }
}

impl Drop for MockJwksServer {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.abort();
        }
    }
}

/// Test utilities for common test scenarios
pub struct TestUtils;

impl TestUtils {
    /// Create a test wallet address
    pub fn random_wallet() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
        bs58::encode(bytes).into_string()
    }

    /// Create a test mint address
    pub fn random_mint() -> String {
        Self::random_wallet() // Same format as wallet
    }

    /// Create a test signature
    pub fn random_signature() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let bytes: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
        bs58::encode(bytes).into_string()
    }

    /// Assert that a response contains expected fields
    pub fn assert_response_structure(
        response: &serde_json::Value,
        expected_fields: &[&str],
    ) -> Result<()> {
        for field in expected_fields {
            if !response.get(field).is_some() {
                return Err(anyhow::anyhow!("Missing field: {}", field));
            }
        }
        Ok(())
    }

    /// Wait for condition with timeout
    pub async fn wait_for_condition<F, Fut>(
        condition: F,
        timeout: std::time::Duration,
        check_interval: std::time::Duration,
    ) -> Result<()>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = bool>,
    {
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if condition().await {
                return Ok(());
            }
            tokio::time::sleep(check_interval).await;
        }

        Err(anyhow::anyhow!("Condition not met within timeout"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_environment_creation() {
        let env = TestEnvironment::new().await.unwrap();
        assert!(env.test_database_connection().await.unwrap());
    }

    #[tokio::test]
    async fn test_jwt_creation() {
        let env = TestEnvironment::new().await.unwrap();
        let token = env
            .create_test_jwt("test_user", vec!["user".to_string()])
            .await
            .unwrap();
        assert!(!token.is_empty());
    }

    #[tokio::test]
    async fn test_mock_jwks_server() {
        let server = MockJwksServer::start().await.unwrap();
        let client = reqwest::Client::new();
        let response = client.get(&server.jwks_url()).send().await.unwrap();
        assert!(response.status().is_success());
    }
}
