-- 0012_minted_cards.sql
-- Table to store minted NFT card information

CREATE TABLE IF NOT EXISTS minted_cards (
  id TEXT PRIMARY KEY,
  moment_id TEXT NOT NULL REFERENCES oof_moments(id),
  nft_mint_address TEXT NOT NULL,
  metadata_uri TEXT,
  owner_wallet TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_minted_cards_moment_id ON minted_cards(moment_id);
CREATE INDEX IF NOT EXISTS idx_minted_cards_owner_wallet ON minted_cards(owner_wallet);
CREATE INDEX IF NOT EXISTS idx_minted_cards_nft_mint_address ON minted_cards(nft_mint_address);