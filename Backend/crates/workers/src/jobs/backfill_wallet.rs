use serde::{Deserialize, Serialize};
use shared::{
    error::{ApiError, ApiResult},
    types::{
        chain::{Action, ChainEvent, EventKind},
        common::{SolAmount, TokenAmount, UsdAmount},
        wallet::WalletAnalysis,
    },
};
use sqlx::PgPool;
use std::{
    collections::{HashMap, VecDeque},
    time::Duration,
};
use time::OffsetDateTime;
use tokio::time::sleep;
use tracing::{debug, error, info, instrument, warn};
use uuid::Uuid;

/// Backfill job for comprehensive wallet historical analysis
pub struct BackfillWalletJob {
    pool: PgPool,
    rpc_client: solana_client::rpc_client::RpcClient,
    redis: Option<redis::Client>,
    max_concurrent_wallets: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillRequest {
    pub job_id: String,
    pub wallet_address: String,
    pub start_date: Option<OffsetDateTime>,
    pub end_date: Option<OffsetDateTime>,
    pub priority: BackfillPriority,
    pub requested_by: String,
    pub include_token_accounts: bool,
    pub include_nft_activity: bool,
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackfillPriority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackfillProgress {
    pub job_id: String,
    pub wallet_address: String,
    pub status: BackfillStatus,
    pub progress_pct: f64,
    pub transactions_processed: u32,
    pub total_transactions: u32,
    pub current_stage: BackfillStage,
    pub started_at: OffsetDateTime,
    pub estimated_completion: Option<OffsetDateTime>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackfillStatus {
    Queued,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackfillStage {
    Initializing,
    FetchingSignatures,
    FetchingTransactions,
    ProcessingTransactions,
    BuildingPositions,
    DetectingMoments,
    Finalizing,
}

impl BackfillWalletJob {
    pub fn new(pool: PgPool, rpc_endpoint: String, redis: Option<redis::Client>) -> Self {
        Self {
            pool,
            rpc_client: solana_client::rpc_client::RpcClient::new(rpc_endpoint),
            redis,
            max_concurrent_wallets: 5,
        }
    }

    /// Process pending backfill requests
    #[instrument(skip(self))]
    pub async fn process_pending_backfills(&self) -> ApiResult<u32> {
        info!("Processing pending wallet backfill requests");

        let mut processed = 0;

        // Fetch high priority requests first
        let pending_requests = self.fetch_pending_requests().await?;

        // Process requests with concurrency control
        let mut active_jobs = VecDeque::new();

        for request in pending_requests {
            // Wait if we have too many concurrent jobs
            while active_jobs.len() >= self.max_concurrent_wallets {
                // Check if any jobs completed
                if let Some(job_handle) = active_jobs.pop_front() {
                    if job_handle.is_finished() {
                        match job_handle.await {
                            Ok(Ok(_)) => processed += 1,
                            Ok(Err(e)) => error!("Backfill job failed: {}", e),
                            Err(e) => error!("Backfill job panicked: {}", e),
                        }
                    } else {
                        // Job still running, put it back
                        active_jobs.push_back(job_handle);
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }

            // Start new job
            let job_pool = self.pool.clone();
            let job_rpc = self.rpc_client.clone();
            let job_redis = self.redis.clone();

            let handle = tokio::spawn(async move {
                let job = BackfillWalletJob {
                    pool: job_pool,
                    rpc_client: job_rpc,
                    redis: job_redis,
                    max_concurrent_wallets: 1, // Individual job doesn't need concurrency
                };
                job.execute_backfill(&request).await
            });

            active_jobs.push_back(handle);
        }

        // Wait for remaining jobs to complete
        while let Some(job_handle) = active_jobs.pop_front() {
            match job_handle.await {
                Ok(Ok(_)) => processed += 1,
                Ok(Err(e)) => error!("Backfill job failed: {}", e),
                Err(e) => error!("Backfill job panicked: {}", e),
            }
        }

        info!("Completed {} backfill jobs", processed);
        Ok(processed)
    }

    /// Execute backfill for a single wallet
    #[instrument(skip(self, request))]
    async fn execute_backfill(&self, request: &BackfillRequest) -> ApiResult<()> {
        info!("Starting backfill for wallet: {}", request.wallet_address);

        // Initialize progress tracking
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::Initializing,
            0.0,
        )
        .await?;

        // Validate wallet address
        let wallet_pubkey = solana_sdk::pubkey::Pubkey::from_str(&request.wallet_address)
            .map_err(|e| ApiError::BadRequest(format!("Invalid wallet address: {}", e)))?;

        // Determine date range
        let end_date = request.end_date.unwrap_or_else(OffsetDateTime::now_utc);
        let start_date = request
            .start_date
            .unwrap_or(end_date - time::Duration::days(730)); // 2 years default

        info!(
            "Backfilling wallet {} from {} to {}",
            request.wallet_address, start_date, end_date
        );

        // Step 1: Fetch transaction signatures
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::FetchingSignatures,
            10.0,
        )
        .await?;
        let signatures = self
            .fetch_transaction_signatures(&wallet_pubkey, start_date, end_date)
            .await?;

        info!(
            "Found {} transaction signatures for wallet",
            signatures.len()
        );

        // Step 2: Fetch transaction details
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::FetchingTransactions,
            20.0,
        )
        .await?;
        let transactions = self
            .fetch_transaction_details(&signatures, &request.job_id)
            .await?;

        info!("Fetched {} transaction details", transactions.len());

        // Step 3: Process and store transactions
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::ProcessingTransactions,
            50.0,
        )
        .await?;
        let chain_events = self
            .process_transactions(&transactions, &request.wallet_address)
            .await?;

        info!("Processed {} chain events", chain_events.len());

        // Step 4: Build position history
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::BuildingPositions,
            70.0,
        )
        .await?;
        self.build_position_history(&request.wallet_address, &chain_events)
            .await?;

        // Step 5: Detect OOF moments
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::DetectingMoments,
            85.0,
        )
        .await?;
        let moments = self
            .detect_oof_moments(&request.wallet_address, &chain_events)
            .await?;

        info!("Detected {} OOF moments for wallet", moments.len());

        // Step 6: Finalize
        self.update_progress(
            &request.job_id,
            BackfillStatus::InProgress,
            BackfillStage::Finalizing,
            95.0,
        )
        .await?;
        self.finalize_backfill(request, &chain_events, &moments)
            .await?;

        // Complete
        self.update_progress(
            &request.job_id,
            BackfillStatus::Completed,
            BackfillStage::Finalizing,
            100.0,
        )
        .await?;

        info!(
            "Successfully completed backfill for wallet: {}",
            request.wallet_address
        );
        Ok(())
    }

