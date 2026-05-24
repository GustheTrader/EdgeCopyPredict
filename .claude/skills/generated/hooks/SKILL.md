---
name: hooks
description: "Skill for the Hooks area of arbitek trade copier. 11 symbols across 8 files."
---

# Hooks

11 symbols | 8 files | Cohesion: 74%

## When to Use

- Working with code in `frontend/`
- Understanding how useLiveActivityFeed, useWebSocket, connect work
- Modifying hooks-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `frontend/src/hooks/useMasters.ts` | useMasters, usePauseMaster, useResumeMaster |
| `frontend/src/hooks/useWebSocket.ts` | useWebSocket, connect |
| `frontend/src/hooks/useSignals.ts` | useLiveActivityFeed |
| `frontend/src/stores/activityStore.ts` | useActivityStore |
| `frontend/src/components/signals/ActivityFeed.tsx` | ActivityFeed |
| `frontend/src/components/signals/SignalRow.tsx` | SignalRow |
| `frontend/src/components/masters/MasterLeaderboard.tsx` | MasterLeaderboard |
| `frontend/src/components/ui/Button.tsx` | Button |

## Entry Points

Start here when exploring this area:

- **`useLiveActivityFeed`** (Function) — `frontend/src/hooks/useSignals.ts:31`
- **`useWebSocket`** (Function) — `frontend/src/hooks/useWebSocket.ts:11`
- **`connect`** (Function) — `frontend/src/hooks/useWebSocket.ts:21`
- **`useActivityStore`** (Function) — `frontend/src/stores/activityStore.ts:11`
- **`ActivityFeed`** (Function) — `frontend/src/components/signals/ActivityFeed.tsx:4`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `useLiveActivityFeed` | Function | `frontend/src/hooks/useSignals.ts` | 31 |
| `useWebSocket` | Function | `frontend/src/hooks/useWebSocket.ts` | 11 |
| `connect` | Function | `frontend/src/hooks/useWebSocket.ts` | 21 |
| `useActivityStore` | Function | `frontend/src/stores/activityStore.ts` | 11 |
| `ActivityFeed` | Function | `frontend/src/components/signals/ActivityFeed.tsx` | 4 |
| `SignalRow` | Function | `frontend/src/components/signals/SignalRow.tsx` | 14 |
| `useMasters` | Function | `frontend/src/hooks/useMasters.ts` | 3 |
| `usePauseMaster` | Function | `frontend/src/hooks/useMasters.ts` | 23 |
| `useResumeMaster` | Function | `frontend/src/hooks/useMasters.ts` | 31 |
| `MasterLeaderboard` | Function | `frontend/src/components/masters/MasterLeaderboard.tsx` | 12 |
| `Button` | Function | `frontend/src/components/ui/Button.tsx` | 16 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Dashboard → Connect` | cross_community | 5 |
| `Signals → Connect` | cross_community | 5 |
| `Dashboard → UseActivityStore` | cross_community | 4 |
| `Dashboard → Badge` | cross_community | 4 |
| `Signals → UseActivityStore` | cross_community | 4 |
| `Signals → Badge` | cross_community | 4 |
| `Alerts → Button` | cross_community | 3 |
| `Masters → UseMasters` | cross_community | 3 |
| `Masters → UsePauseMaster` | cross_community | 3 |
| `Masters → UseResumeMaster` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Pages | 2 calls |

## How to Explore

1. `gitnexus_context({name: "useLiveActivityFeed"})` — see callers and callees
2. `gitnexus_query({query: "hooks"})` — find related execution flows
3. Read key files listed above for implementation details
