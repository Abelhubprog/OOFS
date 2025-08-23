#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source .env 2>/dev/null || true
psql "$DATABASE_URL" -f db/migrations/0001_tx_raw.sql \
  -f db/migrations/0002_actions.sql \
  -f db/migrations/0003_participants.sql \
  -f db/migrations/0004_positions.sql \
  -f db/migrations/0005_moments_extremes.sql \
  -f db/migrations/0006_prices.sql \
  -f db/migrations/0007_views.sql \
  -f db/migrations/0008_plans_policy.sql \
  -f db/migrations/0009_job_queue.sql \
  -f db/migrations/0010_token_facts.sql
