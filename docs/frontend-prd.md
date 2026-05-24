# Arbitek Dashboard — Frontend PRD

Version: 1.0
Date: 2026-05-17
Owner: Product / Engineering
Status: Build-ready
Stack: React 18 · Vite · TypeScript · Tailwind CSS · TanStack Query · Zustand · lightweight-charts

---

## 1. Overview

The Arbitek Dashboard is the operator-facing UI for the prediction-market trade copier backend. It provides real-time visibility into master trader activity, signal processing decisions, follower execution, P&L attribution, and operational controls. It is a single-page application that communicates with the Rust API server (`crates/api`) over REST and WebSocket.

The dashboard is not a public-facing product. Users are system operators, portfolio managers, follower account owners, and on-call engineers. All pages assume authenticated access (auth integration is a Phase 1 requirement — see §9).

---

## 2. Design Principles

- **Data density over decoration.** The primary consumers are operators making fast decisions. Prefer compact tables and badges over cards with whitespace.
- **Dark-first.** All pages use the dark `#0f1117` background. Light mode is out of scope.
- **Real-time by default.** The activity feed, system status bar, and alert count update live via WebSocket without manual refresh.
- **Monospace numbers.** All numeric data (prices, P&L, latency, sizes) renders in a monospace font for vertical alignment.
- **Fail visible.** Connection failures, stale data, and API errors must be surfaced immediately — never silently hidden.
- **No unnecessary modals.** Pause/resume actions use inline optimistic mutations. Complex flows use dedicated pages.

---

## 3. Users and Use Cases

### 3.1 System Operator
- Needs to know at a glance whether the system is running, which masters are active, and whether any critical alerts exist.
- Takes emergency action (stop-copy, pause master) from the header or master table without navigating away.
- Adds and configures new masters and followers via the CLI or Settings page; the dashboard surfaces the live result.

### 3.2 Portfolio Manager
- Reviews P&L per follower, per master, per category.
- Examines which signals were rejected and why.
- Tracks copy accuracy and slippage trends over 7d / 30d / 90d windows.

### 3.3 Follower Account Owner
- Checks their account's status, current risk config, and individual trade execution.
- Requests changes to risk parameters (routed through operator in MVP).

### 3.4 On-Call Engineer
- Monitors queue depth, WebSocket connection health, API error rates, and reconciliation mismatches.
- Acknowledges alerts and triggers kill switch.

---

## 4. Page Inventory

| Route | Page | Primary Persona |
|---|---|---|
| `/` | Dashboard (overview) | All |
| `/signals` | Signal feed | Operator, Portfolio Manager |
| `/masters` | Master leaderboard | Operator, Portfolio Manager |
| `/followers` | Follower accounts | Operator, Follower Owner |
| `/execution` | Execution log | Operator, Engineer |
| `/pnl` | P&L dashboard | Portfolio Manager |
| `/alerts` | Alerts | Engineer, Operator |
| `/settings` | System settings | Operator |

---

## 5. Page Specifications

### 5.1 Dashboard (`/`)

**Purpose:** Single-screen health overview.

**Layout:** Two-column grid. Left 2/3: live activity feed + P&L summary. Right 1/3: master table + open alerts.

**Requirements:**
- Activity feed auto-scrolls with incoming WebSocket signals; max 200 items retained in memory.
- Each signal row shows: master name, market title (truncated), outcome (YES/NO), side (BUY/SELL), price in cents, notional in USD, detection latency in ms, status badge, and relative time.
- P&L summary shows realized, unrealized, total P&L, copy accuracy %, and avg slippage per follower.
- Master table shows status badge, weight, platforms, last-seen time, and inline pause/resume.
- Alert panel shows open alerts only; clicking "Ack" removes the item optimistically.
- Top bar shows WebSocket connectivity dots (green/yellow/red) for Polymarket and Kalshi, queue depth, active master count, copy mode label, and Stop/Resume Copy button.

**Real-time:** Activity feed via `/ws/activity`; system status polled every 10 s.

---

### 5.2 Signals (`/signals`)

**Purpose:** Full-page signal feed with filtering.

**Requirements:**
- Filter bar: platform selector (all / polymarket / kalshi), master selector, status selector (all / approved / rejected / filtered), search by market title.
- Paginated table with same columns as dashboard feed but with sortable columns.
- Clicking a row expands an inline signal timeline panel showing all timestamp milestones: `event_timestamp → raw_event_received_at → signal_created_at → enrichment_completed_at → risk_decision_at → execution_request_created_at → order_submitted_at → platform_ack_at → first_fill_at → final_fill_at → reconciled_at`.
- Timeline panel also shows: target vs actual contracts, master price, follower avg price, slippage, fees, rejection reason or error code.
- Export button downloads current filtered view as CSV (frontend-only, uses fetched data).

