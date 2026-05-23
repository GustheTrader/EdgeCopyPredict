# Arbitek Trade Copier

Automated prediction-market trade copier for Polymarket and Kalshi.

## Repository layout

```
arbitek-trade-copier/
├── backend/                   # Rust workspace
│   ├── Cargo.toml             # Workspace manifest
│   ├── crates/
│   │   ├── api/               # Axum HTTP + WebSocket API server
│   │   ├── cli/               # Operator CLI
│   │   ├── config/            # Env-based config loading
│   │   ├── db/                # SQLx database layer
│   │   ├── domain/            # Core types and value objects
│   │   ├── execution/         # Order execution orchestration
│   │   ├── ingest/            # Raw event ingestion
│   │   ├── platform-kalshi/   # Kalshi adapter
│   │   ├── platform-polymarket/ # Polymarket adapter
│   │   ├── queue/             # Redis Streams abstraction
│   │   ├── reconcile/         # Reconciliation service
│   │   ├── risk/              # Risk engine
│   │   ├── signal/            # Signal normalizer and enricher
│   │   └── worker/            # Background worker binary
│   └── migrations/            # PostgreSQL migrations (sqlx)
│
├── frontend/                  # React + TypeScript dashboard
│   ├── src/
│   │   ├── api/               # Typed REST client (one file per domain)
│   │   ├── components/        # ui/, charts/, layout/, masters/, followers/,
│   │   │                      # signals/, execution/, pnl/, alerts/
│   │   ├── hooks/             # Data fetching and WebSocket hooks
│   │   ├── pages/             # Route-level page components
│   │   ├── stores/            # Zustand ephemeral state
│   │   └── types/             # Shared TypeScript types
│   └── package.json
│
├── docs/
│   ├── prediction-market-trade-copier-prd.md   # Full system PRD
│   ├── frontend-prd.md                          # Dashboard UI PRD
│   └── rust-mvp-goals-and-milestones.md
│
├── docker-compose.yml
└── .env.example
```

## Quick start

### Infrastructure

The current MVP flow is set up for your existing Supabase Postgres database plus
local Redis. Docker is optional and is not required for the dry-run smoke path.

Required local services/tools:

```bash
redis-cli -h 127.0.0.1 -p 6379 ping
psql --version
sqlx --version
```

Keep secrets in the repository root `.env`; it is gitignored. Do not commit real
exchange keys or wallet private keys.

### Backend

```bash
cd backend
sqlx migrate run             # uses DATABASE_URL from your environment
cargo run -p arbitek-api
```

API starts at `http://127.0.0.1:8080`. Health check: `GET /health` or
`GET /healthz`.

When using Supabase, the first Postgres connection may take several seconds.
Startup logs now show `connecting database pool` before the API binds and
`database pool connected` after SQLx opens the pool.

Core MVP read endpoints:

- `GET /api/v1/masters`
- `GET /api/v1/followers`
- `GET /api/v1/signals?platform=&master_id=&status=&page=&per_page=`
- `GET /api/v1/orders?follower_account_id=&status=&platform=&per_page=`
- `GET /api/v1/fills?follower_account_id=&platform=&market_id=&page=&per_page=`
- `GET /api/v1/positions?follower_account_id=&platform=&market_id=&open_only=&page=&per_page=`
- `GET /api/v1/pnl?follower_account_id=`
- `GET /api/v1/alerts?status=`

### Frontend

```bash
cd frontend
npm ci
PORT=5174 npm run dev
```

Dashboard starts at `http://127.0.0.1:5174` when `PORT=5174` is used. The
custom esbuild dev server proxies `/api/*` to the backend at `:8080`; the
current MVP dev server does not proxy WebSocket upgrades.

Production dashboard bundle:

```bash
cd frontend
npm run build
```

### Worker

```bash
cd backend
cargo run -p arbitek-worker
```

Worker behavior is controlled by `WORKER_MODE`:

- `idle`: start config/tracing, then wait for shutdown.
- `polymarket-once`: poll active Polymarket master accounts, store raw events, normalize new raw events into signals, then create and publish copy intents for the configured target platform.
- `polymarket-loop`: run the same pipeline repeatedly using `POLYMARKET_POLL_INTERVAL_SECONDS`.
- `polymarket-normalize-once`: normalize already-stored Polymarket raw events without polling the Data API.
- `polymarket-risk-once`: evaluate normalized Polymarket signals against active follower accounts on `EDGECOPY_COPY_TARGET_PLATFORM`.
- `polymarket-to-kalshi-risk-once`: explicit alias for Polymarket signal source to Kalshi copy-intent target.
- `kalshi-once`: poll authenticated Kalshi portfolio fills for active Kalshi master accounts, then normalize them into signals.
- `kalshi-loop`: run the Kalshi read-only fills pipeline repeatedly using `KALSHI_POLL_INTERVAL_SECONDS`.
- `kalshi-fills-once`: poll authenticated Kalshi fills and store idempotent raw events only.
- `kalshi-normalize-once`: normalize already-stored Kalshi raw fill events into signals.
- `execution-dry-run-once`: turn approved copy intents into simulated order rows and publish `execution.report`.
- `reconcile-once`: convert filled/dry-run-filled orders into fills and position rows.

The current default target is `EDGECOPY_COPY_TARGET_PLATFORM=kalshi`, meaning
Polymarket master trades produce Kalshi follower copy intents. Keep
`LIVE_TRADING_ENABLED=false` and `EDGECOPY_EXECUTION_PROVIDER=dry_run` until the
execution layer has passed sandbox tests, Polymarket-to-Kalshi market mapping is
implemented, and the pasted credentials have been rotated.

Kalshi public market trades do not expose the trader account. The Kalshi MVP
ingestion path therefore requires authenticated fills for a master account/key;
it cannot copy arbitrary public Kalshi users from `/markets/trades`.

Register master accounts with the operator CLI:

```bash
cd backend
cargo run -p arbitek-cli -- master add \
  --name "Poly Master 1" \
  --platform polymarket \
  --account 0xabc
```

Register Kalshi follower accounts with the same CLI:

```bash
cd backend
cargo run -p arbitek-cli -- follower add \
  --name "Kalshi Follower 1" \
  --platform kalshi \
  --account kalshi-follower-1 \
  --credential-ref env:KAL_API_KEY
```

After at least one active Polymarket master and one active Kalshi follower
account exist, run the Polymarket-to-Kalshi dry-run flow:

```bash
cd backend
WORKER_MODE=polymarket-once cargo run -p arbitek-worker
WORKER_MODE=execution-dry-run-once cargo run -p arbitek-worker
WORKER_MODE=reconcile-once cargo run -p arbitek-worker
```

Local offline pipeline smoke test:

```bash
cd backend
scripts/smoke_dry_run.sh
```

Requires Postgres, Redis, `psql`, and `sqlx` CLI. The seed inserts one raw Polymarket fixture, so this smoke test does not call external trading APIs.

## Documentation

- [System PRD](docs/prediction-market-trade-copier-prd.md) — full backend architecture, data models, platform specs
- [Frontend PRD](docs/frontend-prd.md) — dashboard page specs, component architecture, API contract
- [Rust MVP Goals](docs/rust-mvp-goals-and-milestones.md) — backend milestones

## Tech stack

| Layer | Technology |
|---|---|
| Backend | Rust, Axum, SQLx, PostgreSQL 16, Redis 7 |
| Frontend | React 18, TypeScript, Tailwind CSS, esbuild |
| Server state | TanStack Query |
| Client state | Zustand |
| Charts | lightweight-charts |
| Dev infra | Supabase Postgres, local Redis, optional Docker Compose |
