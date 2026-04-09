# Head-to-Head R11 Adversarial Review

Run under review:

- Results: `eval/results/head2head-results-r11-20260409.jsonl`
- Pairs: `eval/results/head2head-pairs-r11-20260409.jsonl`
- Artifacts: `eval/results/head2head-artifacts/h2h-r11-20260409/`

Summary:

- All 5 pairs scored `1.0`.
- Aggregate result favors jig on cost, total tokens, and duration.
- That topline is directionally useful, but `r11` still contains harness and recipe confounds that overstate correctness and understate avoidable jig overhead.

## Failure Taxonomy

This run surfaced three different classes of problems, and they should not all be treated the same.

### A. Library problems

Definition:

- Core jig behavior is wrong or too weak for the recipe contract.
- The recipe asks for a reasonable structural operation, but the engine cannot express it safely or silently produces the wrong thing.

Current assessment for `r11`:

- I do **not** have strong evidence of a core library bug in the two main scenario failures.
- I do see a library **improvement opportunity**: add stronger guardrails so recipes that are structurally underspecified fail loudly instead of succeeding with semantically wrong placement.

Why this distinction matters:

- A recipe can be wrong even if the engine is doing exactly what the recipe asked.
- We should avoid blaming the library for recipe misuse.

### B. LLM failures

Definition:

- The model interprets the skill poorly, runs avoidable extra commands, or fails to sanity-check the result.

Current assessment for `r11`:

- The `${CLAUDE_SKILL_DIR}` first-command failure is an LLM/tool-usage failure, amplified by skill ergonomics.
- The model also did not catch the query-layer and structured-logging semantic problems after the recipe ran.

### C. Harmless failures

Definition:

- The output differs from expected formatting or exact literal shape, but the behavioral contract is still likely satisfied.

Current assessment for `r11`:

- Some exact-shape drift in migration and view/test files falls into this bucket.
- These should not be scored the same as semantically broken outputs.
- Today the harness does not distinguish them cleanly.

## Blame Matrix

### Query-layer misplaced manager attribute

Observed behavior:

- `objects = EntityManager()` landed inside `EntityQuerySet` instead of `Entity`.

Likely classification:

- Primary: recipe/skill failure
- Secondary: harness failure
- Not primarily: core library bug

Reasoning:

- The recipe anchors the manager attribute patch to the first class body:
  - `eval/head2head/profiles/jig/skills/query-layer-discipline/recipe.yaml:29-35`
- The library appears to have done exactly that.
- The recipe should have targeted the `Entity` class specifically.
- The harness then failed to catch the bad placement.

What we may want from the library anyway:

- Optional postconditions or scoped assertions that can fail the recipe if a rendered line lands in the wrong structural container.
- Better diagnostics around patch target selection when the anchor pattern is broad and multiple plausible targets exist.

### Structured-logging unreachable `.done` log

Observed behavior:

- The `.done` log landed after `return`.

Likely classification:

- Primary: recipe/skill failure
- Secondary: harness failure
- Not primarily: core library bug

Reasoning:

- `before_close` is documented as "line before closing delimiter or dedent":
  - `docs/ARCHITECTURE.md:254`
- The recipe used `function_body + before_close`:
  - `eval/head2head/profiles/jig/skills/structured-logging-contract/recipe.yaml:32-41`
- In a function with an early `return`, that semantics is insufficient for "log before returning".
- This is a bad recipe choice, not evidence that the engine mis-executed the declared anchor semantics.

What we may want from the library anyway:

- A richer position primitive like "before first return" / "before all returns" for common control-flow-sensitive edits.
- Or recipe-time validation that warns when `before_close` is being used for a pattern that likely assumes pre-return execution.

### First `jig run` fails because `${CLAUDE_SKILL_DIR}` does not resolve in-line

Observed behavior:

- Query-layer jig and structured-logging jig both burned a failed first command before retrying successfully.

Likely classification:

- Primary: LLM/tool-usage failure
- Secondary: skill ergonomics failure
- Not: library failure

Reasoning:

- The engine was never reached successfully on the first attempt because the shell expression resolved incorrectly.
- The skill docs make this easy to get wrong by showing `${CLAUDE_SKILL_DIR}` literally.
- The fastest fix is to make the usage snippet more idiot-proof for the model.

What we may want from the library anyway:

- A simpler "run local skill recipe" convention that avoids shell env expansion entirely.

### Migration contract drift

Observed behavior:

- Control passed while differing from expected exact shape, especially around migration field naming literals.

