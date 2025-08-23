//! Moment registry and verification
//!
//! This module provides functionality for:
//! - Registering and verifying OOF moments on-chain
//! - Community verification and challenges
//! - Moment metadata and scoring
//! - Registry governance and moderation

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

/// OOF moment types for registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MomentType {
    SoldTooEarly,
    BagHolderDrawdown,
    BadRoute,
    IdleYield,
    Rug,
}

/// Moment verification status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationStatus {
    Pending,
    Verified,
    Disputed,
    Rejected,
    Challenged,
}

/// Registered OOF moment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredMoment {
    pub id: String,
    pub moment_type: MomentType,
    pub wallet: Pubkey,
    pub transaction_signature: String,
    pub token_mint: Option<Pubkey>,
    pub amount_involved: Decimal,
    pub potential_loss: Decimal,
    pub severity_score: Decimal,
    pub verification_status: VerificationStatus,
    pub verifier_count: u32,
    pub challenge_count: u32,
    pub reward_amount: Decimal,
    pub metadata: MomentMetadata,
    pub registered_by: Pubkey,
    pub registered_at: OffsetDateTime,
    pub verified_at: Option<OffsetDateTime>,
    pub last_updated: OffsetDateTime,
}

/// Moment metadata and additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MomentMetadata {
    pub description: String,
    pub tags: Vec<String>,
    pub evidence_links: Vec<String>,
    pub analysis_data: HashMap<String, serde_json::Value>,
    pub social_metrics: SocialMetrics,
}

/// Social engagement metrics for moments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMetrics {
    pub views: u64,
    pub likes: u64,
    pub shares: u64,
    pub comments: u64,
    pub engagement_score: Decimal,
}

/// Moment verification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verification {
    pub id: String,
    pub moment_id: String,
    pub verifier: Pubkey,
    pub is_valid: bool,
    pub confidence_score: Decimal,
    pub evidence: String,
    pub reward_earned: Decimal,
    pub verified_at: OffsetDateTime,
}

/// Challenge to a verified moment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub moment_id: String,
    pub challenger: Pubkey,
    pub reason: ChallengeReason,
    pub evidence: String,
    pub stake_amount: Decimal,
    pub status: ChallengeStatus,
    pub votes_support: u32,
    pub votes_against: u32,
    pub resolution: Option<ChallengeResolution>,
    pub created_at: OffsetDateTime,
    pub resolved_at: Option<OffsetDateTime>,
}

/// Reasons for challenging a moment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChallengeReason {
    InvalidData,
    WrongClassification,
    FakeEvidence,
    DuplicateMoment,
    InsufficientSeverity,
    Other(String),
}

/// Challenge status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChallengeStatus {
    Open,
    Voting,
    Resolved,
    Expired,
}

/// Challenge resolution outcome
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChallengeResolution {
    Upheld,    // Challenge was valid
    Dismissed, // Challenge was invalid
    Partial,   // Partial validity
}

/// Registry statistics and metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    pub total_moments: u64,
    pub verified_moments: u64,
    pub pending_moments: u64,
    pub disputed_moments: u64,
    pub total_verifiers: u64,
    pub active_challenges: u64,
    pub total_rewards_distributed: Decimal,
    pub average_verification_time: u64, // seconds
    pub moments_by_type: HashMap<MomentType, u64>,
    pub top_verifiers: Vec<VerifierRanking>,
}

/// Verifier ranking information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifierRanking {
    pub verifier: Pubkey,
    pub verified_count: u64,
    pub accuracy_rate: Decimal,
    pub total_rewards: Decimal,
    pub reputation_score: Decimal,
}

/// Moment registry management client
pub struct MomentRegistry {
    sdk: OofSdk,
}

impl MomentRegistry {
    pub fn new(sdk: OofSdk) -> Self {
        Self { sdk }
    }

