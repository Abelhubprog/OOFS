use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Plan configuration and limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub code: String,
    pub name: String,
    pub price_usd: Decimal,
    pub daily_wallets: i32,
    pub backfill_days: i32,
    pub cadence: PlanCadence,
    pub alerts: i32,
    pub api_rows: i64,
    pub perks: PlanPerks,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanCadence {
    Manual,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPerks {
    pub priority_queue: bool,
    pub custom_cards: bool,
    pub api_access: bool,
    pub historical_data: bool,
    pub real_time_alerts: bool,
    pub nft_minting: bool,
    pub export_data: bool,
}

impl Default for PlanPerks {
    fn default() -> Self {
        Self {
            priority_queue: false,
            custom_cards: false,
            api_access: false,
            historical_data: false,
            real_time_alerts: false,
            nft_minting: false,
            export_data: false,
        }
    }
}

/// User plan subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPlan {
    pub user_id: String,
    pub plan_code: String,
    pub started_at: OffsetDateTime,
    pub expires_at: OffsetDateTime,
    pub auto_renew: bool,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Active,
    Suspended,
    Expired,
    Cancelled,
}

/// Policy state for rate limiting and quotas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyState {
    pub user_id: String,
    pub analyses_today: i32,
    pub api_calls_today: i64,
    pub last_reset_at: OffsetDateTime,
    pub rate_limit_violations: i32,
    pub last_violation_at: Option<OffsetDateTime>,
}

impl PolicyState {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            analyses_today: 0,
            api_calls_today: 0,
            last_reset_at: OffsetDateTime::now_utc(),
            rate_limit_violations: 0,
            last_violation_at: None,
        }
    }

    pub fn should_reset(&self) -> bool {
        let now = OffsetDateTime::now_utc();
        let hours_since_reset = (now - self.last_reset_at).whole_hours();
        hours_since_reset >= 24
    }

    pub fn reset_daily_counters(&mut self) {
        self.analyses_today = 0;
        self.api_calls_today = 0;
        self.last_reset_at = OffsetDateTime::now_utc();
    }
}

/// User context for authorization
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: String,
    pub wallet_address: Option<String>,
    pub plan: Plan,
    pub policy_state: PolicyState,
    pub is_authenticated: bool,
    pub auth_method: AuthMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMethod {
    JWT,
    WalletSignature,
    ApiKey,
    Anonymous,
}

impl UserContext {
    pub fn can_analyze_wallet(&self) -> bool {
        self.is_authenticated && self.policy_state.analyses_today < self.plan.daily_wallets
    }

    pub fn can_make_api_call(&self) -> bool {
        self.policy_state.api_calls_today < self.plan.api_rows
    }

    pub fn remaining_analyses(&self) -> i32 {
        (self.plan.daily_wallets - self.policy_state.analyses_today).max(0)
    }

    pub fn remaining_api_calls(&self) -> i64 {
        (self.plan.api_rows - self.policy_state.api_calls_today).max(0)
    }
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_size: u32,
    pub per_ip_limit: u32,
    pub per_user_limit: u32,
    pub whitelist: Vec<String>,
    pub blacklist: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            burst_size: 10,
            per_ip_limit: 100,
            per_user_limit: 200,
            whitelist: Vec::new(),
            blacklist: Vec::new(),
        }
    }
}

/// Staking boost calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingBoost {
    pub wallet_address: String,
    pub staked_amount: Decimal,
    pub boost_multiplier: Decimal,
    pub boost_expires_at: OffsetDateTime,
    pub perks: BoostPerks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoostPerks {
    pub extra_daily_wallets_pct: Decimal, // e.g., 0.50 for +50%
    pub card_mint_fee_discount_pct: Decimal, // e.g., 0.50 for -50%
    pub priority_queue: bool,
    pub fast_lane: bool,
}

impl Default for BoostPerks {
    fn default() -> Self {
        Self {
            extra_daily_wallets_pct: Decimal::ZERO,
            card_mint_fee_discount_pct: Decimal::ZERO,
            priority_queue: false,
            fast_lane: false,
        }
    }
}

impl StakingBoost {
    pub fn apply_to_plan(&self, mut plan: Plan) -> Plan {
        // Apply staking boosts to plan limits
        let extra_wallets = (Decimal::from(plan.daily_wallets) * self.perks.extra_daily_wallets_pct).to_i32().unwrap_or(0);
        plan.daily_wallets += extra_wallets;

        // Update perks
        plan.perks.priority_queue = plan.perks.priority_queue || self.perks.priority_queue;

        plan
    }

    pub fn is_active(&self) -> bool {
        OffsetDateTime::now_utc() < self.boost_expires_at && self.staked_amount > Decimal::ZERO
    }
}

/// Job queue priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

impl Default for JobPriority {
    fn default() -> Self {
        JobPriority::Normal
    }
}

/// Quota enforcement result
#[derive(Debug, Clone)]
pub enum QuotaCheck {
    Allowed,
    DailyLimitExceeded,
    RateLimitExceeded,
    PlanExpired,
    InsufficientPermissions,
}

impl QuotaCheck {
    pub fn is_allowed(&self) -> bool {
        matches!(self, QuotaCheck::Allowed)
    }

    pub fn error_message(&self) -> &'static str {
        match self {
            QuotaCheck::Allowed => "Allowed",
            QuotaCheck::DailyLimitExceeded => "Daily analysis limit exceeded",
            QuotaCheck::RateLimitExceeded => "Rate limit exceeded",
            QuotaCheck::PlanExpired => "Plan has expired",
            QuotaCheck::InsufficientPermissions => "Insufficient permissions",
        }
    }
}