Likely classification:

- Primary: harmless-or-uncertain variance plus harness under-specification
- Not primarily: library failure
- Not primarily: meaningful LLM failure unless exact contract fidelity is the metric

Reasoning:

- This is exactly the kind of difference the harness should classify separately from "semantically broken".
- Right now it is lumped into the same scoreboard.

### `file_score` blind to structure

Observed behavior:

- Broken structural placements can still score `1.0`.

Likely classification:

- Primary: harness/scoring failure

Reasoning:

- The current scorer treats files as sets of lines, not ordered structures.
- This is the direct reason query-layer jig and structured-logging jig looked cleaner than they were.

## Findings

### 1. `file_score` is structurally blind and can rate broken code as an exact match

The current diff metric in `eval/lib/diff.ts` converts files into sets of trimmed non-empty lines and scores Jaccard overlap. That means line placement does not matter. Moving a line into the wrong class or below a `return` can still score `1.0` if the same lines are present somewhere in the file.

Evidence:

- Scorer logic: `eval/lib/diff.ts:6-28`
- Aggregation: `eval/lib/diff.ts:30-50`

Concrete false positives from `r11`:

- Query-layer jig scored `file_score=1.0` even though `objects = EntityManager()` was inserted into `EntityQuerySet` instead of `Entity`. See:
  - expected file: `eval/scenarios/h2h-query-layer-discipline/expected/models.py`
  - actual artifact: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-38-52-446Z__h2h-query-layer-discipline__claude-code__jig__rep1__jx1fwg/workspace/models.py`
- Structured-logging jig scored `file_score=1.0` even though the `.done` log was inserted after the `return`, making it unreachable. See:
  - expected file: `eval/scenarios/h2h-structured-logging-contract/expected/services/core_service.py`
  - actual diff: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-40-36-376Z__h2h-structured-logging-contract__claude-code__jig__rep1__4iola7/git-diff.patch`

Implication:

- `file_score` is currently a weak consistency hint, not a reliable exact-shape metric.
- Any claim that jig matched the expected output "exactly" is not supported by the current scorer.

### 2. Query-layer jig is a real recipe failure that the harness scored as a full pass

This is the strongest false positive in the run.

The query-layer scenario only asserts that:

- `class EntityQuerySet` exists
- `objects = EntityManager()` exists somewhere in `models.py`
- the selector function exists
- the view calls the selector

See `eval/scenarios/h2h-query-layer-discipline/scenario.yaml:28-40`.

The jig recipe for the manager attribute uses:

- `anchor.pattern: "^class "`
- `scope: class_body`
- `position: before`

See `eval/head2head/profiles/jig/skills/query-layer-discipline/recipe.yaml:29-35`.

Given the current `models.py`, that patch lands inside the first class body, which is `EntityQuerySet`, not `Entity`. The artifact shows exactly that:

- `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-38-52-446Z__h2h-query-layer-discipline__claude-code__jig__rep1__jx1fwg/workspace/models.py`

That output is likely broken at import time because `EntityManager` is referenced before it is defined, and even if the import survived, the manager is attached to the wrong class.

Control did the better thing here:

- control diff: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-38-31-446Z__h2h-query-layer-discipline__claude-code__control__rep1__ko761i/git-diff.patch`
- jig diff: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-38-52-446Z__h2h-query-layer-discipline__claude-code__jig__rep1__jx1fwg/git-diff.patch`

Implication:

- The `h2h-query-layer-discipline` pair should not currently be treated as a clean jig win, even though the scoreboard says both arms passed.

### 3. Structured-logging jig lost on efficiency partly because the model burned a failed first `jig run`

The structured-logging jig skill tells the model to use:

- `jig run ${CLAUDE_SKILL_DIR}/recipe.yaml ...`

See `eval/head2head/profiles/jig/skills/structured-logging-contract/SKILL.md:20-27`.

In `r11`, the model first attempted:

- `CLAUDE_SKILL_DIR=.claude/skills/structured-logging-contract jig run ${CLAUDE_SKILL_DIR}/recipe.yaml ...`

That failed with `recipe file not found` at `/recipe.yaml`, then it retried with an exported variable and succeeded. See:

