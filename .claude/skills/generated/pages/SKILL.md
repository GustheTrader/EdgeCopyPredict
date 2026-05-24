---
name: pages
description: "Skill for the Pages area of arbitek trade copier. 17 symbols across 13 files."
---

# Pages

17 symbols | 13 files | Cohesion: 89%

## When to Use

- Working with code in `frontend/`
- Understanding how usePnlSummary, useOrders, AlertsList work
- Modifying pages-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `frontend/src/hooks/usePnl.ts` | usePnlSummary, useOrders |
| `frontend/src/components/pnl/PnlSummary.tsx` | pnlColor, PnlSummary |
| `frontend/src/components/ui/Card.tsx` | Card, CardHeader |
| `frontend/src/pages/Followers.tsx` | fmt, Followers |
| `frontend/src/components/alerts/AlertsList.tsx` | AlertsList |
| `frontend/src/components/ui/Badge.tsx` | Badge |
| `frontend/src/pages/Alerts.tsx` | Alerts |
| `frontend/src/pages/Dashboard.tsx` | Dashboard |
| `frontend/src/pages/Execution.tsx` | Execution |
| `frontend/src/pages/Masters.tsx` | Masters |

## Entry Points

Start here when exploring this area:

- **`usePnlSummary`** (Function) — `frontend/src/hooks/usePnl.ts:3`
- **`useOrders`** (Function) — `frontend/src/hooks/usePnl.ts:11`
- **`AlertsList`** (Function) — `frontend/src/components/alerts/AlertsList.tsx:13`
- **`PnlSummary`** (Function) — `frontend/src/components/pnl/PnlSummary.tsx:10`
- **`Badge`** (Function) — `frontend/src/components/ui/Badge.tsx:18`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `usePnlSummary` | Function | `frontend/src/hooks/usePnl.ts` | 3 |
| `useOrders` | Function | `frontend/src/hooks/usePnl.ts` | 11 |
| `AlertsList` | Function | `frontend/src/components/alerts/AlertsList.tsx` | 13 |
| `PnlSummary` | Function | `frontend/src/components/pnl/PnlSummary.tsx` | 10 |
| `Badge` | Function | `frontend/src/components/ui/Badge.tsx` | 18 |
| `Card` | Function | `frontend/src/components/ui/Card.tsx` | 3 |
| `CardHeader` | Function | `frontend/src/components/ui/Card.tsx` | 21 |
| `Alerts` | Function | `frontend/src/pages/Alerts.tsx` | 3 |
| `Dashboard` | Function | `frontend/src/pages/Dashboard.tsx` | 6 |
| `Execution` | Function | `frontend/src/pages/Execution.tsx` | 26 |
| `Followers` | Function | `frontend/src/pages/Followers.tsx` | 18 |
| `Masters` | Function | `frontend/src/pages/Masters.tsx` | 3 |
| `PnL` | Function | `frontend/src/pages/PnL.tsx` | 3 |
| `Settings` | Function | `frontend/src/pages/Settings.tsx` | 2 |
| `Signals` | Function | `frontend/src/pages/Signals.tsx` | 3 |
| `pnlColor` | Function | `frontend/src/components/pnl/PnlSummary.tsx` | 4 |
| `fmt` | Function | `frontend/src/pages/Followers.tsx` | 12 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Dashboard → Connect` | cross_community | 5 |
| `Signals → Connect` | cross_community | 5 |
| `Dashboard → UseActivityStore` | cross_community | 4 |
| `Dashboard → Badge` | cross_community | 4 |
| `Signals → UseActivityStore` | cross_community | 4 |
| `Signals → Badge` | cross_community | 4 |
| `Dashboard → UsePnlSummary` | intra_community | 3 |
| `Dashboard → Card` | intra_community | 3 |
| `Dashboard → PnlColor` | intra_community | 3 |
| `Alerts → Badge` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Hooks | 5 calls |

## How to Explore

1. `gitnexus_context({name: "usePnlSummary"})` — see callers and callees
2. `gitnexus_query({query: "pages"})` — find related execution flows
3. Read key files listed above for implementation details