**Real-time:** Feed updates via WebSocket; table auto-refreshes every 5 s.

---

### 5.3 Masters (`/masters`)

**Purpose:** Full master leaderboard with per-master drill-down.

**Requirements:**
- Leaderboard table columns: display name, status badge, platforms, weight, 7d ROI %, 30d ROI %, win rate, Sharpe score, max drawdown, copyability score, avg slippage, open trades copied, last seen.
- Sortable by any numeric column.
- Status badge reflects `active / probation / paused / disabled` with distinct colors.
- Inline actions: pause, resume. Both trigger optimistic update with rollback on API error.
- Clicking a row navigates to `/masters/:id` (detail view — Phase 1 scope).
- Detail view: signal history for that master, stats by category, by platform, quality score trend chart.

**Real-time:** Table refetches every 30 s.

---

### 5.4 Followers (`/followers`)

**Purpose:** Follower account overview and risk config display.

**Requirements:**
- Card grid (2-3 columns). Each card shows: display name, status badge, platform accounts, risk config summary (max trade %, max market %, max daily loss %, max drawdown %, max slippage).
- Inline pause / resume / switch-to-dry-run actions.
- "Edit risk config" opens a side panel with a form for all RiskConfig fields with validation:
  - All percentage fields: 0–100, step 0.1.
  - Max slippage: 0–10 cents integer.
  - Checkboxes for `copy_sells`, `copy_closes`, `allow_taker_orders`.
  - Submit calls `PATCH /followers/:id/risk-config`; shows error inline on failure.
  - All changes append to the audit log on the backend (frontend is not responsible for this).
- Shows current open position count and exposure next to each account.

**Real-time:** Refetches every 60 s.

---

### 5.5 Execution (`/execution`)

**Purpose:** Order log with slippage and fill analytics.

**Requirements:**
- Table columns: market title, follower name, side + outcome, master price (cents), follower fill price (cents), slippage (cents), fill ratio (filled / requested contracts), status badge, age.
- Slippage column: yellow if > 2¢, red if > 3¢, green if ≤ 1¢.
- Filter bar: follower selector, status, platform, date range.
- Pagination: 50 rows per page.
- Clicking a row expands the full order timeline (same milestone list as signal detail).
- Summary bar above table: total orders, filled %, avg slippage, total fees in session.

**Real-time:** Refetches every 5 s.

---

### 5.6 P&L (`/pnl`)

**Purpose:** P&L analysis per follower, master, category.

**Requirements:**
- Summary cards (one per follower): realized P&L, unrealized P&L, total P&L, copy accuracy %, avg slippage. Values are color-coded: green positive, red negative.
- Time-series chart: cumulative realized P&L over time per follower (lightweight-charts line chart). Window selector: 7d / 30d / 90d.
- Attribution table: breakdown of P&L delta vs scaled master return by category. Columns: category, master P&L (scaled), follower P&L, delta, slippage, missed signals, partial fills.
- Follower selector to filter all views to a single follower.

**Real-time:** Summary refetches every 15 s; chart on window change.

---

### 5.7 Alerts (`/alerts`)

**Purpose:** Alert management.

**Requirements:**
- Tab bar: "Open" (default) / "Acknowledged".
- Each alert row: severity badge (critical = red, warning = yellow, info = blue), message, type, entity (master/follower/market if applicable), relative timestamp, "Ack" button.
- "Ack All" button at top of open list.
- Critical alerts show a persistent banner at top of every page until acknowledged.
- Clicking an entity name navigates to the relevant master/follower/market page.

**Real-time:** Refetches every 10 s; unread count in sidebar badge.

---

### 5.8 Settings (`/settings`)

**MVP Scope:** Read-only display of current system configuration pulled from the API. Shows:
- `copy_mode`, `live_trading_enabled`, `environment`.
- Active platform connections.
- Link to CLI docs for changing config.

**Phase 1 Scope:**
- Form to edit global default risk limits.
- Market mapping review queue: list pending cross-platform market mappings with approve / reject inline actions.
- Credential vault status (connected/disconnected indicator per platform credential ref).

---

## 6. Component Architecture

