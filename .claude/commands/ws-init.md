# Initialize Workstream

Create the document structure for a new workstream.

## Usage
```
/ws-init <workstream-name> [--discovery]
```

## Arguments
$ARGUMENTS

---

Run the init script:

```bash
./scripts/ws-init.sh $ARGUMENTS
```

After initialization, the workstream will have:
- `PLAN.md` — Phases, milestones, status
- `SPEC.md` — Requirements with EARS-format acceptance criteria
- `SHARED-CONTEXT.md` — Accumulated knowledge
- `NARRATIVE.md` — Human-readable explanation

With `--discovery`, only a `discovery/` folder is created for the research phase.

Next step: `/ws-plan <name>` for collaborative planning.
