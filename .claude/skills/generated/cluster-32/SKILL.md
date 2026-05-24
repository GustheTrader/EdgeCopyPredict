---
name: cluster-32
description: "Skill for the Cluster_32 area of arbitek trade copier. 5 symbols across 1 files."
---

# Cluster_32

5 symbols | 1 files | Cohesion: 92%

## When to Use

- Working with code in `backend/`
- Understanding how execute_dry_run work
- Modifying cluster_32-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/execution/src/lib.rs` | execute_dry_run, test_input, dry_run_fills_at_price_limit, dry_run_request_payload_includes_mode, dry_run_response_marks_dry_run_true |

## Entry Points

Start here when exploring this area:

- **`execute_dry_run`** (Function) — `backend/crates/execution/src/lib.rs:67`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `execute_dry_run` | Function | `backend/crates/execution/src/lib.rs` | 67 |
| `test_input` | Function | `backend/crates/execution/src/lib.rs` | 121 |
| `dry_run_fills_at_price_limit` | Function | `backend/crates/execution/src/lib.rs` | 138 |
| `dry_run_request_payload_includes_mode` | Function | `backend/crates/execution/src/lib.rs` | 150 |
| `dry_run_response_marks_dry_run_true` | Function | `backend/crates/execution/src/lib.rs` | 156 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Run_execution_dry_run_once → ExecutionOutput` | cross_community | 3 |
| `Dry_run_fills_at_price_limit → ExecutionInput` | intra_community | 3 |
| `Dry_run_fills_at_price_limit → ExecutionOutput` | intra_community | 3 |
| `Dry_run_request_payload_includes_mode → ExecutionOutput` | intra_community | 3 |
| `Dry_run_request_payload_includes_mode → ExecutionInput` | intra_community | 3 |
| `Dry_run_response_marks_dry_run_true → ExecutionOutput` | intra_community | 3 |
| `Dry_run_response_marks_dry_run_true → ExecutionInput` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "execute_dry_run"})` — see callers and callees
2. `gitnexus_query({query: "cluster_32"})` — find related execution flows
3. Read key files listed above for implementation details
