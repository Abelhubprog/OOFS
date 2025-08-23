read the @prompt.md to uunderstand what we are building, we had skeleton with empty files first and we continue to code the backend, we are coding at production ready only, no stabs, no placeholders, no fallbacks, the backend is a data unicorn for our frontend not in the codebase, but @oofsFRONTENDrepo.md can provide some ideas. the empty skeleton backend folder needs to be build until completion. the prompt and @agent.md should put to perspective what we are building. the backend has already began implementation, check what we have done already after you have understood the project, do not just read high level files and begin coding lol, master the project and continue building, keep a log of what you do in todo.md , plan.md and log.md you have been granted all permissions, do not ask any. persist until when the job is done!

# MASTER BUILD PROMPT — OOF Backend (Rust, 2-year wallet analytics, OOF Moments)

**You are an expert Rust backend engineer.** Implement a production-grade, low-cost backend that analyzes Solana wallets (up to 2 years of SPL history per plan), reconstructs entries/exits/re-entries, computes and ranks OOF Moments (S2E, BHD, BadRoute, Idle, Rug), renders shareable cards, and exposes stable REST/SSE APIs. Prioritize correctness, idempotency, and minimal recurring cost.

## 0) Constraints & Principles

* **Language & Stack:** Rust 1.79+, Tokio async, Axum, SQLx(Postgres `NUMERIC`), `rust_decimal` or `bigdecimal`, `ulid`, `serde`, `time`, `reqwest`, `tracing`, `prometheus`, optional `redis`. Rendering via `askama`/`tera` + `usvg`/`resvg`.
* **Data accuracy:** All amounts in decimal (`NUMERIC(38,18)`); no f32/f64. All prices/time windows deterministic and reproducible.
* **Idempotency:** Every write is idempotent. Primary keys: `(sig)` for tx, `(sig, log_idx)` or ULID for actions/events. Unique indexes prevent dupes.
* **Cost posture:** Dedupe-first ingest (store each signature once), narrow normalized tables (`actions`, `participants`). Store raw JSON compressed in object store. Price buckets only for **mints we touch**. Optional 30-day “hot snapshot”.
* **Security:** Verify Helius webhook HMAC. JWT via Dynamic JWKS or wallet-signature Bearer. Per-IP and per-wallet rate limits. CORS locked to front-end origin.
* **Observability:** `/metrics` Prometheus; structured logs with `tracing`.
* **SLO:** P95 moment availability < 60s post-tx (webhook→detect→render).

---

## 1) Workspace & Files

Create this exact tree and fill with code. (You may already have the skeleton.)

```
backend/
├─ Cargo.toml, rust-toolchain.toml, .env.example, Makefile, README.md
├─ crates/
│  ├─ shared/                # config, db pool, telemetry, security, types
│  ├─ api/                   # Axum public API + SSE/WS
│  ├─ indexer/               # webhook + optional WS subs + price refresher
│  ├─ detectors/             # stream workers (S2E/BHD/BadRoute/Idle/Rug)
│  ├─ workers/               # backfill, finalize, price snapshots, alerts
│  ├─ renderer/              # SVG→PNG cards
│  └─ anchor_sdk/            # ix builders (campaigns/staking/registry)
├─ db/
│  ├─ migrations/            # 0001..0010 (defined below)
│  ├─ queries/               # sqlx `.sql` checked queries
│  └─ seed/plans_seed.sql
├─ configs/                  # detectors.yaml, plans.yaml, rate_limits.yaml
├─ schemas/                  # http schemas (DTO), bus protos
└─ infra/                    # dockerfiles, compose, observability
```

Coding standards:

* Use `sqlx::query_file!` with compile-time checking.
* DTOs in `api::dto`, no floats. Use stringified decimals in JSON responses.
* Keyset pagination for lists (`(t_event DESC, id DESC)`).

---

## 2) Environment Variables (`.env.example`)

