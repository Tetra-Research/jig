# Structured Logging Head-to-Head Review (r20-r24)

## Scope

This review covers the valid structured-logging head-to-head runs after the sub-scope recipe fix:

- `r20-structured`
- `r21`
- `r22-structured`
- `r23-structured`
- `r24-structured`

It focuses on four questions:

1. Are control and jig both correct?
2. Is the output stable?
3. Is the measured efficiency difference stable?
4. What is product behavior versus agent behavior versus harness behavior?

## High-Level Read

The correctness problem is fixed.

Across all five valid runs:

- control scored `1.00` on the scenario every time
- jig scored `1.00` on the scenario every time
- control produced the same final file every time
- jig produced the same final file every time

That means the code-generation outcome is now stable. The remaining noise is almost entirely in agent execution path and token accounting.

## Run Matrix

| Run | Control tokens | Jig tokens | Delta (jig-control) | Control cost | Jig cost | Delta | Control ms | Jig ms | Delta |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `r20` | 67,800 | 115,402 | +47,602 | 0.5536 | 0.4296 | -0.1240 | 12,405 | 22,071 | +9,666 |
| `r21` | 67,849 | 92,013 | +24,164 | 0.2821 | 0.3647 | +0.0826 | 12,620 | 18,007 | +5,387 |
| `r22` | 113,405 | 88,425 | -24,980 | 0.4081 | 0.2984 | -0.1098 | 20,508 | 14,724 | -5,784 |
| `r23` | 135,600 | 88,478 | -47,122 | 0.4366 | 0.2981 | -0.1385 | 22,161 | 14,331 | -7,830 |
| `r24` | 92,154 | 114,512 | +22,358 | 0.3682 | 0.3878 | +0.0195 | 19,589 | 17,167 | -2,422 |

Five-run averages:

- control tokens: `95,361.6`
- jig tokens: `99,766.0`
- average token delta: `+4,404.4` in jig's favor of being worse
- control cost: `$0.4097`
- jig cost: `$0.3557`
- average cost delta: `-$0.0540` in jig's favor of being cheaper
- control duration: `17.46s`
- jig duration: `17.26s`
- average duration delta: `-0.20s` in jig's favor of being slightly faster

Interpretation:

- The single-run claim that jig is categorically worse on this scenario does not hold.
- The single-run claim that jig is categorically better also does not hold.
- On this scenario, correctness is stable, while efficiency is borderline and highly path-dependent.

## Skill Side-by-Side

### Control Skill

Source: `eval/head2head/profiles/control/skills/structured-logging-contract/SKILL.md`

Core contract:

- edit the target file directly
- add `import logging`
- add `logger = logging.getLogger(__name__)`
- add `.start` log before the work
- add `.done` log before the return
- keep exact `extra={"method", "step", "entity_id"}` keys
- preserve surrounding behavior and return shape

This is a natural-language specification of the target code shape.

### Jig Skill

Source: `eval/head2head/profiles/jig/skills/structured-logging-contract/SKILL.md`

Core contract:

- run one recipe with five variables
- recipe injects logger setup
- recipe patches function entry for `.start`
- recipe injects `.done` immediately before `return {`

This is a command-and-template path to the same contract.

### Main Difference

The control skill specifies the output contract.
The jig skill specifies the output contract plus the mechanism for producing it.

That distinction matters here because the final code differs structurally between the two arms, even though both satisfy the scenario.

## Before / After Frame

### Before

Source: `eval/scenarios/h2h-structured-logging-contract/codebase/services/core_service.py`

```python
from datetime import datetime


def create_record(record_id):
    timestamp = datetime.utcnow().isoformat()
    return {
        "id": record_id,
        "created_at": timestamp,
    }
```

### Scenario Expected Output

Source: `eval/scenarios/h2h-structured-logging-contract/expected/services/core_service.py`

```python
from datetime import datetime
import logging

logger = logging.getLogger(__name__)


def create_record(record_id):
    logger.info(
        "core_service.create_record.start",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    timestamp = datetime.utcnow().isoformat()
    logger.info(
        "core_service.create_record.done",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    return {
        "id": record_id,
        "created_at": timestamp,
    }
```

### Control Output Shape

Representative artifact: `eval/results/head2head-artifacts/h2h-r24-structured-20260409/2026-04-09T17-47-55-620Z__h2h-structured-logging-contract__claude-code__control__rep1__s30t4s/workspace/services/core_service.py`

```python
import logging
from datetime import datetime

logger = logging.getLogger(__name__)


def create_record(record_id):
    logger.info(
        "core_service.create_record.start",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    timestamp = datetime.utcnow().isoformat()
    result = {
        "id": record_id,
        "created_at": timestamp,
    }
    logger.info(
        "core_service.create_record.done",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    return result
```

### Jig Output Shape

Representative artifact: `eval/results/head2head-artifacts/h2h-r24-structured-20260409/2026-04-09T17-48-12-985Z__h2h-structured-logging-contract__claude-code__jig__rep1__lfbeb1/workspace/services/core_service.py`