    /// Fetch transaction signatures for a wallet in date range
    #[instrument(skip(self))]
    async fn fetch_transaction_signatures(
        &self,
        wallet: &solana_sdk::pubkey::Pubkey,
        start_date: OffsetDateTime,
        end_date: OffsetDateTime,
    ) -> ApiResult<Vec<solana_sdk::signature::Signature>> {
        debug!("Fetching transaction signatures for wallet: {}", wallet);

        let mut all_signatures = Vec::new();
        let mut before = None;

        // Fetch signatures in batches
        loop {
            let signatures = self
                .rpc_client
                .get_signatures_for_address_with_config(
                    wallet,
                    solana_client::rpc_config::RpcTransactionHistoryConfig {
                        limit: Some(1000),
                        before,
                        until: None,
                        commitment: Some(
                            solana_sdk::commitment_config::CommitmentConfig::confirmed(),
                        ),
                    },
                )
                .await
                .map_err(|e| ApiError::ExternalService {
                    service: "Solana RPC".to_string(),
                    error: e.to_string(),
                })?;

            if signatures.is_empty() {
                break;
            }

            let mut batch_signatures = Vec::new();

            for sig_info in signatures {
                // Parse signature
                let signature = solana_sdk::signature::Signature::from_str(&sig_info.signature)
                    .map_err(|e| ApiError::Internal(format!("Invalid signature: {}", e)))?;

                // Check date range if block_time is available
                if let Some(block_time) = sig_info.block_time {
                    let tx_time = OffsetDateTime::from_unix_timestamp(block_time)
                        .map_err(|e| ApiError::Internal(format!("Invalid block time: {}", e)))?;

                    if tx_time < start_date {
                        // Reached start date, stop fetching
                        return Ok(all_signatures);
                    }

                    if tx_time <= end_date {
                        batch_signatures.push(signature);
                    }
                } else {
                    // No block time, include signature
                    batch_signatures.push(signature);
                }

                before = Some(signature);
            }

            all_signatures.extend(batch_signatures);

            // Rate limiting
            sleep(Duration::from_millis(100)).await;
        }

        Ok(all_signatures)
    }

