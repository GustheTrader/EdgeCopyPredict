---
name: api
description: "Skill for the Api area of arbitek trade copier. 20 symbols across 5 files."
---

# Api

20 symbols | 5 files | Cohesion: 98%

## When to Use

- Working with code in `frontend/`
- Understanding how get, post, patch work
- Modifying api-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `frontend/src/api/masters.ts` | list, get, signals, setStatus, pause (+1) |
| `frontend/src/api/client.ts` | ApiError, request, get, post, patch |
| `frontend/src/api/followers.ts` | setStatus, pause, resume, list, get |
| `frontend/src/api/signals.ts` | list, get, orders |
| `backend/crates/api/src/main.rs` | router |

## Entry Points

Start here when exploring this area:

- **`get`** (Function) — `frontend/src/api/client.ts:27`
- **`post`** (Function) — `frontend/src/api/client.ts:28`
- **`patch`** (Function) — `frontend/src/api/client.ts:30`
- **`setStatus`** (Function) — `frontend/src/api/followers.ts:6`
- **`pause`** (Function) — `frontend/src/api/followers.ts:8`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `get` | Function | `frontend/src/api/client.ts` | 27 |
| `post` | Function | `frontend/src/api/client.ts` | 28 |
| `patch` | Function | `frontend/src/api/client.ts` | 30 |
| `setStatus` | Function | `frontend/src/api/followers.ts` | 6 |
| `pause` | Function | `frontend/src/api/followers.ts` | 8 |
| `resume` | Function | `frontend/src/api/followers.ts` | 9 |
| `list` | Function | `frontend/src/api/masters.ts` | 4 |
| `get` | Function | `frontend/src/api/masters.ts` | 5 |
| `signals` | Function | `frontend/src/api/masters.ts` | 10 |
| `setStatus` | Function | `frontend/src/api/masters.ts` | 6 |
| `pause` | Function | `frontend/src/api/masters.ts` | 8 |
| `resume` | Function | `frontend/src/api/masters.ts` | 9 |
| `list` | Function | `frontend/src/api/signals.ts` | 4 |
| `get` | Function | `frontend/src/api/signals.ts` | 18 |
| `orders` | Function | `frontend/src/api/signals.ts` | 19 |
| `list` | Function | `frontend/src/api/followers.ts` | 4 |
| `get` | Function | `frontend/src/api/followers.ts` | 5 |
| `ApiError` | Class | `frontend/src/api/client.ts` | 2 |
| `router` | Function | `backend/crates/api/src/main.rs` | 252 |
| `request` | Function | `frontend/src/api/client.ts` | 12 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → ApiError` | cross_community | 5 |
| `Get → ApiError` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "get"})` — see callers and callees
2. `gitnexus_query({query: "api"})` — find related execution flows
3. Read key files listed above for implementation details
