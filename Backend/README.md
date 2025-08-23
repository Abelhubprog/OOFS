OOF Backend (Rust)

This workspace implements a production-grade backend for analyzing Solana wallets (up to 2 years), computing OOF Moments (S2E, BHD, BadRoute, Idle), and serving REST/SSE APIs plus PNG card rendering.

Key crates:
- crates/shared: config, db pool, telemetry, auth, types
- crates/api: Axum API server (public endpoints, SSE, metrics)
- crates/indexer: Webhook endpoint for Helius; dedup ingest of tx/actions/participants
- crates/workers: background job runner (backfill, compute, price snapshot refresh)
- crates/detectors: position engine + moment detectors
- crates/renderer: SVG to PNG renderer for shareable cards
- crates/anchor_sdk: minimal builders for program-related instructions (extensible)

See Makefile targets for local dev and infra/docker for containerization.

Quick start
- Copy `.env.example` to `.env` and fill `DATABASE_URL`, `RPC_PRIMARY`, `ASSET_BUCKET`, `CDN_BASE`.
- Run migrations and seed: `make migrate && make seed`.
- Start services: `make run-indexer`, `make run-workers`, `make run-api`.
- Webhooks: point Helius to `POST /webhooks/helius` on the indexer.
- Analyze wallets: `POST /v1/analyze` with `{ wallets: ["<pubkey>"] }` (requires JWT if configured).
- Stream moments: `GET /v1/stream/moments`.
- List moments: `GET /v1/moments?wallet=<pubkey>&limit=50`.
- Render card: `GET /v1/cards/moment/<id>.png`.

Security & Quotas
- JWT required on analyze route when `JWT_SECRET` or `DYNAMIC_JWKS_URL` set.
- Daily quotas enforced via `plans` and `policy_state` tables.
- Rate limits: per-IP limiter applied to hot read endpoints.

Notes
- All monetary fields use DECIMAL types and are serialized as strings in responses.
- Object store supports `file://` (dev) and `r2://` (Cloudflare R2). Build with `--features shared/with-r2` or set in Cargo config. R2 requires `R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY` envs.