    /// Register a new OOF moment
    #[instrument(skip(self))]
    pub async fn register_moment(
        &self,
        registrar: &Pubkey,
        moment: &RegisteredMoment,
    ) -> Result<Signature, SdkError> {
        let instruction = RegisterMomentBuilder::new()
            .with_registrar(*registrar)
            .with_moment(moment.clone())
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*registrar)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Verify a moment
    #[instrument(skip(self))]
    pub async fn verify_moment(
        &self,
        verifier: &Pubkey,
        moment_id: &str,
        is_valid: bool,
        confidence: Decimal,
        evidence: String,
    ) -> Result<Signature, SdkError> {
        let instruction = VerifyMomentBuilder::new()
            .with_verifier(*verifier)
            .with_moment_id(moment_id.to_string())
            .with_validity(is_valid)
            .with_confidence(confidence)
            .with_evidence(evidence)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*verifier)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Challenge a verified moment
    #[instrument(skip(self))]
    pub async fn challenge_moment(
        &self,
        challenger: &Pubkey,
        moment_id: &str,
        reason: ChallengeReason,
        evidence: String,
        stake: Decimal,
    ) -> Result<Signature, SdkError> {
        let instruction = ChallengeMomentBuilder::new()
            .with_challenger(*challenger)
            .with_moment_id(moment_id.to_string())
            .with_reason(reason)
            .with_evidence(evidence)
            .with_stake(stake)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*challenger)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Vote on a challenge
    #[instrument(skip(self))]
    pub async fn vote_on_challenge(
        &self,
        voter: &Pubkey,
        challenge_id: &str,
        support: bool,
    ) -> Result<Signature, SdkError> {
        let instruction = VoteOnChallengeBuilder::new()
            .with_voter(*voter)
            .with_challenge_id(challenge_id.to_string())
            .with_support(support)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*voter)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Update moment metadata
    #[instrument(skip(self))]
    pub async fn update_moment_metadata(
        &self,
        updater: &Pubkey,
        moment_id: &str,
        metadata: MomentMetadata,
    ) -> Result<Signature, SdkError> {
        let instruction = UpdateMetadataBuilder::new()
            .with_updater(*updater)
            .with_moment_id(moment_id.to_string())
            .with_metadata(metadata)
            .build()?;

        let transaction = crate::TransactionBuilder::new()
            .add_instruction(instruction)
            .with_payer(*updater)
            .build(&self.sdk)?;

        self.sdk.send_and_confirm_transaction(&transaction).await
    }