```
src/
├── api/                    # Typed fetch wrappers — one file per resource domain
│   ├── client.ts           # Base fetch wrapper + ApiError class
│   ├── masters.ts
│   ├── followers.ts
│   ├── signals.ts
│   ├── pnl.ts
│   └── alerts.ts
│
├── components/
│   ├── ui/                 # Zero-dependency primitives
│   │   ├── Badge.tsx       # Status/severity chips
│   │   ├── Button.tsx      # primary / danger / ghost variants
│   │   ├── Card.tsx        # Surface container + CardHeader
│   │   └── StatusDot.tsx   # Colored connectivity dot
│   │
│   ├── charts/             # lightweight-charts wrappers
│   │   ├── PnlLineChart.tsx
│   │   └── LatencyHistogram.tsx
│   │
│   ├── layout/             # App shell — renders once
│   │   ├── AppShell.tsx    # Sidebar + TopBar + <Outlet>
│   │   ├── Sidebar.tsx     # Nav links + alert badge
│   │   └── TopBar.tsx      # WS status + copy mode + kill switch
│   │
│   ├── masters/
│   │   ├── MasterLeaderboard.tsx
│   │   └── MasterStatusBadge.tsx
│   │
│   ├── followers/
│   │   ├── FollowerCard.tsx
│   │   └── RiskConfigPanel.tsx
│   │
│   ├── signals/
│   │   ├── ActivityFeed.tsx    # Reads from activityStore
│   │   ├── SignalRow.tsx
│   │   └── SignalTimeline.tsx
│   │
│   ├── execution/
│   │   ├── ExecutionTable.tsx
│   │   └── OrderTimeline.tsx
│   │
│   ├── pnl/
│   │   ├── PnlSummary.tsx
│   │   └── PnlAttribution.tsx
│   │
│   └── alerts/
│       ├── AlertsList.tsx
│       └── CriticalBanner.tsx
│
├── hooks/                  # Custom hooks — data fetching + WebSocket
│   ├── useWebSocket.ts     # Reconnecting WS with exponential backoff
│   ├── useMasters.ts
│   ├── useSignals.ts       # Includes useLiveActivityFeed()
│   └── usePnl.ts
│
├── pages/                  # Route-level components — thin shells
│   ├── Dashboard.tsx
│   ├── Signals.tsx
│   ├── Masters.tsx
│   ├── Followers.tsx
│   ├── Execution.tsx
│   ├── PnL.tsx
│   ├── Alerts.tsx
│   └── Settings.tsx
│
├── stores/                 # Zustand — ephemeral client state only
│   ├── activityStore.ts    # Live signal feed (ring buffer, 200 items)
│   ├── alertStore.ts       # Open alert list + optimistic ack
│   └── systemStore.ts      # SystemStatus snapshot from TopBar
│
├── types/                  # TypeScript types mirroring backend JSON schemas
│   └── index.ts
│
├── App.tsx                 # Router + QueryClientProvider
├── main.tsx                # React root mount
└── styles.css              # Tailwind directives + scrollbar polish
```

### Separation of concerns rules

- `api/` files are pure async functions — no React, no hooks, no side effects.
- `components/ui/` components take only typed props — no store access, no API calls.
- `hooks/` own all data-fetching and WebSocket logic — components never call `api/` directly.
- `pages/` compose domain components — no business logic or data fetching inline.
- `stores/` hold only client-side ephemeral state that must survive re-renders but not a page reload (feed, alert ack state, system status snapshot). Server state lives in TanStack Query cache.

---

## 7. Data Flow

### REST data (server state)
```
api/ functions → TanStack Query cache → hooks (useQuery) → components
```
- Query keys are stable and hierarchical: `["masters"]`, `["masters", id, "stats"]`, `["signals", params]`.
- Mutations invalidate their parent query on success.
- All queries retry once on failure; network errors show a generic error state.

### Real-time data (WebSocket)
```
Backend /ws/activity → useWebSocket → activityStore (push) → ActivityFeed
Backend /ws/alerts   → useWebSocket → alertStore (setAlerts)
```
- WebSocket reconnects with exponential backoff (1 s → 2 s → 4 s → … → 30 s cap).
- On reconnect, TanStack Query refetches stale queries to close any gaps.
- If WebSocket is unavailable, the activity feed pauses and shows a reconnecting indicator; REST polling continues.

### Vite dev proxy
```
Frontend :5173 /api/* → Backend :8080 /*
Frontend :5173 /ws/*  → Backend :8080 /* (ws)
```
Production: served as static files from the Rust API's `/static` route or a CDN with the same proxy.

---

## 8. API Contract (consumed from `crates/api`)

The frontend expects the following REST endpoints from the backend. See the main PRD section 13 for full field definitions.

| Method | Path | Returns |
|---|---|---|
| GET | `/healthz` | `{ status, service }` |
| GET | `/system/status` | `SystemStatus` |
| POST | `/system/stop-copy` | 200 |
| POST | `/system/resume-copy` | 200 |
| GET | `/masters` | `Master[]` |
| GET | `/masters/:id` | `Master` |
| GET | `/masters/:id/stats` | `MasterStats[]` |
| POST | `/masters/:id/pause` | 200 |
| POST | `/masters/:id/resume` | 200 |
| GET | `/followers` | `Follower[]` |
| GET | `/followers/:id` | `Follower` |
| PATCH | `/followers/:id/risk-config` | `Follower` |
| POST | `/followers/:id/pause` | 200 |
| POST | `/followers/:id/resume` | 200 |
| GET | `/signals` | `Paginated<Signal>` |
| GET | `/signals/:id/timeline` | `Order[]` |
| GET | `/orders` | `Paginated<Order>` |
| GET | `/pnl/summary` | `PnlSummary[]` |
| GET | `/pnl/series` | `PnlPoint[]` |
| GET | `/pnl/attribution` | `Record<string, string>` |
| GET | `/alerts` | `Alert[]` |
| POST | `/alerts/:id/ack` | 200 |

