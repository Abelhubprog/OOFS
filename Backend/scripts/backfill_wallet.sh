#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
WALLET=${1:?wallet required}
psql "$DATABASE_URL" -c "INSERT INTO job_queue (id, kind, payload_json) VALUES ('$(uuidgen | tr '[:upper:]' '[:lower:]')','backfill','{"wallet":"$WALLET"}')"
