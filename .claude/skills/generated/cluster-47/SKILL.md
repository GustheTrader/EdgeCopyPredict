---
name: cluster-47
description: "Skill for the Cluster_47 area of arbitek trade copier. 7 symbols across 1 files."
---

# Cluster_47

7 symbols | 1 files | Cohesion: 88%

## When to Use

- Working with code in `backend/`
- Understanding how from_json, evaluate_copy_intent work
- Modifying cluster_47-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/risk/src/lib.rs` | from_json, evaluate_copy_intent, rejected, price_limit, parses_config_from_json_with_defaults (+2) |

## Entry Points

Start here when exploring this area:

- **`from_json`** (Function) — `backend/crates/risk/src/lib.rs:33`
- **`evaluate_copy_intent`** (Function) — `backend/crates/risk/src/lib.rs:85`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `from_json` | Function | `backend/crates/risk/src/lib.rs` | 33 |
| `evaluate_copy_intent` | Function | `backend/crates/risk/src/lib.rs` | 85 |
| `rejected` | Function | `backend/crates/risk/src/lib.rs` | 150 |
| `price_limit` | Function | `backend/crates/risk/src/lib.rs` | 167 |
| `parses_config_from_json_with_defaults` | Function | `backend/crates/risk/src/lib.rs` | 208 |
| `approves_with_scaled_and_capped_notional` | Function | `backend/crates/risk/src/lib.rs` | 222 |
| `rejects_below_minimum_notional` | Function | `backend/crates/risk/src/lib.rs` | 246 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → From_json` | cross_community | 4 |
| `Run_polymarket_copy_intents_once → RiskDecision` | cross_community | 4 |
| `Approves_with_scaled_and_capped_notional → RiskDecision` | intra_community | 4 |
| `Rejects_below_minimum_notional → RiskDecision` | intra_community | 4 |
| `Run_polymarket_copy_intents_once → Price_limit` | cross_community | 3 |
| `Approves_with_scaled_and_capped_notional → Price_limit` | intra_community | 3 |
| `Rejects_below_minimum_notional → Price_limit` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "from_json"})` — see callers and callees
2. `gitnexus_query({query: "cluster_47"})` — find related execution flows
3. Read key files listed above for implementation details
