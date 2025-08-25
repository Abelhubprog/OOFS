
> *You are a senior Rust + TypeScript architect dropped into the OOFS monorepo.
> The repo has **two back-ends** (legacy Node/Express, new Rust/Axum) and a React front-end.
> The Node side still powers several `/api/*` routes but uses mock data; the Rust side already streams real Solana analytics, price-feeds (via Jupiter), job-queues and detectors.
> Your task: eliminate mocks, consolidate around the Rust API, finish the missing MVP pieces, and wire the front-end to real endpoints.*&#x20;

---

#### 1 ➜ Replace mock Solana & Rug logic (Node) with real data

```
📁 Frontend/server/services/solanaService.ts
📁 Frontend/server/services/rugDetectionService.ts
```

* Replace `initializeTokens`, `updateTokenPrices`, `detectNewTokens`, `analyzeTokenRisk` with real calls:

  * Fetch prices/volume from Rust `/v1/tokens/:mint/prices` (or Jupiter API).
  * Discover new tokens via Helius/Birdeye.
  * Compute risk from actual liquidity, holder distribution, contract flags.

#### 2 ➜ Expose “Trending Tokens” in Rust

```
📁 Backend/crates/api/src/routes
```

* Add `/v1/trending/tokens` → SELECT tokens ordered by 24 h volume Δ or VWAP momentum.
* Pagination (`?limit=&cursor=`) with default 20.

#### 3 ➜ Real Campaigns Engine

```
📁 Backend/db/migrations
📁 Backend/crates/api/src/routes/campaigns.rs (new)
📁 Frontend/client/src/features/campaigns/*
```

* Tables: `campaigns`, `campaign_actions`, `campaign_participations`.
* Endpoints: `POST /v1/campaigns` (admin), `GET /v1/campaigns`, `POST /v1/campaigns/:id/participate`.
* Verify actions via Twitter/X & Farcaster APIs.

#### 4 ➜ Unify Auth & Rate-Limiting

* Use Dynamic.xyz JWT across **all** routes; remove wallet-only auth in Node.
* Standardise rate-limits using Rust middleware; Node simply proxies to Rust.

#### 5 ➜ Front-End API swap

* Replace every call in React using `/api/*` with `/v1/*` helpers.
* Implement SSE hook for `/v1/analyze/:job/stream`.
* Add Redux/Zustand slice for trending & campaigns.

#### 6 ➜ NFT mint from Moment PNG (Solana only)

```
📁 Backend/crates/renderer
📁 Backend/crates/workers/src/mint_nft.rs (new)
```

* After PNG saved, mint an SPL token/NFT using Metaplex Candy Guard (not Zora/Base).
* Record mint details in `minted_cards` table, expose via `/v1/moments/:id/nft`.

#### 7 ➜ Delete or proxy obsolete Node routes

* Remove `/api/tokens`, `/api/rug-detection`, `/api/campaigns` once real equivalents exist.
* Keep simple `/api/health` proxying to Rust `/health`.

---

### Execution checklist (order)

1. **Data feed swap** → remove randomness.
2. **Rust trending route** → front-end list.
3. **JWT integration** → single auth flow.
4. **Campaign schema + routes**.
5. **Front-end API refactor & SSE**.
6. **NFT mint worker**.
7. **De-node-ify**: delete mocks, update Docker compose.

Return a PR per milestone with tests & DB migrations; run `cargo test` + `pnpm test` green before each merge.


### 1. Architecture & code reality

* **Two parallel backends:** the repo contains a `Frontend/server` Node/Express API and a new `Backend` written in Rust (Axum).  The Node server was used for early demos; it still handles routes like `/api/tokens` and `/api/rug-detection`, but it uses a local database and mock services (see below).  The Rust backend is designed to be the production backend and has robust modules for authentication, rate limiting, real‑time price retrieval via Jupiter, job queues, detectors for trading mistakes and wallet analytics, and streaming.

* **Node server uses simulated token data:** the `solanaService` populates a `knownTokens` list and generates price/volume updates by applying random volatility to the last price; holder counts and other stats are hard‑coded.  It even creates “new tokens” by generating random addresses.  Likewise, the `RugDetectionService` returns random liquidity, holder and social numbers and uses random values for contract analysis and a simple heuristic to calculate risk.  These mocks are fine for a demo but cannot power a production product.

* **Rust backend processes real data:** the `CompositePriceProvider` in `Backend/crates/detectors/src/prices.rs` fetches token prices from Jupiter’s REST API, caches them in Redis and falls back to database/VWAP data if necessary.  The API’s `analyze` endpoint validates input, enqueues backfill and compute jobs, and estimates completion time; the SSE endpoint streams progress.  Routes like `/v1/moments`, `/v1/wallets/:wallet/summary` and `/v1/tokens/:mint/prices` build SQL queries against Postgres to return real analytics data.  
Background workers process jobs, refresh prices and run detectors without any `todo!` placeholders.  The detectors themselves implement concrete logic for “Sold Too Early” (S2E), “Bag‑Holder Drawdown” (BHD) and other mistake types by comparing entry/exit prices to the peak/trough prices from real price ranges.

### 2. Missing pieces and outdated code

1. **Replace Node mock services with real integrations.**  The Node `solanaService` and `rugDetectionService` generate random token prices, volumes, liquidity and risk scores.  To make the front‑end display real data:

   * Use the Rust backend’s `/v1/tokens/:mint/prices` endpoint to fetch real-time prices, volumes and VWAP data.  You could add a new route in the Rust API to return a list of trending memecoins (e.g. top by volume or velocity).
   * Replace the `detectNewTokens` simulation with a call to a blockchain event indexer (Helius or Birdeye) so new Solana tokens and their metadata are pulled from the network.
   * Replace the random risk analysis with an on‑chain audit service or integrate with existing rug‑pull detection APIs.

