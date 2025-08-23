use serde::{Deserialize, Serialize};
use shared::{
    types::{
        chain::{Action, ChainEvent},
        common::{SolAmount, TokenAmount, UsdAmount},
        moment::{Moment, MomentKind, MomentSeverity},
        wallet::{PerformanceMetrics, PositionSnapshot, WalletAnalysis},
    },
    validation::{validate_pubkey, validate_timeframe},
};
use std::collections::HashMap;
use time::OffsetDateTime;
use utoipa::ToSchema;
use validator::{Validate, ValidationError};

/// Request to analyze a wallet for OOF moments
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct AnalyzeRequest {
    /// Wallet public key to analyze
    #[validate(custom = "validate_pubkey")]
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Analysis timeframe in days (default: 365, max: 730)
    #[validate(range(min = 1, max = 730))]
    #[schema(example = 365, minimum = 1, maximum = 730)]
    pub timeframe_days: Option<u32>,

    /// Specific moment types to analyze (if empty, analyze all)
    #[schema(example = "[\"SoldTooEarly\", \"BagHolderDrawdown\"]")]
    pub moment_types: Option<Vec<MomentKind>>,

    /// Minimum severity threshold
    #[schema(example = "Medium")]
    pub min_severity: Option<MomentSeverity>,

    /// Include detailed transaction data
    #[schema(example = true)]
    pub include_transactions: Option<bool>,

    /// Include position snapshots
    #[schema(example = true)]
    pub include_positions: Option<bool>,

    /// Include performance metrics
    #[schema(example = true)]
    pub include_metrics: Option<bool>,

    /// Force refresh of cached data
    #[schema(example = false)]
    pub force_refresh: Option<bool>,

    /// Cursor for pagination (resume from previous analysis)
    #[schema(example = "eyJsYXN0X3RpbWVzdGFtcCI6MTY5MjA1NjAwMH0=")]
    pub cursor: Option<String>,

    /// Maximum number of moments to return
    #[validate(range(min = 1, max = 1000))]
    #[schema(example = 100, minimum = 1, maximum = 1000)]
    pub limit: Option<u32>,
}

/// Response from wallet analysis
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalyzeResponse {
    /// Wallet being analyzed
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Analysis summary
    pub summary: AnalysisSummary,

    /// Detected OOF moments
    pub moments: Vec<Moment>,

    /// Current position snapshots (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positions: Option<Vec<PositionSnapshot>>,

    /// Performance metrics (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metrics: Option<PerformanceMetrics>,

    /// Detailed transaction data (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<TransactionDetail>>,

    /// Analysis metadata
    pub metadata: AnalysisMetadata,

    /// Pagination cursor for next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// Summary of analysis results
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalysisSummary {
    /// Total number of OOF moments detected
    #[schema(example = 15)]
    pub total_moments: u32,

    /// Breakdown by moment type
    pub moments_by_type: HashMap<MomentKind, u32>,

    /// Breakdown by severity
    pub moments_by_severity: HashMap<MomentSeverity, u32>,

    /// Total potential value lost
    #[schema(example = "50000.00")]
    pub total_value_lost: UsdAmount,

    /// Most severe moment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worst_moment: Option<MomentSummary>,

    /// OOF score (0-100, higher is worse)
    #[schema(example = 75.5, minimum = 0.0, maximum = 100.0)]
    pub oof_score: f64,

    /// Analysis timeframe
    pub timeframe: TimeframeSummary,

    /// Performance overview
    pub performance: PerformanceOverview,
}

/// Brief summary of a moment
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MomentSummary {
    /// Moment type
    pub moment_type: MomentKind,

    /// Severity level
    pub severity: MomentSeverity,

    /// Value lost in USD
    #[schema(example = "15000.00")]
    pub value_lost: UsdAmount,

    /// Token involved
    #[schema(example = "SOL")]
    pub token_symbol: String,

    /// When it occurred
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: OffsetDateTime,

    /// Brief description
    #[schema(example = "Sold 100 SOL too early, missed 3x gains")]
    pub description: String,
}

/// Timeframe information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeframeSummary {
    /// Start of analysis period
    #[schema(example = "2023-01-01T00:00:00Z")]
    pub start_date: OffsetDateTime,

    /// End of analysis period
    #[schema(example = "2024-01-01T00:00:00Z")]
    pub end_date: OffsetDateTime,

    /// Number of days analyzed
    #[schema(example = 365)]
    pub days: u32,

    /// Number of transactions processed
    #[schema(example = 1250)]
    pub transaction_count: u32,

    /// Number of unique tokens traded
    #[schema(example = 45)]
    pub unique_tokens: u32,
}

