#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source .env 2>/dev/null || true
psql "$DATABASE_URL" -f db/seed/plans_seed.sql
