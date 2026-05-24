---
name: cluster-21
description: "Skill for the Cluster_21 area of arbitek trade copier. 5 symbols across 2 files."
---

# Cluster_21

5 symbols | 2 files | Cohesion: 62%

## When to Use

- Working with code in `backend/`
- Understanding how list_by_platform_and_status, list_active_by_platform, insert_idempotent work
- Modifying cluster_21-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/db/src/lib.rs` | list_by_platform_and_status, list_active_by_platform, insert_idempotent |
| `backend/crates/worker/src/main.rs` | run_polymarket_copy_intents_once, copy_intent_from_decision |

## Entry Points

Start here when exploring this area:

- **`list_by_platform_and_status`** (Function) — `backend/crates/db/src/lib.rs:466`
- **`list_active_by_platform`** (Function) — `backend/crates/db/src/lib.rs:519`
- **`insert_idempotent`** (Function) — `backend/crates/db/src/lib.rs:561`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `list_by_platform_and_status` | Function | `backend/crates/db/src/lib.rs` | 466 |
| `list_active_by_platform` | Function | `backend/crates/db/src/lib.rs` | 519 |
| `insert_idempotent` | Function | `backend/crates/db/src/lib.rs` | 561 |
| `run_polymarket_copy_intents_once` | Function | `backend/crates/worker/src/main.rs` | 186 |
| `copy_intent_from_decision` | Function | `backend/crates/worker/src/main.rs` | 275 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → List_by_platform_and_status` | cross_community | 4 |
| `Main → List_active_by_platform` | cross_community | 4 |
| `Main → From_json` | cross_community | 4 |
| `Run_polymarket_copy_intents_once → RiskDecision` | cross_community | 4 |
| `Run_polymarket_copy_intents_once → Price_limit` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_47 | 2 calls |
| Publish | 1 calls |

## How to Explore

1. `gitnexus_context({name: "list_by_platform_and_status"})` — see callers and callees
2. `gitnexus_query({query: "cluster_21"})` — find related execution flows
3. Read key files listed above for implementation details