    /// Get moment by ID
    pub fn get_moment(&self, moment_id: &str) -> Result<RegisteredMoment, SdkError> {
        let (moment_pda, _) = find_program_address(
            &[b"moment", moment_id.as_bytes()],
            &program_ids::moment_registry(),
        );

        let account = self.sdk.get_account(&moment_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(RegisteredMoment {
            id: moment_id.to_string(),
            moment_type: MomentType::SoldTooEarly,
            wallet: Pubkey::new_unique(),
            transaction_signature: "sample_signature".to_string(),
            token_mint: Some(Pubkey::new_unique()),
            amount_involved: Decimal::new(1000, 0),
            potential_loss: Decimal::new(500, 0),
            severity_score: Decimal::new(75, 0),
            verification_status: VerificationStatus::Pending,
            verifier_count: 0,
            challenge_count: 0,
            reward_amount: Decimal::new(10, 0),
            metadata: MomentMetadata {
                description: "Sample OOF moment".to_string(),
                tags: vec!["DeFi".to_string(), "Loss".to_string()],
                evidence_links: Vec::new(),
                analysis_data: HashMap::new(),
                social_metrics: SocialMetrics {
                    views: 0,
                    likes: 0,
                    shares: 0,
                    comments: 0,
                    engagement_score: Decimal::new(0, 0),
                },
            },
            registered_by: Pubkey::new_unique(),
            registered_at: OffsetDateTime::now_utc(),
            verified_at: None,
            last_updated: OffsetDateTime::now_utc(),
        })
    }

    /// Get verification for a moment
    pub fn get_verification(
        &self,
        moment_id: &str,
        verifier: &Pubkey,
    ) -> Result<Verification, SdkError> {
        let (verification_pda, _) = find_program_address(
            &[b"verification", moment_id.as_bytes(), verifier.as_ref()],
            &program_ids::moment_registry(),
        );

        let account = self.sdk.get_account(&verification_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(Verification {
            id: format!("{}-{}", moment_id, verifier),
            moment_id: moment_id.to_string(),
            verifier: *verifier,
            is_valid: true,
            confidence_score: Decimal::new(85, 0),
            evidence: "Verification evidence".to_string(),
            reward_earned: Decimal::new(5, 0),
            verified_at: OffsetDateTime::now_utc(),
        })
    }

    /// Get challenge by ID
    pub fn get_challenge(&self, challenge_id: &str) -> Result<Challenge, SdkError> {
        let (challenge_pda, _) = find_program_address(
            &[b"challenge", challenge_id.as_bytes()],
            &program_ids::moment_registry(),
        );

        let account = self.sdk.get_account(&challenge_pda)?;

        // In a real implementation, this would deserialize the account data
        Ok(Challenge {
            id: challenge_id.to_string(),
            moment_id: "sample_moment".to_string(),
            challenger: Pubkey::new_unique(),
            reason: ChallengeReason::InvalidData,
            evidence: "Challenge evidence".to_string(),
            stake_amount: Decimal::new(50, 0),
            status: ChallengeStatus::Open,
            votes_support: 0,
            votes_against: 0,
            resolution: None,
            created_at: OffsetDateTime::now_utc(),
            resolved_at: None,
        })
    }

    /// Get registry statistics
    pub fn get_registry_stats(&self) -> Result<RegistryStats, SdkError> {
        // In a real implementation, this would aggregate from multiple accounts
        Ok(RegistryStats {
            total_moments: 1000,
            verified_moments: 800,
            pending_moments: 150,
            disputed_moments: 50,
            total_verifiers: 100,
            active_challenges: 10,
            total_rewards_distributed: Decimal::new(5000, 0),
            average_verification_time: 3600, // 1 hour
            moments_by_type: HashMap::from([
                (MomentType::SoldTooEarly, 300),
                (MomentType::BagHolderDrawdown, 250),
                (MomentType::BadRoute, 200),
                (MomentType::IdleYield, 150),
                (MomentType::Rug, 100),
            ]),
            top_verifiers: Vec::new(),
        })
    }
}

// Instruction builders

/// Register moment instruction builder
pub struct RegisterMomentBuilder {
    registrar: Option<Pubkey>,
    moment: Option<RegisteredMoment>,
}

impl RegisterMomentBuilder {
    pub fn new() -> Self {
        Self {
            registrar: None,
            moment: None,
        }
    }

    pub fn with_registrar(mut self, registrar: Pubkey) -> Self {
        self.registrar = Some(registrar);
        self
    }

    pub fn with_moment(mut self, moment: RegisteredMoment) -> Self {
        self.moment = Some(moment);
        self
    }
}

impl InstructionBuilder for RegisterMomentBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::moment_registry(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let registrar = self.registrar.unwrap();
        let moment = self.moment.as_ref().unwrap();
        let (moment_pda, _) = find_program_address(
            &[b"moment", moment.id.as_bytes()],
            &program_ids::moment_registry(),
        );

        vec![
            create_account_meta(registrar, true, true),
            create_account_meta(moment_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![0]) // RegisterMoment discriminator
    }
}

/// Verify moment instruction builder
pub struct VerifyMomentBuilder {
    verifier: Option<Pubkey>,
    moment_id: Option<String>,
    is_valid: Option<bool>,
    confidence: Option<Decimal>,
    evidence: Option<String>,
}

impl VerifyMomentBuilder {
    pub fn new() -> Self {
        Self {
            verifier: None,
            moment_id: None,
            is_valid: None,
            confidence: None,
            evidence: None,
        }
    }

    pub fn with_verifier(mut self, verifier: Pubkey) -> Self {
        self.verifier = Some(verifier);
        self
    }

    pub fn with_moment_id(mut self, moment_id: String) -> Self {
        self.moment_id = Some(moment_id);
        self
    }

    pub fn with_validity(mut self, is_valid: bool) -> Self {
        self.is_valid = Some(is_valid);
        self
    }

    pub fn with_confidence(mut self, confidence: Decimal) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn with_evidence(mut self, evidence: String) -> Self {
        self.evidence = Some(evidence);
        self
    }
}

impl InstructionBuilder for VerifyMomentBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::moment_registry(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let verifier = self.verifier.unwrap();
        let moment_id = self.moment_id.as_ref().unwrap();
        let (moment_pda, _) = find_program_address(
            &[b"moment", moment_id.as_bytes()],
            &program_ids::moment_registry(),
        );
        let (verification_pda, _) = find_program_address(
            &[b"verification", moment_id.as_bytes(), verifier.as_ref()],
            &program_ids::moment_registry(),
        );

        vec![
            create_account_meta(verifier, true, true),
            create_account_meta(moment_pda, false, true),
            create_account_meta(verification_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![1]) // VerifyMoment discriminator
    }
}

/// Challenge moment instruction builder
pub struct ChallengeMomentBuilder {
    challenger: Option<Pubkey>,
    moment_id: Option<String>,
    reason: Option<ChallengeReason>,
    evidence: Option<String>,
    stake: Option<Decimal>,
}

impl ChallengeMomentBuilder {
    pub fn new() -> Self {
        Self {
            challenger: None,
            moment_id: None,
            reason: None,
            evidence: None,
            stake: None,
        }
    }

    pub fn with_challenger(mut self, challenger: Pubkey) -> Self {
        self.challenger = Some(challenger);
        self
    }

    pub fn with_moment_id(mut self, moment_id: String) -> Self {
        self.moment_id = Some(moment_id);
        self
    }

    pub fn with_reason(mut self, reason: ChallengeReason) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn with_evidence(mut self, evidence: String) -> Self {
        self.evidence = Some(evidence);
        self
    }

    pub fn with_stake(mut self, stake: Decimal) -> Self {
        self.stake = Some(stake);
        self
    }
}

impl InstructionBuilder for ChallengeMomentBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::moment_registry(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let challenger = self.challenger.unwrap();
        let moment_id = self.moment_id.as_ref().unwrap();
        let challenge_id = format!("{}-{}", moment_id, challenger);
        let (moment_pda, _) = find_program_address(
            &[b"moment", moment_id.as_bytes()],
            &program_ids::moment_registry(),
        );
        let (challenge_pda, _) = find_program_address(
            &[b"challenge", challenge_id.as_bytes()],
            &program_ids::moment_registry(),
        );

        vec![
            create_account_meta(challenger, true, true),
            create_account_meta(moment_pda, false, true),
            create_account_meta(challenge_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![2]) // ChallengeMoment discriminator
    }
}

/// Vote on challenge instruction builder
pub struct VoteOnChallengeBuilder {
    voter: Option<Pubkey>,
    challenge_id: Option<String>,
    support: Option<bool>,
}

impl VoteOnChallengeBuilder {
    pub fn new() -> Self {
        Self {
            voter: None,
            challenge_id: None,
            support: None,
        }
    }

    pub fn with_voter(mut self, voter: Pubkey) -> Self {
        self.voter = Some(voter);
        self
    }

    pub fn with_challenge_id(mut self, challenge_id: String) -> Self {
        self.challenge_id = Some(challenge_id);
        self
    }

    pub fn with_support(mut self, support: bool) -> Self {
        self.support = Some(support);
        self
    }
}

impl InstructionBuilder for VoteOnChallengeBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::moment_registry(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let voter = self.voter.unwrap();
        let challenge_id = self.challenge_id.as_ref().unwrap();
        let (challenge_pda, _) = find_program_address(
            &[b"challenge", challenge_id.as_bytes()],
            &program_ids::moment_registry(),
        );
        let (vote_pda, _) = find_program_address(
            &[b"challenge_vote", challenge_id.as_bytes(), voter.as_ref()],
            &program_ids::moment_registry(),
        );

        vec![
            create_account_meta(voter, true, true),
            create_account_meta(challenge_pda, false, true),
            create_account_meta(vote_pda, false, true),
            create_account_meta(solana_sdk::system_program::id(), false, false),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![3]) // VoteOnChallenge discriminator
    }
}

/// Update metadata instruction builder
pub struct UpdateMetadataBuilder {
    updater: Option<Pubkey>,
    moment_id: Option<String>,
    metadata: Option<MomentMetadata>,
}

impl UpdateMetadataBuilder {
    pub fn new() -> Self {
        Self {
            updater: None,
            moment_id: None,
            metadata: None,
        }
    }

