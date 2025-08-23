//! Staking and governance mechanisms
//!
//! This module provides functionality for:
//! - OOF token staking and unstaking
//! - Governance voting and proposals
//! - Validator delegation and rewards
//! - Staking pool management

use crate::{
    create_account_meta, find_program_address, program_ids, InstructionBuilder, OofSdk, SdkError,
};
use anchor_lang::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Signature,
    signer::Signer,
};
use std::collections::HashMap;
use time::OffsetDateTime;
use tracing::{info, instrument};

/// Staking pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingPool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub authority: Pubkey,
    pub stake_mint: Pubkey,
    pub reward_mint: Pubkey,
    pub total_staked: Decimal,
    pub total_rewards: Decimal,
    pub annual_yield: Decimal,
    pub minimum_stake: Decimal,
    pub lock_duration: u64, // seconds
    pub fee_rate: Decimal,
    pub is_active: bool,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// User stake position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakePosition {
    pub id: String,
    pub pool_id: String,
    pub staker: Pubkey,
    pub staked_amount: Decimal,
    pub reward_debt: Decimal,
    pub pending_rewards: Decimal,
    pub stake_time: OffsetDateTime,
    pub lock_end_time: Option<OffsetDateTime>,
    pub last_reward_time: OffsetDateTime,
    pub is_locked: bool,
}

/// Governance proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub proposer: Pubkey,
    pub proposal_type: ProposalType,
    pub status: ProposalStatus,
    pub votes_for: Decimal,
    pub votes_against: Decimal,
    pub total_votes: Decimal,
    pub quorum_threshold: Decimal,
    pub approval_threshold: Decimal,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
    pub execution_time: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
}

/// Types of governance proposals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalType {
    /// Change protocol parameters
    ParameterChange(HashMap<String, serde_json::Value>),
    /// Treasury spending proposal
    TreasurySpend { recipient: Pubkey, amount: Decimal },
    /// Upgrade program
    ProgramUpgrade { new_program_id: Pubkey },
    /// Add/remove validators
    ValidatorChange {
        validator: Pubkey,
        action: ValidatorAction,
    },
    /// Custom proposal
    Custom(String),
}

/// Validator actions for proposals
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ValidatorAction {
    Add,
    Remove,
    UpdateCommission(Decimal),
}

/// Proposal status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Active,
    Passed,
    Failed,
    Executed,
    Cancelled,
}

/// User vote on a proposal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub proposal_id: String,
    pub voter: Pubkey,
    pub vote_power: Decimal,
    pub choice: VoteChoice,
    pub timestamp: OffsetDateTime,
}

/// Vote choice
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VoteChoice {
    For,
    Against,
    Abstain,
}

/// Staking management client
pub struct StakingManager {
    sdk: OofSdk,
}

impl StakingManager {
    pub fn new(sdk: OofSdk) -> Self {
        Self { sdk }
    }