```
SOLANA_CLUSTER=mainnet-beta

RPC_PRIMARY=https://rpc.helius.dev/?api-key=...
RPC_SECONDARY=

HELIUS_WEBHOOK_SECRET=...

DATABASE_URL=postgres://...
REDIS_URL=redis://...            # optional but supported

ASSET_BUCKET=s3://oof-cards
CDN_BASE=https://cdn.oof.example

JUPITER_BASE_URL=https://price.jup.ag/v3
PYTH_HERMES_SSE=https://hermes.pyth.network/v2/updates/price/stream

DYNAMIC_JWKS_URL=https://.../.well-known/jwks.json
JWT_SECRET=...

OOF_TOKEN_MINT=...
OOF_STAKING_PROGRAM_ID=...
OOF_CAMPAIGNS_PROGRAM_ID=...
OOF_MOMENT_REGISTRY_PROGRAM_ID=
```

---

## 3) Database Schema (migrations)

Create SQL migrations with these tables and indexes:

### 0001\_tx\_raw\.sql

* `tx_raw(sig PK, slot BIGINT, ts TIMESTAMPTZ, status TEXT, object_key TEXT, size_bytes INT)`

  * `object_key` points to compressed JSON (`tx/<prefix>/<sig>.json.zst`)

### 0002\_actions.sql

* `actions(id PK TEXT, sig FK, log_idx INT, slot BIGINT, ts TIMESTAMPTZ, program_id TEXT, kind TEXT, mint TEXT, amount_dec NUMERIC(38,18), exec_px_usd_dec NUMERIC(38,18), route TEXT, flags_json JSONB)`
* Indexes: `(sig, log_idx)`, `(mint, ts)`, `GIN(flags_json)`

### 0003\_participants.sql

* `participants(sig FK, wallet TEXT, PRIMARY KEY(sig, wallet))`
* Index: `(wallet, sig)`

### 0004\_positions.sql

* `lots(lot_id PK, wallet, mint, episode_id, entry_ts, qty_initial NUMERIC, qty_remaining NUMERIC, entry_px_usd_dec NUMERIC)`
* `realized_trades(exit_id PK, wallet, mint, episode_id, ts, qty NUMERIC, vwavg_exit_px_usd_dec NUMERIC, realized_pnl_usd_dec NUMERIC, sig)`

  * Index: `(wallet, ts DESC)`
* `episodes(episode_id PK, wallet, mint, start_ts, end_ts, basis_usd_dec NUMERIC, realized_pnl_usd_dec NUMERIC, roi_pct_dec NUMERIC)`

  * Index: `(wallet, mint, start_ts)`

### 0005\_moments\_extremes.sql

* `oof_moments(id PK, wallet, mint, kind, t_event, window, pct_dec NUMERIC, missed_usd_dec NUMERIC, severity_dec NUMERIC, sig_ref, slot_ref BIGINT, version TEXT, explain_json JSONB, preview_png_url TEXT, nft_minted BOOL DEFAULT FALSE, nft_mint TEXT)`

  * Index: `(wallet, t_event DESC)`, `(kind, t_event DESC)`
* `wallet_extremes(wallet PK, computed_at TIMESTAMPTZ, json JSONB)`

### 0006\_prices.sql

* `token_prices(mint, ts, price NUMERIC, source TEXT, PRIMARY KEY(mint, ts))`

### 0007\_views.sql

* Create **materialized views**:

  * `mv_price_1m` (≤7d), `mv_price_5m` (7–180d), `mv_price_1h` (180d–2y)
* Consider BRIN on `ts` for large ranges.

### 0008\_plans\_policy.sql

* `plans(code PK, price_usd_dec NUMERIC(10,2), daily_wallets INT, backfill_days INT, cadence TEXT, alerts INT, api_rows BIGINT, perks_json JSONB)`
* `user_plans(user_id, plan_code FK, started_at, expires_at, PRIMARY KEY(user_id, plan_code))`
* `policy_state(user_id PK, analyses_today INT DEFAULT 0, last_reset_at TIMESTAMPTZ)`
* `wallet_cursors(wallet PK, from_ts TIMESTAMPTZ, to_ts TIMESTAMPTZ, last_cursor_sig TEXT)`

