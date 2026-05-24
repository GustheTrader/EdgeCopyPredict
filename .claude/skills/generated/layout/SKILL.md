---
name: layout
description: "Skill for the Layout area of arbitek trade copier. 7 symbols across 6 files."
---

# Layout

7 symbols | 6 files | Cohesion: 100%

## When to Use

- Working with code in `frontend/`
- Understanding how useAlertStore, useSystemStore, AppShell work
- Modifying layout-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `frontend/src/components/layout/TopBar.tsx` | TopBar, setStatus |
| `frontend/src/stores/alertStore.ts` | useAlertStore |
| `frontend/src/stores/systemStore.ts` | useSystemStore |
| `frontend/src/components/layout/AppShell.tsx` | AppShell |
| `frontend/src/components/layout/Sidebar.tsx` | Sidebar |
| `frontend/src/components/ui/StatusDot.tsx` | StatusDot |

## Entry Points

Start here when exploring this area:

- **`useAlertStore`** (Function) — `frontend/src/stores/alertStore.ts:9`
- **`useSystemStore`** (Function) — `frontend/src/stores/systemStore.ts:8`
- **`AppShell`** (Function) — `frontend/src/components/layout/AppShell.tsx:4`
- **`Sidebar`** (Function) — `frontend/src/components/layout/Sidebar.tsx:15`
- **`TopBar`** (Function) — `frontend/src/components/layout/TopBar.tsx:7`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `useAlertStore` | Function | `frontend/src/stores/alertStore.ts` | 9 |
| `useSystemStore` | Function | `frontend/src/stores/systemStore.ts` | 8 |
| `AppShell` | Function | `frontend/src/components/layout/AppShell.tsx` | 4 |
| `Sidebar` | Function | `frontend/src/components/layout/Sidebar.tsx` | 15 |
| `TopBar` | Function | `frontend/src/components/layout/TopBar.tsx` | 7 |
| `setStatus` | Function | `frontend/src/components/layout/TopBar.tsx` | 13 |
| `StatusDot` | Function | `frontend/src/components/ui/StatusDot.tsx` | 11 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `AppShell → UseAlertStore` | intra_community | 3 |
| `AppShell → UseSystemStore` | intra_community | 3 |
| `AppShell → SetStatus` | intra_community | 3 |
| `AppShell → StatusDot` | intra_community | 3 |

## How to Explore

1. `gitnexus_context({name: "useAlertStore"})` — see callers and callees
2. `gitnexus_query({query: "layout"})` — find related execution flows
3. Read key files listed above for implementation details
