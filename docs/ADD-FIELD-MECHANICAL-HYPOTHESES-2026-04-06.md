# Add-Field Mechanical Hypotheses (2026-04-06)

## Context

`add-field` is correct in both baseline and jig arms, but jig is currently less efficient (higher tool-call count, output tokens, duration, and cost).  
This suggests a workflow/mechanical issue, not a task-correctness issue.

## Mechanical Hypotheses

1. **Mixed edit mechanics increase loops**  
   The `add-field` recipe mixes `patch` and `inject` operations across four files, which may cause extra read/verify/re-open cycles.

2. **Anchor/insertion points are causing re-check churn**  
   List insertions in `admin.py` and `serializers.py` may be producing low-trust edits, so the agent repeatedly revalidates them.

3. **`--vars` interface friction increases reasoning overhead**  
   JSON escaping and nested quoting in `jig run ... --vars` may lead to over-planning and extra corrective passes.

4. **Skill fallback guidance triggers unnecessary verification**  
   Current `SKILL.md` guidance (especially fallback behavior) may be encouraging broad manual validation even when the first jig run is sufficient.

5. **No hard stop condition permits redundant jig runs**  
   Without an explicit "one run, verify assertions, stop" rule, the agent can re-run jig on already-correct edits.
