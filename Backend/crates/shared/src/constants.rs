use rust_decimal::Decimal;
use std::str::FromStr;
use time::Duration;

/// System-wide constants
pub mod system {
    use super::*;

    /// Maximum backfill days for any plan (2 years)
    pub const MAX_BACKFILL_DAYS: i64 = 730;

    /// Default job retry attempts
    pub const DEFAULT_JOB_RETRIES: i32 = 5;

    /// Position engine snapshot interval (every N events)
    pub const POSITION_SNAPSHOT_INTERVAL: usize = 500;

    /// Maximum signatures per backfill batch
    pub const MAX_SIGNATURES_PER_BATCH: usize = 1000;

    /// Rate limiting window duration
    pub const RATE_LIMIT_WINDOW: Duration = Duration::minutes(1);

    /// Card image dimensions
    pub const CARD_WIDTH: u32 = 1200;
    pub const CARD_HEIGHT: u32 = 630;

    /// Price bucket time ranges
    pub const PRICE_1M_DAYS: i64 = 7;
    pub const PRICE_5M_DAYS: i64 = 180;
    pub const PRICE_1H_DAYS: i64 = 730;
}

/// OOF moment detection thresholds
pub mod thresholds {
    use super::*;

    /// S2E (Sold Too Early) minimum percentage to trigger
    pub fn s2e_min_pct() -> Decimal {
        Decimal::from_str("0.25").unwrap()
    }

    /// S2E minimum USD amount to trigger
    pub fn s2e_min_usd() -> Decimal {
        Decimal::from_str("25").unwrap()
    }

    /// BHD (Bag Holder Drawdown) minimum percentage to trigger
    pub fn bhd_min_pct() -> Decimal {
        Decimal::from_str("-0.30").unwrap()
    }

    /// Bad Route minimum percentage worse to trigger
    pub fn bad_route_min_pct() -> Decimal {
        Decimal::from_str("0.01").unwrap()
    }

    /// Idle Yield minimum USD amount to trigger
    pub fn idle_min_usd() -> Decimal {
        Decimal::from_str("25").unwrap()
    }

    /// OOF token annual percentage rate for idle calculations
    pub fn oof_apr() -> Decimal {
        Decimal::from_str("0.08").unwrap() // 8% APR
    }
}

/// Moment severity calculation
pub mod severity {
    use super::*;

    /// Calculate S2E severity (0.0 to 1.0)
    pub fn s2e_severity(missed_pct: Decimal) -> Decimal {
        let base_pct = Decimal::from_str("0.75").unwrap();
        (missed_pct / base_pct).clamp(Decimal::ZERO, Decimal::ONE)
    }

    /// Calculate BHD severity (0.0 to 1.0)
    pub fn bhd_severity(dd_pct: Decimal) -> Decimal {
        let base_dd = Decimal::from_str("0.30").unwrap();
        let max_dd = Decimal::from_str("0.50").unwrap();
        ((dd_pct.abs() - base_dd) / max_dd).clamp(Decimal::ZERO, Decimal::ONE)
    }

    /// Calculate Bad Route severity (0.0 to 1.0)
    pub fn bad_route_severity(worse_pct: Decimal) -> Decimal {
        let max_worse = Decimal::from_str("0.10").unwrap(); // 10% worse = max severity
        (worse_pct / max_worse).clamp(Decimal::ZERO, Decimal::ONE)
    }
}

/// Solana program IDs and addresses
pub mod solana {
    /// Native SOL mint address
    pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

    /// USDC mint address
    pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

    /// Token Program ID
    pub const TOKEN_PROGRAM: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

    /// Token-2022 Program ID
    pub const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

    /// Associated Token Program ID
    pub const ASSOCIATED_TOKEN_PROGRAM: &str = "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL";

    /// System Program ID
    pub const SYSTEM_PROGRAM: &str = "11111111111111111111111111111111";
}

/// API versioning and headers
pub mod api {
    /// Current API version
    pub const VERSION: &str = "v1";

    /// Request ID header
    pub const REQUEST_ID_HEADER: &str = "x-request-id";

    /// Rate limit headers
    pub const RATE_LIMIT_LIMIT_HEADER: &str = "x-ratelimit-limit";
    pub const RATE_LIMIT_REMAINING_HEADER: &str = "x-ratelimit-remaining";
    pub const RATE_LIMIT_RESET_HEADER: &str = "x-ratelimit-reset";

    /// Default pagination limit
    pub const DEFAULT_PAGE_SIZE: usize = 50;
    pub const MAX_PAGE_SIZE: usize = 1000;
}

/// Cache keys and TTLs
pub mod cache {
    use super::*;

    /// Wallet extremes cache TTL
    pub const WALLET_EXTREMES_TTL: Duration = Duration::minutes(10);

    /// Price cache TTL
    pub const PRICE_CACHE_TTL: Duration = Duration::minutes(5);

    /// JWT cache TTL
    pub const JWT_CACHE_TTL: Duration = Duration::hours(1);

    /// Rate limit cache TTL
    pub const RATE_LIMIT_TTL: Duration = Duration::minutes(1);

    /// Cache key prefixes
    pub const WALLET_EXTREMES_PREFIX: &str = "wallet_extremes:";
    pub const PRICE_PREFIX: &str = "price:";
    pub const RATE_LIMIT_PREFIX: &str = "rate_limit:";
    pub const JWT_PREFIX: &str = "jwt:";
}
