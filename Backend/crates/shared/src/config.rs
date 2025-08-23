use serde::Deserialize;
use std::{env, fs};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingRequiredVar(String),
    #[error("Invalid configuration value for {key}: {value}")]
    InvalidValue { key: String, value: String },
    #[error("Configuration file error: {0}")]
    FileError(#[from] std::io::Error),
    #[error("YAML parsing error: {0}")]
    YamlError(#[from] serde_yaml::Error),
}

type ConfigResult<T> = Result<T, ConfigError>;

#[derive(Debug, Clone, Deserialize)]
pub struct CorsCfg {
    pub allow_origin: String,
    #[serde(default)]
    pub allow_headers: Vec<String>,
    #[serde(default)]
    pub allow_methods: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiCfg {
    pub bind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigFile {
    pub cors: Option<CorsCfg>,
    pub api: Option<ApiCfg>,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    // Database and Redis (Railway.com)
    pub database_url: String,
    pub redis_url: Option<String>,

    // Solana RPC endpoints
    pub rpc_primary: String,
    pub rpc_secondary: Option<String>,

    // External service integration
    pub helius_webhook_secret: String,
    pub jupiter_base_url: String,
    pub pyth_sse: String,

    // Dynamic.xyz Authentication
    pub dynamic_environment_id: String,
    pub dynamic_api_key: String,
    pub dynamic_jwks_url: String,
    pub dynamic_webhook_secret: Option<String>,

    // Cloudflare R2 Storage
    pub r2_endpoint: String,
    pub r2_access_key_id: String,
    pub r2_secret_access_key: String,
    pub r2_bucket_name: String,
    pub r2_public_url: String,

    // Server configuration
    pub api_bind: String,
    pub indexer_bind: String,
    pub cors_allow_origin: Option<String>,

    // Additional security
    pub app_secret: String,
    pub environment: String,
}

impl AppConfig {
    pub fn from_env() -> ConfigResult<Self> {
        Self::from_env_with_defaults()
    }

    pub fn from_env_with_defaults() -> ConfigResult<Self> {
        dotenvy::dotenv().ok();
        let config_path = env::var("CONFIG_FILE").ok();
        let file_cfg: Option<ConfigFile> = config_path
            .as_deref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_yaml::from_str(&s).ok());

        let api_bind = env::var("API_BIND")
            .ok()
            .or_else(|| {
                file_cfg
                    .as_ref()
                    .and_then(|f| f.api.as_ref().map(|a| a.bind.clone()))
            })
            .unwrap_or_else(|| "0.0.0.0:8080".to_string());
        let indexer_bind = env::var("INDEXER_BIND")
            .ok()
            .unwrap_or_else(|| "0.0.0.0:8081".to_string());
        let cors_allow_origin = env::var("ALLOW_ORIGIN").ok().or_else(|| {
            file_cfg
                .as_ref()
                .and_then(|f| f.cors.as_ref().map(|c| c.allow_origin.clone()))
        });

        Ok(Self {
            // Database and Redis (Railway.com)
            database_url: Self::get_required_var("DATABASE_URL")?,
            redis_url: env::var("REDIS_URL").ok(),

            // Solana RPC endpoints
            rpc_primary: Self::get_required_var("RPC_PRIMARY")?,
            rpc_secondary: env::var("RPC_SECONDARY").ok(),

            // External service integration
            helius_webhook_secret: env::var("HELIUS_WEBHOOK_SECRET").unwrap_or_default(),
            jupiter_base_url: env::var("JUPITER_BASE_URL")
                .unwrap_or_else(|_| "https://price.jup.ag/v3".into()),
            pyth_sse: env::var("PYTH_HERMES_SSE").unwrap_or_default(),

            // Dynamic.xyz Authentication
            dynamic_environment_id: Self::get_required_var("DYNAMIC_ENVIRONMENT_ID")?,
            dynamic_api_key: Self::get_required_var("DYNAMIC_API_KEY")?,
            dynamic_jwks_url: env::var("DYNAMIC_JWKS_URL").unwrap_or_else(|_| {
                "https://app.dynamic.xyz/api/v0/environments/{ENVIRONMENT_ID}/keys".into()
            }),
            dynamic_webhook_secret: env::var("DYNAMIC_WEBHOOK_SECRET").ok(),

            // Cloudflare R2 Storage
            r2_endpoint: Self::get_required_var("R2_ENDPOINT")?,
            r2_access_key_id: Self::get_required_var("R2_ACCESS_KEY_ID")?,
            r2_secret_access_key: Self::get_required_var("R2_SECRET_ACCESS_KEY")?,
            r2_bucket_name: Self::get_required_var("R2_BUCKET_NAME")?,
            r2_public_url: Self::get_required_var("R2_PUBLIC_URL")?,

            // Server configuration
            api_bind,
            indexer_bind,
            cors_allow_origin,

            // Additional security
            app_secret: Self::get_required_var("APP_SECRET")?,
            environment: env::var("ENVIRONMENT").unwrap_or_else(|_| "development".into()),
        })
    }

    fn get_required_var(key: &str) -> ConfigResult<String> {
        env::var(key).map_err(|_| ConfigError::MissingRequiredVar(key.to_string()))
    }

    pub fn validate(&self) -> ConfigResult<()> {
        // Validate configuration values
        if self.app_secret.len() < 32 {
            return Err(ConfigError::InvalidValue {
                key: "APP_SECRET".to_string(),
                value: "[REDACTED - too short]".to_string(),
            });
        }

        if !self.r2_public_url.starts_with("https://") && self.environment == "production" {
            return Err(ConfigError::InvalidValue {
                key: "R2_PUBLIC_URL".to_string(),
                value: "Must use HTTPS in production".to_string(),
            });
        }

        Ok(())
    }
}