- artifact stdout: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-40-36-376Z__h2h-structured-logging-contract__claude-code__jig__rep1__4iola7/stdout.log`

This is not a meaningful capability difference between control and jig. It is avoidable overhead introduced by the usage example and shell semantics.

The same first-command failure also happened in query-layer jig:

- `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-38-52-446Z__h2h-query-layer-discipline__claude-code__jig__rep1__jx1fwg/stdout.log`

Implication:

- Some of jig's token and time cost in `r11` is harness/procedure overhead, not templating overhead.
- The logging reversal is therefore partly contaminated by a retry tax.

### 4. Structured-logging jig also contains a recipe-anchor bug, not just a retry tax

The structured logging recipe inserts the completion log with:

- `scope: function_body`
- `position: before_close`

See `eval/head2head/profiles/jig/skills/structured-logging-contract/recipe.yaml:25-31`.

In the target function, that placed the `.done` log after the `return`, making it unreachable:

- `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-40-36-376Z__h2h-structured-logging-contract__claude-code__jig__rep1__4iola7/git-diff.patch`

The scenario assertions do not detect this because they only look for string presence:

- `eval/scenarios/h2h-structured-logging-contract/scenario.yaml:25-37`

Control produced a semantically better implementation:

- it introduced a `result` variable
- emitted `.done` before returning

See:

- `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-40-15-374Z__h2h-structured-logging-contract__claude-code__control__rep1__ebj7ck/git-diff.patch`

Implication:

- The logging scenario currently overstates jig correctness and understates control quality.

### 5. Schema-migration control passed despite diverging from the intended exact shape

The control migration output scored `1.0` on assertions but only `0.8535` on `file_score`.

That divergence is not just formatting. The control artifact uses:

- `model_name="Entity"`

while the expected file and jig output use:

- `model_name="entity"`

Compare:

- expected migration: `eval/scenarios/h2h-schema-migration-safety/expected/migrations/0008_add_entity_classification.py`
- control artifact: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-39-43-095Z__h2h-schema-migration-safety__claude-code__control__rep1__nsq3n4/git-diff.patch`
- jig artifact: `eval/results/head2head-artifacts/h2h-r11-20260409/2026-04-09T15-40-00-697Z__h2h-schema-migration-safety__claude-code__jig__rep1__5tfwcv/git-diff.patch`

If we care about exact migration contract shape, this scenario is currently under-asserted.

Implication:

- This pair is still useful as an efficiency comparison.
- It is not yet a reliable "same exact contract" comparison.

### 6. Negative assertions are far too weak for Python correctness

Current negative assertions mostly check only for `SyntaxError`.

See:

- `eval/scenarios/h2h-query-layer-discipline/scenario.yaml:42-45`
- `eval/scenarios/h2h-structured-logging-contract/scenario.yaml:39-42`
- `eval/scenarios/h2h-schema-migration-safety/scenario.yaml:47-50`

And the engine only applies regex checks over file contents:

- `eval/harness/score.ts:30-73`

That means the harness does not catch:

- unreachable code
- import-time `NameError` patterns caused by bad class wiring
- wrong Django migration identifiers
- wrong control flow placement

Implication:

- Several `1.0` scores in `r11` are "contains the right strings" passes, not strong behavior passes.

### 7. We need a distinction between semantic failure and harmless exact-shape drift

Right now the review is forced to describe very different outcomes with the same vocabulary:

- query-layer jig: semantically wrong
- structured-logging jig: semantically wrong
- migration control: possibly acceptable but not exact
- view/test newline or placement differences: harmless

Those should not all collapse into a single pass/fail axis.

Implication:

- We need at least three buckets in analysis:
  - exact contract match
  - acceptable semantic variant
  - semantic failure

That split will make the experiment much easier to reason about.

## Trustworthiness by Scenario

### Clean enough to trust directionally

- `h2h-deterministic-service-test`
  - Both outputs are essentially identical.
  - Jig is cheaper and simpler here.

- `h2h-view-contract-enforcer`
  - Both outputs look substantively correct.
  - Jig appears cleaner on efficiency.

### Useful but biased

- `h2h-schema-migration-safety`
  - Good efficiency signal.
  - Under-asserted on exact migration semantics.

### Not clean enough to trust as-is

- `h2h-query-layer-discipline`
  - Jig recipe bug passed as success.

- `h2h-structured-logging-contract`
  - Jig retry tax plus unreachable `.done` log.

## Concrete Next Fixes

1. Replace line-set `file_score` with a stricter ordered diff metric.
   - At minimum: sequence-aware line similarity.
   - Better: AST-aware checks for Python scenarios.

