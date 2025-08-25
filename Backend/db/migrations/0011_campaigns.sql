-- 0011_campaigns.sql

CREATE TABLE IF NOT EXISTS campaigns (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  budget NUMERIC(38,18) NOT NULL,
  start_date TIMESTAMPTZ NOT NULL,
  end_date TIMESTAMPTZ NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS campaign_actions (
  id TEXT PRIMARY KEY,
  campaign_id TEXT NOT NULL REFERENCES campaigns(id),
  action_type TEXT NOT NULL, -- e.g., 'twitter_like', 'farcaster_recast'
  reward_amount NUMERIC(38,18) NOT NULL,
  max_participants INT
);

CREATE TABLE IF NOT EXISTS campaign_participations (
  id TEXT PRIMARY KEY,
  campaign_action_id TEXT NOT NULL REFERENCES campaign_actions(id),
  user_id TEXT NOT NULL,
  proof_data TEXT, -- e.g., URL to the tweet
  status TEXT NOT NULL DEFAULT 'pending', -- e.g., 'pending', 'approved', 'rejected'
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
