---
name: cluster-0
description: "Skill for the Cluster_0 area of arbitek trade copier. 12 symbols across 2 files."
---

# Cluster_0

12 symbols | 2 files | Cohesion: 92%

## When to Use

- Working with code in `backend/`
- Understanding how get_by_id, update_status, update_risk_config work
- Modifying cluster_0-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/api/src/main.rs` | into_response, get_master, get_follower, update_follower_status, update_follower_risk_config (+4) |
| `backend/crates/db/src/lib.rs` | get_by_id, update_status, update_risk_config |

## Entry Points

Start here when exploring this area:

- **`get_by_id`** (Function) — `backend/crates/db/src/lib.rs:722`
- **`update_status`** (Function) — `backend/crates/db/src/lib.rs:733`
- **`update_risk_config`** (Function) — `backend/crates/db/src/lib.rs:742`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `get_by_id` | Function | `backend/crates/db/src/lib.rs` | 722 |
| `update_status` | Function | `backend/crates/db/src/lib.rs` | 733 |
| `update_risk_config` | Function | `backend/crates/db/src/lib.rs` | 742 |
| `into_response` | Function | `backend/crates/api/src/main.rs` | 43 |
| `get_master` | Function | `backend/crates/api/src/main.rs` | 310 |
| `get_follower` | Function | `backend/crates/api/src/main.rs` | 342 |
| `update_follower_status` | Function | `backend/crates/api/src/main.rs` | 352 |
| `update_follower_risk_config` | Function | `backend/crates/api/src/main.rs` | 365 |
| `get_signal` | Function | `backend/crates/api/src/main.rs` | 403 |
| `follower_to_dto` | Function | `backend/crates/api/src/main.rs` | 544 |
| `signal_to_dto` | Function | `backend/crates/api/src/main.rs` | 556 |
| `not_found` | Function | `backend/crates/api/src/main.rs` | 613 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Update_follower_status → FollowerDto` | intra_community | 3 |
| `Update_follower_risk_config → FollowerDto` | intra_community | 3 |
| `Update_follower_risk_config → Into_response` | intra_community | 3 |
| `Get_master → MasterDto` | cross_community | 3 |
| `Get_master → Into_response` | intra_community | 3 |
| `Get_follower → FollowerDto` | intra_community | 3 |
| `Get_follower → Into_response` | intra_community | 3 |
| `Get_signal → SignalDto` | intra_community | 3 |
| `Get_signal → Into_response` | intra_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_4 | 1 calls |

## How to Explore

1. `gitnexus_context({name: "get_by_id"})` — see callers and callees
2. `gitnexus_query({query: "cluster_0"})` — find related execution flows
3. Read key files listed above for implementation details
