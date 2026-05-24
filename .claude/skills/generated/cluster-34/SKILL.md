---
name: cluster-34
description: "Skill for the Cluster_34 area of arbitek trade copier. 6 symbols across 2 files."
---

# Cluster_34

6 symbols | 2 files | Cohesion: 100%

## When to Use

- Working with code in `backend/`
- Understanding how polymarket_trade_to_raw_command, from_raw, external_event_id work
- Modifying cluster_34-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/platform-polymarket/src/lib.rs` | from_raw, external_event_id, parses_data_api_trade_fixture, rejects_trade_without_transaction_hash |
| `backend/crates/ingest/src/lib.rs` | polymarket_trade_to_raw_command, polymarket_trade_maps_to_raw_command |

## Entry Points

Start here when exploring this area:

- **`polymarket_trade_to_raw_command`** (Function) — `backend/crates/ingest/src/lib.rs:122`
- **`from_raw`** (Function) — `backend/crates/platform-polymarket/src/lib.rs:96`
- **`external_event_id`** (Function) — `backend/crates/platform-polymarket/src/lib.rs:114`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `polymarket_trade_to_raw_command` | Function | `backend/crates/ingest/src/lib.rs` | 122 |
| `from_raw` | Function | `backend/crates/platform-polymarket/src/lib.rs` | 96 |
| `external_event_id` | Function | `backend/crates/platform-polymarket/src/lib.rs` | 114 |
| `polymarket_trade_maps_to_raw_command` | Function | `backend/crates/ingest/src/lib.rs` | 287 |
| `parses_data_api_trade_fixture` | Function | `backend/crates/platform-polymarket/src/lib.rs` | 170 |
| `rejects_trade_without_transaction_hash` | Function | `backend/crates/platform-polymarket/src/lib.rs` | 191 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Polymarket_trade_maps_to_raw_command → IngestRawEventCommand` | intra_community | 3 |
| `Polymarket_trade_maps_to_raw_command → External_event_id` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "polymarket_trade_to_raw_command"})` — see callers and callees
2. `gitnexus_query({query: "cluster_34"})` — find related execution flows
3. Read key files listed above for implementation details
