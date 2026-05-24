---
name: cluster-1
description: "Skill for the Cluster_1 area of arbitek trade copier. 5 symbols across 3 files."
---

# Cluster_1

5 symbols | 3 files | Cohesion: 62%

## When to Use

- Working with code in `backend/`
- Understanding how init_tracing work
- Modifying cluster_1-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/worker/src/main.rs` | main, run_polymarket_pipeline_loop, wait_for_shutdown |
| `backend/crates/api/src/main.rs` | main |
| `backend/crates/config/src/lib.rs` | init_tracing |

## Entry Points

Start here when exploring this area:

- **`init_tracing`** (Function) — `backend/crates/config/src/lib.rs:164`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `init_tracing` | Function | `backend/crates/config/src/lib.rs` | 164 |
| `main` | Function | `backend/crates/api/src/main.rs` | 226 |
| `main` | Function | `backend/crates/worker/src/main.rs` | 23 |
| `run_polymarket_pipeline_loop` | Function | `backend/crates/worker/src/main.rs` | 51 |
| `wait_for_shutdown` | Function | `backend/crates/worker/src/main.rs` | 520 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Decimal_field` | cross_community | 5 |
| `Main → String_field` | cross_community | 5 |
| `Main → Normalize_side` | cross_community | 5 |
| `Main → Normalize_outcome` | cross_community | 5 |
| `Main → ApiError` | cross_community | 5 |
| `Main → Run_migrations` | cross_community | 4 |
| `Main → List_active_by_platform` | cross_community | 4 |
| `Main → Poll_accounts` | cross_community | 4 |
| `Main → List_without_signal_by_platform` | cross_community | 4 |
| `Main → Upsert_stub` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Api | 1 calls |
| Cluster_18 | 1 calls |
| Cluster_20 | 1 calls |
| Cluster_21 | 1 calls |
| Cluster_30 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "init_tracing"})` — see callers and callees
2. `gitnexus_query({query: "cluster_1"})` — find related execution flows
3. Read key files listed above for implementation details
