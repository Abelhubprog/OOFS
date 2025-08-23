use serde::{Deserialize, Serialize};
use shared::{
    types::{
        chain::{Action, ChainEvent},
        common::{SolAmount, TokenAmount, UsdAmount},
        moment::{Moment, MomentContext, MomentKind, MomentMetadata, MomentSeverity},
    },
    validation::validate_pubkey,
};
use std::collections::HashMap;
use time::OffsetDateTime;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

/// Request to list moments for a wallet
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MomentsListRequest {
    /// Wallet public key
    #[validate(custom = "validate_pubkey")]
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Filter by moment types
    #[schema(example = "[\"SoldTooEarly\", \"BagHolderDrawdown\"]")]
    pub moment_types: Option<Vec<MomentKind>>,

    /// Filter by minimum severity
    #[schema(example = "Medium")]
    pub min_severity: Option<MomentSeverity>,

    /// Filter by maximum severity
    #[schema(example = "High")]
    pub max_severity: Option<MomentSeverity>,

    /// Filter by start date
    #[schema(example = "2024-01-01T00:00:00Z")]
    pub start_date: Option<OffsetDateTime>,

    /// Filter by end date
    #[schema(example = "2024-12-31T23:59:59Z")]
    pub end_date: Option<OffsetDateTime>,

    /// Filter by minimum value lost
    #[schema(example = "1000.00")]
    pub min_value_lost: Option<UsdAmount>,

    /// Filter by token symbol
    #[schema(example = "SOL")]
    pub token_symbol: Option<String>,

    /// Sort field
    #[schema(example = "value_lost")]
    pub sort_by: Option<MomentSortField>,

    /// Sort direction
    #[schema(example = "desc")]
    pub sort_order: Option<SortOrder>,

    /// Pagination offset
    #[validate(range(min = 0))]
    #[schema(example = 0, minimum = 0)]
    pub offset: Option<u32>,

    /// Number of results to return
    #[validate(range(min = 1, max = 1000))]
    #[schema(example = 50, minimum = 1, maximum = 1000)]
    pub limit: Option<u32>,

    /// Include detailed context
    #[schema(example = true)]
    pub include_context: Option<bool>,

    /// Include related transactions
    #[schema(example = true)]
    pub include_transactions: Option<bool>,
}

/// Response containing list of moments
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentsListResponse {
    /// Wallet address
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// List of moments
    pub moments: Vec<MomentDetail>,

    /// Pagination information
    pub pagination: PaginationInfo,

    /// Summary statistics
    pub summary: MomentsSummary,

    /// Applied filters
    pub filters: AppliedFilters,
}

/// Detailed moment information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentDetail {
    /// Unique moment identifier
    #[schema(example = "moment_123456")]
    pub id: String,

    /// Core moment data
    #[serde(flatten)]
    pub moment: Moment,

    /// Additional context (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<DetailedContext>,

    /// Related transactions (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TransactionInfo>>,

    /// Shareable card URL
    #[schema(example = "https://api.oof.com/cards/moment_123456.png")]
    pub card_url: String,

    /// Moment analysis
    pub analysis: MomentAnalysis,
}

/// Detailed context for a moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DetailedContext {
    /// Position before the moment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_before: Option<PositionInfo>,

    /// Position after the moment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_after: Option<PositionInfo>,

    /// Market conditions at the time
    pub market_conditions: MarketConditions,

    /// Portfolio context
    pub portfolio_context: PortfolioContext,

    /// What should have been done differently
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alternative_action: Option<AlternativeAction>,
}

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PositionInfo {
    /// Token amount held
    #[schema(example = "100.0")]
    pub amount: TokenAmount,

    /// Average cost basis
    #[schema(example = "50.00")]
    pub avg_cost: UsdAmount,

    /// Current market price
    #[schema(example = "75.00")]
    pub market_price: UsdAmount,

    /// Total value
    #[schema(example = "7500.00")]
    pub total_value: UsdAmount,

    /// Unrealized P&L
    #[schema(example = "2500.00")]
    pub unrealized_pnl: UsdAmount,

    /// P&L percentage
    #[schema(example = 50.0)]
    pub pnl_pct: f64,
}

