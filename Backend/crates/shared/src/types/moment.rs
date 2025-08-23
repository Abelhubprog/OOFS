use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use ulid::Ulid;

/// Represents an OOF moment detected by the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Moment {
    pub id: String,
    pub wallet: String,
    pub mint: Option<String>,
    pub kind: MomentKind,
    pub t_event: OffsetDateTime,
    pub window: Option<Duration>,
    pub pct_dec: Option<Decimal>,
    pub missed_usd_dec: Option<Decimal>,
    pub severity_dec: Option<Decimal>,
    pub sig_ref: Option<String>,
    pub slot_ref: Option<i64>,
    pub version: String,
    pub explain_json: serde_json::Value,
    pub preview_png_url: Option<String>,
    pub nft_minted: bool,
    pub nft_mint: Option<String>,
}

/// Types of OOF moments
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MomentKind {
    #[serde(rename = "S2E")]
    SoldTooEarly,
    #[serde(rename = "BHD")]
    BagHolderDrawdown,
    #[serde(rename = "BAD_ROUTE")]
    BadRoute,
    #[serde(rename = "IDLE_YIELD")]
    IdleYield,
    #[serde(rename = "RUG")]
    Rug,
}

impl MomentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MomentKind::SoldTooEarly => "S2E",
            MomentKind::BagHolderDrawdown => "BHD",
            MomentKind::BadRoute => "BAD_ROUTE",
            MomentKind::IdleYield => "IDLE_YIELD",
            MomentKind::Rug => "RUG",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            MomentKind::SoldTooEarly => "Sold Too Early",
            MomentKind::BagHolderDrawdown => "Bag Holder Drawdown",
            MomentKind::BadRoute => "Bad Route",
            MomentKind::IdleYield => "Idle Yield",
            MomentKind::Rug => "Rug Pull",
        }
    }
}

impl std::str::FromStr for MomentKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "S2E" => Ok(MomentKind::SoldTooEarly),
            "BHD" => Ok(MomentKind::BagHolderDrawdown),
            "BAD_ROUTE" => Ok(MomentKind::BadRoute),
            "IDLE_YIELD" => Ok(MomentKind::IdleYield),
            "RUG" => Ok(MomentKind::Rug),
            _ => Err(format!("Invalid moment kind: {}", s)),
        }
    }
}

impl Moment {
    /// Create a new moment with generated ID
    pub fn new(
        wallet: String,
        mint: Option<String>,
        kind: MomentKind,
        t_event: OffsetDateTime,
    ) -> Self {
        Self {
            id: Ulid::new().to_string(),
            wallet,
            mint,
            kind,
            t_event,
            window: None,
            pct_dec: None,
            missed_usd_dec: None,
            severity_dec: None,
            sig_ref: None,
            slot_ref: None,
            version: "1".to_string(),
            explain_json: serde_json::json!({}),
            preview_png_url: None,
            nft_minted: false,
            nft_mint: None,
        }
    }

    /// Calculate severity score (0.0 to 1.0)
    pub fn calculate_severity(&mut self) {
        use crate::constants::severity::*;

        self.severity_dec = match self.kind {
            MomentKind::SoldTooEarly => self.pct_dec.map(s2e_severity),
            MomentKind::BagHolderDrawdown => self.pct_dec.map(bhd_severity),
            MomentKind::BadRoute => self.pct_dec.map(bad_route_severity),
            MomentKind::IdleYield | MomentKind::Rug => {
                // For these types, severity is based on USD amount
                self.missed_usd_dec.map(|usd| {
                    let max_usd = Decimal::from_str_exact("10000").unwrap(); // $10k = max severity
                    (usd / max_usd).clamp(Decimal::ZERO, Decimal::ONE)
                })
            }
        };
    }

    /// Get the card image URL for this moment
    pub fn get_card_url(&self, cdn_base: &str) -> String {
        if let Some(url) = &self.preview_png_url {
            url.clone()
        } else {
            format!("{}/cards/moments/{}.png", cdn_base, self.id)
        }
    }
}

/// Wallet extremes summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletExtremes {
    pub wallet: String,
    pub computed_at: OffsetDateTime,
    pub highest_win: Option<ExtremeEntry>,
    pub highest_loss: Option<ExtremeEntry>,
    pub top_s2e: Option<ExtremeEntry>,
    pub worst_bhd: Option<ExtremeEntry>,
    pub worst_bad_route: Option<ExtremeEntry>,
    pub largest_idle: Option<ExtremeEntry>,
    pub worst_rug: Option<ExtremeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtremeEntry {
    pub id: String,
    pub mint: Option<String>,
    pub value_usd: Decimal,
    pub value_pct: Option<Decimal>,
    pub timestamp: OffsetDateTime,
    pub card_url: String,
}

impl WalletExtremes {
    pub fn new(wallet: String) -> Self {
        Self {
            wallet,
            computed_at: OffsetDateTime::now_utc(),
            highest_win: None,
            highest_loss: None,
            top_s2e: None,
            worst_bhd: None,
            worst_bad_route: None,
            largest_idle: None,
            worst_rug: None,
        }
    }
}

/// Moment context for detection
#[derive(Debug, Clone)]
pub struct MomentContext {
    pub detector_version: u8,
    pub price_source: String,
    pub confidence: String,
    pub window_start: OffsetDateTime,
    pub window_end: OffsetDateTime,
}

/// S2E specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S2EData {
    pub exit_price: Decimal,
    pub peak_price: Decimal,
    pub peak_timestamp: OffsetDateTime,
    pub qty_sold: Decimal,
    pub missed_gains_usd: Decimal,
    pub missed_gains_pct: Decimal,
}

/// BHD specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BHDData {
    pub entry_price: Decimal,
    pub trough_price: Decimal,
    pub trough_timestamp: OffsetDateTime,
    pub qty_bought: Decimal,
    pub drawdown_pct: Decimal,
    pub unrealized_loss_usd: Decimal,
}

/// Bad Route specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadRouteData {
    pub executed_price: Decimal,
    pub best_available_price: Decimal,
    pub price_difference_pct: Decimal,
    pub amount_lost_usd: Decimal,
    pub route_used: String,
    pub best_route: String,
}

/// Idle Yield specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdleYieldData {
    pub balance: Decimal,
    pub idle_days: i64,
    pub apr_rate: Decimal,
    pub missed_yield_usd: Decimal,
    pub avg_token_price: Decimal,
}

/// Rug specific data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RugData {
    pub entry_price: Decimal,
    pub rug_price: Decimal,
    pub ath_price: Decimal,
    pub holding_amount: Decimal,
    pub loss_usd: Decimal,
    pub loss_pct: Decimal,
}

#[cfg(test)]
mod moment_tests;
