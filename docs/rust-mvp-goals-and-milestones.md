# Rust MVP Goals and Milestones

Project: Prediction Market Trade Copier
Date: 2026-05-17
Source PRD: `prediction-market-trade-copier-prd.md`
Build language: Rust-first backend and worker stack

## 1. MVP Mission

Build a same-platform prediction market trade copier that can monitor configured master traders on Polymarket and Kalshi, normalize their trades into a shared signal model, apply conservative filters and risk controls, and place scaled copy trades into authorized follower accounts.

The MVP must prove four things:

1. We can reliably detect master activity.
2. We can make deterministic copy or skip decisions.
3. We can execute follower orders without duplicate orders.
4. We can reconcile fills, positions, P&L, and latency after execution.

The MVP should start in dry-run mode, then move to live small-notional trading only after replay tests, sandbox tests, and reconciliation tests pass.

## 2. MVP Boundaries

### In Scope

- Rust backend services and workers.
- PostgreSQL for durable state.
- Redis Streams for queues, dedupe cache, locks, and token buckets.
- Polymarket same-platform copy:
  - master monitoring through Data API first
  - market/orderbook context through CLOB and WebSocket where available
  - follower execution through Polymarket CLOB client integration or direct API implementation
- Kalshi same-platform copy:
  - authorized master monitoring through Kalshi WebSocket fills or REST fallback
  - follower execution through Kalshi REST API
- Multi-master monitoring for 10+ masters.
- Multi-follower fanout for up to 5 followers.
- Conservative risk controls.
- Dashboard API for state inspection and controls.
- Simple web dashboard can be added after the backend read models are stable.
- Dry-run mode and live mode.

### Out of Scope for MVP

- Cross-platform routing or arbitrage.
- Autonomous strategy generation.
- Copying unauthorized Kalshi accounts.
- Polymarket legacy subgraph dependency as production source of truth.
- Negative-risk and complex multi-outcome strategy handling.
- Advanced portfolio optimizer.
- Rust ultra-low-latency optimization before correctness is proven.
- Mobile app.

## 3. Rust Architecture Decision

Use a Rust workspace with small crates that map directly to the PRD layers. Keep domain logic independent from platform adapters so the risk engine and signal engine can be tested without live APIs.

Recommended workspace layout:

```text
arbitek-trade-copier/
  Cargo.toml
  crates/
    domain/              # core types, state machines, validation
    config/              # environment and typed app config
    db/                  # sqlx repositories and migrations
    queue/               # Redis Streams abstraction
    platform-polymarket/ # Polymarket API, WS, CLOB execution
    platform-kalshi/     # Kalshi REST, WS, signing, execution
    ingest/              # master monitoring and raw event ingestion
    signal/              # normalization, enrichment, consensus-lite
    risk/                # filters, sizing, portfolio limits
    execution/           # copy intent fanout and order orchestration
    reconcile/           # fills, positions, P&L, settlement checks
    api/                 # Axum dashboard/admin API
    worker/              # background worker binary
    cli/                 # admin/dev CLI
  migrations/
  docker-compose.yml
  .env.example
  docs/
```

Recommended Rust stack:

| Concern | Rust choice |
|---|---|
| Async runtime | `tokio` |
| HTTP server | `axum` |
| HTTP client | `reqwest` |
| WebSocket client | `tokio-tungstenite` |
| Database | `sqlx` with PostgreSQL |
| Redis | `redis` async client or `deadpool-redis` |
| JSON | `serde`, `serde_json` |
| Decimal money/prices | `rust_decimal` |
| IDs | `uuid` |
| Time | `time` or `chrono` with UTC only |
| Errors | `thiserror` in libraries, `anyhow` in binaries |
| Config | `config`, `dotenvy` |
| Secrets in process | `secrecy` |
| Logging/tracing | `tracing`, `tracing-subscriber`, `opentelemetry` later |
| CLI | `clap` |
| Tests | Rust unit tests, integration tests, `wiremock`, `testcontainers` optional |

Engineering rules:

