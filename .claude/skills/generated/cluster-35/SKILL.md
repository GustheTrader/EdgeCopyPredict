---
name: cluster-35
description: "Skill for the Cluster_35 area of arbitek trade copier. 6 symbols across 1 files."
---

# Cluster_35

6 symbols | 1 files | Cohesion: 100%

## When to Use

- Working with code in `backend/`
- Understanding how ensure_polymarket_account, validate_command, ensure_present work
- Modifying cluster_35-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/ingest/src/lib.rs` | ensure_polymarket_account, validate_command, ensure_present, validate_command_rejects_empty_required_fields, ensure_polymarket_account_rejects_wrong_platform (+1) |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `ensure_polymarket_account` | Function | `backend/crates/ingest/src/lib.rs` | 137 |
| `validate_command` | Function | `backend/crates/ingest/src/lib.rs` | 227 |
| `ensure_present` | Function | `backend/crates/ingest/src/lib.rs` | 235 |
| `validate_command_rejects_empty_required_fields` | Function | `backend/crates/ingest/src/lib.rs` | 271 |
| `ensure_polymarket_account_rejects_wrong_platform` | Function | `backend/crates/ingest/src/lib.rs` | 310 |
| `ensure_polymarket_account_accepts_active_polymarket_account` | Function | `backend/crates/ingest/src/lib.rs` | 326 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Ensure_polymarket_account_rejects_wrong_platform → Ensure_present` | intra_community | 3 |
| `Ensure_polymarket_account_accepts_active_polymarket_account → Ensure_present` | intra_community | 3 |
| `Validate_command_rejects_empty_required_fields → Ensure_present` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "ensure_polymarket_account"})` — see callers and callees
2. `gitnexus_query({query: "cluster_35"})` — find related execution flows
3. Read key files listed above for implementation details
