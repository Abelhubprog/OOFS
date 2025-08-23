//! Campaign management and reward distribution
//!
//! This module provides functionality for:
//! - Creating and managing OOF campaigns
//! - Distributing rewards to participants
//! - Tracking campaign progress and analytics
//! - Managing campaign treasury and funding

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

/// Campaign status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CampaignStatus {
    Draft,
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// Campaign type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CampaignType {
    /// Reward users for analyzing wallets
    WalletAnalysis,
    /// Reward users for discovering OOF moments
    MomentDiscovery,
    /// Community engagement campaigns
    Engagement,
    /// Referral rewards
    Referral,
    /// Custom campaign type
    Custom(String),
}

/// Reward distribution method
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RewardDistribution {
    /// Equal distribution among all participants
    Equal,
    /// Proportional to contribution/activity
    Proportional,
    /// Tiered rewards based on ranking
    Tiered(Vec<Decimal>),
    /// Fixed amount per action
    PerAction(Decimal),
}

/// Campaign configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub description: String,
    pub campaign_type: CampaignType,
    pub status: CampaignStatus,
    pub creator: Pubkey,
    pub treasury: Pubkey,
    pub reward_mint: Pubkey, // Token mint for rewards (could be OOF token)
    pub total_budget: Decimal,
    pub distributed_amount: Decimal,
    pub reward_distribution: RewardDistribution,
    pub start_time: OffsetDateTime,
    pub end_time: OffsetDateTime,
    pub max_participants: Option<u32>,
    pub participant_count: u32,
    pub requirements: CampaignRequirements,
    pub metadata: HashMap<String, String>,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

/// Campaign requirements and eligibility criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignRequirements {
    /// Minimum wallet analysis count
    pub min_analyses: Option<u32>,
    /// Minimum OOF moments discovered
    pub min_moments: Option<u32>,
    /// Required token holdings
    pub required_tokens: Vec<TokenRequirement>,
    /// Minimum account age (in days)
    pub min_account_age: Option<u32>,
    /// Custom requirements
    pub custom: HashMap<String, serde_json::Value>,
}

/// Token holding requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRequirement {
    pub mint: Pubkey,
    pub minimum_amount: Decimal,
}

/// Participant in a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignParticipant {
    pub wallet: Pubkey,
    pub campaign_id: String,
    pub joined_at: OffsetDateTime,
    pub activity_score: Decimal,
    pub actions_completed: u32,
    pub rewards_earned: Decimal,
    pub last_activity: OffsetDateTime,
    pub metadata: HashMap<String, String>,
}

/// Campaign reward claim
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardClaim {
    pub id: String,
    pub campaign_id: String,
    pub participant: Pubkey,
    pub amount: Decimal,
    pub claimed_at: OffsetDateTime,
    pub transaction_signature: Option<String>,
}

/// Campaign management client
pub struct CampaignManager {
    sdk: OofSdk,
}

impl CampaignManager {
    pub fn new(sdk: OofSdk) -> Self {
        Self { sdk }
    }

