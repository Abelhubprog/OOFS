use serde::{Deserialize, Serialize};
use shared::{
    types::{
        common::{SolAmount, TokenAmount, UsdAmount},
        moment::{MomentKind, MomentSeverity},
        wallet::{PerformanceMetrics, PortfolioSnapshot, Position, WalletAnalysis},
    },
    validation::validate_pubkey,
};
use std::collections::HashMap;
use time::OffsetDateTime;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

/// Request to get wallet information
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct GetWalletRequest {
    /// Wallet public key
    #[validate(custom = "validate_pubkey")]
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Include current positions
    #[schema(example = true)]
    pub include_positions: Option<bool>,

    /// Include performance metrics
    #[schema(example = true)]
    pub include_metrics: Option<bool>,

    /// Include recent activity
    #[schema(example = true)]
    pub include_activity: Option<bool>,

    /// Include OOF summary
    #[schema(example = true)]
    pub include_oof_summary: Option<bool>,
}

/// Wallet information response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetWalletResponse {
    /// Wallet address
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Wallet metadata
    pub metadata: WalletMetadata,

    /// Current portfolio snapshot
    pub portfolio: PortfolioSummary,

    /// Current positions (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positions: Option<Vec<PositionDetail>>,

    /// Performance metrics (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<PerformanceMetrics>,

    /// Recent activity (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_activity: Option<Vec<ActivityItem>>,

    /// OOF summary (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oof_summary: Option<OofSummary>,
}

/// Wallet metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletMetadata {
    /// Whether wallet is currently being tracked
    #[schema(example = true)]
    pub is_tracked: bool,

    /// First transaction date
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2023-01-15T10:30:00Z")]
    pub first_transaction: Option<OffsetDateTime>,

    /// Last transaction date
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-08-21T15:45:00Z")]
    pub last_transaction: Option<OffsetDateTime>,

    /// Total number of transactions
    #[schema(example = 1250)]
    pub transaction_count: u32,

    /// Number of unique tokens traded
    #[schema(example = 45)]
    pub unique_tokens: u32,

    /// Last analysis date
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "2024-08-21T12:00:00Z")]
    pub last_analyzed: Option<OffsetDateTime>,

    /// Data freshness score (0-1)
    #[schema(example = 0.95)]
    pub data_freshness: f64,

    /// Wallet labels/tags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

/// Portfolio summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortfolioSummary {
    /// Total portfolio value in USD
    #[schema(example = "75000.00")]
    pub total_value: UsdAmount,

    /// 24h change in USD
    #[schema(example = "1250.00")]
    pub change_24h: UsdAmount,

    /// 24h change percentage
    #[schema(example = 1.7)]
    pub change_24h_pct: f64,

    /// Number of active positions
    #[schema(example = 8)]
    pub active_positions: u32,

    /// Largest position percentage
    #[schema(example = 35.5)]
    pub largest_position_pct: f64,

    /// Portfolio diversification score (0-1)
    #[schema(example = 0.7)]
    pub diversification: f64,

    /// SOL balance
    #[schema(example = "12.5")]
    pub sol_balance: SolAmount,

    /// Top holdings
    pub top_holdings: Vec<HoldingSummary>,
}

/// Holding summary
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HoldingSummary {
    /// Token symbol
    #[schema(example = "SOL")]
    pub symbol: String,

    /// Token name
    #[schema(example = "Solana")]
    pub name: String,

    /// Token mint address
    #[schema(example = "So11111111111111111111111111111111111111112")]
    pub mint: String,

    /// Amount held
    #[schema(example = "100.0")]
    pub amount: TokenAmount,

    /// Current price
    #[schema(example = "150.00")]
    pub price: UsdAmount,

    /// Total value
    #[schema(example = "15000.00")]
    pub value: UsdAmount,

    /// Percentage of portfolio
    #[schema(example = 20.0)]
    pub portfolio_pct: f64,

    /// 24h change percentage
    #[schema(example = 2.5)]
    pub change_24h_pct: f64,
}