    /// Fetch transaction details for signatures
    #[instrument(skip(self, signatures))]
    async fn fetch_transaction_details(
        &self,
        signatures: &[solana_sdk::signature::Signature],
        job_id: &str,
    ) -> ApiResult<Vec<solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta>> {
        debug!("Fetching {} transaction details", signatures.len());

        let mut transactions = Vec::new();

        // Fetch in batches to avoid overwhelming RPC
        for (i, batch) in signatures.chunks(20).enumerate() {
            let mut batch_transactions = Vec::new();

            for signature in batch {
                match self
                    .rpc_client
                    .get_transaction_with_config(
                        signature,
                        solana_client::rpc_config::RpcTransactionConfig {
                            encoding: Some(
                                solana_transaction_status::UiTransactionEncoding::JsonParsed,
                            ),
                            commitment: Some(
                                solana_sdk::commitment_config::CommitmentConfig::confirmed(),
                            ),
                            max_supported_transaction_version: Some(0),
                        },
                    )
                    .await
                {
                    Ok(transaction) => batch_transactions.push(transaction),
                    Err(e) => {
                        warn!("Failed to fetch transaction {}: {}", signature, e);
                        continue;
                    }
                }
            }

            transactions.extend(batch_transactions);

            // Update progress
            let progress = 20.0 + (30.0 * (i + 1) as f64 / signatures.chunks(20).len() as f64);
            self.update_progress(
                job_id,
                BackfillStatus::InProgress,
                BackfillStage::FetchingTransactions,
                progress,
            )
            .await?;

            // Rate limiting
            sleep(Duration::from_millis(200)).await;
        }

        Ok(transactions)
    }

    /// Process transactions into chain events
    #[instrument(skip(self, transactions))]
    async fn process_transactions(
        &self,
        transactions: &[solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta],
        wallet_address: &str,
    ) -> ApiResult<Vec<ChainEvent>> {
        debug!(
            "Processing {} transactions into chain events",
            transactions.len()
        );

        let mut chain_events = Vec::new();

        for transaction in transactions {
            match self
                .parse_transaction_to_events(transaction, wallet_address)
                .await
            {
                Ok(mut events) => chain_events.append(&mut events),
                Err(e) => {
                    warn!("Failed to parse transaction: {}", e);
                    continue;
                }
            }
        }

        // Sort events by timestamp
        chain_events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Store events in database
        self.store_chain_events(&chain_events).await?;

        Ok(chain_events)
    }

    /// Parse a single transaction into chain events
    async fn parse_transaction_to_events(
        &self,
        transaction: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta,
        wallet_address: &str,
    ) -> ApiResult<Vec<ChainEvent>> {
        let mut events = Vec::new();

        // Extract basic transaction info
        let signature = transaction
            .transaction
            .signatures
            .first()
            .ok_or_else(|| ApiError::Internal("Transaction missing signature".to_string()))?;

        let block_time = transaction
            .block_time
            .ok_or_else(|| ApiError::Internal("Transaction missing block time".to_string()))?;

        let timestamp = OffsetDateTime::from_unix_timestamp(block_time)
            .map_err(|e| ApiError::Internal(format!("Invalid block time: {}", e)))?;

        // Parse pre and post token balances to detect transfers
        if let (Some(pre_balances), Some(post_balances)) = (
            &transaction.meta.as_ref()?.pre_token_balances,
            &transaction.meta.as_ref()?.post_token_balances,
        ) {
            // Detect token transfers by comparing balances
            let balance_changes = self
                .calculate_balance_changes(pre_balances, post_balances, wallet_address)
                .await?;

            for change in balance_changes {
                let event = ChainEvent {
                    id: Uuid::new_v4().to_string(),
                    signature: signature.clone(),
                    wallet_address: wallet_address.to_string(),
                    timestamp,
                    kind: EventKind::Transfer,
                    action: if change.amount > 0.0 {
                        Action::Buy
                    } else {
                        Action::Sell
                    },
                    token_mint: change.token_mint,
                    token_symbol: change.token_symbol,
                    amount: TokenAmount::from(change.amount.abs()),
                    price_usd: UsdAmount::from(0.0), // Would fetch from price service
                    fee_sol: SolAmount::from(0.0),   // Would calculate from transaction
                    metadata: serde_json::json!({
                        "balance_change": change.amount,
                        "account_index": change.account_index
                    }),
                };
                events.push(event);
            }
        }

        Ok(events)
    }