- Use `Decimal`, never `f64`, for price, size, notional, P&L, and balances.
- Use UTC timestamps everywhere.
- Every external event gets an idempotency key before processing.
- Every order gets a deterministic `client_order_id`.
- All platform-native payloads are stored as JSON for audit and replay.
- Every copy decision must be replayable from stored inputs.
- Platform adapters must not contain business/risk logic.
- Risk engine must not call external APIs directly; it consumes enriched inputs.
- Executor must fail closed if secrets, orderbook, market metadata, or risk approval are missing.

## 4. MVP Success Criteria

The MVP is complete when:

- 10+ master accounts can be configured.
- 1 to 5 follower accounts can be configured.
- Raw master events are stored without duplicates.
- Normalized signals are created with platform, market, outcome, side, size, price, timestamp, and master.
- Risk engine can approve, reject, and explain decisions.
- Dry-run mode produces copy intents without placing orders.
- Live mode can place small test orders on both platforms.
- Duplicate ingestion events do not create duplicate follower orders.
- Reconciliation updates order status, fills, positions, and P&L.
- Dashboard/API shows activity feed, execution log, positions, P&L, alerts, and pause controls.
- End-to-end p95 detection-to-submit latency is under 5 seconds in normal conditions.

## 5. Layer-by-Layer Goals

### Layer 0: Product, Compliance, and Access

Goal:

Make sure the MVP can be legally and operationally tested before engineering spends time on live execution.

What to do:

- Confirm platform accounts are available for Polymarket and Kalshi.
- Confirm follower accounts are authorized for automated trading.
- Confirm Kalshi masters are authorized account integrations; do not assume public trader identity.
- Decide which accounts run live and which run dry-run.
- Decide initial bankroll and max loss for live pilot.
- Create API key and wallet key handling policy.
- Define kill switch owner and escalation process.

Deliverables:

- `docs/access-and-compliance-checklist.md`
- List of master accounts and their platform access mode.
- List of follower accounts and live/dry-run status.
- Initial MVP risk configuration.

Acceptance criteria:

- No live order placement starts until account access, ToS review, and key storage approach are approved.

### Layer 1: Repository and Local Infrastructure

Goal:

Create a Rust workspace that can run locally with PostgreSQL, Redis, migrations, and a worker process.

What to do:

- Initialize Cargo workspace.
- Add crates listed in the architecture section.
- Add Docker Compose for PostgreSQL and Redis.
- Add `.env.example`.
- Add app config loader.
- Add structured logging with `tracing`.
- Add healthcheck endpoint in `api`.
- Add worker binary that can start and idle cleanly.
- Add migration runner.
- Add CI-ready commands:
  - `cargo fmt`
  - `cargo clippy`
  - `cargo test`
  - `sqlx migrate run`

Deliverables:

- Rust workspace.
- Local dev stack.
- First migration.
- Empty API and worker binaries.

Acceptance criteria:

- `cargo test` passes.
- Local Postgres and Redis start.
- API returns `/healthz`.
- Worker starts without panics and logs service status.

### Layer 2: Domain Model

Goal:

Create strongly typed Rust models for the shared copier language before integrating external APIs.

What to do:

- Define platform enum:
  - `Polymarket`
  - `Kalshi`
- Define core IDs:
  - `MasterId`
  - `FollowerId`
  - `FollowerAccountId`
  - `MarketId`
  - `SignalId`
  - `CopyIntentId`
  - `OrderId`
- Define money and price value types:
  - `Price`
  - `Contracts`
  - `Notional`
  - `Pnl`
- Define normalized models:
  - `RawEvent`
  - `TradeSignal`
  - `Market`
  - `OrderbookSnapshot`
  - `RiskDecision`
  - `CopyIntent`
  - `ExecutionRequest`
  - `ExecutionReport`
  - `Position`
  - `PnlSnapshot`
- Define order lifecycle state machine:
  - `Pending`
  - `Approved`
  - `Queued`
  - `Submitted`
  - `PartiallyFilled`
  - `Filled`
  - `Rejected`
  - `Cancelled`
  - `Expired`
  - `Reconciled`
- Define validation:
  - price between 0 and 1
  - positive size
  - nonempty market ID
  - timestamp sanity
  - side/outcome compatibility

Deliverables:

- `crates/domain`
- Unit tests for validation and state transitions.

Acceptance criteria:

- All domain models serialize and deserialize with `serde`.
- State transitions reject invalid moves.
- Price and notional math uses `Decimal`.

### Layer 3: Database and Persistence

Goal:

Persist all durable MVP state and make every decision auditable.

What to do:

- Add SQL migrations based on the PRD schema.
- Implement repositories for:
  - masters
  - master accounts
  - followers
  - follower accounts
  - markets
  - raw events
  - signals
  - copy intents
  - orders
  - fills
  - positions
  - alerts
  - audit log
- Add unique constraints for idempotency:
  - raw event key
  - platform client order ID
  - copy intent idempotency key
- Add transaction helpers for:
  - insert raw event and enqueue signal work
  - create copy intent and execution request
  - update order plus fills atomically

Deliverables:

- `crates/db`
- SQL migrations.
- Repository tests.

Acceptance criteria:

- Duplicate raw event insert is handled safely.
- Duplicate client order ID cannot create a second order.
- Repositories can run against local Postgres.

### Layer 4: Queue and Worker Runtime

Goal:

Create reliable internal movement between ingestion, signal, risk, execution, and reconciliation.

What to do:

- Implement Redis Streams abstraction.
- Define stream names:
  - `raw.events`
  - `signals.normalized`
  - `copy.intents`
  - `execution.requests`
  - `execution.reports`
  - `alerts`
- Add consumer groups.
- Add retry and dead-letter strategy.
- Add message envelope:
  - `message_id`
  - `correlation_id`
  - `event_type`
  - `payload`
  - `created_at`
  - `attempt`
- Add worker supervisor that runs selected consumers.

Deliverables:

- `crates/queue`
- `crates/worker`
- Queue integration tests.

Acceptance criteria:

- A raw event message can be produced and consumed.
- Failed messages retry.
- Poison messages land in dead-letter stream.

### Layer 5: Market Metadata Layer

Goal:

Keep a local market catalog so signals and execution can map platform IDs to outcomes and tradeable instruments.

What to do:

- Polymarket:
  - fetch Gamma/Data/CLOB market metadata needed for MVP
  - store condition ID, token IDs, event slug, question, outcomes, status, close time, category
- Kalshi:
  - fetch markets and events
  - store ticker, event ticker, title, rules, status, close time, settlement fields
- Add periodic sync jobs.
- Add metadata refresh on cache miss.
- Add category mapping.
- Add MVP exclusion flags:
  - multi-outcome unsupported
  - negative-risk unsupported
  - ambiguous mapping unsupported

Deliverables:

- Metadata sync workers.
- Market repository populated from both platforms.

Acceptance criteria:

- Given a Polymarket asset/token ID, system resolves outcome and market.
- Given a Kalshi ticker, system resolves outcome semantics and market.
- Closed or unsupported markets are marked non-tradeable.

### Layer 6: Platform Adapters

Goal:

Implement platform-specific API access behind clean Rust traits.

Core traits:

```rust
trait MarketDataClient {
    async fn get_market(&self, market_ref: &str) -> Result<PlatformMarket>;
    async fn get_orderbook(&self, market_ref: &str) -> Result<OrderbookSnapshot>;
}

trait MasterActivityClient {
    async fn poll_recent_trades(&self, account_ref: &str) -> Result<Vec<PlatformTrade>>;
}

trait ExecutionClient {
    async fn submit_order(&self, request: PlatformOrderRequest) -> Result<PlatformOrderAck>;
    async fn cancel_order(&self, platform_order_id: &str) -> Result<CancelAck>;
}
```

What to do for Polymarket:

- Implement Data API trade polling for master wallets.
- Implement CLOB market/orderbook reads.
- Implement follower order placement only after dry-run path is working.
- Implement order signing integration.
- Implement rate-limit backoff.
- Store raw platform responses.

What to do for Kalshi:

- Implement authenticated REST client.
- Implement RSA-PSS request signing.
- Implement authorized user fills WebSocket.
- Implement REST fallback for fills.
- Implement market/orderbook reads.
- Implement order placement with client order ID.
- Store raw platform responses.

Deliverables:

- `crates/platform-polymarket`
- `crates/platform-kalshi`
- Mockable trait interfaces.
- Unit tests using fixture payloads.