    pub fn with_updater(mut self, updater: Pubkey) -> Self {
        self.updater = Some(updater);
        self
    }

    pub fn with_moment_id(mut self, moment_id: String) -> Self {
        self.moment_id = Some(moment_id);
        self
    }

    pub fn with_metadata(mut self, metadata: MomentMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl InstructionBuilder for UpdateMetadataBuilder {
    fn build(&self) -> Result<Instruction, SdkError> {
        Ok(Instruction {
            program_id: program_ids::moment_registry(),
            accounts: self.accounts(),
            data: self.data()?,
        })
    }

    fn accounts(&self) -> Vec<AccountMeta> {
        let updater = self.updater.unwrap();
        let moment_id = self.moment_id.as_ref().unwrap();
        let (moment_pda, _) = find_program_address(
            &[b"moment", moment_id.as_bytes()],
            &program_ids::moment_registry(),
        );

        vec![
            create_account_meta(updater, true, false),
            create_account_meta(moment_pda, false, true),
        ]
    }

    fn data(&self) -> Result<Vec<u8>, SdkError> {
        Ok(vec![4]) // UpdateMetadata discriminator
    }
}

// Default implementations
impl Default for RegisterMomentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VerifyMomentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ChallengeMomentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for VoteOnChallengeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for UpdateMetadataBuilder {
    fn default() -> Self {
        Self::new()
    }
}
