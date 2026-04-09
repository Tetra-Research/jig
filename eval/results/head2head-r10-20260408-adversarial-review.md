# Head-to-Head Adversarial Review

Date: 2026-04-08

Primary run under review:
- `eval/results/head2head-results-h2h-r10-20260408.jsonl`
- `eval/results/head2head-pairs-h2h-r10-20260408.jsonl`

Supporting auto-analysis:
- `eval/results/head2head-r10-20260408-adversarial-review.auto.md`

Diagnostic probe:
- `eval/results/head2head-results-deterministic-execution-probe-20260408.jsonl`
- `eval/results/head2head-pairs-deterministic-execution-probe-20260408.jsonl`

## Executive Summary

The current `r10` results are directionally useful, but they are not yet clean enough to treat as a pure control-vs-jig capability comparison.

The biggest confound is prompt/profile semantics in the control arm:
- On `h2h-deterministic-service-test`, control went `0/10` with the original directed prompt by repeatedly returning checklist analysis instead of editing files.
- When the prompt was changed only to explicitly say "implement by editing files" and "do not return a checklist", control immediately moved to `1/1` pass on the same scenario.

That is strong evidence that at least part of the current delta is a harness artifact, not a genuine inability of the non-jig path.

The second major issue is that some scenarios reward or penalize the wrong things:
- `file_score` can grant substantial partial credit to untouched codebases.
- Several assertions are overly literal and punish variable names or event suffix choices rather than the intended contract.
- At least one scenario (`h2h-schema-migration-safety`) allows control to score `1.0` while still diverging meaningfully from the expected shape.

## Findings

### 1. Directed control prompts are inducing "read the skill and summarize it" behavior

Evidence:
- `h2h-deterministic-service-test`: control was analysis-only in 10/10 runs.
- `h2h-query-layer-discipline`: control was analysis-only in 10/10 runs.
- `h2h-view-contract-enforcer`: control was analysis-only in 7/10 runs.

Representative artifacts:
- `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T22-50-04-529Z__h2h-deterministic-service-test__claude-code__control__rep1__927opf/combined.log`
- `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-21-03-856Z__h2h-view-contract-enforcer__claude-code__control__rep2__e5e8jo/combined.log`

Probe result:
- With the same control skill, same agent, and same scenario, but a custom prompt that explicitly required editing files, control scored `1.0`.
- Probe artifact: `eval/results/head2head-artifacts/deterministic-execution-probe-20260408/2026-04-09T00-05-47-155Z__h2h-deterministic-service-test__claude-code__control__rep1__1y3xvr`

Interpretation:
- The current control `CLAUDE.md` plus checklist-style `SKILL.md` files are too easy for the model to interpret as review/spec work.
- The string `Use the <skill> skill.` is not sufficient to force execution.

### 2. `file_score` is inflated by baseline overlap

No-op `file_score` baselines against the untouched `codebase/`:

| Scenario | No-op file_score |
| --- | ---: |
| `h2h-deterministic-service-test` | `0.00` |
| `h2h-query-layer-discipline` | `0.31` |
| `h2h-schema-migration-safety` | `0.25` |
| `h2h-structured-logging-contract` | `0.39` |
| `h2h-view-contract-enforcer` | `0.48` |

Implication:
- `file_score` should not be read as "amount of useful work completed".
- In `h2h-view-contract-enforcer`, a totally untouched run still looks almost halfway similar to expected.
- In `h2h-query-layer-discipline`, the current report can visually imply progress even when control never edits.

Recommended interpretation:
- Treat `file_score` as a shape-similarity debug metric, not a performance metric.
- If kept in summary tables, also show the scenario's no-op baseline and maybe a baseline-adjusted file score.

### 3. Several assertions are overly literal or misaligned with the skill semantics

#### `h2h-structured-logging-contract`

Observed control behavior:
- Control edited successfully in 10/10 runs.
- Control consistently emitted `.complete`.
- Jig consistently emitted `.done`.

Representative control artifact:
- `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-13-14-217Z__h2h-structured-logging-contract__claude-code__control__rep1__798rx5/stdout.log`

Why this matters:
- The control skill says "Emit a completion event before return" and "Keep event names stable as `<event_namespace>.<phase>`".
- That wording does not uniquely require `done`.
- The scenario assertion requires the exact string `core_service.create_record.done`.

Conclusion:
- This scenario currently measures template-specific phase naming, not just contract adherence.

#### `h2h-view-contract-enforcer`