/// Detailed position information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PositionDetail {
    /// Token information
    pub token: TokenInfo,

    /// Position size
    #[schema(example = "500.0")]
    pub amount: TokenAmount,

    /// Average cost basis
    #[schema(example = "80.00")]
    pub avg_cost: UsdAmount,

    /// Current market price
    #[schema(example = "120.00")]
    pub market_price: UsdAmount,

    /// Total cost basis
    #[schema(example = "40000.00")]
    pub total_cost: UsdAmount,

    /// Current market value
    #[schema(example = "60000.00")]
    pub market_value: UsdAmount,

    /// Unrealized P&L
    #[schema(example = "20000.00")]
    pub unrealized_pnl: UsdAmount,

    /// Unrealized P&L percentage
    #[schema(example = 50.0)]
    pub unrealized_pnl_pct: f64,

    /// Realized P&L (from closed portions)
    #[schema(example = "5000.00")]
    pub realized_pnl: UsdAmount,

    /// Total P&L (realized + unrealized)
    #[schema(example = "25000.00")]
    pub total_pnl: UsdAmount,

    /// Position entry date
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub entry_date: OffsetDateTime,

    /// Days held
    #[schema(example = 218)]
    pub days_held: u32,

    /// Position risk level
    pub risk_level: RiskLevel,

    /// Associated OOF moments count
    #[schema(example = 2)]
    pub oof_moments_count: u32,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenInfo {
    /// Token symbol
    #[schema(example = "SOL")]
    pub symbol: String,

    /// Token name
    #[schema(example = "Solana")]
    pub name: String,

    /// Token mint address
    #[schema(example = "So11111111111111111111111111111111111111112")]
    pub mint: String,

    /// Token decimals
    #[schema(example = 9)]
    pub decimals: u8,

    /// Token logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "https://example.com/logo.png")]
    pub logo_url: Option<String>,

    /// Coingecko ID
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "solana")]
    pub coingecko_id: Option<String>,
}

/// Position risk level
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum RiskLevel {
    /// Low risk position
    Low,
    /// Medium risk position
    Medium,
    /// High risk position
    High,
    /// Very high risk position
    VeryHigh,
}

/// Recent activity item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ActivityItem {
    /// Activity type
    pub activity_type: ActivityType,

    /// Timestamp
    #[schema(example = "2024-08-21T15:30:00Z")]
    pub timestamp: OffsetDateTime,

    /// Description
    #[schema(example = "Bought 100 SOL at $150.00")]
    pub description: String,

    /// Token involved (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<TokenInfo>,

    /// Amount involved
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "100.0")]
    pub amount: Option<TokenAmount>,

    /// Value in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "15000.00")]
    pub value_usd: Option<UsdAmount>,

    /// Transaction signature
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "5KJf7...")]
    pub transaction: Option<String>,

    /// Impact on portfolio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impact: Option<PortfolioImpact>,
}

/// Activity type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum ActivityType {
    /// Token purchase
    Buy,
    /// Token sale
    Sell,
    /// Token swap
    Swap,
    /// Token transfer in
    TransferIn,
    /// Token transfer out
    TransferOut,
    /// Staking
    Stake,
    /// Unstaking
    Unstake,
    /// Reward claim
    ClaimReward,
    /// OOF moment detected
    OofMoment,
    /// Position opened
    PositionOpened,
    /// Position closed
    PositionClosed,
}

/// Portfolio impact
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortfolioImpact {
    /// Change in total value
    #[schema(example = "1000.00")]
    pub value_change: UsdAmount,

    /// Change in diversification score
    #[schema(example = 0.05)]
    pub diversification_change: f64,

    /// Change in risk level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk_change: Option<RiskChange>,
}

/// Risk level change
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RiskChange {
    /// Previous risk level
    pub from: RiskLevel,

    /// New risk level
    pub to: RiskLevel,
}

/// OOF summary for a wallet
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OofSummary {
    /// OOF score (0-100, higher is worse)
    #[schema(example = 75.5, minimum = 0.0, maximum = 100.0)]
    pub oof_score: f64,

    /// OOF rank compared to other wallets
    #[schema(example = "Top 10%")]
    pub oof_rank: String,

    /// Total OOF moments
    #[schema(example = 15)]
    pub total_moments: u32,

    /// Total value lost to OOF moments
    #[schema(example = "50000.00")]
    pub total_value_lost: UsdAmount,

    /// Most common OOF moment type
    pub most_common_type: MomentKind,

    /// Recent OOF moments (last 30 days)
    #[schema(example = 3)]
    pub recent_moments: u32,

    /// Breakdown by moment type
    pub moments_by_type: HashMap<MomentKind, u32>,

    /// Breakdown by severity
    pub moments_by_severity: HashMap<MomentSeverity, u32>,

    /// Efficiency score (actual vs potential returns)
    #[schema(example = 33.3, minimum = 0.0, maximum = 100.0)]
    pub efficiency_score: f64,

    /// Improvement suggestions
    pub suggestions: Vec<ImprovementSuggestion>,
}

/// Improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImprovementSuggestion {
    /// Suggestion category
    pub category: SuggestionCategory,

    /// Priority level
    pub priority: Priority,

    /// Brief title
    #[schema(example = "Hold longer during volatility")]
    pub title: String,

    /// Detailed description
    #[schema(
        example = "You tend to sell during temporary dips. Consider holding positions longer during market volatility."
    )]
    pub description: String,

    /// Potential value saved if followed
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "15000.00")]
    pub potential_value_saved: Option<UsdAmount>,

    /// Relevant moment types this addresses
    pub addresses_moment_types: Vec<MomentKind>,
}

/// Suggestion category
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum SuggestionCategory {
    /// Timing-related suggestions
    Timing,
    /// Diversification suggestions
    Diversification,
    /// Risk management suggestions
    RiskManagement,
    /// Research and analysis suggestions
    Research,
    /// Emotional control suggestions
    Psychology,
    /// Portfolio optimization suggestions
    Optimization,
}

/// Priority level
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum Priority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Portfolio comparison request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CompareWalletsRequest {
    /// Primary wallet
    #[validate(custom = "validate_pubkey")]
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet_a: String,

    /// Comparison wallet
    #[validate(custom = "validate_pubkey")]
    #[schema(example = "22222222222222222222222222222223")]
    pub wallet_b: String,

    /// Comparison timeframe in days
    #[validate(range(min = 1, max = 730))]
    #[schema(example = 365, minimum = 1, maximum = 730)]
    pub timeframe_days: Option<u32>,

    /// Include detailed comparison
    #[schema(example = true)]
    pub include_details: Option<bool>,
}

/// Portfolio comparison response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompareWalletsResponse {
    /// Wallet A summary
    pub wallet_a: WalletComparisonSummary,

    /// Wallet B summary
    pub wallet_b: WalletComparisonSummary,

    /// Head-to-head comparison
    pub comparison: ComparisonMetrics,

    /// Winner analysis
    pub winner_analysis: WinnerAnalysis,

    /// Detailed breakdown (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detailed_comparison: Option<DetailedComparison>,
}

/// Wallet summary for comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletComparisonSummary {
    /// Wallet address
    pub wallet: String,

    /// Portfolio value
    #[schema(example = "75000.00")]
    pub portfolio_value: UsdAmount,

    /// Total return percentage
    #[schema(example = 150.0)]
    pub total_return_pct: f64,

    /// OOF score
    #[schema(example = 65.5)]
    pub oof_score: f64,

    /// Total OOF moments
    #[schema(example = 12)]
    pub oof_moments: u32,

    /// Value lost to OOF moments
    #[schema(example = "25000.00")]
    pub value_lost: UsdAmount,

    /// Efficiency score
    #[schema(example = 45.2)]
    pub efficiency_score: f64,
}

/// Comparison metrics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComparisonMetrics {
    /// Performance comparison
    pub performance: PerformanceComparison,

    /// OOF moment comparison
    pub oof_comparison: OofComparison,

    /// Risk comparison
    pub risk_comparison: RiskComparison,

    /// Strategy comparison
    pub strategy_comparison: StrategyComparison,
}

/// Performance comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceComparison {
    /// Return difference
    #[schema(example = 25.5)]
    pub return_difference_pct: f64,

    /// Value difference
    #[schema(example = "15000.00")]
    pub value_difference: UsdAmount,

    /// Winner
    #[schema(example = "wallet_a")]
    pub winner: String,

    /// Performance gap explanation
    #[schema(example = "Wallet A had better timing on major trades")]
    pub explanation: String,
}

/// OOF comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OofComparison {
    /// OOF score difference
    #[schema(example = -15.5)]
    pub score_difference: f64,

    /// Moments count difference
    #[schema(example = -5)]
    pub moments_difference: i32,

    /// Value lost difference
    #[schema(example = "-10000.00")]
    pub value_lost_difference: UsdAmount,

    /// Better wallet (lower OOF score)
    #[schema(example = "wallet_b")]
    pub better_wallet: String,

    /// Key differences in OOF patterns
    pub key_differences: Vec<String>,
}

