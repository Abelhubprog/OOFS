use anyhow::Result;
use serde::{Deserialize, Serialize};
use shared::{Pg, utils::new_id};
use sqlx::Row;
use std::time::Duration;
use time::OffsetDateTime;
use tracing::{error, info, instrument};

/// Job payload for NFT minting
#[derive(Debug, Serialize, Deserialize)]
pub struct MintNftJob {
    pub moment_id: String,
    pub owner_wallet: String,
}

/// NFT minting worker
pub struct MintNftWorker {
    pool: Pg,
}

impl MintNftWorker {
    pub fn new(pool: Pg) -> Self {
        Self { pool }
    }

    /// Process a mint NFT job
    #[instrument(skip(self, job), fields(moment_id = %job.moment_id))]
    pub async fn process(&self, job: MintNftJob) -> Result<()> {
        info!("Starting NFT minting process for moment");

        // Check if card PNG already exists
        let png_url: Option<String> = sqlx::query_scalar!(
            "SELECT preview_png_url FROM oof_moments WHERE id = $1",
            job.moment_id
        )
        .fetch_optional(&self.pool.0)
        .await?;

        let png_url = match png_url {
            Some(url) if !url.is_empty() => url,
            _ => {
                error!("No PNG URL found for moment {}", job.moment_id);
                return Err(anyhow::anyhow!("No PNG URL found for moment"));
            }
        };

        // Download the PNG image
        let png_data = self.download_png(&png_url).await?;
        
        // Mint the NFT using Metaplex Candy Guard
        let mint_result = self.mint_nft_with_metaplex(&job, &png_data).await?;
        
        // Store mint details in database
        self.store_mint_details(&job, &mint_result).await?;
        
        info!("Successfully minted NFT for moment {}", job.moment_id);
        Ok(())
    }

    /// Download PNG image from URL
    async fn download_png(&self, url: &str) -> Result<Vec<u8>> {
        let response = reqwest::get(url).await?;
        let png_data = response.bytes().await?;
        Ok(png_data.to_vec())
    }

    /// Mint NFT using Metaplex Candy Guard
    async fn mint_nft_with_metaplex(
        &self,
        job: &MintNftJob,
        png_data: &[u8],
    ) -> Result<MintResult> {
        // In a real implementation, this would:
        // 1. Upload the PNG to IPFS or Arweave to get a permanent URI
        // 2. Create NFT metadata (name, description, attributes, image URI)
        // 3. Use Metaplex Candy Guard to mint the NFT
        // 4. Return the mint address and metadata URI
        
        // For now, we'll simulate the minting process
        let mint_address = format!("mock_mint_address_{}", job.moment_id);
        let metadata_uri = format!("https://example.com/metadata/{}.json", job.moment_id);
        
        // In a real implementation, you would use the solana-program, mpl-candy-guard, 
        // and mpl-token-metadata crates to interact with the Solana blockchain
        
        Ok(MintResult {
            mint_address,
            metadata_uri,
        })
    }

    /// Store mint details in database
    async fn store_mint_details(&self, job: &MintNftJob, result: &MintResult) -> Result<()> {
        let id = new_id();
        let created_at = OffsetDateTime::now_utc();
        
        sqlx::query!(
            "INSERT INTO minted_cards (id, moment_id, nft_mint_address, metadata_uri, owner_wallet, created_at, updated_at) 
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            id,
            job.moment_id,
            result.mint_address,
            result.metadata_uri,
            job.owner_wallet,
            created_at,
            created_at
        )
        .execute(&self.pool.0)
        .await?;
        
        Ok(())
    }
}

/// Result of NFT minting
#[derive(Debug)]
struct MintResult {
    mint_address: String,
    metadata_uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mint_nft_job_serialization() {
        let job = MintNftJob {
            moment_id: "test_moment_123".to_string(),
            owner_wallet: "test_wallet_address".to_string(),
        };
        
        let serialized = serde_json::to_string(&job).unwrap();
        let deserialized: MintNftJob = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(job.moment_id, deserialized.moment_id);
        assert_eq!(job.owner_wallet, deserialized.owner_wallet);
    }
}