/// Market conditions at moment time
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MarketConditions {
    /// Token price at moment
    #[schema(example = "75.00")]
    pub price_at_moment: UsdAmount,

    /// Price 1 hour later
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "78.50")]
    pub price_1h_later: Option<UsdAmount>,

    /// Price 24 hours later
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "90.00")]
    pub price_24h_later: Option<UsdAmount>,

    /// Price 7 days later
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "150.00")]
    pub price_7d_later: Option<UsdAmount>,

    /// 24h volume at moment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "50000000.00")]
    pub volume_24h: Option<UsdAmount>,

    /// Market cap at moment
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "1000000000.00")]
    pub market_cap: Option<UsdAmount>,

    /// Volatility indicator
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 0.15)]
    pub volatility: Option<f64>,
}

/// Portfolio context
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PortfolioContext {
    /// Total portfolio value at moment
    #[schema(example = "50000.00")]
    pub total_value: UsdAmount,

    /// Percentage of portfolio this represented
    #[schema(example = 15.0)]
    pub portfolio_pct: f64,

    /// Number of other positions held
    #[schema(example = 12)]
    pub other_positions: u32,

    /// Diversification score (0-1)
    #[schema(example = 0.7)]
    pub diversification: f64,
}

/// Alternative action that could have been taken
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AlternativeAction {
    /// Recommended action
    #[schema(example = "Hold for 30 more days")]
    pub action: String,

    /// Potential additional value
    #[schema(example = "15000.00")]
    pub potential_value: UsdAmount,

    /// Confidence level (0-1)
    #[schema(example = 0.85)]
    pub confidence: f64,

    /// Reasoning
    #[schema(example = "Strong technical indicators suggested continued uptrend")]
    pub reasoning: String,
}

/// Transaction information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionInfo {
    /// Transaction signature
    #[schema(example = "5KJf7...")]
    pub signature: String,

    /// Block timestamp
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: OffsetDateTime,

    /// Transaction action
    pub action: Action,

    /// Amount involved
    #[schema(example = "100.0")]
    pub amount: TokenAmount,

    /// Price at transaction
    #[schema(example = "50.00")]
    pub price: UsdAmount,

    /// Total value
    #[schema(example = "5000.00")]
    pub value: UsdAmount,

    /// Gas fee
    #[schema(example = "0.000005")]
    pub fee: SolAmount,

    /// Role in the moment (trigger, related, etc.)
    #[schema(example = "trigger")]
    pub role: TransactionRole,
}

/// Role of transaction in the moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TransactionRole {
    /// Transaction that triggered the moment
    Trigger,
    /// Related transaction (e.g., previous buy)
    Related,
    /// Follow-up transaction
    FollowUp,
}

/// Moment analysis details
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentAnalysis {
    /// What went wrong
    #[schema(example = "Sold during temporary dip instead of holding through volatility")]
    pub what_happened: String,

    /// Why it was an OOF moment
    #[schema(example = "Price recovered 200% within 30 days of sale")]
    pub why_oof: String,

    /// Key factors that contributed
    pub contributing_factors: Vec<String>,

    /// Lessons learned
    pub lessons: Vec<String>,

    /// Emotional impact score (0-10)
    #[schema(example = 8.5, minimum = 0.0, maximum = 10.0)]
    pub emotional_impact: f64,

    /// Similar moments in crypto history
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub historical_parallels: Vec<HistoricalParallel>,
}

/// Historical parallel to this moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalParallel {
    /// Token/event name
    #[schema(example = "Bitcoin 2017 dip")]
    pub name: String,

    /// Date of parallel event
    #[schema(example = "2017-09-15")]
    pub date: String,

    /// Brief description
    #[schema(example = "Similar pattern of panic selling before massive recovery")]
    pub description: String,

    /// Outcome
    #[schema(example = "Recovered 500% over next 3 months")]
    pub outcome: String,
}

/// Pagination information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationInfo {
    /// Current offset
    #[schema(example = 0)]
    pub offset: u32,

    /// Number of results in this page
    #[schema(example = 25)]
    pub limit: u32,

    /// Total number of moments
    #[schema(example = 150)]
    pub total: u32,

    /// Whether there are more results
    #[schema(example = true)]
    pub has_more: bool,

    /// Next page offset
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 25)]
    pub next_offset: Option<u32>,
}