### 0009\_job\_queue.sql

* `job_queue(id PK, kind TEXT, payload_json JSONB, run_after TIMESTAMPTZ, attempts INT DEFAULT 0, max_attempts INT DEFAULT 5, locked_by TEXT, locked_at TIMESTAMPTZ, status TEXT DEFAULT 'queued', created_at TIMESTAMPTZ DEFAULT NOW())`

### 0010\_token\_facts.sql

* `token_facts(mint PK, symbol TEXT, decimals INT, logo_url TEXT, ath_price NUMERIC, atl_price NUMERIC, liquidity_band TEXT, updated_at TIMESTAMPTZ)`

Seed: `plans_seed.sql` with FREE/LITE/PRO defaults.

---

## 4) Ingest & Dedupe

### Indexer (crates/indexer)

* Route: `POST /webhooks/helius`

  * Verify HMAC signature.
  * For each signature:

    * If `tx_raw.sig` absent → store compressed raw (object store) + insert `tx_raw`.
    * Flatten provider actions → insert `actions` (idempotent by `(sig, log_idx)`).
    * Insert `participants(sig, wallet)` for all involved pubkeys.
* Optional: `logsSubscribe` / `accountSubscribe` for your Anchor PDAs (staking/campaigns); map to `actions(kind='anchor_event')`.
* Price refresher: prefetch recent candles for **touched mints** (Jupiter v3); persist in `token_prices` and MVs.

**Idempotency:** All inserts must be upserts (conflict do nothing/update). Use ULID for IDs.

---

## 5) Backfill Worker (crates/workers)

* Input: wallet, `from_ts = now - min(plan.backfill_days, 730d)`.
* Step 1: **Coverage check** against `participants` for `(wallet, ts ≥ from_ts)`.
* Step 2: If gaps → page `getSignaturesForAddress` newest→oldest until `from_ts` or per-run ceilings.
* Step 3: For each `sig`:

  * If `tx_raw` exists → skip fetch.
  * Else fetch, compress to object store, insert `tx_raw`, `actions`, `participants`.
* Step 4: Enqueue **compute** (positions + detectors) for this wallet and time window.
* Ceilings per plan: `max_signatures`, `max_enhanced_tx`, `max_cpu_ms`, `max_candle_reads`. On hitting ceilings, return partial and enqueue continuation (no extra daily quota).

---

## 6) Position Engine (crates/detectors::position)

* Deterministic per `(wallet, mint)`:

  * State: `exposure`, `lots(VecDeque<Lot>)`, `episode_id`, `realized_pnl`.
* Transitions:

  * **BUY**: if exposure==0 → start episode; push lot; update avg cost.
  * **SELL/OUT**: pop lots FIFO, compute realized PnL per fill → `realized_trades`; if exposure→0 → close episode (`episodes`).
  * **TRANSFER self**: move basis; no PnL.
  * **TRANSFER to CEX**: treat as realization at mid (configurable).
* Snapshots every N=500 events for reboot speed. Persist `lots`, `realized_trades`, `episodes`.

---

## 7) Price Provider (crates/detectors::prices)

* Primary: **Jupiter v3**. Persist buckets: **1m** (≤7d), **5m** (7–180d), **1h** (180d–2y).
* Optional realtime ticks (SOL/USDC) via **Pyth Hermes SSE**; cache for API/UI only.
* Fallback: VWAP from observed swaps vs SOL/USDC if external price missing; mark confidence in `explain_json`.

API:

```rust
trait PriceProvider {
  fn at_minute(&self, mint: &str, t: OffsetDateTime) -> Option<Decimal>;
  fn max_in(&self, mint: &str, t0: OffsetDateTime, t1: OffsetDateTime) -> Option<(OffsetDateTime, Decimal)>;
  fn min_in(&self, mint: &str, t0: OffsetDateTime, t1: OffsetDateTime) -> Option<(OffsetDateTime, Decimal)>;
}
```

