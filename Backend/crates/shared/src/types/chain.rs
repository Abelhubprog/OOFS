use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use ulid::Ulid;

/// Represents a normalized chain event extracted from transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEvent {
    pub id: String,
    pub signature: String,
    pub log_idx: i32,
    pub slot: i64,
    pub timestamp: OffsetDateTime,
    pub wallet: String,
    pub mint: Option<String>,
    pub program_id: String,
    pub kind: EventKind,
    pub amount: Option<Decimal>,
    pub price_usd: Option<Decimal>,
    pub route: Option<String>,
    pub metadata: serde_json::Value,
}

/// Types of chain events we track
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// Token purchase/buy
    Buy,
    /// Token sale/sell
    Sell,
    /// Token swap (DEX)
    Swap,
    /// Token transfer between wallets
    Transfer,
    /// Token mint operation
    Mint,
    /// Token burn operation
    Burn,
    /// Liquidity pool operations
    LpAdd,
    LpRemove,
    /// Anchor program events
    AnchorEvent,
    /// Generic transaction marker
    Transaction,
}

impl EventKind {
    /// Returns true if this event type affects position calculations
    pub fn affects_position(&self) -> bool {
        matches!(
            self,
            EventKind::Buy | EventKind::Sell | EventKind::Swap | EventKind::Transfer
        )
    }

    /// Returns true if this is an entry event (increases position)
    pub fn is_entry(&self) -> bool {
        matches!(self, EventKind::Buy)
    }

    /// Returns true if this is an exit event (decreases position)
    pub fn is_exit(&self) -> bool {
        matches!(self, EventKind::Sell)
    }
}

/// Transaction raw data pointer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxRaw {
    pub signature: String,
    pub slot: i64,
    pub timestamp: OffsetDateTime,
    pub status: String,
    pub object_key: String,
    pub size_bytes: i32,
}

/// Participant in a transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub signature: String,
    pub wallet: String,
}

/// Action extracted from transaction logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub signature: String,
    pub log_idx: i32,
    pub slot: i64,
    pub timestamp: OffsetDateTime,
    pub program_id: String,
    pub kind: String,
    pub mint: Option<String>,
    pub amount_dec: Option<Decimal>,
    pub exec_px_usd_dec: Option<Decimal>,
    pub route: Option<String>,
    pub flags_json: serde_json::Value,
}

impl Action {
    /// Create a new action with ULID
    pub fn new(
        signature: String,
        log_idx: i32,
        slot: i64,
        timestamp: OffsetDateTime,
        program_id: String,
        kind: String,
    ) -> Self {
        Self {
            id: Ulid::new().to_string(),
            signature,
            log_idx,
            slot,
            timestamp,
            program_id,
            kind,
            mint: None,
            amount_dec: None,
            exec_px_usd_dec: None,
            route: None,
            flags_json: serde_json::json!({}),
        }
    }

    /// Convert to chain event for a specific wallet
    pub fn to_chain_event(&self, wallet: &str) -> ChainEvent {
        let kind = match self.kind.as_str() {
            "buy" => EventKind::Buy,
            "sell" => EventKind::Sell,
            "swap" => EventKind::Swap,
            "transfer" => EventKind::Transfer,
            "mint" => EventKind::Mint,
            "burn" => EventKind::Burn,
            "lp_add" => EventKind::LpAdd,
            "lp_remove" => EventKind::LpRemove,
            "anchor_event" => EventKind::AnchorEvent,
            _ => EventKind::Transaction,
        };

        ChainEvent {
            id: self.id.clone(),
            signature: self.signature.clone(),
            log_idx: self.log_idx,
            slot: self.slot,
            timestamp: self.timestamp,
            wallet: wallet.to_string(),
            mint: self.mint.clone(),
            program_id: self.program_id.clone(),
            kind,
            amount: self.amount_dec,
            price_usd: self.exec_px_usd_dec,
            route: self.route.clone(),
            metadata: self.flags_json.clone(),
        }
    }
}

/// Helper for transaction processing
#[derive(Debug, Clone)]
pub struct TxContext {
    pub signature: String,
    pub slot: i64,
    pub timestamp: OffsetDateTime,
    pub account_keys: Vec<String>,
    pub instructions: Vec<serde_json::Value>,
    pub token_transfers: Vec<serde_json::Value>,
}

impl TxContext {
    /// Extract all participants from the transaction
    pub fn extract_participants(&self) -> Vec<String> {
        let mut participants = self.account_keys.clone();

        // Add any additional participants from token transfers
        for transfer in &self.token_transfers {
            if let Some(from) = transfer.get("fromUserAccount").and_then(|v| v.as_str()) {
                if !participants.contains(&from.to_string()) {
                    participants.push(from.to_string());
                }
            }
            if let Some(to) = transfer.get("toUserAccount").and_then(|v| v.as_str()) {
                if !participants.contains(&to.to_string()) {
                    participants.push(to.to_string());
                }
            }
        }

        participants
    }

    /// Extract actions from token transfers
    pub fn extract_actions(&self) -> Vec<Action> {
        let mut actions = Vec::new();

        // Process token transfers
        for (idx, transfer) in self.token_transfers.iter().enumerate() {
            let mut action = Action::new(
                self.signature.clone(),
                idx as i32,
                self.slot,
                self.timestamp,
                String::new(), // Will be determined by transfer type
                "transfer".to_string(),
            );

            if let Some(mint) = transfer.get("mint").and_then(|v| v.as_str()) {
                action.mint = Some(mint.to_string());
            }

            if let Some(amount_str) = transfer.get("tokenAmount").and_then(|v| v.as_str()) {
                action.amount_dec = Decimal::from_str_exact(amount_str).ok();
            }

            let from_user = transfer.get("fromUserAccount").and_then(|v| v.as_str());
            let to_user = transfer.get("toUserAccount").and_then(|v| v.as_str());

            action.flags_json = serde_json::json!({
                "from": from_user,
                "to": to_user,
            });

            actions.push(action);
        }

        // If no token transfers, create a minimal transaction marker
        if actions.is_empty() {
            actions.push(Action::new(
                self.signature.clone(),
                0,
                self.slot,
                self.timestamp,
                String::new(),
                "tx".to_string(),
            ));
        }

        actions
    }
}
