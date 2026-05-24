<!-- gitnexus:start -->
# GitNexus — Code Intelligence

This project is indexed by GitNexus as **arbitek trade copier** (1375 symbols, 2434 relationships, 76 execution flows). Use the GitNexus MCP tools to understand code, assess impact, and navigate safely.

> If any GitNexus tool warns the index is stale, run `npx gitnexus analyze` in terminal first.

## Always Do

- **MUST run impact analysis before editing any symbol.** Before modifying a function, class, or method, run `gitnexus_impact({target: "symbolName", direction: "upstream"})` and report the blast radius (direct callers, affected processes, risk level) to the user.
- **MUST run `gitnexus_detect_changes()` before committing** to verify your changes only affect expected symbols and execution flows.
- **MUST warn the user** if impact analysis returns HIGH or CRITICAL risk before proceeding with edits.
- When exploring unfamiliar code, use `gitnexus_query({query: "concept"})` to find execution flows instead of grepping. It returns process-grouped results ranked by relevance.
- When you need full context on a specific symbol — callers, callees, which execution flows it participates in — use `gitnexus_context({name: "symbolName"})`.

## Never Do

- NEVER edit a function, class, or method without first running `gitnexus_impact` on it.
- NEVER ignore HIGH or CRITICAL risk warnings from impact analysis.
- NEVER rename symbols with find-and-replace — use `gitnexus_rename` which understands the call graph.
- NEVER commit changes without running `gitnexus_detect_changes()` to check affected scope.

## Resources

| Resource | Use for |
|----------|---------|
| `gitnexus://repo/arbitek trade copier/context` | Codebase overview, check index freshness |
| `gitnexus://repo/arbitek trade copier/clusters` | All functional areas |
| `gitnexus://repo/arbitek trade copier/processes` | All execution flows |
| `gitnexus://repo/arbitek trade copier/process/{name}` | Step-by-step execution trace |

## CLI

| Task | Read this skill file |
|------|---------------------|
| Understand architecture / "How does X work?" | `.claude/skills/gitnexus/gitnexus-exploring/SKILL.md` |
| Blast radius / "What breaks if I change X?" | `.claude/skills/gitnexus/gitnexus-impact-analysis/SKILL.md` |
| Trace bugs / "Why is X failing?" | `.claude/skills/gitnexus/gitnexus-debugging/SKILL.md` |
| Rename / extract / split / refactor | `.claude/skills/gitnexus/gitnexus-refactoring/SKILL.md` |
| Tools, resources, schema reference | `.claude/skills/gitnexus/gitnexus-guide/SKILL.md` |
| Index, status, clean, wiki CLI commands | `.claude/skills/gitnexus/gitnexus-cli/SKILL.md` |
| Work in the Api area (20 symbols) | `.claude/skills/generated/api/SKILL.md` |
| Work in the Pages area (17 symbols) | `.claude/skills/generated/pages/SKILL.md` |
| Work in the Cluster_0 area (12 symbols) | `.claude/skills/generated/cluster-0/SKILL.md` |
| Work in the Hooks area (11 symbols) | `.claude/skills/generated/hooks/SKILL.md` |
| Work in the Cluster_50 area (9 symbols) | `.claude/skills/generated/cluster-50/SKILL.md` |
| Work in the Cluster_20 area (7 symbols) | `.claude/skills/generated/cluster-20/SKILL.md` |
| Work in the Cluster_47 area (7 symbols) | `.claude/skills/generated/cluster-47/SKILL.md` |
| Work in the Layout area (7 symbols) | `.claude/skills/generated/layout/SKILL.md` |
| Work in the Cluster_34 area (6 symbols) | `.claude/skills/generated/cluster-34/SKILL.md` |
| Work in the Cluster_35 area (6 symbols) | `.claude/skills/generated/cluster-35/SKILL.md` |
| Work in the Cluster_1 area (5 symbols) | `.claude/skills/generated/cluster-1/SKILL.md` |
| Work in the Cluster_18 area (5 symbols) | `.claude/skills/generated/cluster-18/SKILL.md` |
| Work in the Cluster_21 area (5 symbols) | `.claude/skills/generated/cluster-21/SKILL.md` |
| Work in the Cluster_30 area (5 symbols) | `.claude/skills/generated/cluster-30/SKILL.md` |
| Work in the Cluster_32 area (5 symbols) | `.claude/skills/generated/cluster-32/SKILL.md` |
| Work in the Cluster_4 area (4 symbols) | `.claude/skills/generated/cluster-4/SKILL.md` |
| Work in the Publish area (4 symbols) | `.claude/skills/generated/publish/SKILL.md` |

<!-- gitnexus:end -->