    /// Create a new campaign
    #[instrument(skip(self))]
    pub async fn create_campaign(
        &self,
        creator: &Pubkey,
        campaign: &Campaign,
    ) -> Result<Signature, SdkError> {
        let instruction = CreateCampaignBuilder::new()
            .with_creator(*creator)
            .with_campaign(campaign.clone())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*creator)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Update campaign status
    #[instrument(skip(self))]
    pub async fn update_campaign_status(
        &self,
        authority: &Pubkey,
        campaign_id: &str,
        status: CampaignStatus,
    ) -> Result<Signature, SdkError> {
        let instruction = UpdateCampaignStatusBuilder::new()
            .with_authority(*authority)
            .with_campaign_id(campaign_id.to_string())
            .with_status(status)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*authority)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Join a campaign
    #[instrument(skip(self))]
    pub async fn join_campaign(
        &self,
        participant: &Pubkey,
        campaign_id: &str,
    ) -> Result<Signature, SdkError> {
        let instruction = JoinCampaignBuilder::new()
            .with_participant(*participant)
            .with_campaign_id(campaign_id.to_string())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*participant)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Record campaign activity
    #[instrument(skip(self))]
    pub async fn record_activity(
        &self,
        participant: &Pubkey,
        campaign_id: &str,
        activity_type: &str,
        value: Decimal,
    ) -> Result<Signature, SdkError> {
        let instruction = RecordActivityBuilder::new()
            .with_participant(*participant)
            .with_campaign_id(campaign_id.to_string())
            .with_activity_type(activity_type.to_string())
            .with_value(value)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*participant)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Claim campaign rewards
    #[instrument(skip(self))]
    pub async fn claim_rewards(
        &self,
        participant: &Pubkey,
        campaign_id: &str,
    ) -> Result<Signature, SdkError> {
        let instruction = ClaimRewardsBuilder::new()
            .with_participant(*participant)
            .with_campaign_id(campaign_id.to_string())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*participant)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Get campaign by ID
    pub fn get_campaign(&self, campaign_id: &str) -> Result<Campaign, SdkError> {
        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign_id.as_bytes()],
            &program_ids::campaigns(),
        );

        let account = self.sdk.get_account(&campaign_pda)?;

        // In a real implementation, this would deserialize the account data
        // For now, return a placeholder
        Ok(Campaign {
            id: campaign_id.to_string(),
            name: "Sample Campaign".to_string(),
            description: "A sample campaign".to_string(),
            campaign_type: CampaignType::WalletAnalysis,
            status: CampaignStatus::Active,
            creator: Pubkey::new_unique(),
            treasury: Pubkey::new_unique(),
            reward_mint: Pubkey::new_unique(),
            total_budget: Decimal::new(1000, 0),
            distributed_amount: Decimal::new(0, 0),
            reward_distribution: RewardDistribution::Equal,
            start_time: OffsetDateTime::now_utc(),
            end_time: OffsetDateTime::now_utc(),
            max_participants: Some(100),
            participant_count: 0,
            requirements: CampaignRequirements {
                min_analyses: None,
                min_moments: None,
                required_tokens: Vec::new(),
                min_account_age: None,
                custom: HashMap::new(),
            },
            metadata: HashMap::new(),
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        })
    }

    /// Get participant info
    pub fn get_participant(
        &self,
        campaign_id: &str,
        participant: &Pubkey,
    ) -> Result<CampaignParticipant, SdkError> {
        let (participant_pda, _) = find_program_address(
            &[b"participant", campaign_id.as_bytes(), participant.as_ref()],
            &program_ids::campaigns(),
        );

        let account = self.sdk.get_account(&participant_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(CampaignParticipant {
            wallet: *participant,
            campaign_id: campaign_id.to_string(),
            joined_at: OffsetDateTime::now_utc(),
            activity_score: Decimal::new(0, 0),
            actions_completed: 0,
            rewards_earned: Decimal::new(0, 0),
            last_activity: OffsetDateTime::now_utc(),
            metadata: HashMap::new(),
        })
    }
}

// Instruction builders

/// Create campaign instruction builder
pub struct CreateCampaignBuilder {
    creator: Option<Pubkey>,
    campaign: Option<Campaign>,
}

impl CreateCampaignBuilder {
    pub fn new() -> Self {
        Self {
            creator: None,
            campaign: None,
        }
    }

    pub fn with_creator(mut self, creator: Pubkey) -> Self {
        self.creator = Some(creator);
        self
    }

    pub fn with_campaign(mut self, campaign: Campaign) -> Self {
        self.campaign = Some(campaign);
        self
    }
}

impl InstructionBuilder for CreateCampaignBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        let creator = self.creator.ok_or(SdkError::InvalidInstructionData)?;
        let campaign = self
            .campaign
            .as_ref()
            .ok_or(SdkError::InvalidInstructionData)?;

        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign.id.as_bytes()],
            &program_ids::campaigns(),
        );

        Ok(Instruction {
            program_id: program_ids::campaigns(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let creator = self.creator.unwrap();
        let campaign = self.campaign.as_ref().unwrap();
        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign.id.as_bytes()],
            &program_ids::campaigns(),
        );

        vec![
            create_account_meta(creator, true, true),
            create_account_meta(campaign_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        // In a real implementation, this would serialize the instruction data
        Ok(vec![0]) // CreateCampaign discriminator
    }
}

/// Update campaign status instruction builder
pub struct UpdateCampaignStatusBuilder {
    authority: Option<Pubkey>,
    campaign_id: Option<String>,
    status: Option<CampaignStatus>,
}

impl UpdateCampaignStatusBuilder {
    pub fn new() -> Self {
        Self {
            authority: None,
            campaign_id: None,
            status: None,
        }
    }

    pub fn with_authority(mut self, authority: Pubkey) -> Self {
        self.authority = Some(authority);
        self
    }

    pub fn with_campaign_id(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    pub fn with_status(mut self, status: CampaignStatus) -> Self {
        self.status = Some(status);
        self
    }
}

impl InstructionBuilder for UpdateCampaignStatusBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::campaigns(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let authority = self.authority.unwrap();
        let campaign_id = self.campaign_id.as_ref().unwrap();
        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign_id.as_bytes()],
            &program_ids::campaigns(),
        );

        vec![
            create_account_meta(authority, true, false),
            create_account_meta(campaign_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![1]) // UpdateCampaignStatus discriminator
    }
}

