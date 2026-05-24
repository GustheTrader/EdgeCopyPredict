---
name: cluster-4
description: "Skill for the Cluster_4 area of arbitek trade copier. 4 symbols across 2 files."
---

# Cluster_4

4 symbols | 2 files | Cohesion: 67%

## When to Use

- Working with code in `backend/`
- Understanding how get_by_id, update_status work
- Modifying cluster_4-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/api/src/main.rs` | update_master_status, master_to_dto |
| `backend/crates/db/src/lib.rs` | get_by_id, update_status |

## Entry Points

Start here when exploring this area:

- **`get_by_id`** (Function) — `backend/crates/db/src/lib.rs:654`
- **`update_status`** (Function) — `backend/crates/db/src/lib.rs:665`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `get_by_id` | Function | `backend/crates/db/src/lib.rs` | 654 |
| `update_status` | Function | `backend/crates/db/src/lib.rs` | 665 |
| `update_master_status` | Function | `backend/crates/api/src/main.rs` | 320 |
| `master_to_dto` | Function | `backend/crates/api/src/main.rs` | 532 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Update_master_status → MasterDto` | intra_community | 3 |
| `Get_master → MasterDto` | cross_community | 3 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_0 | 2 calls |

## How to Explore

1. `gitnexus_context({name: "get_by_id"})` — see callers and callees
2. `gitnexus_query({query: "cluster_4"})` — find related execution flows
3. Read key files listed above for implementation details
