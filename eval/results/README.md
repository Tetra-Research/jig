# Head-to-Head Benchmark

This is the benchmark page linked from the marketing site.

It explains the current public proof claim for `jig`, points at the exact data files behind that claim, and shows how to rerun the same head-to-head comparison yourself.

## What This Measures

This benchmark compares two arms on the same scenario codebase, prompt, agent, and rep index:

- `control`: a normal skill that specifies the required output contract in prose
- `jig`: a matching skill that uses `jig` to generate the same target shape

The current suite covers five routine backend edit patterns:

- deterministic service tests
- query-layer discipline
- schema migration safety
- structured logging contract
- view contract enforcement

The active pair definitions live in [eval/head2head/HEAD2HEAD_SKILL_PAIRS.md](/Users/tylerobriant/code/tetra/jig/eval/head2head/HEAD2HEAD_SKILL_PAIRS.md).

## Current Public Result

Source dataset:

- pair rows: [head2head-pairs-r25-20260409.jsonl](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-pairs-r25-20260409.jsonl)
- trial rows: [head2head-results-r25-20260409.jsonl](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-results-r25-20260409.jsonl)
- artifacts: [h2h-r25-20260409](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-artifacts/h2h-r25-20260409)

Run shape:

- `5` scenarios
- `3` reps per scenario
- `15` pair rows / `30` trial rows total
- agent: `claude-code`

### Aggregate Outcome

All `15/15` pairs passed in both arms.

| Metric | Control | Jig | Delta | Improvement |
| --- | ---: | ---: | ---: | ---: |
| Tokens used | 3,208,641 | 1,745,296 | -1,463,345 | 45.6% less |
| Duration | 436s | 243s | -194s | 44.4% faster |
| Cost | $9.80 | $5.74 | -$4.06 | 41.4% lower |
| Tool calls | 149 | 64 | -85 | 57.0% fewer |

### At-A-Glance Chart

```text
Tokens   45.6% less   ████████████████████
Time     44.4% faster ███████████████████
Cost     41.4% lower  ██████████████████
Tools    57.0% fewer  ███████████████████████
```

## Per-Scenario Read

The overall win is real, but it is not uniform across every pattern. `structured-logging-contract` is the important exception: it reaches correctness parity, but it is not an efficiency win in this dataset.

The table below uses median pair deltas across the three reps for each scenario. Negative values mean `jig` used less / finished faster / cost less.

| Scenario | Correctness parity | Median token delta | Median time delta | Median cost delta | Read |
| --- | ---: | ---: | ---: | ---: | --- |
| `h2h-deterministic-service-test` | 3/3 | -73,012 | -7.0s | -$0.18 | Clear win |
| `h2h-query-layer-discipline` | 3/3 | -178,413 | -21.0s | -$0.47 | Strong win |
| `h2h-schema-migration-safety` | 3/3 | -203,801 | -25.6s | -$0.51 | Strong win |
| `h2h-structured-logging-contract` | 3/3 | +72,069 | +8.5s | +$0.16 | Honest exception |
| `h2h-view-contract-enforcer` | 3/3 | -87,022 | -19.1s | -$0.32 | Clear win |

### Per-Scenario Percentage Chart

Percentages below are the median `jig` improvement over control for each scenario and metric. Negative values mean `jig` was worse on that metric.

| Scenario | Tokens | Time | Cost |
| --- | ---: | ---: | ---: |
| `h2h-deterministic-service-test` | 39.6% | 28.8% | 33.9% |
| `h2h-query-layer-discipline` | 66.7% | 65.2% | 60.4% |
| `h2h-schema-migration-safety` | 64.5% | 64.0% | 57.5% |
| `h2h-structured-logging-contract` | -54.6% | -59.2% | -37.0% |
| `h2h-view-contract-enforcer` | 43.7% | 59.5% | 45.9% |

## How To Rerun It

From [`eval/`](/Users/tylerobriant/code/tetra/jig/eval):

```bash
npx tsx head2head/run.ts \
  --agent claude-code \
  --reps 3 \
  --prompt-source directed \
  --thinking-mode \
  --control-profile head2head/profiles/control \
  --jig-profile head2head/profiles/jig \
  --results results/head2head-results-local.jsonl \
  --pairs results/head2head-pairs-local.jsonl \
  --artifacts-dir results/head2head-artifacts/local
```

This reproduces the same experiment shape:

- same `5` scenarios
- same `control` and `jig` profiles
- same single-agent setup
- same `3` reps per scenario

If you want to smoke-test one pattern first:

```bash
npx tsx head2head/run.ts \
  --scenario h2h-query-layer-discipline \
  --agent claude-code \
  --reps 1 \
  --prompt-source directed \
  --thinking-mode \
  --control-profile head2head/profiles/control \
  --jig-profile head2head/profiles/jig
```

Runner details and flags are documented in [eval/head2head/README.md](/Users/tylerobriant/code/tetra/jig/eval/head2head/README.md).

## How To Inspect The Output

The pair file is the easiest place to start:

- [head2head-pairs-r25-20260409.jsonl](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-pairs-r25-20260409.jsonl)

Each row already contains both arms side by side:

- `score`
- `file_score`
- `duration_ms`
- `tool_calls`
- `context_tokens`
- `output_tokens`
- `tokens_used`
- `cost_usd`

The trial file contains per-arm detail:

- [head2head-results-r25-20260409.jsonl](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-results-r25-20260409.jsonl)

The artifacts directory contains:

- the exact prompt
- stdout / stderr
- git diff
- changed-file snapshots

See [h2h-r25-20260409](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-artifacts/h2h-r25-20260409).

## Interpretation Rules

The claim supported by this dataset is narrow on purpose:

- `jig` helps on routine, shape-constrained backend edits
- `jig` reduces tokens, time, and cost on aggregate in this suite
- `jig` is not a universal win on every repeated task

The claim this dataset does **not** support is:

- `jig` helps all coding-agent work in general

That broader claim would need more task breadth and more model breadth.

## Related Review Notes

If you want the adversarial analysis behind the current harness and skill setup:

- [head2head-r11-20260409-adversarial-review.md](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-r11-20260409-adversarial-review.md)
- [head2head-structured-logging-r20-r24-review-20260409.md](/Users/tylerobriant/code/tetra/jig/eval/results/head2head-structured-logging-r20-r24-review-20260409.md)

Those documents explain why the current benchmark is stricter than the earlier runs and why the structured-logging scenario is treated as an explicit exception instead of being hidden.
