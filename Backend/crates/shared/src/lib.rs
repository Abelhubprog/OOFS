pub mod auth;
pub mod config;
pub mod constants;
pub mod db;
pub mod errors;
pub mod metrics;
pub mod observability;
pub mod policy;
pub mod redis;
pub mod security;
pub mod store;
pub mod telemetry;
pub mod types;
pub mod utils;

// Re-export commonly used types
pub use config::AppConfig;
pub use db::Pg;
pub use errors::{ApiError, ApiResult};
pub use metrics::{metrics_router, Metrics};
pub use policy::PolicyService;
pub use redis::{MaybeRedis, RedisClient};
pub use store::{make_store, ObjectStore};
pub use telemetry::{init_telemetry, service_name, service_version};
pub use types::{
    chain::{Action, ChainEvent, EventKind, Participant, TxContext, TxRaw},
    moment::{ExtremeEntry, Moment, MomentContext, MomentKind, WalletExtremes},
    policy::{
        AuthMethod, Plan, PolicyState, QuotaCheck, RateLimitConfig, StakingBoost, UserContext,
        UserPlan,
    },
    price::{
        Candle, PriceBucket, PriceConfidence, PricePoint, PriceProvider, PriceRange, PriceSource,
    },
};

// Helper trait for decimal serialization
use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer, Serializer};

pub struct DecimalSerde;

impl DecimalSerde {
    pub fn serialize<S: Serializer>(d: &Decimal, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&d.normalize().to_string())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Decimal, D::Error> {
        let s = String::deserialize(d)?;
        Decimal::from_str_exact(&s).map_err(serde::de::Error::custom)
    }
}

// Result extension trait
pub trait ResultExt<T> {
    fn or_500(self) -> ApiResult<T>;
    fn or_bad_request(self, msg: &str) -> ApiResult<T>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn or_500(self) -> ApiResult<T> {
        self.map_err(|e| ApiError::Internal(anyhow::anyhow!(e.to_string())))
    }

    fn or_bad_request(self, msg: &str) -> ApiResult<T> {
        self.map_err(|_| ApiError::BadRequest(msg.to_string()))
    }
}

// Common validation functions
pub mod validation {
    use crate::errors::{ApiError, ApiResult};

    /// Validate a Solana wallet address
    pub fn validate_wallet_address(addr: &str) -> ApiResult<()> {
        if addr.len() < 32 || addr.len() > 44 {
            return Err(ApiError::InvalidWallet);
        }

        // Basic base58 validation
        if !addr
            .chars()
            .all(|c| "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz".contains(c))
        {
            return Err(ApiError::InvalidWallet);
        }

        Ok(())
    }

    /// Validate pagination parameters
    pub fn validate_pagination(
        limit: Option<usize>,
        cursor: Option<&str>,
    ) -> ApiResult<(usize, Option<String>)> {
        let limit = limit
            .unwrap_or(crate::constants::api::DEFAULT_PAGE_SIZE)
            .min(crate::constants::api::MAX_PAGE_SIZE);

        if limit == 0 {
            return Err(ApiError::BadRequest(
                "Limit must be greater than 0".to_string(),
            ));
        }

        Ok((limit, cursor.map(|s| s.to_string())))
    }

    /// Validate moment kind filter
    pub fn validate_moment_kinds(kinds: Option<&str>) -> ApiResult<Vec<crate::MomentKind>> {
        match kinds {
            None => Ok(vec![]),
            Some(kinds_str) => {
                let mut valid_kinds = Vec::new();
                for kind_str in kinds_str.split(',') {
                    let kind = kind_str.trim().parse::<crate::MomentKind>().map_err(|_| {
                        ApiError::BadRequest(format!("Invalid moment kind: {}", kind_str))
                    })?;
                    valid_kinds.push(kind);
                }
                Ok(valid_kinds)
            }
        }
    }
}

#[cfg(test)]
mod lib_tests;
