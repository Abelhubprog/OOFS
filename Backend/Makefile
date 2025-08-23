include .env

build:
	cargo build --workspace

run-api:
	cargo run -p api

run-indexer:
	cargo run -p indexer

run-workers:
	cargo run -p workers

migrate:
	psql "$$DATABASE_URL" -f db/migrations/0001_tx_raw.sql \
		-f db/migrations/0002_actions.sql \
		-f db/migrations/0003_participants.sql \
		-f db/migrations/0004_positions.sql \
		-f db/migrations/0005_moments_extremes.sql \
		-f db/migrations/0006_prices.sql \
		-f db/migrations/0007_views.sql \
		-f db/migrations/0008_plans_policy.sql \
		-f db/migrations/0009_job_queue.sql \
		-f db/migrations/0010_token_facts.sql

seed:
	psql "$$DATABASE_URL" -f db/seed/plans_seed.sql

dev-up:
	bash scripts/dev_up.sh

test:
	cargo test --workspace
