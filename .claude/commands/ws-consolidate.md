# Workstream Consolidate

Capture learnings from completed work and update durable documentation.

## Usage
```
/ws-consolidate
```

---

## Process

### 1. Identify What Changed

Review recent commits, modified files, and completed tasks.

### 2. Update Workstream Docs

- **PLAN.md** — Mark completed milestones with `[x]`, update status
- **SHARED-CONTEXT.md** — Add decisions, patterns, known issues discovered during implementation
- **SPEC.md** — Update if requirements changed during implementation

### 3. Check for Promotions

Should any learnings be promoted to project-level docs?

| Target | Promote When |
|--------|-------------|
| `INVARIANTS.md` | New constraint that applies to all future work |
| `ARCHITECTURE.md` | New interface or system boundary |
| `CLAUDE.md` / `AGENTS.md` | New convention or build command |

### 4. Clean Up

- Archive ephemeral task docs (CONTEXT.md, VALIDATION.md) — their knowledge is now in SHARED-CONTEXT.md
- Clean up `exec/` iteration artifacts (keep summaries, remove individual iterations)

### 5. Commit

Create a single consolidation commit with all doc updates.
