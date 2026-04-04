# Workstream Plan Review

Dual-agent adversarial review of planning documents for a workstream.

## Usage
```
/ws-plan-review <workstream-name> [--agent claude|codex|both]
```

## Arguments
$ARGUMENTS

---

Run the plan review script:

```bash
./scripts/ws-plan-review.sh $ARGUMENTS
```

## How It Works

The script runs both Claude and Codex against the same adversarial review prompt in parallel:

1. Builds review prompt from all workstream docs (SPEC, PLAN, SHARED-CONTEXT, NARRATIVE, execution plans, INVARIANTS, ARCHITECTURE)
2. Runs agents with the prompt
3. Saves timestamped outputs to `docs/workstreams/<name>/reviews/`

## Review Checks

- **Consistency** — contradictions between SPEC and PLAN
- **EARS audit** — every AC is in EARS format with ID and test mapping
- **Completeness** — all phases have validation criteria, all FR/NFR have ACs
- **Invariant alignment** — plan honors every invariant in INVARIANTS.md
- **Risk assessment** — underestimated complexity, missing error cases

## After Review

1. Review findings from both agents
2. Fix critical/major issues in SPEC.md and PLAN.md
3. `/ws-execute <name>` to start building
