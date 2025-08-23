//! OOF Anchor SDK
//!
//! Comprehensive SDK for interacting with OOF Solana programs including:
//! - Campaign management and reward distribution
//! - Staking and governance mechanisms
//! - Moment registry and verification
//! - Instruction builders and transaction helpers

use anchor_client::{Client, Program};
use anchor_lang::prelude::*;
use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction,
    transaction::Transaction,
};
use std::{collections::HashMap, rc::Rc, str::FromStr};
use time::OffsetDateTime;
use tracing::{error, info, instrument};

pub mod campaigns;
pub mod moment_registry;
pub mod staking;

pub use campaigns::*;
pub use moment_registry::*;
pub use staking::*;

/// OOF program IDs (these would be actual deployed program addresses)
pub mod program_ids {
    use super::*;

    // Placeholder program IDs - in production these would be actual deployed addresses
    pub const CAMPAIGNS: &str = "CampaignProgram11111111111111111111111111111";
    pub const STAKING: &str = "StakingProgram111111111111111111111111111111";
    pub const MOMENT_REGISTRY: &str = "MomentRegistry111111111111111111111111111111";

    pub fn campaigns() -> Pubkey {
        Pubkey::from_str(CAMPAIGNS).unwrap()
    }

    pub fn staking() -> Pubkey {
        Pubkey::from_str(STAKING).unwrap()
    }

    pub fn moment_registry() -> Pubkey {
        Pubkey::from_str(MOMENT_REGISTRY).unwrap()
    }
}

/// Common errors for the SDK
#[derive(thiserror::Error, Debug)]
pub enum SdkError {
    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    #[error("Invalid instruction data")]
    InvalidInstructionData,

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// SDK client for interacting with OOF programs
pub struct OofSdk {
    client: RpcClient,
    commitment: CommitmentConfig,
}

impl OofSdk {
    /// Create a new SDK instance
    pub fn new(rpc_url: &str) -> Result<Self> {
        let client =
            RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
        Ok(Self {
            client,
            commitment: CommitmentConfig::confirmed(),
        })
    }

    /// Create SDK instance with custom commitment
    pub fn with_commitment(rpc_url: &str, commitment: CommitmentConfig) -> Result<Self> {
        let client = RpcClient::new_with_commitment(rpc_url.to_string(), commitment);
        Ok(Self { client, commitment })
    }

    /// Get the RPC client
    pub fn client(&self) -> &RpcClient {
        &self.client
    }

    /// Send and confirm a transaction
    #[instrument(skip(self, transaction))]
    pub async fn send_and_confirm_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<Signature, SdkError> {
        let signature = self
            .client
            .send_and_confirm_transaction(transaction)
            .map_err(|e| SdkError::TransactionFailed(e.to_string()))?;

        info!("Transaction confirmed: {}", signature);
        Ok(signature)
    }

    /// Get account info
    pub fn get_account(&self, pubkey: &Pubkey) -> Result<solana_sdk::account::Account, SdkError> {
        self.client
            .get_account(pubkey)
            .map_err(|e| SdkError::AccountNotFound(e.to_string()))
    }

    /// Check if account exists
    pub fn account_exists(&self, pubkey: &Pubkey) -> bool {
        self.client.get_account(pubkey).is_ok()
    }

    /// Get SOL balance
    pub fn get_balance(&self, pubkey: &Pubkey) -> Result<u64, SdkError> {
        self.client
            .get_balance(pubkey)
            .map_err(|e| SdkError::RpcError(e.to_string()))
    }