/// Summary of moments in the list
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentsSummary {
    /// Total number of moments (across all pages)
    #[schema(example = 150)]
    pub total_moments: u32,

    /// Breakdown by type
    pub by_type: HashMap<MomentKind, u32>,

    /// Breakdown by severity
    pub by_severity: HashMap<MomentSeverity, u32>,

    /// Total value lost across all moments
    #[schema(example = "125000.00")]
    pub total_value_lost: UsdAmount,

    /// Average value lost per moment
    #[schema(example = "833.33")]
    pub avg_value_lost: UsdAmount,

    /// Most common moment type
    pub most_common_type: MomentKind,

    /// Date range of moments
    pub date_range: DateRange,
}

/// Date range information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateRange {
    /// Earliest moment date
    #[schema(example = "2023-01-15T10:30:00Z")]
    pub earliest: OffsetDateTime,

    /// Latest moment date
    #[schema(example = "2024-08-20T15:45:00Z")]
    pub latest: OffsetDateTime,

    /// Number of days spanned
    #[schema(example = 583)]
    pub days_spanned: u32,
}

/// Applied filters information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AppliedFilters {
    /// Moment types filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moment_types: Option<Vec<MomentKind>>,

    /// Severity filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity_range: Option<(MomentSeverity, MomentSeverity)>,

    /// Date range filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range: Option<(OffsetDateTime, OffsetDateTime)>,

    /// Value filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value_lost: Option<UsdAmount>,

    /// Token filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_symbol: Option<String>,

    /// Sort applied
    pub sort: SortInfo,
}

/// Sort information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SortInfo {
    /// Field being sorted by
    pub field: MomentSortField,

    /// Sort direction
    pub order: SortOrder,
}

/// Available sort fields for moments
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum MomentSortField {
    /// Sort by timestamp
    #[serde(rename = "timestamp")]
    Timestamp,
    /// Sort by value lost
    #[serde(rename = "value_lost")]
    ValueLost,
    /// Sort by severity
    #[serde(rename = "severity")]
    Severity,
    /// Sort by moment type
    #[serde(rename = "type")]
    Type,
    /// Sort by token symbol
    #[serde(rename = "token")]
    Token,
    /// Sort by percentage lost
    #[serde(rename = "percentage_lost")]
    PercentageLost,
}

/// Sort order
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum SortOrder {
    /// Ascending order
    #[serde(rename = "asc")]
    Ascending,
    /// Descending order
    #[serde(rename = "desc")]
    Descending,
}

/// Request to get a specific moment by ID
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct GetMomentRequest {
    /// Moment ID
    #[schema(example = "moment_123456")]
    pub moment_id: String,

    /// Include detailed context
    #[schema(example = true)]
    pub include_context: Option<bool>,

    /// Include related transactions
    #[schema(example = true)]
    pub include_transactions: Option<bool>,

    /// Include similar moments
    #[schema(example = false)]
    pub include_similar: Option<bool>,
}

/// Response for a specific moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetMomentResponse {
    /// Detailed moment information
    pub moment: MomentDetail,

    /// Similar moments (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similar_moments: Option<Vec<SimilarMoment>>,

    /// Social sharing options
    pub sharing: SharingOptions,
}

/// Similar moment reference
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SimilarMoment {
    /// Moment ID
    #[schema(example = "moment_789012")]
    pub id: String,

    /// Brief description
    #[schema(example = "Similar SOL sale timing mistake")]
    pub description: String,

    /// Similarity score (0-1)
    #[schema(example = 0.85)]
    pub similarity: f64,

    /// Value lost
    #[schema(example = "8500.00")]
    pub value_lost: UsdAmount,

    /// When it occurred
    #[schema(example = "2024-02-10T14:20:00Z")]
    pub timestamp: OffsetDateTime,
}

/// Social sharing options
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SharingOptions {
    /// Twitter share URL
    #[schema(example = "https://twitter.com/intent/tweet?text=...")]
    pub twitter_url: String,

    /// Discord share URL
    #[schema(example = "https://discord.com/channels/.../...")]
    pub discord_url: String,

    /// Direct card image URL
    #[schema(example = "https://api.oof.com/cards/moment_123456.png")]
    pub card_image_url: String,

    /// Shareable web URL
    #[schema(example = "https://oof.com/moments/moment_123456")]
    pub web_url: String,
}

