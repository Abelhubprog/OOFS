# OOFSOL Platform

## Overview
OOFSOL is a full-stack platform for analyzing, visualizing, and monetizing crypto trading moments, with a focus on Solana. It combines advanced AI, real-time analytics, and viral content generation to create the world's first "Regret Economy"—turning trading mistakes into shareable, monetizable art and insights.

---

## Backend (Rust)
- **Purpose:** Detect, analyze, and render "OOF Moments" (trading mistakes) from Solana wallets.
- **Key Crates:**
  - `shared`: config, db pool, telemetry, auth, types
  - `api`: Axum API server (REST/SSE endpoints, metrics)
  - `indexer`: Webhook ingest for Helius, deduplication, participant tracking
  - `workers`: Background jobs (backfill, compute, price refresh)
  - `detectors`: Position engine + moment detectors
  - `renderer`: SVG→PNG card rendering
  - `anchor_sdk`: Builders for program instructions
- **Tech:** Rust, PostgreSQL, Redis, Cloudflare R2, Railway.com, JWT/Dynamic.xyz auth
- **Endpoints:**
  - Analyze wallets: `POST /v1/analyze`
  - Stream moments: `GET /v1/stream/moments`
  - Render card: `GET /v1/cards/moment/<id>.png`
- **Security:** JWT, quotas, rate limits, HMAC service auth

---

## Frontend (TypeScript/React)
- **Purpose:** Interactive web app for wallet analysis, moment visualization, campaign creation, and viral sharing.
- **Key Features:**
  - OOF Moments Generation Engine (multi-agent AI orchestration)
  - Wallet Analyzer Psychology Engine (trading DNA, regret quantification)
  - OOF Campaigns Viral Engine (community growth, token integration)
  - Real-time social engagement (WebSockets)
  - NFT minting and cross-chain monetization (Solana→Base)
- **Tech:** React, Vite, Tailwind CSS, shadcn/ui, TanStack Query, Wouter, Drizzle ORM
- **Structure:**
  - `client/src/components`: Reusable UI components
  - `client/src/pages`: Feature pages (OOFMoments, WalletAnalyzer, OOFsCampaigns, etc.)
  - `client/src/services`: API calls, business logic
  - `client/src/hooks`: Custom React hooks
  - `server/`: Express.js backend, AI orchestrator, routes, middleware
  - `shared/`: Types and schemas
- **Setup:**
  - Copy `.env.example` to `.env` and configure keys
  - `npm install` then `npm run dev`

---

## Strategic Innovations
- **Emotional AI:** Psychological profiling of traders for personalized content and marketing
- **Viral Content Factory:** Multi-agent AI generates and optimizes shareable moments
- **Cross-Chain Monetization:** Bridge Solana trading pain to Base NFTs for new revenue streams
- **B2B Data Licensing:** Emotional intelligence as a service

---

## Quick Start
1. **Backend:**
   - Configure `.env`, run migrations/seed, start services with Makefile
2. **Frontend:**
   - Configure `.env`, install dependencies, run dev server
3. **API Usage:**
   - Analyze wallets, stream moments, render cards, create campaigns

---

## License
MIT

## Authors & Contributors
See individual crate and frontend docs for details.
