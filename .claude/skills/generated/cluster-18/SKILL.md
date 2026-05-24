---
name: cluster-18
description: "Skill for the Cluster_18 area of arbitek trade copier. 5 symbols across 3 files."
---

# Cluster_18

5 symbols | 3 files | Cohesion: 73%

## When to Use

- Working with code in `backend/`
- Understanding how run_migrations, list_active_by_platform, poll_accounts work
- Modifying cluster_18-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/db/src/lib.rs` | run_migrations, list_active_by_platform |
| `backend/crates/worker/src/main.rs` | run_polymarket_pipeline_once, run_polymarket_poll_once |
| `backend/crates/ingest/src/lib.rs` | poll_accounts |

## Entry Points

Start here when exploring this area:

- **`run_migrations`** (Function) — `backend/crates/db/src/lib.rs:17`
- **`list_active_by_platform`** (Function) — `backend/crates/db/src/lib.rs:173`
- **`poll_accounts`** (Function) — `backend/crates/ingest/src/lib.rs:107`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `run_migrations` | Function | `backend/crates/db/src/lib.rs` | 17 |
| `list_active_by_platform` | Function | `backend/crates/db/src/lib.rs` | 173 |
| `poll_accounts` | Function | `backend/crates/ingest/src/lib.rs` | 107 |
| `run_polymarket_pipeline_once` | Function | `backend/crates/worker/src/main.rs` | 75 |
| `run_polymarket_poll_once` | Function | `backend/crates/worker/src/main.rs` | 82 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Decimal_field` | cross_community | 5 |
| `Main → String_field` | cross_community | 5 |
| `Main → Normalize_side` | cross_community | 5 |
| `Main → Normalize_outcome` | cross_community | 5 |
| `Main → Run_migrations` | cross_community | 4 |
| `Main → List_active_by_platform` | cross_community | 4 |
| `Main → Poll_accounts` | cross_community | 4 |
| `Main → List_without_signal_by_platform` | cross_community | 4 |
| `Main → Upsert_stub` | cross_community | 4 |
| `Main → List_by_platform_and_status` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_20 | 1 calls |
| Cluster_21 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "run_migrations"})` — see callers and callees
2. `gitnexus_query({query: "cluster_18"})` — find related execution flows
3. Read key files listed above for implementation details
