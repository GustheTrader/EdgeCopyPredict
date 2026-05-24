---
name: cluster-20
description: "Skill for the Cluster_20 area of arbitek trade copier. 7 symbols across 2 files."
---

# Cluster_20

7 symbols | 2 files | Cohesion: 75%

## When to Use

- Working with code in `backend/`
- Understanding how list_without_signal_by_platform, upsert_stub, insert_idempotent work
- Modifying cluster_20-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/worker/src/main.rs` | run_polymarket_normalize_once, raw_event_for_signal, market_stub_from_signal, new_signal_from_normalized |
| `backend/crates/db/src/lib.rs` | list_without_signal_by_platform, upsert_stub, insert_idempotent |

## Entry Points

Start here when exploring this area:

- **`list_without_signal_by_platform`** (Function) — `backend/crates/db/src/lib.rs:298`
- **`upsert_stub`** (Function) — `backend/crates/db/src/lib.rs:343`
- **`insert_idempotent`** (Function) — `backend/crates/db/src/lib.rs:400`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `list_without_signal_by_platform` | Function | `backend/crates/db/src/lib.rs` | 298 |
| `upsert_stub` | Function | `backend/crates/db/src/lib.rs` | 343 |
| `insert_idempotent` | Function | `backend/crates/db/src/lib.rs` | 400 |
| `run_polymarket_normalize_once` | Function | `backend/crates/worker/src/main.rs` | 130 |
| `raw_event_for_signal` | Function | `backend/crates/worker/src/main.rs` | 358 |
| `market_stub_from_signal` | Function | `backend/crates/worker/src/main.rs` | 370 |
| `new_signal_from_normalized` | Function | `backend/crates/worker/src/main.rs` | 385 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Decimal_field` | cross_community | 5 |
| `Main → String_field` | cross_community | 5 |
| `Main → Normalize_side` | cross_community | 5 |
| `Main → Normalize_outcome` | cross_community | 5 |
| `Main → List_without_signal_by_platform` | cross_community | 4 |
| `Main → Upsert_stub` | cross_community | 4 |
| `Run_polymarket_normalize_once → RawEventForSignal` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_50 | 1 calls |
| Publish | 1 calls |

## How to Explore

1. `gitnexus_context({name: "list_without_signal_by_platform"})` — see callers and callees
2. `gitnexus_query({query: "cluster_20"})` — find related execution flows
3. Read key files listed above for implementation details