```python
from datetime import datetime
import logging

logger = logging.getLogger(__name__)


def create_record(record_id):
    logger.info(
        "core_service.create_record.start",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    timestamp = datetime.utcnow().isoformat()
    logger.info(
        "core_service.create_record.done",
        extra={
            "method": "create_record",
            "step": "validate_input",
            "entity_id": record_id,
        },
    )
    return {
        "id": record_id,
        "created_at": timestamp,
    }
```

## What The Harness Is Actually Seeing

The scenario expected file matches the jig shape exactly.

That is why the jig arm gets:

- `file_score = 1.0`

The control output is semantically valid but structurally different because it rewrites the return into:

- `result = {...}`
- log `.done`
- `return result`

That is why the control arm gets:

- `file_score = 0.7692307692`

Important point:

This is not a correctness failure.
This is an exact-shape mismatch.

So the structured-logging scenario now has two distinct signals:

- `score`: semantic contract success
- `file_score`: match to the expected template shape

That split is useful and should remain.

## Adversarial Findings

### 1. The final code is stable; the telemetry is not.

The SHA-256 hashes of the generated output files are constant across all five valid runs for each arm.

- control output hash: `0a1dbd9f...`
- jig output hash: `fa434384...`

So neither arm is producing random code drift.

The measured variance comes from the path taken by the agent before it lands on the same code.

### 2. Structured logging is no longer evidence of a jig correctness problem.

The earlier structured-logging jig failure was a recipe bug caused by narrowing into the `return { ... }` sub-scope.
That was fixed by switching the `.done` insertion to a direct `before: "^\\s+return \\{$"` inject.

After that fix, jig passes every observed run.

This should now be treated as closed on correctness.

### 3. The biggest remaining confound is agent exploration overhead.

Observed control paths:

- low-overhead runs use `Read, Read, Write`
- higher-overhead runs use `Bash, Bash, Read, Read, Edit`

Observed jig paths:

- low-overhead runs use `Bash, Read, Bash`
- higher-overhead runs use extra skill discovery and recipe inspection before the final `jig run`

That means the measured efficiency is sensitive to whether the model:

- trusts the skill immediately
- explores the skill directory first
- reads the recipe and templates first
- rewrites directly versus editing surgically

This is agent behavior, not product behavior.

### 4. The current jig skill still leaves too much room for exploratory overhead.

The jig skill says to run the recipe, but the model often still does some combination of:

- `ls .claude/skills`
- `ls .claude/skills/structured-logging-contract`
- `Read SKILL.md`
- `Read recipe.yaml`
- `ls templates`
- `ls services`

The shortest successful jig runs did not need most of that.

Implication:

- the product path is fine
- the skill wrapper is still under-constrained for speed

### 5. The control skill also has path variance, just in a different way.

The fastest control runs are very direct:

- read skill
- read target
- rewrite file

The slower control runs treat the task more like an exploration exercise and spend additional turns verifying context before making the exact same edit.

So the path-variance problem is not unique to jig.

### 6. Cost and token measurements are not moving in lockstep.

Across these five runs:

- jig averages slightly more total tokens
- jig averages lower cost
- jig averages nearly identical duration

This implies the underlying provider billing and cache behavior matter enough that “more total tokens” does not automatically mean “more expensive” in this sample.

We should keep logging all token categories and cost separately and avoid reducing this to one scalar.

## Classification: Product vs LLM vs Harness

### Product / Library

Current status:

- no evidence of a remaining jig library failure on structured logging
- current recipe shape is behaving correctly

### Skill / Prompt Wrapper

Current status:

- still room to tighten the jig skill so the model goes straight to `jig run`
- still room to tighten the control wrapper so it avoids unnecessary exploration on trivial files

### LLM Behavior

Current status:

- most of the variability comes from the model deciding how much to inspect before acting
- that affects both arms

### Harness

Current status:

- harness is behaving as intended here
- `score` correctly treats both outputs as successful
- `file_score` correctly exposes that only jig matches the exact expected template

The only caution is interpretive:

- readers must not confuse lower `file_score` for control with semantic failure

## Recommendations

1. Keep this scenario in the suite.
   It now measures a real distinction: semantic parity with different exactness properties.

2. Tighten the jig skill wrapper for speed.
   The best next change is to make the usage instruction more imperative and minimize discovery steps.

3. Keep reporting `score` and `file_score` separately.
   They are both informative here.

4. Do not make any further library changes because of this scenario alone.
   The remaining variability is not pointing at a library defect.

5. For comparative claims, use at least 3-5 reps on this scenario.
   Single-run reads are too noisy.

## Bottom Line

Structured logging is no longer a correctness concern for jig.

The real story after five valid runs is:

- both arms are correct
- jig matches the expected template exactly
- control produces a semantically good but structurally different variant
- efficiency is close enough that agent path variance can flip the per-run winner
- this is now mostly a skill-wrapper and measurement-interpretation problem, not a product problem