2. **Move production analytics to Rust.**  The Node server uses Drizzle ORM to persist tokens, moments, predictions and ads, but most of the analytical features (wallet summary, moment detection, leaderboard) are either stubbed out or not connected to real blockchain data.  The Rust backend already implements:

   * **Wallet analysis** via the `/v1/analyze` endpoint, which queues a job to backfill transactions and compute regret metrics.
   * **Moment streaming** through Server‑Sent Events (SSE) to track analysis progress.
   * **Price refresh** and **job processing** workers that continuously update price data and process queued jobs.
     Consolidating the application around the Rust backend (and exposing any missing routes to the front‑end) removes duplication and ensures that the same detectors and analytics logic are used everywhere.

3. **Fill in unimplemented MVP features.**

   * **Trending memecoins / tokens by volume:** The current code sorts tokens by `volume24h` from the `tokens` table, but there is no dedicated endpoint for trending memecoins.  Add a `/v1/trending/tokens` endpoint in the Rust API that queries the `token_prices` or `tokens` table for 24‑hour volume growth and returns a paginated list.  On the indexer side, compute 24‑hour volume and store it.
   * **Trending pay apps and campaigns engine:** The Node server has a placeholder `/api/campaigns` that returns a mock campaign.  To make this a real feature, you need to design a campaigns table (storing social media tasks, budgets and participation rules), create endpoints for launching campaigns and verifying participation (via X/Twitter, Farcaster etc.), and integrate an on‑chain or off‑chain reward system.  This work is not yet started.
   * **NFT minting and cross‑chain monetization:** The repository mentions Zora and Base integration but there is no code- we only SOLANA NO MULTICHAIN EDIT ALL FILES TALKING OTHERWISE!.  Implement a worker to mint NFTs from generated OOF moments (PNG cards) and use a bridging service to move them across chains. 
   * **Real‑time social engagement & leaderboard:** The Rust `/v1/leaderboard` route exists but the data model behind it isn't shown; ensure it aggregates user scores and achievements from the `moments` and `wallet_analysis` tables.  For real‑time engagement (likes, shares), integrate websockets and persist interactions in a `moment_interactions` table (already defined in `storage.ts`).

4. **Unify authentication and rate‑limiting.**  The Rust backend uses JWT/HMAC authentication and per‑IP rate‑limiting middleware, while the Node server has a simple rate limiter.  Adopt a single authentication model (e.g. JWT with Dynamic.xyz) and share secrets/configs via environment variables; unify rate limits to avoid inconsistent behaviour.

5. **Hardening & observability.**  Both the Node and Rust servers implement metrics and health checks, but unify them.  Use the Rust `observability_mw` to expose `/metrics` endpoints; instrument the React front‑end to report errors.  Add data validation and error handling around job queue operations; implement fallback/resilience for external API failures.

### 3. Roadmap to production with real data

| Area                                | Current status (code)                                                                       | Production‑ready steps                                                                                                                                                              |
| ----------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Token data & trending memecoins** | Node `solanaService` seeds three known tokens and updates prices using random volatility.   | Integrate with the Rust `/v1/tokens/:mint/prices` endpoint; add a `trending` endpoint in Rust that queries real 24‑hour volume/price change; remove random price updates.           |
| **Rug‑pull detection**              | Node `RugDetectionService` generates random liquidity, holder and social metrics.           | Replace with an on‑chain audit/AMM data service; compute risk from actual liquidity pools, holder distribution and contract flags; integrate with Rust detectors if possible.       |
| **Wallet analysis**                 | Rust `/v1/analyze` enqueues jobs and uses detectors for S2E/BHD etc..                       | Expose these endpoints to the front‑end; implement UI to trigger analyses and stream results; ensure indexer correctly ingests Helius webhooks and stores events.                   |
| **Moment cards & NFT minting**      | Rust `card_png` route returns PNG cards from object store; Node has no real implementation. | Use the renderer crate to generate SVG/PNG cards; NOT integration of Zora/Base APIs for minting-REMOVE THIS!; store solana minted NFTs and metadata in the database.                                             |
| **Campaigns & social growth**       | Node returns a hard‑coded campaign.                                                         | Design a campaigns model; build endpoints to create, list and participate in campaigns; integrate social APIs (Twitter, Farcaster) for verification; track participation in the DB. |
| **Authentication & quotas**         | Rust uses JWT & user plans; Node has simple wallet‑based auth.                              | Consolidate auth in Rust; remove redundant auth code in Node; ensure quotas are enforced on the API for analysis and moment streaming.                                              |
| **Front‑end integration**           | React calls Node endpoints (`/api/tokens`, `/api/rug-detection`).                           | Update API calls to use the new Rust endpoints; implement state management for streaming results; integrate websockets for real‑time updates.                                       |

### 4. Summary

The high‑level docs in the repo are indeed outdated compared with the actual code. The Node/Express server still uses simulated token data and mock rug‑pull analysis, while the new Rust backend implements robust real‑data processing with detectors and job queues. To make OOFS production‑ready with real data:

1. **Retire the mock services**—port the front‑end to the Rust API or rewrite the Node services to consume the Rust endpoints or other blockchain APIs.
2. **Fill in missing MVP features** such as trending memecoins, real campaigns, NFT minting and cross‑chain bridging.
3. **Unify authentication, rate limiting and observability** across the stack.
4. **Ensure real‑time data ingestion** via the indexer (Helius webhooks), properly process jobs in the workers and persist the results.

Taking these steps will ensure that every MVP uses real on‑chain data, analytics are accurate, and the application is secure and scalable.