/// Performance overview
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PerformanceOverview {
    /// Total portfolio value at start
    #[schema(example = "10000.00")]
    pub initial_value: UsdAmount,

    /// Current portfolio value
    #[schema(example = "25000.00")]
    pub current_value: UsdAmount,

    /// Total return percentage
    #[schema(example = 150.0)]
    pub total_return_pct: f64,

    /// Value that could have been gained without OOF moments
    #[schema(example = "75000.00")]
    pub potential_value: UsdAmount,

    /// Potential return percentage without OOF moments
    #[schema(example = 650.0)]
    pub potential_return_pct: f64,

    /// Efficiency score (actual vs potential)
    #[schema(example = 33.3, minimum = 0.0, maximum = 100.0)]
    pub efficiency_score: f64,
}

/// Detailed transaction information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TransactionDetail {
    /// Transaction signature
    #[schema(example = "3Bv7j...")]
    pub signature: String,

    /// Block timestamp
    #[schema(example = "2024-01-15T10:30:00Z")]
    pub timestamp: OffsetDateTime,

    /// Transaction type/action
    pub action: Action,

    /// Token involved
    #[schema(example = "SOL")]
    pub token_symbol: String,

    /// Token mint address
    #[schema(example = "So11111111111111111111111111111111111111112")]
    pub token_mint: String,

    /// Amount involved
    #[schema(example = "100.0")]
    pub amount: TokenAmount,

    /// Price at time of transaction
    #[schema(example = "50.00")]
    pub price_usd: UsdAmount,

    /// Value in USD at time of transaction
    #[schema(example = "5000.00")]
    pub value_usd: UsdAmount,

    /// Current value if still held
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = "15000.00")]
    pub current_value_usd: Option<UsdAmount>,

    /// Associated OOF moment ID (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moment_id: Option<String>,

    /// Gas fee paid
    #[schema(example = "0.000005")]
    pub fee_sol: SolAmount,

    /// Gas fee in USD
    #[schema(example = "0.25")]
    pub fee_usd: UsdAmount,
}

/// Analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalysisMetadata {
    /// When the analysis was performed
    #[schema(example = "2024-08-21T12:00:00Z")]
    pub analyzed_at: OffsetDateTime,

    /// How long the analysis took (in seconds)
    #[schema(example = 15.5)]
    pub duration_seconds: f64,

    /// Data freshness (last update timestamp)
    #[schema(example = "2024-08-21T11:45:00Z")]
    pub data_last_updated: OffsetDateTime,

    /// Whether data was cached
    #[schema(example = false)]
    pub from_cache: bool,

    /// Cache expiry (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_expires_at: Option<OffsetDateTime>,

    /// Analysis version/algorithm used
    #[schema(example = "v2.1.0")]
    pub version: String,

    /// Warning messages (if any)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

/// Streaming analysis progress update
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AnalysisProgress {
    /// Wallet being analyzed
    #[schema(example = "11111111111111111111111111111112")]
    pub wallet: String,

    /// Current progress stage
    pub stage: AnalysisStage,

    /// Progress percentage (0-100)
    #[schema(example = 45.5, minimum = 0.0, maximum = 100.0)]
    pub progress_pct: f64,

    /// Current stage progress (0-100)
    #[schema(example = 75.0, minimum = 0.0, maximum = 100.0)]
    pub stage_progress_pct: f64,

    /// Estimated time remaining (seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(example = 30)]
    pub eta_seconds: Option<u32>,

    /// Current processing message
    #[schema(example = "Analyzing swap transactions...")]
    pub message: String,

    /// Moments detected so far
    #[schema(example = 3)]
    pub moments_found: u32,

    /// Transactions processed so far
    #[schema(example = 500)]
    pub transactions_processed: u32,

    /// Total transactions to process
    #[schema(example = 1200)]
    pub total_transactions: u32,
}

/// Analysis processing stages
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum AnalysisStage {
    /// Initializing analysis
    Initializing,
    /// Fetching transaction history
    FetchingTransactions,
    /// Building position history
    BuildingPositions,
    /// Fetching price data
    FetchingPrices,
    /// Detecting moments
    DetectingMoments,
    /// Calculating metrics
    CalculatingMetrics,
    /// Finalizing results
    Finalizing,
    /// Analysis complete
    Complete,
    /// Analysis failed
    Failed,
}