Acceptance criteria:

- Platform adapters parse real or fixture payloads into internal platform DTOs.
- API errors map into typed errors.
- No business logic is embedded in platform adapters.

### Layer 7: Master Monitoring and Raw Ingestion

Goal:

Continuously detect master trades and store immutable raw events.

What to do:

- Load active master accounts from registry.
- Start Polymarket polling loops per priority tier.
- Start Kalshi WebSocket listeners for authorized masters.
- Add Kalshi REST fallback polling.
- Generate raw event idempotency keys.
- Insert raw events into database.
- Publish raw event IDs to `raw.events`.
- Record ingestion lag and errors.
- Add stale-source alerting.

Deliverables:

- `crates/ingest`
- Polymarket master polling worker.
- Kalshi master fill worker.
- Raw event producer.

Acceptance criteria:

- Same raw platform event cannot be stored twice.
- Ingested event appears in DB and queue.
- Worker restarts without losing cursor state.

### Layer 8: Signal Normalization

Goal:

Convert raw platform events into clean trade signals.

What to do:

- Polymarket normalization:
  - map `conditionId` and asset/token ID to local market
  - map outcome
  - map buy/sell side
  - compute contracts, notional, and entry price
  - preserve transaction hash and fill identifiers
- Kalshi normalization:
  - map ticker to local market
  - convert YES-leg bid/ask semantics into normalized buy/sell YES/NO
  - compute contracts, notional, and price
  - join order ID when available
- Add signal status:
  - `new`
  - `filtered`
  - `approved`
  - `rejected`
  - `expired`
- Add correlation ID from raw event to signal.

Deliverables:

- `crates/signal`
- Normalizer consumers.
- Fixture tests for both platforms.

Acceptance criteria:

- Every supported raw trade fixture creates the expected normalized signal.
- Unsupported raw events are stored and marked with reason, not silently dropped.

### Layer 9: Enrichment and Filtering

Goal:

Add enough market and liquidity context to make copy decisions safely.

What to do:

- Fetch current orderbook snapshot.
- Compute spread.
- Compute executable depth within slippage tiers.
- Load market status, close time, category, open interest/liquidity where available.
- Apply MVP filters:
  - min master notional
  - min follower target notional
  - max signal age
  - min time to close
  - max spread
  - min liquidity
  - unsupported market type
  - paused master/follower/category/market
- Store enrichment snapshot JSON.
- Emit filter reason for all skipped signals.

Deliverables:

- Enrichment service in `crates/signal`.
- Filter reason enum.
- Metrics for rejection reasons.

Acceptance criteria:

- A signal with stale timestamp is rejected.
- A signal in a thin orderbook is rejected.
- A valid signal receives orderbook depth and proceeds to risk.

### Layer 10: Risk and Sizing

Goal:

Compute per-follower copy intents with conservative risk controls.

What to do:

- Implement follower risk config.
- Implement MVP sizing:
  - follower bankroll percent
  - copy multiplier
  - master notional cap
  - liquidity depth cap
  - per-trade cap
  - per-market cap
  - per-category cap
  - daily loss cap
- Implement stop-copy mode.
- Implement master pause and category pause.
- Implement price limit:
  - buy limit equals master entry plus max slippage
  - sell limit equals master entry minus max slippage
- Write `RiskDecision` with:
  - approved boolean
  - target notional
  - target contracts
  - price limit
  - rejection reasons
  - config version
- Create copy intents for approved followers.

Deliverables:

- `crates/risk`
- Unit tests for every risk rule.
- Dry-run copy intent generation.

Acceptance criteria:

- Risk engine explains every rejection.
- Same input and config always produce the same output.
- Stop-copy mode blocks new copy intents.
- Approved signal creates one copy intent per eligible follower.

### Layer 11: Execution Orchestration

Goal:

Turn approved copy intents into platform-specific order requests without duplicate orders.

What to do:

- Consume `copy.intents`.
- Partition execution by platform and follower account.
- Create deterministic `client_order_id`.
- Convert normalized intent to platform-native request.
- Write order row before platform submission.
- Submit to executor only after order row exists.
- Publish execution report.
- Handle platform response:
  - acknowledged
  - rejected
  - partially filled
  - filled
  - expired
  - cancelled