2. Strengthen scenario assertions from substring checks to contract checks.
   - Query-layer: assert `objects = EntityManager()` appears inside `class Entity(models.Model)`.
   - Structured logging: assert the `.done` log occurs before the `return`.
   - Migration: assert `model_name="entity"` and `RunPython.noop` explicitly.

3. Add lightweight execution validation where possible.
   - Python compile/import smoke test per scenario.
   - Scenario-specific test command when the codebase supports it.
   - Distinguish "compile/import failed" from "exact-shape drift".

4. Fix the jig skill usage examples that encourage a failed first command.
   - Prefer explicit recipe paths over `${CLAUDE_SKILL_DIR}` in docs shown to the model.

5. Fix the recipes themselves.
   - Query-layer: patch manager attr into the target model class, not the first class body.
   - Structured logging: patch the completion log before the return point, not `before_close`.

6. Add an LLM-judge backup layer for semantic review, but do not make it the primary scorer.
   - Good use: inspect artifact diff + expected file + scenario contract and answer:
     - exact match?
     - acceptable semantic variant?
     - semantically broken?
   - Bad use: replace deterministic assertions entirely.
   - Reason: an LLM judge is useful for catching unreachable logs, misplaced manager attributes, and other semantic issues that substring checks miss, but it is still probabilistic and can drift.

## What To Improve Where

### Improve the library

Only after we decide the recipe category really needs stronger engine support.

Current candidates:

- richer structural positions for control-flow-sensitive edits
- optional recipe postconditions / scoped validations
- better ambiguity diagnostics when a broad anchor matches multiple structural targets

I would treat these as product improvements, not proven defects from `r11`.

### Improve the skills / recipes

This is the clearest action from `r11`.

- Query-layer recipe is underspecified and targets the wrong class.
- Structured-logging recipe uses the wrong insertion strategy for the completion log.
- Some usage snippets are too easy for the model to mis-execute.

### Improve the model-facing skill docs

- Reduce shell/env ambiguity.
- Prefer explicit commands that the model can copy literally.
- Tell the model to sanity-check the edited file after jig runs if the pattern is prone to structural placement errors.

### Improve the harness

- Stronger assertions
- order-sensitive diffing
- lightweight execution checks
- semantic-vs-harmless distinction
- optional LLM judge backup

## Bottom Line

`r11` is a useful run, but it is not clean enough to support a strong claim that "jig and control both produced equally correct outputs across all five scenarios."

The defensible claim is narrower:

- On the current harness, jig still shows a strong aggregate efficiency advantage.
- Two scenarios (`query-layer`, `structured-logging`) contain false-positive passes that should be fixed before using them as serious evidence.

More specifically:

- The main `r11` problems do **not** currently look like clear core-library failures.
- They look like a mix of recipe/skill design mistakes, LLM execution mistakes, and harness scoring weaknesses.
- If we want to improve the library from this run, the right framing is "add guardrails and better primitives for these recipe classes", not "the engine is fundamentally misbehaving."

## Follow-On Plan

The goal of this plan is to improve confidence in head-to-head results without overreacting to a single noisy run.

### Phase 1: Fix the harness so broken outputs stop scoring as clean passes

Priority: highest

Reason:

- Right now the harness is the weakest link in the chain.
- If we do not fix scoring first, reruns will still mix real wins with false positives.

Work:

1. Replace `file_score` with an order-sensitive metric.
   - Minimum viable: ordered line similarity rather than line-set Jaccard.
   - Better: scenario-specific structure-aware comparison for Python files.

2. Strengthen scenario assertions for the known weak cases.
   - Query-layer:
     - assert `objects = EntityManager()` appears inside `class Entity(models.Model)`
     - assert it does not appear inside `EntityQuerySet`
   - Structured logging:
     - assert `.done` appears before `return`
     - assert the function returns the expected payload after logging
   - Migration:
     - assert `model_name="entity"`
     - assert `migrations.RunPython.noop`
     - assert the field is nullable in migration 1 and non-nullable in migration 2

3. Add lightweight execution checks.
   - Python parse/compile smoke check for edited files
   - import smoke check where feasible
   - scenario-specific test invocation where cheap and deterministic

4. Separate output grading into three labels:
   - exact contract match
   - acceptable semantic variant
   - semantic failure

Exit criteria:

- Query-layer jig from `r11` would no longer score as a clean pass.
- Structured-logging jig from `r11` would no longer score as a clean pass.
- Migration control drift would be distinguishable from semantic failure.

### Phase 2: Add an LLM judge as a backup semantic reviewer

Priority: high

Reason:

- Deterministic checks should remain primary.
- An LLM judge is useful for the semantic cases our assertions will still miss.

Work:

1. Build a judge pass that receives:
   - scenario contract
   - expected files
   - actual changed files
   - optionally the artifact diff

2. Ask the judge to classify each run into:
   - exact match
   - acceptable semantic variant
   - semantic failure

3. Ask the judge to explain:
   - what is wrong
   - whether the problem is harmless
   - whether the code would likely fail at runtime

4. Record judge output separately from deterministic score.
   - Do not collapse them into one number initially.
   - Use the judge as an audit layer first.

Guardrails:

- Use the LLM judge as backup, not replacement.
- Keep deterministic assertions as the authoritative hard checks.
- Compare judge output against a few manually reviewed runs before trusting it broadly.

Exit criteria:

- We can quickly spot semantic failures that string assertions miss.
- The review process no longer depends entirely on manual artifact inspection.

### Phase 3: Fix the known recipe and skill issues

Priority: high

Reason:

- Two scenarios already show recipe-level problems.
- These should be repaired before using them for stronger claims.

Work:

1. Query-layer recipe:
   - target the `Entity` class explicitly for manager attachment
   - avoid generic "first class body" anchoring for model manager insertion
   - add a post-run sanity check in the skill instructions if needed

2. Structured-logging recipe:
   - stop using `before_close` for completion logging in functions that may `return`
   - choose a pre-return insertion strategy
   - if necessary, template a `result = ...; log; return result` rewrite pattern

3. Jig skill usage examples:
   - remove ambiguous `${CLAUDE_SKILL_DIR}` usage where the model may inline it incorrectly
   - prefer explicit recipe paths the model can copy literally

4. Model-facing instructions:
   - after `jig run`, read the changed file and sanity-check structural placement for high-risk patterns

Exit criteria:

- Query-layer jig produces the manager on the target model class.
- Structured logging jig emits `.done` before returning.
- First-command retry failures disappear on these skills.

### Phase 4: Decide whether the library needs new primitives or guardrails

Priority: medium

Reason:

- We should not expand the engine until we know the remaining pain is not solved by better recipes and better scoring.

Work:

1. Review recurring failures after Phases 1-3.
2. If the same recipe classes still require awkward workarounds, consider library improvements such as:
   - control-flow-aware positions like `before_first_return` or `before_all_returns`
   - postcondition checks on recipe outputs
   - ambiguity detection when an anchor pattern is too broad

Decision rule:

- If better recipe authoring fixes the issue, do not add engine complexity.
- If multiple recipes need the same missing structural primitive, add it to the library.

Exit criteria:

- We have a short, evidence-based list of library improvements rather than speculative feature creep.

### Phase 5: Rerun the scenarios with confidence labels

Priority: after Phases 1-3

Reason:

- Only after scoring and recipe quality improve does it make sense to treat reruns as meaningful evidence.

Work:

1. Rerun at least:
   - `h2h-query-layer-discipline`
   - `h2h-structured-logging-contract`
   - `h2h-schema-migration-safety`

2. Report per scenario:
   - deterministic score
   - exactness score
   - semantic classification
   - tokens, cost, duration
   - retry count / command count

3. Keep the old `r11` review as the pre-fix baseline.

Exit criteria:

- We can say which gains are real efficiency wins versus artifacts of weak scoring.

## Recommended Execution Order

If we want the fastest path to better evidence, the order should be:

1. harness scoring fixes
2. stronger scenario assertions
3. recipe fixes
4. LLM judge backup
5. rerun targeted scenarios
6. only then consider library primitives

This ordering matters because it prevents us from "fixing the product" in response to what may actually be a measurement problem.

## Follow-Up: Harness And Recipe Corrections (2026-04-09)

After the initial `r11`/`r12` review, I made the harness changes first, reran the weak scenarios, and then fixed the affected jig skills. This follow-up section captures what changed and what we learned.

### What changed in the harness

1. Structured-logging assertions were relaxed from exact literal `return {` matching to semantic ordered matching ending in `return`.
2. Query-layer assertions were tightened so `objects = EntityManager()` must appear inside `class Entity`.
3. Ordered assertion support and order-sensitive `file_score` were added, so misplaced or unreachable code no longer looks like a clean pass.

These changes corrected two measurement problems:

- structured-logging control had been over-penalized for returning via `result`
- query-layer jig had been over-credited for placing the manager on the wrong class

### New library constraint discovered

The most important product finding from the follow-up work is this:

- `jig 0.1.0` does **not** support templated regex fields at recipe-parse time.

In practice, fields such as:

- `anchor.pattern`
- `before`
- `after`

must be valid regex strings before variable substitution. Recipes like:

- `^class {{ model_name }}\(`
- `^def {{ function_name }}\(`

fail during recipe validation/parsing, because the literal `{{ ... }}` is compiled as regex before Jinja-style substitution.

Evidence:

- query-layer jig failure artifact:
  - `eval/results/head2head-artifacts/h2h-r13-targeted-20260409/2026-04-09T16-25-46-363Z__h2h-query-layer-discipline__claude-code__jig__rep1__9enxce/stdout.log`
- structured-logging jig failure artifact:
  - `eval/results/head2head-artifacts/h2h-r13-targeted-20260409/2026-04-09T16-28-00-186Z__h2h-structured-logging-contract__claude-code__jig__rep1__20kt5z/stdout.log`

This matters because it changes the blame:

- the failed templated-anchor versions are not valid jig recipes under `0.1.0`
- any "success" under those versions can come from model fallback/manual edits, not from the intended jig execution path

### Query-layer follow-up result

I replaced the invalid templated regex anchors with static anchors that match the eval contract:

- `^class Entity\(`
- `^def entity_list\(`

This is acceptable for the eval profile because these head-to-head skills are benchmark fixtures, not general-purpose reusable skills.

Result:

- targeted rerun `r14`:
  - `eval/results/head2head-pairs-r14-targeted-20260409.jsonl`
- control score: `1.00`
- jig score: `1.00`

Efficiency on that rerun:

- control: `311,628` tokens, `$0.8451`
- jig: `91,689` tokens, `$0.3510`

Interpretation:

- The original query-layer jig failure is fixed.
- The corrected recipe now produces the intended structural output.
- On this scenario, jig is now a legitimate efficiency win on equal correctness.

### Structured-logging follow-up result

This scenario required three separate corrections:

1. Harness fix:
   - stop requiring the literal token `return {` in the assertion
2. Recipe fix:
   - stop using the invalid templated `anchor.pattern`
3. Library-aware primitive fix:
   - stop using `patch` with a narrowed scope that lands inside the returned dict

What failed along the way:

1. Templated `anchor.pattern` version:
   - invalid under `jig 0.1.0`
   - caused timeout / exploratory tool churn rather than a clean run
2. `patch` with `find: "return"`:
   - narrowed into the brace scope of `return {`
   - inserted the `.done` log inside the returned dict
3. `patch` on `scope: line` for the `return {` line:
   - resolved as an `after` insertion on that line
   - still placed `.done` inside the returned dict

The working fix was:

- `inject before "^\\s+return \\{$"` for the `.done` log block

Final result:

- targeted rerun `r16`:
  - `eval/results/head2head-pairs-r16-structured-20260409.jsonl`
- control score: `1.00`
- jig score: `1.00`

Efficiency on that rerun:

- control: `67,856` tokens, `$0.2822`
- jig: `135,017` tokens, `$0.4266`

Interpretation:

- Structured-logging correctness is now fixed.
- The remaining issue is not correctness; it is efficiency.
- On the current implementation and prompt path, jig is **not** an efficiency win for this scenario.

### Revised blame after follow-up

#### Query-layer

- Initial false positive: harness failure
- Initial structural miss: recipe failure
- Follow-up constraint discovered: library limitation on templated regex fields
- Final state: fixed for the eval profile

#### Structured logging

- Initial false positive: harness failure
- Initial unreachable `.done`: recipe failure
- Follow-up parse failures: library limitation on templated regex fields
- Intermediate bad placements: recipe/library-primitive mismatch
- Final state: correctness fixed, efficiency still unfavorable

### What this means for next experiments

1. We should treat "templated regex anchors are unsupported in `jig 0.1.0`" as a durable product constraint.
2. Eval skills can still use static anchors when the benchmark contract is intentionally narrow.
3. We should not assume a jig-powered skill is an efficiency win just because it is now correct.
4. Structured logging is currently a good counterexample scenario:
   - same correctness
   - worse jig efficiency

### Updated practical conclusion

The original concern was correct:

- part of the apparent result spread was harness bias
- part of it was real recipe/library-envelope mismatch

After corrections:

- query-layer is now a clean head-to-head success case for jig
- structured logging is now a clean head-to-head case where jig correctness matches control but efficiency does not

That is exactly the kind of side-by-side evidence the head-to-head harness is supposed to produce.
