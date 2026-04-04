# Workstream Task Init

Parse PLAN.md phases into per-phase task directories for sequential execution.

## Usage
```
/ws-task-init <workstream> [--force]
```

## Arguments
$ARGUMENTS

---

Run the task init script:

```bash
./scripts/ws-task-init.sh $ARGUMENTS
```

## How It Works

The script parses `PLAN.md` phase headers and creates:

1. `tasks/phase-N/CONTEXT.md` — Phase plan + relevant SPEC acceptance criteria
2. `tasks/phase-N/VALIDATION.md` — AC traceability table with PENDING status

Each task directory is self-contained context for `ws-execute`.

## After Init

1. Review generated CONTEXT.md and VALIDATION.md files
2. Execute phase-by-phase: `/ws-execute <ws> phase-1 --max-iter 5`
3. After each phase passes validation, move to the next