- Implement bounded retries only when safe.

Deliverables:

- `crates/execution`
- Orchestrator worker.
- Platform-specific order request builders.

Acceptance criteria:

- Duplicate copy intent cannot submit duplicate platform orders.
- Order lifecycle is visible in DB.
- Dry-run mode creates simulated execution reports without live orders.

### Layer 12: Polymarket Execution

Goal:

Place follower orders on Polymarket safely after dry-run validation.

What to do:

- Resolve normalized outcome to Polymarket token ID.
- Build marketable limit order with slippage cap.
- Sign order using configured wallet/deposit wallet method.
- Submit through CLOB API.
- Parse order ack and fill response.
- Poll or subscribe for follower fills.
- Update execution reports.
- Add cancel support for resting limit orders if used.

Deliverables:

- Live-capable Polymarket executor.
- Integration test with mocked CLOB API.
- Small-notional manual live test checklist.

Acceptance criteria:

- Executor refuses to submit without valid key, token ID, price limit, and risk approval.
- Small live order can be placed and reconciled.
- Platform reject is recorded with raw response.

### Layer 13: Kalshi Execution

Goal:

Place follower orders on Kalshi safely after demo validation.

What to do:

- Implement V2 order creation.
- Sign requests with RSA-PSS private key.
- Convert normalized YES/NO order into Kalshi YES-leg bid/ask.
- Set `client_order_id`.
- Set time-in-force:
  - `immediate_or_cancel` for default MVP
  - `fill_or_kill` for all-or-nothing follower profiles
- Parse order ack and fills.
- Subscribe to user orders/fills for follower confirmation.
- Poll REST fallback for reconciliation.

Deliverables:

- Live-capable Kalshi executor.
- Demo environment test.
- Small-notional live test checklist.

Acceptance criteria:

- Demo order placement works.
- YES/NO conversion tests pass.
- Duplicate `client_order_id` does not create duplicate exposure.

### Layer 14: Reconciliation and P&L

Goal:

Keep internal state aligned with platform truth.

What to do:

- Reconcile follower orders and fills.
- Reconcile follower positions.
- Reconcile balances if available.
- Compute realized and unrealized P&L.
- Compute slippage vs master price.
- Compute latency:
  - master event timestamp to raw receive
  - raw receive to signal
  - signal to risk
  - risk to order submit
  - order submit to fill
- Detect mismatches:
  - order missing on platform
  - fill missing internally
  - position delta mismatch
  - stale open order
- Pause affected account/market on severe mismatch.

Deliverables:

- `crates/reconcile`
- P&L snapshots.
- Reconciliation alerts.

Acceptance criteria:

- Filled orders update positions.
- P&L updates from current market marks.
- Reconciliation mismatch creates alert and does not silently continue.

### Layer 15: API and Dashboard Read Model

Goal:

Expose enough API for operators to configure, observe, and control the MVP.

What to do:

- Build Axum API endpoints:
  - health
  - masters
  - followers
  - signals
  - copy intents
  - orders
  - fills
  - positions
  - P&L
  - alerts
  - pause/resume controls
  - global stop-copy
- Add pagination and filters.
- Add audit logging for control actions.
- Add simple auth placeholder for local MVP, then proper auth before production.
- Optional first UI:
  - React/Next.js dashboard consuming API
  - or server-rendered Rust UI if the team wants a simpler MVP

Deliverables:

- `crates/api`
- OpenAPI spec optional.
- Operator endpoints.

Acceptance criteria:

- Operator can pause and resume master/follower/category.
- Operator can view signal-to-order timeline.
- Operator can activate global stop-copy.

### Layer 16: Observability and Alerts

Goal:

Make system behavior debuggable before live trading.

What to do:

- Add structured logs with correlation IDs.
- Add metrics:
  - raw events per source
  - ingestion lag
  - queue depth
  - signal latency
  - risk rejection counts
  - order submission latency
  - platform error counts
  - reconciliation mismatch counts
- Add alert rules:
  - stale WebSocket
  - API auth failure
  - API 429 sustained
  - order rejected
  - high slippage
  - position mismatch
  - daily loss breach
