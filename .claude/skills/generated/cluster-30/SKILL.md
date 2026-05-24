---
name: cluster-30
description: "Skill for the Cluster_30 area of arbitek trade copier. 5 symbols across 3 files."
---

# Cluster_30

5 symbols | 3 files | Cohesion: 73%

## When to Use

- Working with code in `backend/`
- Understanding how list_approved_without_orders, as_str work
- Modifying cluster_30-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/worker/src/main.rs` | run_execution_dry_run_once, execution_input_from_intent, new_order_from_output |
| `backend/crates/db/src/lib.rs` | list_approved_without_orders |
| `backend/crates/execution/src/lib.rs` | as_str |

## Entry Points

Start here when exploring this area:

- **`list_approved_without_orders`** (Function) — `backend/crates/db/src/lib.rs:1239`
- **`as_str`** (Function) — `backend/crates/execution/src/lib.rs:53`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `list_approved_without_orders` | Function | `backend/crates/db/src/lib.rs` | 1239 |
| `as_str` | Function | `backend/crates/execution/src/lib.rs` | 53 |
| `run_execution_dry_run_once` | Function | `backend/crates/worker/src/main.rs` | 410 |
| `execution_input_from_intent` | Function | `backend/crates/worker/src/main.rs` | 453 |
| `new_order_from_output` | Function | `backend/crates/worker/src/main.rs` | 469 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Run_execution_dry_run_once → ExecutionInput` | intra_community | 3 |
| `Run_execution_dry_run_once → ExecutionOutput` | cross_community | 3 |
| `Run_execution_dry_run_once → NewOrder` | intra_community | 3 |
| `Run_execution_dry_run_once → As_str` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_32 | 1 calls |
| Publish | 1 calls |

## How to Explore

1. `gitnexus_context({name: "list_approved_without_orders"})` — see callers and callees
2. `gitnexus_query({query: "cluster_30"})` — find related execution flows
3. Read key files listed above for implementation details