    /// Create a new staking pool
    #[instrument(skip(self))]
    pub async fn create_pool(
        &self,
        authority: &Pubkey,
        pool: &StakingPool,
    ) -> Result<Signature, SdkError> {
        let instruction = CreatePoolBuilder::new()
            .with_authority(*authority)
            .with_pool(pool.clone())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*authority)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Stake tokens in a pool
    #[instrument(skip(self))]
    pub async fn stake(
        &self,
        staker: &Pubkey,
        pool_id: &str,
        amount: Decimal,
    ) -> Result<Signature, SdkError> {
        let instruction = StakeBuilder::new()
            .with_staker(*staker)
            .with_pool_id(pool_id.to_string())
            .with_amount(amount)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*staker)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Unstake tokens from a pool
    #[instrument(skip(self))]
    pub async fn unstake(
        &self,
        staker: &Pubkey,
        pool_id: &str,
        amount: Decimal,
    ) -> Result<Signature, SdkError> {
        let instruction = UnstakeBuilder::new()
            .with_staker(*staker)
            .with_pool_id(pool_id.to_string())
            .with_amount(amount)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*staker)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Claim staking rewards
    #[instrument(skip(self))]
    pub async fn claim_rewards(
        &self,
        staker: &Pubkey,
        pool_id: &str,
    ) -> Result<Signature, SdkError> {
        let instruction = ClaimRewardsBuilder::new()
            .with_staker(*staker)
            .with_pool_id(pool_id.to_string())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*staker)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Create a governance proposal
    #[instrument(skip(self))]
    pub async fn create_proposal(
        &self,
        proposer: &Pubkey,
        proposal: &Proposal,
    ) -> Result<Signature, SdkError> {
        let instruction = CreateProposalBuilder::new()
            .with_proposer(*proposer)
            .with_proposal(proposal.clone())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*proposer)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Vote on a proposal
    #[instrument(skip(self))]
    pub async fn vote_on_proposal(
        &self,
        voter: &Pubkey,
        proposal_id: &str,
        choice: VoteChoice,
    ) -> Result<Signature, SdkError> {
        let instruction = VoteBuilder::new()
            .with_voter(*voter)
            .with_proposal_id(proposal_id.to_string())
            .with_choice(choice)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*voter)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Execute a passed proposal
    #[instrument(skip(self))]
    pub async fn execute_proposal(
        &self,
        executor: &Pubkey,
        proposal_id: &str,
    ) -> Result<Signature, SdkError> {
        let instruction = ExecuteProposalBuilder::new()
            .with_executor(*executor)
            .with_proposal_id(proposal_id.to_string())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*executor)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Get staking pool info
    pub fn get_pool(&self, pool_id: &str) -> Result<StakingPool, SdkError> {
        let (pool_pda, _) =
            find_program_address(&[b"pool", pool_id.as_bytes()], &program_ids::staking());

        let account = self.sdk.get_account(&pool_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(StakingPool {
            id: pool_id.to_string(),
            name: "OOF Staking Pool".to_string(),
            description: "Stake OOF tokens to earn rewards".to_string(),
            authority: Pubkey::new_unique(),
            stake_mint: Pubkey::new_unique(),
            reward_mint: Pubkey::new_unique(),
            total_staked: Decimal::new(0, 0),
            total_rewards: Decimal::new(0, 0),
            annual_yield: Decimal::new(15, 2), // 15%
            minimum_stake: Decimal::new(100, 0),
            lock_duration: 30 * 24 * 3600, // 30 days
            fee_rate: Decimal::new(5, 2),  // 5%
            is_active: true,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        })
    }

    /// Get user's stake position
    pub fn get_stake_position(
        &self,
        pool_id: &str,
        staker: &Pubkey,
    ) -> Result<StakePosition, SdkError> {
        let (position_pda, _) = find_program_address(
            &[b"position", pool_id.as_bytes(), staker.as_ref()],
            &program_ids::staking(),
        );

        let account = self.sdk.get_account(&position_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(StakePosition {
            id: format!("{}-{}", pool_id, staker),
            pool_id: pool_id.to_string(),
            staker: *staker,
            staked_amount: Decimal::new(0, 0),
            reward_debt: Decimal::new(0, 0),
            pending_rewards: Decimal::new(0, 0),
            stake_time: OffsetDateTime::now_utc(),
            lock_end_time: None,
            last_reward_time: OffsetDateTime::now_utc(),
            is_locked: false,
        })
    }

    /// Get proposal info
    pub fn get_proposal(&self, proposal_id: &str) -> Result<Proposal, SdkError> {
        let (proposal_pda, _) = find_program_address(
            &[b"proposal", proposal_id.as_bytes()],
            &program_ids::staking(),
        );

        let account = self.sdk.get_account(&proposal_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(Proposal {
            id: proposal_id.to_string(),
            title: "Sample Proposal".to_string(),
            description: "A sample governance proposal".to_string(),
            proposer: Pubkey::new_unique(),
            proposal_type: ProposalType::Custom("Sample".to_string()),
            status: ProposalStatus::Active,
            votes_for: Decimal::new(0, 0),
            votes_against: Decimal::new(0, 0),
            total_votes: Decimal::new(0, 0),
            quorum_threshold: Decimal::new(10, 2),   // 10%
            approval_threshold: Decimal::new(60, 2), // 60%
            start_time: OffsetDateTime::now_utc(),
            end_time: OffsetDateTime::now_utc(),
            execution_time: None,
            created_at: OffsetDateTime::now_utc(),
        })
    }
}

// Instruction builders

/// Create pool instruction builder
pub struct CreatePoolBuilder {
    authority: Option<Pubkey>,
    pool: Option<StakingPool>,
}

impl CreatePoolBuilder {
    pub fn new() -> Self {
        Self {
            authority: None,
            pool: None,
        }
    }

    pub fn with_authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn with_pool(mut self, pool: StakingPool) -> Self {
        self.pool = Some(pool);
        self
    }
}

impl InstructionBuilder for CreatePoolBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let authority = self.authority.unwrap();
        let pool = self.pool.as_ref().unwrap();
        let (pool_pda, _) =
            find_program_address(&[b"pool", pool.id.as_bytes()], &program_ids::staking());

        vec![
            create_account_meta(authority, true, true),
            create_account_meta(pool_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![0]) // CreatePool discriminator
    }
}

/// Stake instruction builder
pub struct StakeBuilder {
    staker: Option<Pubkey>,
    pool_id: Option<String>,
    amount: Option<Decimal>,
}

impl StakeBuilder {
    pub fn new() -> Self {
        Self {
            staker: None,
            pool_id: None,
            amount: None,
        }
    }

    pub fn with_staker(mut self, staker: Pubkey) -> Self {
        self.staker = Some(staker);
        self
    }

    pub fn with_pool_id(mut self, pool_id: String) -> Self {
        self.pool_id = Some(pool_id);
        self
    }

    pub fn with_amount(mut self, amount: Decimal) -> Self {
        self.amount = Some(amount);
        self
    }
}

impl InstructionBuilder for StakeBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let staker = self.staker.unwrap();
        let pool_id = self.pool_id.as_ref().unwrap();
        let (pool_pda, _) =
            find_program_address(&[b"pool", pool_id.as_bytes()], &program_ids::staking());
        let (position_pda, _) = find_program_address(
            &[b"position", pool_id.as_bytes(), staker.as_ref()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(staker, true, true),
            create_account_meta(pool_pda, false, true),
            create_account_meta(position_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![1]) // Stake discriminator
    }
}

/// Unstake instruction builder
pub struct UnstakeBuilder {
    staker: Option<Pubkey>,
    pool_id: Option<String>,
    amount: Option<Decimal>,
}

impl UnstakeBuilder {
    pub fn new() -> Self {
        Self {
            staker: None,
            pool_id: None,
            amount: None,
        }
    }

    pub fn with_staker(mut self, staker: Pubkey) -> Self {
        self.staker = Some(staker);
        self
    }

    pub fn with_pool_id(mut self, pool_id: String) -> Self {
        self.pool_id = Some(pool_id);
        self
    }

    pub fn with_amount(mut self, amount: Decimal) -> Self {
        self.amount = Some(amount);
        self
    }
}

impl InstructionBuilder for UnstakeBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let staker = self.staker.unwrap();
        let pool_id = self.pool_id.as_ref().unwrap();
        let (pool_pda, _) =
            find_program_address(&[b"pool", pool_id.as_bytes()], &program_ids::staking());
        let (position_pda, _) = find_program_address(
            &[b"position", pool_id.as_bytes(), staker.as_ref()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(staker, true, true),
            create_account_meta(pool_pda, false, true),
            create_account_meta(position_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![2]) // Unstake discriminator
    }
}

/// Claim rewards instruction builder
pub struct ClaimRewardsBuilder {
    staker: Option<Pubkey>,
    pool_id: Option<String>,
}

impl ClaimRewardsBuilder {
    pub fn new() -> Self {
        Self {
            staker: None,
            pool_id: None,
        }
    }

    pub fn with_staker(mut self, staker: Pubkey) -> Self {
        self.staker = Some(staker);
        self
    }

    pub fn with_pool_id(mut self, pool_id: String) -> Self {
        self.pool_id = Some(pool_id);
        self
    }
}

impl InstructionBuilder for ClaimRewardsBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let staker = self.staker.unwrap();
        let pool_id = self.pool_id.as_ref().unwrap();
        let (pool_pda, _) =
            find_program_address(&[b"pool", pool_id.as_bytes()], &program_ids::staking());
        let (position_pda, _) = find_program_address(
            &[b"position", pool_id.as_bytes(), staker.as_ref()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(staker, true, true),
            create_account_meta(pool_pda, false, true),
            create_account_meta(position_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![3]) // ClaimRewards discriminator
    }
}

/// Create proposal instruction builder
pub struct CreateProposalBuilder {
    proposer: Option<Pubkey>,
    proposal: Option<Proposal>,
}

impl CreateProposalBuilder {
    pub fn new() -> Self {
        Self {
            proposer: None,
            proposal: None,
        }
    }

    pub fn with_proposer(mut self, proposer: Pubkey) -> Self {
        self.proposer = Some(proposer);
        self
    }

    pub fn with_proposal(mut self, proposal: Proposal) -> Self {
        self.proposal = Some(proposal);
        self
    }
}

impl InstructionBuilder for CreateProposalBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let proposer = self.proposer.unwrap();
        let proposal = self.proposal.as_ref().unwrap();
        let (proposal_pda, _) = find_program_address(
            &[b"proposal", proposal.id.as_bytes()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(proposer, true, true),
            create_account_meta(proposal_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![4]) // CreateProposal discriminator
    }
}

/// Vote instruction builder
pub struct VoteBuilder {
    voter: Option<Pubkey>,
    proposal_id: Option<String>,
    choice: Option<VoteChoice>,
}

impl VoteBuilder {
    pub fn new() -> Self {
        Self {
            voter: None,
            proposal_id: None,
            choice: None,
        }
    }

    pub fn with_voter(mut self, voter: Pubkey) -> Self {
        self.voter = Some(voter);
        self
    }

    pub fn with_proposal_id(mut self, proposal_id: String) -> Self {
        self.proposal_id = Some(proposal_id);
        self
    }

    pub fn with_choice(mut self, choice: VoteChoice) -> Self {
        self.choice = Some(choice);
        self
    }
}

impl InstructionBuilder for VoteBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let voter = self.voter.unwrap();
        let proposal_id = self.proposal_id.as_ref().unwrap();
        let (proposal_pda, _) = find_program_address(
            &[b"proposal", proposal_id.as_bytes()],
            &program_ids::staking(),
        );
        let (vote_pda, _) = find_program_address(
            &[b"vote", proposal_id.as_bytes(), voter.as_ref()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(voter, true, true),
            create_account_meta(proposal_pda, false, true),
            create_account_meta(vote_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![5]) // Vote discriminator
    }
}

/// Execute proposal instruction builder
pub struct ExecuteProposalBuilder {
    executor: Option<Pubkey>,
    proposal_id: Option<String>,
}

impl ExecuteProposalBuilder {
    pub fn new() -> Self {
        Self {
            executor: None,
            proposal_id: None,
        }
    }

    pub fn with_executor(mut self, executor: Pubkey) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn with_proposal_id(mut self, proposal_id: String) -> Self {
        self.proposal_id = Some(proposal_id);
        self
    }
}

impl InstructionBuilder for ExecuteProposalBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::staking(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let executor = self.executor.unwrap();
        let proposal_id = self.proposal_id.as_ref().unwrap();
        let (proposal_pda, _) = find_program_address(
            &[b"proposal", proposal_id.as_bytes()],
            &program_ids::staking(),
        );

        vec![
            create_account_meta(executor, true, false),
            create_account_meta(proposal_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![6]) // ExecuteProposal discriminator
    }
}

// Default implementations
impl Default for CreatePoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StakeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UnstakeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ClaimRewardsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for CreateProposalBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VoteBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ExecuteProposalBuilder {
    fn default() -> Self {
        Self::new()
    }
}