Observed control behavior:
- The successful control edits used `request_schema.is_valid(raise_exception=True)`.
- The assertion requires the exact string `request_contract.is_valid(raise_exception=True)`.

Representative control artifact:
- `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-21-03-856Z__h2h-view-contract-enforcer__claude-code__control__rep2__e5e8jo/stdout.log`

Why this matters:
- The control implementation is semantically fine on request validation.
- The scenario is rewarding a specific variable name chosen by the jig template.
- The same control implementation also added explicit permission handling, which is arguably more aligned with the control skill, but that behavior is not positively scored.

Conclusion:
- This scenario currently mixes semantic evaluation with template naming conventions.

#### `h2h-schema-migration-safety`

Observed control behavior:
- Control scores `1.0` on assertions in 10/10 runs.
- Mean control `file_score` is only `0.82`.

Representative control artifact:
- `eval/results/head2head-artifacts/h2h-r10-20260408/2026-04-08T23-02-53-445Z__h2h-schema-migration-safety__claude-code__control__rep1__w65vn0/stdout.log`

Why this matters:
- The control output uses a different backfill implementation shape.
- It also writes a custom reverse function with `pass` instead of the expected `migrations.RunPython.noop`.
- Those differences do not affect the current assertion score.

Conclusion:
- The scenario is currently too permissive to expose real consistency/template advantages in the control-vs-jig comparison.

#### `h2h-deterministic-service-test`

Probe behavior:
- After adding an execution-forcing sentence to the prompt, control scored `1.0`.
- But its `file_score` was only `0.14`, while jig scored `1.0`.

Implication:
- The current assertions for this scenario are extremely coarse.
- They are enough to detect "did a test file exist with expected anchor strings", but not enough to distinguish loosely compliant manual output from the intended templated structure.

### 4. Pair ordering still risks bias

Current runner behavior:
- For each scenario and rep, the runner executes `control` first and `jig` second.

Why this matters:
- Even though the profiles differ, the prompt, scenario, and agent are highly related across the pair.
- Any hidden cache, warm-start, rate-shaping, or tool startup effect will be systematically correlated with arm.
- The token telemetry already shows large cache-read volumes, so cache-sensitive reporting should be treated carefully.

Recommendation:
- Randomize arm order per rep, or explicitly alternate order.
- Record `arm_order` in the pair result.

### 5. Both arms install the full skill suite

Current behavior:
- Every run exposes all five skills inside the selected profile.

Why this matters:
- This increases context load unnecessarily.
- It also creates cross-skill leakage potential for a scenario that is supposed to compare one specific pair.

Recommendation:
- Install only the active skill for the active scenario, plus any explicitly required shared helper.

## What Changed In The Harness

To improve future forensic analysis, the artifact capture now preserves actual post-run workspace outputs in addition to prompt/stdout/stderr:
- `git-status.txt`
- `git-diff-stat.txt`
- `git-diff.patch`
- `changed-files.txt`
- `workspace/` snapshots for changed files

Relevant code:
- `eval/lib/workspace-artifacts.ts`
- `eval/head2head/artifacts.ts`
- `eval/harness/artifacts.ts`

This is important because the previous artifact format forced us to infer output quality from tool chatter, especially on jig runs that changed files through Bash instead of `Edit`/`Write`.

## Recommended Next Experiments

1. Tighten the control execution contract.
Use either a stronger directed prompt or a stronger control `CLAUDE.md` line that explicitly says control skills are implementation instructions, not review checklists.

2. Run an A/B prompt-shape validation pass.
For `h2h-deterministic-service-test` and `h2h-query-layer-discipline`, rerun with only one change: add an explicit "edit files" instruction. That will quantify how much of the current gap is pure harness wording.

3. Baseline-adjust `file_score`.
Either subtract the no-op score or replace it with improvement-over-codebase rather than similarity-to-expected alone.

4. Rewrite assertions to be semantic where possible.
Examples:
- View contract: assert validation call shape without requiring `request_contract` as the variable name.
- Structured logging: either require `done` in both skill and expected output, or accept an allowed set such as `done|complete`.
- Migration safety: assert reversibility/idempotence details directly.

5. Separate "can execute" from "matches template exactly".
The deterministic probe shows that control can succeed under explicit execution, but its shape still differs sharply from jig. That should be measured as a separate dimension, not collapsed into one score.

6. Randomize pair order and isolate active skill installation.
Those are low-cost harness changes that remove avoidable bias.