/// Bulk analysis request for multiple wallets
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct BulkAnalyzeRequest {
    /// List of wallet addresses to analyze
    #[validate(length(min = 1, max = 10))]
    #[validate(custom = "validate_wallet_list")]
    pub wallets: Vec<String>,

    /// Shared analysis parameters
    #[serde(flatten)]
    pub params: AnalyzeRequestParams,

    /// Enable parallel processing
    #[schema(example = true)]
    pub parallel: Option<bool>,

    /// Batch ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
}

/// Shared analysis parameters (extracted from AnalyzeRequest)
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct AnalyzeRequestParams {
    /// Analysis timeframe in days
    #[validate(range(min = 1, max = 730))]
    pub timeframe_days: Option<u32>,

    /// Specific moment types to analyze
    pub moment_types: Option<Vec<MomentKind>>,

    /// Minimum severity threshold
    pub min_severity: Option<MomentSeverity>,

    /// Include detailed transaction data
    pub include_transactions: Option<bool>,

    /// Include position snapshots
    pub include_positions: Option<bool>,

    /// Include performance metrics
    pub include_metrics: Option<bool>,

    /// Force refresh of cached data
    pub force_refresh: Option<bool>,

    /// Maximum number of moments to return per wallet
    #[validate(range(min = 1, max = 1000))]
    pub limit: Option<u32>,
}

/// Bulk analysis response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkAnalyzeResponse {
    /// Batch ID
    #[schema(example = "batch_123456")]
    pub batch_id: String,

    /// Individual wallet results
    pub results: Vec<WalletAnalysisResult>,

    /// Batch summary
    pub summary: BulkAnalysisSummary,

    /// Batch metadata
    pub metadata: BulkAnalysisMetadata,
}

/// Individual wallet analysis result in bulk operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WalletAnalysisResult {
    /// Wallet address
    pub wallet: String,

    /// Analysis result (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis: Option<AnalyzeResponse>,

    /// Error (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Processing status
    pub status: AnalysisStatus,
}

/// Analysis processing status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum AnalysisStatus {
    /// Analysis completed successfully
    Success,
    /// Analysis failed
    Failed,
    /// Analysis is still in progress
    InProgress,
    /// Analysis was skipped
    Skipped,
}

/// Summary of bulk analysis
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkAnalysisSummary {
    /// Total wallets processed
    #[schema(example = 5)]
    pub total_wallets: u32,

    /// Successful analyses
    #[schema(example = 4)]
    pub successful: u32,

    /// Failed analyses
    #[schema(example = 1)]
    pub failed: u32,

    /// Total moments detected across all wallets
    #[schema(example = 75)]
    pub total_moments: u32,

    /// Total value lost across all wallets
    #[schema(example = "250000.00")]
    pub total_value_lost: UsdAmount,

    /// Average OOF score
    #[schema(example = 65.5)]
    pub average_oof_score: f64,
}

/// Bulk analysis metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BulkAnalysisMetadata {
    /// When the batch was started
    pub started_at: OffsetDateTime,

    /// When the batch was completed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<OffsetDateTime>,

    /// Total processing time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,

    /// Whether parallel processing was used
    pub parallel_processing: bool,

    /// API version used
    pub version: String,
}

/// Custom validation for wallet list
fn validate_wallet_list(wallets: &[String]) -> Result<(), ValidationError> {
    for wallet in wallets {
        validate_pubkey(wallet)?;
    }
    Ok(())
}

impl Default for AnalyzeRequest {
    fn default() -> Self {
        Self {
            wallet: String::new(),
            timeframe_days: Some(365),
            moment_types: None,
            min_severity: None,
            include_transactions: Some(false),
            include_positions: Some(true),
            include_metrics: Some(true),
            force_refresh: Some(false),
            cursor: None,
            limit: Some(100),
        }
    }
}

impl Default for AnalyzeRequestParams {
    fn default() -> Self {
        Self {
            timeframe_days: Some(365),
            moment_types: None,
            min_severity: None,
            include_transactions: Some(false),
            include_positions: Some(true),
            include_metrics: Some(true),
            force_refresh: Some(false),
            limit: Some(100),
        }
    }
}