/// Join campaign instruction builder
pub struct JoinCampaignBuilder {
    participant: Option<Pubkey>,
    campaign_id: Option<String>,
}

impl JoinCampaignBuilder {
    pub fn new() -> Self {
        Self {
            participant: None,
            campaign_id: None,
        }
    }

    pub fn with_participant(mut self, participant: Pubkey) -> Self {
        self.participant = Some(participant);
        self
    }

    pub fn with_campaign_id(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }
}

impl InstructionBuilder for JoinCampaignBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::campaigns(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let participant = self.participant.unwrap();
        let campaign_id = self.campaign_id.as_ref().unwrap();
        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign_id.as_bytes()],
            &program_ids::campaigns(),
        );
        let (participant_pda, _) = find_program_address(
            &[b"participant", campaign_id.as_bytes(), participant.as_ref()],
            &program_ids::campaigns(),
        );

        vec![
            create_account_meta(participant, true, true),
            create_account_meta(campaign_pda, false, true),
            create_account_meta(participant_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![2]) // JoinCampaign discriminator
    }
}

/// Record activity instruction builder
pub struct RecordActivityBuilder {
    participant: Option<Pubkey>,
    campaign_id: Option<String>,
    activity_type: Option<String>,
    value: Option<Decimal>,
}

impl RecordActivityBuilder {
    pub fn new() -> Self {
        Self {
            participant: None,
            campaign_id: None,
            activity_type: None,
            value: None,
        }
    }

    pub fn with_participant(mut self, participant: Pubkey) -> Self {
        self.participant = Some(participant);
        self
    }

    pub fn with_campaign_id(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }

    pub fn with_activity_type(mut self, activity_type: String) -> Self {
        self.activity_type = Some(activity_type);
        self
    }

    pub fn with_value(mut self, value: Decimal) -> Self {
        self.value = Some(value);
        self
    }
}

impl InstructionBuilder for RecordActivityBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::campaigns(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let participant = self.participant.unwrap();
        let campaign_id = self.campaign_id.as_ref().unwrap();
        let (participant_pda, _) = find_program_address(
            &[b"participant", campaign_id.as_bytes(), participant.as_ref()],
            &program_ids::campaigns(),
        );

        vec![
            create_account_meta(participant, true, false),
            create_account_meta(participant_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![3]) // RecordActivity discriminator
    }
}

/// Claim rewards instruction builder
pub struct ClaimRewardsBuilder {
    participant: Option<Pubkey>,
    campaign_id: Option<String>,
}

impl ClaimRewardsBuilder {
    pub fn new() -> Self {
        Self {
            participant: None,
            campaign_id: None,
        }
    }

    pub fn with_participant(mut self, participant: Pubkey) -> Self {
        self.participant = Some(participant);
        self
    }

    pub fn with_campaign_id(mut self, campaign_id: String) -> Self {
        self.campaign_id = Some(campaign_id);
        self
    }
}

impl InstructionBuilder for ClaimRewardsBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::campaigns(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let participant = self.participant.unwrap();
        let campaign_id = self.campaign_id.as_ref().unwrap();
        let (campaign_pda, _) = find_program_address(
            &[b"campaign", campaign_id.as_bytes()],
            &program_ids::campaigns(),
        );
        let (participant_pda, _) = find_program_address(
            &[b"participant", campaign_id.as_bytes(), participant.as_ref()],
            &program_ids::campaigns(),
        );

        vec![
            create_account_meta(participant, true, true),
            create_account_meta(campaign_pda, false, true),
            create_account_meta(participant_pda, false, true),
            // Add token accounts for reward distribution
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![4]) // ClaimRewards discriminator
    }
}

// Default implementations
impl Default for CreateCampaignBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UpdateCampaignStatusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for JoinCampaignBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RecordActivityBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ClaimRewardsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