/// Risk comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RiskComparison {
    /// Diversification comparison
    #[schema(example = 0.15)]
    pub diversification_difference: f64,

    /// Volatility comparison
    #[schema(example = -0.25)]
    pub volatility_difference: f64,

    /// More conservative wallet
    #[schema(example = "wallet_b")]
    pub more_conservative: String,

    /// Risk analysis
    #[schema(example = "Wallet B shows better risk management")]
    pub analysis: String,
}

/// Strategy comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StrategyComparison {
    /// Trading frequency comparison
    #[schema(example = "Wallet A trades 3x more frequently")]
    pub trading_frequency: String,

    /// Position holding time comparison
    #[schema(example = "Wallet B holds positions 2x longer on average")]
    pub holding_periods: String,

    /// Token preferences comparison
    pub token_preferences: Vec<String>,

    /// Strategy insights
    pub insights: Vec<String>,
}

/// Winner analysis
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WinnerAnalysis {
    /// Overall winner
    #[schema(example = "wallet_a")]
    pub overall_winner: String,

    /// Winning margin
    #[schema(example = "25% better returns")]
    pub winning_margin: String,

    /// Key success factors
    pub success_factors: Vec<String>,

    /// What the losing wallet could learn
    pub lessons_for_loser: Vec<String>,

    /// Confidence in analysis
    #[schema(example = 0.85, minimum = 0.0, maximum = 1.0)]
    pub confidence: f64,
}

/// Detailed comparison breakdown
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DetailedComparison {
    /// Month-by-month performance
    pub monthly_performance: Vec<MonthlyComparison>,

    /// Token-by-token comparison
    pub token_comparison: HashMap<String, TokenComparison>,

    /// OOF moment timeline
    pub oof_timeline: Vec<OofTimelineItem>,

    /// Statistical significance
    pub statistical_analysis: StatisticalAnalysis,
}

/// Monthly comparison data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MonthlyComparison {
    /// Month
    #[schema(example = "2024-01")]
    pub month: String,

    /// Wallet A return
    #[schema(example = 15.5)]
    pub wallet_a_return: f64,

    /// Wallet B return
    #[schema(example = 12.3)]
    pub wallet_b_return: f64,

    /// Difference
    #[schema(example = 3.2)]
    pub difference: f64,

    /// Winner
    #[schema(example = "wallet_a")]
    pub winner: String,
}

/// Token comparison
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenComparison {
    /// Token symbol
    pub symbol: String,

    /// Wallet A performance
    #[schema(example = 45.5)]
    pub wallet_a_performance: f64,

    /// Wallet B performance
    #[schema(example = 38.2)]
    pub wallet_b_performance: f64,

    /// Better performer
    #[schema(example = "wallet_a")]
    pub better_performer: String,

    /// Key differences
    pub differences: Vec<String>,
}

/// OOF timeline item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OofTimelineItem {
    /// Date
    #[schema(example = "2024-03-15T14:30:00Z")]
    pub date: OffsetDateTime,

    /// Which wallet had the OOF moment
    #[schema(example = "wallet_a")]
    pub wallet: String,

    /// Moment type
    pub moment_type: MomentKind,

    /// Value lost
    #[schema(example = "5000.00")]
    pub value_lost: UsdAmount,

    /// Brief description
    #[schema(example = "Sold SOL too early")]
    pub description: String,
}

/// Statistical analysis
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatisticalAnalysis {
    /// Correlation coefficient
    #[schema(example = 0.65)]
    pub correlation: f64,

    /// Statistical significance p-value
    #[schema(example = 0.025)]
    pub p_value: f64,

    /// Confidence interval
    pub confidence_interval: (f64, f64),

    /// Sample size (days compared)
    #[schema(example = 365)]
    pub sample_size: u32,

    /// Analysis reliability
    #[schema(example = "High")]
    pub reliability: String,
}

impl Default for GetWalletRequest {
    fn default() -> Self {
        Self {
            wallet: String::new(),
            include_positions: Some(true),
            include_metrics: Some(true),
            include_activity: Some(true),
            include_oof_summary: Some(true),
        }
    }
}

impl Default for CompareWalletsRequest {
    fn default() -> Self {
        Self {
            wallet_a: String::new(),
            wallet_b: String::new(),
            timeframe_days: Some(365),
            include_details: Some(false),
        }
    }
}