WebSocket endpoints:

| Path | Event shape |
|---|---|
| `/ws/activity` | `Signal` — new signal events pushed as JSON frames |
| `/ws/alerts` | `Alert` — new/updated alert events |
| `/ws/system` | `SystemStatus` — status updates |

All error responses: `{ error: string }` with appropriate HTTP status code.

---

## 9. Authentication (Phase 1)

MVP ships without auth (operator runs on localhost or a private VPN). Phase 1 adds:

- Session-based auth with a single operator password (bcrypt, stored in DB).
- JWT access token (15 min TTL) + refresh token (7 d, HTTP-only cookie).
- `/login` page: email + password form; on success stores access token in memory (not localStorage).
- `api/client.ts` adds `Authorization: Bearer <token>` header and handles 401 → redirect to login.
- Backend: auth middleware on all routes except `/healthz` and `/login`.
- No RBAC in MVP. Phase 2 adds viewer / operator / admin roles.

---

## 10. Non-Functional Requirements

### Performance
- Initial page load: < 1.5 s on a local network connection.
- Activity feed must handle 10 signals/second sustained without visible jank (ring buffer capped at 200).
- P&L chart: renders up to 90 days of hourly data points (≈ 2,160 points) without blocking the main thread — use lightweight-charts which renders to canvas.
- Bundle size target: < 400 KB gzipped (no UI framework beyond React + Tailwind).

### Accessibility
- All interactive elements are keyboard accessible.
- Color is never the sole indicator of status — badges always include a text label.
- Minimum contrast ratio 4.5:1 for body text against dark backgrounds.

### Browser support
- Latest two major versions of Chrome, Firefox, Safari.
- No IE11 or legacy Edge support.

### Error states
- Every data-fetching component shows a graceful empty state or error message — no unhandled white screens.
- API errors display the error message from the backend response body.
- WebSocket disconnection shows a persistent yellow indicator in the TopBar and a banner in the activity feed.

---

## 11. Build and Deployment

### Dev
```bash
cd frontend
npm install
npm run dev          # starts Vite at :5173, proxies /api and /ws to :8080
```
Backend must be running separately:
```bash
cd backend
cargo run -p arbitek-api
```

### Production build
```bash
cd frontend
npm run build        # outputs to frontend/dist/
```

The Rust API serves `frontend/dist/` as static files at `/` in production. Add to `crates/api`:
```rust
// serve static files from /dist with SPA fallback
```

### Docker Compose
`docker-compose.yml` at repo root runs:
- `postgres` on 5432
- `redis` on 6379
- `backend` (Rust API) on 8080
- `frontend` (Vite dev) on 5173 — dev only; production uses static file serving from backend

---

## 12. Phased Scope

### MVP (current)
- All 8 pages with the layouts described above.
- Live activity feed via WebSocket.
- Master pause/resume.
- Stop/Resume Copy global kill switch.
- Alert acknowledgement.
- P&L summary cards.
- Follower risk config display.

### Phase 1
- Authentication (§9).
- Master detail page (`/masters/:id`) with signal history and stats charts.
- Follower risk config edit panel with validation.
- Market mapping review queue in Settings.
- P&L time-series chart with 7d / 30d / 90d selector.
- Attribution table.
- Signal filter bar with platform / master / status / search.

### Phase 2
- Backtesting replay viewer — replayable raw event timeline with per-step signal/risk decisions.
- Cross-platform price comparison columns in master and signal views.
- Follower performance comparison chart (overlay multiple followers).
- Notification preferences (email / Slack webhook config form).
- Role-based access control (viewer vs operator vs admin).

---

## 13. Open Questions

- Should the activity feed auto-pause when the user scrolls up (to avoid the list jumping while they read)? Default assumption: yes — pause on scroll, resume on scroll-to-top.
- Phase 1 auth: single operator password or per-user accounts from the start? Recommendation: single password for MVP operator simplicity, multi-user by Phase 2.
- Should the frontend handle chart empty states with a "no data yet" placeholder, or hide the chart entirely? Recommendation: show placeholder.
- Production static file serving: embedded in Rust binary via `include_dir!`, or separate nginx/CDN layer? For MVP Docker Compose: separate nginx sidecar to avoid recompiling Rust to update UI.