---

## 8) Detectors (crates/detectors)

General:

* Each detector implements:

```rust
pub trait Detector {
  fn name(&self) -> &'static str;
  fn version(&self) -> u8;
  fn process(&self, ev: &ChainEvent, ctx: &Ctx) -> anyhow::Result<Option<Moment>>;
}
```

* **S2E (Sold-Too-Early)** — triggered on **exit events**:

  * `peak = max_in(mint, [t_exit, t_exit+7d])`
  * `missed_usd = qty_exited * (peak - exit_px)`, `missed_pct = peak/exit - 1`
  * Emit if `missed_pct ≥ 0.25 && missed_usd ≥ 25`
  * `severity = clamp((missed_pct)/0.75, 0..1)`
* **BHD (Bag-Holder Drawdown)** — triggered on **buy lots**:

  * `trough = min_in(mint, [t_buy, t_buy+7d])`
  * `dd_pct = trough/entry - 1` (≤ −0.30 → emit)
  * `severity = clamp((|dd_pct| - 0.30)/0.50, 0..1)`
* **BadRoute** — every swap:

  * Compare executed mid vs best Jupiter quote @ block-minute.
  * `worse_pct = exec_mid/best_mid - 1` (≥ 0.01 → emit)
* **Idle Yield** — continuous OOF balance windows:

  * `missed = bal * apr * days/365 * avg_px` (≥ 25 → emit)
* **Rug** (optional) — from `token_facts` (ATH collapse + persistence); if holding across onset → emit worst.

All moments write to `oof_moments` and publish `"new_oof_moment"` SSE/WS.

---

## 9) Ranking “Highest” Moments & Extremes

* **Highest win/loss (\$)** from `realized_trades`:

```sql
SELECT * FROM realized_trades WHERE wallet=$1 ORDER BY realized_pnl_usd_dec DESC LIMIT 1;
SELECT * FROM realized_trades WHERE wallet=$1 ORDER BY realized_pnl_usd_dec ASC  LIMIT 1;
```

* **Top S2E**:

```sql
SELECT * FROM oof_moments WHERE wallet=$1 AND kind='S2E'
ORDER BY missed_usd_dec DESC, pct_dec DESC LIMIT 1;
```

* Worst **BHD** (pct), **BadRoute** (pct), largest **Idle** (USD) analogous.
* Cache to `wallet_extremes` (`computed_at`, `json`) with TTL 10–60 min.

---

## 10) Public API (crates/api)

Routes (Axum):

* `POST /v1/analyze` → `{ wallets:[pubkey], planCode? }`

  * AuthN (JWT or wallet sig) + AuthZ (quota).
  * Enqueue backfill/compute with ceilings based on plan.
  * Return `{ jobId }`.
* `GET /v1/analyze/:job/stream` (SSE): events `progress`, `moment`, `done`.
* `GET /v1/moments?wallet=&kinds=&since=&min_usd=&limit=&cursor=`

  * Keyset pagination (`(t_event, id)`), string decimals.
* `GET /v1/moments/:id` → detailed DTO with provenance (`sig_ref`, `slot_ref`, `price_source`, confidence).
* `GET /v1/wallets/:pub/summary` → holdings (current), realized PnL, counts by kind, last analyzed range.
* `GET /v1/wallets/:pub/extremes` → top win/loss/S2E/BHD/BadRoute/Idle with `card.shareImageUrl`.
* `GET /v1/cards/moment/:id.png?theme=&size=` → PNG (renderer crate).
* `POST /v1/cards/moment/:id/mint` → returns unsigned base64 instruction(s) or tx for user wallet to sign.
* `GET /v1/tokens/:mint/prices?tf=1m|5m|1h&since=ISO`
* `GET /v1/leaderboard?period=7d|30d`
* `GET /v1/health` (DB/RPC/Redis checks), `/metrics` (Prometheus).

Middleware:

* `auth.rs` (Dynamic JWKS / wallet-sig), `rate_limit.rs`, `errors.rs`.
* SSE/WS multiplexing in `ws_sse.rs` (topics: `"new_oof_moment"`, `"price_tick"`, `"campaign_update"`).

DTO rules:

* All numeric amounts serialized as strings.
* Include `moment.version`, `price_source`, and `confidence` in responses.

---

## 11) Renderer (crates/renderer)

* Provide `render_moment_card(moment: MomentDTO, theme: &str, size: &str) -> Result<Vec<u8>>`.
* Implement with SVG templates + `resvg` → PNG **1200×630** default.
* Store to object store at `cards/moments/{id}.png`; return `CDN_BASE/...`.

---

## 12) Plans, Quotas, Cost Control

`configs/plans.yaml` (example):

* **FREE**: `daily_wallets=2`, `backfill_days=180`, `cadence=manual`, `alerts=0`, `api_rows=0`
* **LITE** (\~\$2): `daily_wallets=5`, `backfill_days=365`, `cadence=weekly`
* **PRO** (\~\$10): `daily_wallets=25`, `backfill_days=730`, `cadence=daily`
* **Boosts** with \$OOF staking: +50% daily wallets, −50% card mint fee, +alerts, fast lane priority.

Per-run ceilings enforced in workers; on hit, return partial + enqueue continuation (no extra quota).

---

## 13) Tests & Fixtures

* `crates/detectors/src/tests/golden_wallet_4y.json`: synthetic 4-year wallet covering re-entries, profits, losses, rugs, bad routes.
* `detectors_test.rs`: asserts counts and top extremes (S2E/BHD etc), reproducible given fixed price windows.
* SQLx offline (`.sqlx/`) for query checking.
* Unit tests for HMAC verify, JWT JWKS cache, price bucket choose logic.

---

## 14) DevOps

* Dockerfiles per crate + `infra/docker/docker-compose.dev.yml` (Postgres, API, Indexer, Detectors).
* Make targets: `dev-up`, `migrate`, `seed`, `run-api`, `run-indexer`, `run-detectors`, `test`.
* Prometheus config + example Grafana dashboard JSON.
* Health checks for Railway/Fly deployments.

---

## 15) Acceptance Criteria (what “done” means)

1. **Paste wallet → Analyze (2y)** returns streaming SSE with progress and at least one detected moment on the golden fixture within **60s** locally.
2. **Moments list** paginates and returns stringified decimals; provenance includes `sig_ref`, `slot_ref`, `price_source`.
3. **Extremes endpoint** returns top win/loss/S2E/BHD/BadRoute/Idle with stable ordering and ties resolved deterministically.
4. **Card PNG** renders successfully and caches at `preview_png_url`.
5. **Backfill dedupe** verified: second analysis of a wallet that shares signatures **does not** call RPC for already-stored `sig`.
6. **Plans/quotas** enforced; FREE limited to 2 wallets/day and 180d; PRO allows 2y depth.
7. **Observability**: `/metrics` exposes ingest rate, detector counts, queue lag, render count; `/v1/health` green.

---

## 16) Implementation Order (for the agent)

1. Migrations 0001..0010 + seed plans.
2. `crates/shared`: config, db pool, telemetry, security (HMAC/JWKS), common types.
3. Indexer webhook → normalize + store (`tx_raw`, `actions`, `participants`).
4. Workers backfill (wallet-first dedupe) + price buckets (Jupiter v3).
5. Position engine + derived tables (lots, realized\_trades, episodes).
6. Detectors (S2E, BHD, BadRoute, Idle; optional Rug) + SSE publish.
7. API routes (analyze, moments, extremes, wallet summary).
8. Renderer (SVG→PNG) + `/cards/moment/:id.png`.
9. AuthZ, rate limits, quotas; metrics; health.
10. Tests: golden wallet fixture, detectors, extremes.

**Deliver code that compiles with `cargo build` and passes `cargo test`.** Favor small, well-typed modules. Document public functions.