    /// Get multiple accounts
    pub fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<solana_sdk::account::Account>>, SdkError> {
        self.client
            .get_multiple_accounts(pubkeys)
            .map_err(|e| SdkError::RpcError(e.to_string()))
    }
}

/// Helper to derive program-derived addresses
pub fn find_program_address(seeds: &[&[u8]], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(seeds, program_id)
}

/// Helper to create account meta for instructions
pub fn create_account_meta(pubkey: Pubkey, is_signer: bool, is_writable: bool) -> AccountMeta {
    AccountMeta {
        pubkey,
        is_signer,
        is_writable,
    }
}

/// Common instruction builder trait
pub trait InstructionBuilder {
    /// Build the instruction
    fn build(&self) -> Result<Instruction, SdkError>;

    /// Get required accounts for the instruction
    fn accounts(&self) -> Vec<AccountMeta>;

    /// Get instruction data
    fn data(&self) -> Result<Vec<u8>, SdkError>;
}

/// Transaction builder helper
pub struct TransactionBuilder {
    instructions: Vec<Instruction>,
    payer: Option<Pubkey>,
    recent_blockhash: Option<solana_sdk::hash::Hash>,
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            payer: None,
            recent_blockhash: None,
        }
    }

    pub fn add_instruction(mut self, instruction: Instruction) -> Self {
        self.instructions.push(instruction);
        self
    }

    pub fn with_payer(mut self, payer: Pubkey) -> Self {
        self.payer = Some(payer);
        self
    }

    pub fn with_recent_blockhash(mut self, blockhash: solana_sdk::hash::Hash) -> Self {
        self.recent_blockhash = Some(blockhash);
        self
    }

    pub fn build(self, sdk: &OofSdk) -> Result<Transaction, SdkError> {
        let payer = self.payer.ok_or(SdkError::InvalidInstructionData)?;

        let recent_blockhash = match self.recent_blockhash {
            Some(hash) => hash,
            None => sdk
                .client()
                .get_latest_blockhash()
                .map_err(|e| SdkError::RpcError(e.to_string()))?,
        };

        Ok(Transaction::new_with_payer(
            &self.instructions,
            Some(&payer),
        ))
    }
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utility functions for common operations
pub mod utils {
    use super::*;

    /// Convert lamports to SOL
    pub fn lamports_to_sol(lamports: u64) -> Decimal {
        Decimal::new(lamports as i64, 9)
    }

    /// Convert SOL to lamports
    pub fn sol_to_lamports(sol: Decimal) -> u64 {
        (sol * Decimal::new(1_000_000_000, 0))
            .to_string()
            .parse()
            .unwrap_or(0)
    }

    /// Generate a random keypair
    pub fn generate_keypair() -> Keypair {
        Keypair::new()
    }

    /// Parse public key from string
    pub fn parse_pubkey(s: &str) -> Result<Pubkey, SdkError> {
        Pubkey::from_str(s).map_err(|e| SdkError::InvalidProgramId(e.to_string()))
    }

    /// Create a transfer instruction
    pub fn create_transfer_instruction(from: &Pubkey, to: &Pubkey, lamports: u64) -> Instruction {
        system_instruction::transfer(from, to, lamports)
    }

    /// Calculate rent for account size
    pub fn calculate_rent(sdk: &OofSdk, data_len: usize) -> Result<u64, SdkError> {
        sdk.client()
            .get_minimum_balance_for_rent_exemption(data_len)
            .map_err(|e| SdkError::RpcError(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_ids() {
        let campaigns_id = program_ids::campaigns();
        let staking_id = program_ids::staking();
        let registry_id = program_ids::moment_registry();

        assert_ne!(campaigns_id, staking_id);
        assert_ne!(staking_id, registry_id);
        assert_ne!(campaigns_id, registry_id);
    }

    #[test]
    fn test_utils() {
        let sol = utils::lamports_to_sol(1_000_000_000);
        assert_eq!(sol, Decimal::new(1, 0));

        let lamports = utils::sol_to_lamports(sol);
        assert_eq!(lamports, 1_000_000_000);
    }

    #[test]
    fn test_transaction_builder() {
        let builder = TransactionBuilder::new().with_payer(Pubkey::new_unique());

        assert!(builder.payer.is_some());
    }
}
