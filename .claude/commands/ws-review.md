# Workstream Review

Dual-agent adversarial code review against SPEC and INVARIANTS.

## Usage
```
/ws-review <workstream> [task] [--agent claude|codex|both]
```

## Arguments
$ARGUMENTS

---

Run the code review script:

```bash
./scripts/ws-review.sh $ARGUMENTS
```

## How It Works

The script runs both Claude and Codex against the same code review prompt in parallel:

1. Builds review prompt from workstream context (SPEC, PLAN, SHARED-CONTEXT, INVARIANTS) + task context if specified
2. Runs agents with the prompt
3. Saves timestamped outputs to `docs/workstreams/<name>/reviews/`

## Review Perspectives

- **Spec alignment** — does implementation match EARS acceptance criteria?
- **Invariant compliance** — does code honor INVARIANTS.md?
- **Design** — right abstractions, right boundaries, essential complexity only?
- **Error handling** — all paths covered with what/where/why/hint?
- **Testing** — tests cover EARS criteria and edge cases?
- **LLM-specific traps** — hallucinated APIs, silent failures, unnecessary complexity?

## After Review

1. Review findings from both agents
2. Fix critical/major issues
3. `/ws-consolidate <name>` to capture learnings