    /// Calculate balance changes from token balance arrays
    async fn calculate_balance_changes(
        &self,
        pre_balances: &[solana_transaction_status::UiTransactionTokenBalance],
        post_balances: &[solana_transaction_status::UiTransactionTokenBalance],
        wallet_address: &str,
    ) -> ApiResult<Vec<BalanceChange>> {
        let mut changes = Vec::new();

        // Create maps for easier comparison
        let pre_map: HashMap<String, &solana_transaction_status::UiTransactionTokenBalance> =
            pre_balances
                .iter()
                .map(|b| (format!("{}:{}", b.account_index, b.mint.clone()), b))
                .collect();

        for post_balance in post_balances {
            let key = format!("{}:{}", post_balance.account_index, post_balance.mint);

            let pre_amount = pre_map
                .get(&key)
                .and_then(|b| b.ui_token_amount.ui_amount)
                .unwrap_or(0.0);

            let post_amount = post_balance.ui_token_amount.ui_amount.unwrap_or(0.0);
            let amount_change = post_amount - pre_amount;

            if amount_change.abs() > 0.000001 {
                // Ignore dust
                changes.push(BalanceChange {
                    token_mint: post_balance.mint.clone(),
                    token_symbol: "UNKNOWN".to_string(), // Would resolve from token registry
                    amount: amount_change,
                    account_index: post_balance.account_index,
                });
            }
        }

        Ok(changes)
    }

    // Helper methods for remaining functionality

    async fn store_chain_events(&self, events: &[ChainEvent]) -> ApiResult<()> {
        debug!("Storing {} chain events", events.len());

        for event in events {
            sqlx::query!(
                "INSERT INTO chain_events (id, signature, wallet_address, timestamp, kind, action, token_mint, token_symbol, amount, price_usd, fee_sol, metadata) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) ON CONFLICT (id) DO NOTHING",
                event.id, event.signature, event.wallet_address, event.timestamp,
                event.kind as _, event.action as _, event.token_mint, event.token_symbol,
                event.amount.0, event.price_usd.0, event.fee_sol.0, event.metadata
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ApiError::Database(format!("Failed to store chain event: {}", e)))?;
        }

        Ok(())
    }

    async fn build_position_history(
        &self,
        _wallet_address: &str,
        _events: &[ChainEvent],
    ) -> ApiResult<()> {
        // Would integrate with position engine to build historical positions
        debug!("Building position history (placeholder)");
        Ok(())
    }

    async fn detect_oof_moments(
        &self,
        _wallet_address: &str,
        _events: &[ChainEvent],
    ) -> ApiResult<Vec<String>> {
        // Would integrate with detector engine to find OOF moments
        debug!("Detecting OOF moments (placeholder)");
        Ok(vec![])
    }

    async fn finalize_backfill(
        &self,
        request: &BackfillRequest,
        _events: &[ChainEvent],
        _moments: &[String],
    ) -> ApiResult<()> {
        debug!("Finalizing backfill for job: {}", request.job_id);

        // Update wallet metadata
        sqlx::query!(
            "UPDATE wallets SET last_backfill = $1, backfill_status = 'completed' WHERE address = $2",
            OffsetDateTime::now_utc(),
            request.wallet_address
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update wallet metadata: {}", e)))?;

        Ok(())
    }

    async fn fetch_pending_requests(&self) -> ApiResult<Vec<BackfillRequest>> {
        let requests = sqlx::query!(
            "SELECT job_id, wallet_address, start_date, end_date, priority, requested_by, include_token_accounts, include_nft_activity, force_refresh FROM backfill_queue WHERE status = 'queued' ORDER BY priority DESC, created_at ASC LIMIT 20"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to fetch pending requests: {}", e)))?;

        let mut backfill_requests = Vec::new();

        for req in requests {
            let request = BackfillRequest {
                job_id: req.job_id,
                wallet_address: req.wallet_address,
                start_date: req.start_date.map(|d| d.assume_utc()),
                end_date: req.end_date.map(|d| d.assume_utc()),
                priority: serde_json::from_str(&req.priority).unwrap_or(BackfillPriority::Normal),
                requested_by: req.requested_by,
                include_token_accounts: req.include_token_accounts,
                include_nft_activity: req.include_nft_activity,
                force_refresh: req.force_refresh,
            };
            backfill_requests.push(request);
        }

        Ok(backfill_requests)
    }

    async fn update_progress(
        &self,
        job_id: &str,
        status: BackfillStatus,
        stage: BackfillStage,
        progress_pct: f64,
    ) -> ApiResult<()> {
        sqlx::query!(
            "UPDATE backfill_queue SET status = $1, stage = $2, progress_pct = $3, updated_at = $4 WHERE job_id = $5",
            serde_json::to_string(&status)?,
            serde_json::to_string(&stage)?,
            progress_pct,
            OffsetDateTime::now_utc(),
            job_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to update progress: {}", e)))?;

        Ok(())
    }
}

// Supporting types
#[derive(Debug, Clone)]
struct BalanceChange {
    token_mint: String,
    token_symbol: String,
    amount: f64,
    account_index: u8,
}