- Start with DB-backed alerts; add Slack/PagerDuty later.

Deliverables:

- Alert model and alert worker.
- Metrics endpoint.
- Log correlation across event lifecycle.

Acceptance criteria:

- Every signal/order can be traced by correlation ID.
- Critical failures create visible alerts.

### Layer 17: Replay, Simulation, and Test Harness

Goal:

Prove logic before live trading.

What to do:

- Store fixture raw events for Polymarket and Kalshi.
- Build replay command:
  - read raw event JSON
  - normalize
  - enrich with mocked orderbook
  - risk score
  - create dry-run copy intent
- Build simulator for:
  - duplicate events
  - stale signals
  - low liquidity
  - high slippage
  - daily loss breach
  - partial fill
  - platform rejection
- Add integration tests that run Postgres and Redis locally.

Deliverables:

- `cli replay`
- fixture library.
- simulation tests.

Acceptance criteria:

- All acceptance scenarios from the PRD pass.
- Replaying the same raw event twice creates one copy intent at most.

## 6. Milestone Plan

### Milestone 0: Decisions and Setup

Objective:

Resolve product and access prerequisites before coding live execution.

Tasks:

- Confirm MVP account access.
- Confirm dry-run first policy.
- Confirm live pilot bankroll.
- Confirm Rust workspace design.
- Confirm PostgreSQL and Redis local stack.

Done when:

- Access checklist exists.
- MVP risk defaults exist.
- No unresolved blocker to start backend scaffolding.

Estimated effort:

- 1 to 2 days.

### Milestone 1: Rust Foundation

Objective:

Create the runnable skeleton.

Tasks:

- Initialize Rust workspace.
- Add core dependencies.
- Add Docker Compose.
- Add config loader.
- Add logging.
- Add health API.
- Add worker binary.
- Add migrations.

Done when:

- `cargo test` passes.
- `cargo clippy` passes.
- Local API and worker run.

Estimated effort:

- 2 to 4 days.

### Milestone 2: Domain, DB, and Queue Backbone

Objective:

Build durable internal rails before platform integrations.

Tasks:

- Implement domain crate.
- Implement database schema and repositories.
- Implement Redis Streams abstraction.
- Implement worker consumer framework.
- Add idempotency constraints.

Done when:

- Raw event can be inserted and queued.
- Queue consumer can process and ack message.
- Duplicate raw event is safely ignored.

Estimated effort:

- 4 to 7 days.

### Milestone 3: Market Metadata

Objective:

Create local market catalog.

Tasks:

- Implement Polymarket market sync.
- Implement Kalshi market sync.
- Store platform market metadata.
- Add unsupported market flags.
- Add lookup functions for token ID/ticker to market.

Done when:

- Local DB can resolve supported markets from both platforms.
- Closed/unsupported markets are filtered.

Estimated effort:

- 4 to 6 days.

### Milestone 4: Read-Only Master Monitoring

Objective:

Detect and store master activity without execution.

Tasks:

- Implement Polymarket Data API polling.
- Implement Kalshi authorized fill WebSocket.
- Implement Kalshi REST fallback.
- Store raw events.
- Publish raw event queue messages.
- Add cursors and restart recovery.

Done when:

- Master activity appears as raw events in DB.
- No duplicate raw events across restarts.
- Ingestion lag is recorded.

Estimated effort:

- 5 to 8 days.

### Milestone 5: Signal Normalization and Filters

Objective:

Turn raw events into deterministic trade signals.

Tasks:

- Implement Polymarket normalizer.
- Implement Kalshi normalizer.
- Implement enrichment with market metadata.
- Implement MVP filters.
- Store signals and rejection reasons.

Done when:

- Fixture raw events produce expected signals.
- Bad or unsupported events produce clear rejection reasons.

Estimated effort:

- 4 to 7 days.

### Milestone 6: Risk Engine and Dry-Run Copier

Objective:

Generate copy intents without placing orders.

Tasks:

- Implement follower risk config.
- Implement sizing formula.
- Implement portfolio limits.
- Implement stop-copy controls.
- Create dry-run copy intents.
- Create simulated execution reports.

Done when:

- Approved signals create follower-specific copy intents.
- Rejected signals explain why.
- Dry-run end-to-end flow works from raw event to simulated order.

Estimated effort:

- 5 to 8 days.

### Milestone 7: Dashboard API and Operator Controls

Objective:

Expose the MVP state and controls.

Tasks:

- Add API endpoints for masters, followers, signals, orders, positions, P&L, alerts.
- Add pause/resume endpoints.
- Add global stop-copy endpoint.
- Add audit logging.
- Add basic local auth guard or operator token.

Done when:

- Operator can inspect end-to-end dry-run trades.
- Operator can pause a master and verify risk blocks new copy intents.

Estimated effort:

- 4 to 6 days.

### Milestone 8: Kalshi Demo Execution

Objective:

Validate live execution logic in Kalshi demo or controlled small-size setup.

Tasks:

- Implement Kalshi order placement.
- Implement request signing.
- Implement YES/NO conversion.
- Implement follower fill subscription.
- Implement cancel path if needed.
- Reconcile demo fills and positions.

Done when:

- Demo order can be placed, filled/rejected, stored, and reconciled.
- Duplicate order test passes.

Estimated effort:

- 5 to 8 days.

### Milestone 9: Polymarket Small-Size Execution

Objective:

Validate Polymarket follower order placement and reconciliation.

Tasks:

- Implement Polymarket order signing.
- Implement CLOB order submission.
- Implement follower fill confirmation.
- Implement token/outcome execution mapping.
- Reconcile fills and positions.

Done when:

- Small notional Polymarket order can be placed and reconciled.
- Executor refuses invalid or stale orders.

Estimated effort:

- 6 to 10 days.

### Milestone 10: Reconciliation, P&L, and Alerts

Objective:

Close the loop from copy decision to portfolio truth.

Tasks:

- Implement platform reconciliation jobs.
- Implement position updates.
- Implement realized/unrealized P&L.
- Implement slippage and latency metrics.
- Implement DB-backed alerts.

Done when:

- Every filled order updates position and P&L.
- Mismatches create alerts.
- Signal-to-fill latency is visible.

Estimated effort:

- 5 to 8 days.

### Milestone 11: Dry-Run Pilot

Objective:

Run the full system without live orders.

Tasks:

- Configure 10+ masters.
- Configure 1+ dry-run follower.
- Run continuously for 1 to 2 weeks.
- Review signals, copy intents, risk rejections, slippage estimates, and missed events.
- Fix ingestion and normalization bugs.

Done when:

- Copy accuracy in dry-run is at least 95%.
- Duplicate order intent rate is zero.
- Reconciliation simulation has no unexplained mismatches.

Estimated effort:

- 1 to 2 weeks calendar time.

### Milestone 12: Live Small-Notional Pilot

Objective:

Trade real small sizes with conservative risk controls.

Tasks:

- Enable one follower account at small limits.
- Enable one platform first, then second platform.
- Use max trade bankroll percentage below production default.
- Monitor every order manually during first sessions.
- Keep global stop-copy ready.
- Compare master entry vs follower fill.

Done when:

- Live copy works on both supported platforms.
- No duplicate live orders occur.
- P&L and positions reconcile daily.
- Slippage is within configured cap.

Estimated effort:

- 1 to 2 weeks calendar time.

## 7. Build Order Summary

Build in this exact order:

1. Access/compliance checklist.
2. Rust workspace and local infra.
3. Domain model.
4. Database schema and repositories.
5. Redis Streams queue layer.
6. Market metadata sync.
7. Platform adapters for read-only data.
8. Master ingestion.
9. Signal normalization.
10. Enrichment and filtering.
11. Risk engine.
12. Dry-run copy intents.
13. Dashboard/admin API.
14. Kalshi demo execution.
15. Polymarket small-size execution.
16. Reconciliation.
17. P&L and alerts.
18. Dry-run pilot.
19. Live small-notional pilot.

Do not build live execution before dry-run copy intents and idempotency tests pass.

## 8. First Sprint Backlog

Sprint goal:

Get the Rust workspace running locally and establish the core data model.

Tasks:

- Create Cargo workspace.
- Create crates:
  - `domain`
  - `config`
  - `db`
  - `queue`
  - `api`
  - `worker`
