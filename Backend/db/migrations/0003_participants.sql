-- 0003_participants.sql
CREATE TABLE IF NOT EXISTS participants (
  sig TEXT NOT NULL,
  wallet TEXT NOT NULL,
  PRIMARY KEY (sig, wallet)
);

CREATE INDEX IF NOT EXISTS idx_participants_wallet_sig ON participants(wallet, sig);
