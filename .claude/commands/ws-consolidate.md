# Workstream Consolidate

Capture learnings from completed work and update durable documentation.

## Usage
```
/ws-consolidate <workstream> [--agent claude|codex]
```

## Arguments
$ARGUMENTS

---

Run the consolidation script:

```bash
./scripts/ws-consolidate.sh $ARGUMENTS
```

## How It Works

The script reviews recent changes and produces a consolidation report:

1. Builds prompt from workstream docs + recent git history
2. Runs a single agent to analyze what changed and what should be updated
3. Saves output to `docs/workstreams/<name>/exec/consolidation-<timestamp>.md`

## What Gets Updated

- **PLAN.md** — mark completed milestones, update phase status
- **SHARED-CONTEXT.md** — add decisions, patterns, known issues from implementation
- **SPEC.md** — update if requirements changed during implementation

## Promotion Checks

The agent checks if any learnings should be promoted to project-level docs:

| Target | Promote When |
|--------|-------------|
| `INVARIANTS.md` | New constraint that applies to all future work |
| `ARCHITECTURE.md` | New interface or system boundary |
| `CLAUDE.md` / `AGENTS.md` | New convention or build command |

## After Consolidation

1. Review the consolidation output
2. Apply the suggested doc updates
3. Commit the updated docs