- Add Docker Compose for Postgres and Redis.
- Add `.env.example`.
- Add `tracing` setup.
- Add `/healthz`.
- Add initial SQL migration.
- Add core domain enums and value types.
- Add repository connection pool.
- Add one integration test that connects to Postgres.

Sprint acceptance:

- `cargo fmt`, `cargo clippy`, and `cargo test` pass.
- `docker compose up` starts dependencies.
- `cargo run -p api` starts API.
- `cargo run -p worker` starts worker.

## 9. Second Sprint Backlog

Sprint goal:

Make raw ingestion durable and replayable.

Tasks:

- Add `raw_events` table and repository.
- Add Redis Streams publisher and consumer.
- Add message envelope.
- Add Polymarket trade fixture parser.
- Add Kalshi fill fixture parser.
- Add idempotency key generation.
- Add `cli replay-raw-event`.

Sprint acceptance:

- A fixture raw event can be inserted once.
- Replaying the same fixture twice does not duplicate it.
- Queue consumer processes raw event ID.

## 10. Third Sprint Backlog

Sprint goal:

Produce normalized signals from raw events.

Tasks:

- Add market metadata schema.
- Add minimal market fixtures.
- Implement Polymarket normalizer.
- Implement Kalshi normalizer.
- Store signals.
- Add rejection reasons for unsupported events.

Sprint acceptance:

- Polymarket fixture creates expected normalized signal.
- Kalshi fixture creates expected normalized signal.
- Unsupported fixture is stored with reason.

## 11. Key Technical Risks to Resolve Early

Polymarket execution signing:

- Risk: Direct implementation mistakes can produce invalid orders or unsafe behavior.
- Mitigation: Prefer official client integration or carefully wrap signing code with fixture tests and small-size live tests.

Kalshi master monitoring:

- Risk: Public market data does not identify arbitrary masters.
- Mitigation: Require authorized master API access for MVP.

Idempotency:

- Risk: At-least-once ingestion can duplicate copy orders.
- Mitigation: Database unique constraints plus deterministic client order IDs.

Decimal precision:

- Risk: Binary floats cause pricing and sizing errors.
- Mitigation: Use `rust_decimal` and platform-specific tick-size rounding functions.

Rate limits:

- Risk: Aggressive polling increases 429s and latency.
- Mitigation: Token buckets, adaptive polling, and WebSocket-first where possible.

Secrets:

- Risk: Keys in logs or database can cause fund loss.
- Mitigation: `secrecy`, no secret logging, KMS/Vault for production, local `.env` only for development.

## 12. MVP Release Gates

### Gate A: Engineering Readiness

Required:

- Rust workspace complete.
- Database and queue integration working.
- Domain tests passing.
- Raw event idempotency working.

### Gate B: Dry-Run Readiness

Required:

- Master ingestion working.
- Signal normalization working.
- Risk engine working.
- Copy intents generated.
- Dashboard/API shows flow.
- No live order code enabled by default.

### Gate C: Live Test Readiness

Required:

- Dry-run pilot complete.
- Duplicate order tests passing.
- Platform credentials secured.
- Global stop-copy tested.
- Per-follower max loss configured.
- Reconciliation working in sandbox/demo.

### Gate D: MVP Complete

Required:

- Live small-notional copy tested on supported platform accounts.
- Reconciliation and P&L working.
- Alerts working.
- Daily operational checklist in place.
- Known risks documented.

## 13. MVP Operational Checklist

Daily:

- Check ingestion lag.
- Check queue depth.
- Check API auth errors.
- Check rejected order count.
- Check high-slippage alerts.
- Check position reconciliation.
- Check daily P&L and drawdown.
- Check paused masters/followers.

Before enabling live copy:

- Confirm follower balance.
- Confirm max trade size.
- Confirm max daily loss.
- Confirm global stop-copy works.
- Confirm no stale open orders.
- Confirm platform status.
- Confirm recent successful reconciliation.

After every live pilot session:

- Compare master and follower prices.
- Review missed signals.
- Review rejected signals.
- Review partial fills.
- Review P&L attribution.
- Adjust risk config only through audited controls.
