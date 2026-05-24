---
name: publish
description: "Skill for the Publish area of arbitek trade copier. 4 symbols across 2 files."
---

# Publish

4 symbols | 2 files | Cohesion: 67%

## When to Use

- Working with code in `backend/`
- Understanding how publish work
- Modifying publish-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `backend/crates/worker/src/main.rs` | publish_copy_intent_created, publish_signal_normalized, publish_execution_report |
| `backend/crates/queue/src/lib.rs` | publish |

## Entry Points

Start here when exploring this area:

- **`publish`** (Function) — `backend/crates/queue/src/lib.rs:80`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `publish` | Function | `backend/crates/queue/src/lib.rs` | 80 |
| `publish_copy_intent_created` | Function | `backend/crates/worker/src/main.rs` | 306 |
| `publish_signal_normalized` | Function | `backend/crates/worker/src/main.rs` | 334 |
| `publish_execution_report` | Function | `backend/crates/worker/src/main.rs` | 492 |

## How to Explore

1. `gitnexus_context({name: "publish"})` — see callers and callees
2. `gitnexus_query({query: "publish"})` — find related execution flows
3. Read key files listed above for implementation details
