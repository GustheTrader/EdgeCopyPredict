---
name: cluster-50
description: "Skill for the Cluster_50 area of arbitek trade copier. 9 symbols across 1 files."
---

# Cluster_50

9 symbols | 1 files | Cohesion: 94%

## When to Use

- Working with code in `backend/`
- Understanding how normalize_polymarket_trade work
- Modifying cluster_50-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/signal/src/lib.rs` | normalize_polymarket_trade, deterministic_id, normalize_side, normalize_outcome, position_effect_for_side (+4) |

## Entry Points

Start here when exploring this area:

- **`normalize_polymarket_trade`** (Function) — `backend/crates/signal/src/lib.rs:75`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `normalize_polymarket_trade` | Function | `backend/crates/signal/src/lib.rs` | 75 |
| `deterministic_id` | Function | `backend/crates/signal/src/lib.rs` | 175 |
| `normalize_side` | Function | `backend/crates/signal/src/lib.rs` | 182 |
| `normalize_outcome` | Function | `backend/crates/signal/src/lib.rs` | 190 |
| `position_effect_for_side` | Function | `backend/crates/signal/src/lib.rs` | 198 |
| `string_field` | Function | `backend/crates/signal/src/lib.rs` | 206 |
| `decimal_field` | Function | `backend/crates/signal/src/lib.rs` | 213 |
| `normalizes_polymarket_trade_fixture` | Function | `backend/crates/signal/src/lib.rs` | 241 |
| `rejects_trade_without_master_id` | Function | `backend/crates/signal/src/lib.rs` | 282 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Decimal_field` | cross_community | 5 |
| `Main → String_field` | cross_community | 5 |
| `Main → Normalize_side` | cross_community | 5 |
| `Main → Normalize_outcome` | cross_community | 5 |
| `Normalizes_polymarket_trade_fixture → Decimal_field` | intra_community | 3 |
| `Normalizes_polymarket_trade_fixture → String_field` | intra_community | 3 |
| `Normalizes_polymarket_trade_fixture → Normalize_side` | intra_community | 3 |
| `Normalizes_polymarket_trade_fixture → Normalize_outcome` | intra_community | 3 |
| `Rejects_trade_without_master_id → Decimal_field` | intra_community | 3 |
| `Rejects_trade_without_master_id → String_field` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "normalize_polymarket_trade"})` — see callers and callees
2. `gitnexus_query({query: "cluster_50"})` — find related execution flows
3. Read key files listed above for implementation details