/// Moment statistics request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct MomentStatsRequest {
    /// Wallet public key (optional, for wallet-specific stats)
    #[validate(custom = "validate_pubkey_optional")]
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: Option<String>,

    /// Time period for statistics
    #[schema(example = "30d")]
    pub period: Option<String>,

    /// Group statistics by this field
    #[schema(example = "moment_type")]
    pub group_by: Option<StatsGroupBy>,
}

/// Statistics grouping options
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum StatsGroupBy {
    /// Group by moment type
    #[serde(rename = "moment_type")]
    MomentType,
    /// Group by severity
    #[serde(rename = "severity")]
    Severity,
    /// Group by time period
    #[serde(rename = "time_period")]
    TimePeriod,
    /// Group by token
    #[serde(rename = "token")]
    Token,
}

/// Moment statistics response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentStatsResponse {
    /// Overall statistics
    pub overall: OverallStats,

    /// Grouped statistics (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub groups: Option<HashMap<String, GroupStats>>,

    /// Trending moments
    pub trending: Vec<TrendingMoment>,

    /// Statistics period
    pub period: StatsPeriod,
}

/// Overall moment statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct OverallStats {
    /// Total moments
    #[schema(example = 50000)]
    pub total_moments: u64,

    /// Total value lost across all moments
    #[schema(example = "10000000.00")]
    pub total_value_lost: UsdAmount,

    /// Average value lost per moment
    #[schema(example = "200.00")]
    pub avg_value_lost: UsdAmount,

    /// Most common moment type
    pub most_common_type: MomentKind,

    /// Growth compared to previous period
    #[schema(example = 15.5)]
    pub growth_pct: f64,
}

/// Group statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GroupStats {
    /// Number of moments in this group
    #[schema(example = 150)]
    pub count: u32,

    /// Total value lost in this group
    #[schema(example = "75000.00")]
    pub total_value_lost: UsdAmount,

    /// Average value lost in this group
    #[schema(example = "500.00")]
    pub avg_value_lost: UsdAmount,

    /// Percentage of total moments
    #[schema(example = 25.5)]
    pub percentage: f64,
}

/// Trending moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrendingMoment {
    /// Moment type
    pub moment_type: MomentKind,

    /// How many times this type occurred recently
    #[schema(example = 45)]
    pub recent_count: u32,

    /// Growth compared to previous period
    #[schema(example = 125.5)]
    pub growth_pct: f64,

    /// Average value lost for this type
    #[schema(example = "850.00")]
    pub avg_value_lost: UsdAmount,
}

/// Statistics time period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsPeriod {
    /// Start of period
    #[schema(example = "2024-07-21T00:00:00Z")]
    pub start: OffsetDateTime,

    /// End of period
    #[schema(example = "2024-08-21T00:00:00Z")]
    pub end: OffsetDateTime,

    /// Period description
    #[schema(example = "Last 30 days")]
    pub description: String,
}

/// Validation function for optional pubkey
fn validate_pubkey_optional(pubkey: &Option<String>) -> Result<(), ValidationError> {
    if let Some(key) = pubkey {
        validate_pubkey(key)?;
    }
    Ok(())
}

impl Default for MomentsListRequest {
    fn default() -> Self {
        Self {
            wallet: String::new(),
            moment_types: None,
            min_severity: None,
            max_severity: None,
            start_date: None,
            end_date: None,
            min_value_lost: None,
            token_symbol: None,
            sort_by: Some(MomentSortField::Timestamp),
            sort_order: Some(SortOrder::Descending),
            offset: Some(0),
            limit: Some(50),
            include_context: Some(false),
            include_transactions: Some(false),
        }
    }
}

impl Default for GetMomentRequest {
    fn default() -> Self {
        Self {
            moment_id: String::new(),
            include_context: Some(true),
            include_transactions: Some(true),
            include_similar: Some(false),
        }
    }
}

impl Default for MomentSortField {
    fn default() -> Self {
        Self::Timestamp
    }
}

impl Default for SortOrder {
    fn default() -> Self {
        Self::Descending
    }
}
