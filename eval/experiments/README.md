# Experiment Log

Structured record of eval runs against the jig agent eval harness. Each experiment tests whether LLM agents discover and correctly use jig skills vs. hand-editing files.

## Schema

Each entry in `experiments.jsonl` captures:

```jsonc
{
  "id": "exp-001",                    // Sequential experiment ID
  "timestamp": "2026-04-06T...",      // ISO 8601
  "description": "...",              // What we're testing and why
  "config": {
    "scenarios": ["add-view"],       // Scenario filter (or "all")
    "agents": ["claude-code"],       // Agent filter (or "all")
    "mode": "baseline",              // "jig" | "baseline"
    "prompt_tier": "natural",        // Filter or "all"
    "claude_md": "shared",           // "shared" | "empty" | "none"
    "reps": 1                        // Repetitions per trial
  },
  "results": {
    "trials": 1,                     // Total trials run
    "mean_score": 0.85,              // Mean total score
    "jig_used_pct": 0.0,             // % of trials where jig was invoked
    "mean_duration_s": 45.2,         // Mean trial duration
    "mean_cost_usd": 0.78,           // Mean cost per trial
    "total_cost_usd": 0.78           // Total experiment cost
  },
  "observations": "..."             // Human notes on what happened
}
```

## Experiment Index

| ID | Date | Description | Trials | Mean Score | Cost |
|----|------|-------------|--------|------------|------|
| (entries added as experiments run) |

## Scoring Mechanism (Exact)

The harness computes trial scores with explicit formulas from `eval/harness/score.ts`:

- `assertion_score = sum(weights of passed assertions) / sum(all assertion weights)`
- `negative_score = 1.0` if all negative assertions pass, else `0.0`
- `total = assertion_score * negative_score` (primary metric)

Secondary diagnostics are recorded but not multiplied into `total`:

- `file_score` (structural similarity signal)
- `jig_used` (at least one `jig` invocation found)
- `jig_correct` (valid JSON `--vars` when present, and invocation count within `max_jig_commands`)

Efficiency accounting is explicit and parseable:

- `tokens_used = input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens`
- `cost_usd` is recorded per trial from agent output
- Efficiency means are only computed on rows with full coverage (no zero-filling)

## Control-Group Reference: Current Takeaways (as of 2026-04-06)

This section intentionally reports only metrics anchored to a baseline control group.

Data used for this snapshot:

- `eval/experiments/experiments.jsonl` (`exp-001` to `exp-004`)
- `eval/results/archive/results-smoke-tests.jsonl` (3 control/treatment smoke rows)

### Matched Control Comparison (Same Scenario / Prompt)

`add-view`, natural prompt, shared `CLAUDE.md`, 1 trial per arm:

| Arm | Mean Score | Tokens | Cost | Duration |
|---|---:|---:|---:|---:|
| Baseline control (`--mode baseline`) | 1.000 | 317,608 | $0.8279 | 84.0s |
| Jig treatment (`--mode jig`) | 1.000 | 241,702 | $0.6505 | 66.2s |

Treatment delta vs control:

- Tokens: `-23.9%`
- Cost: `-21.4%`
- Duration: `-21.3%`

### Strict No-Jig Control

`add-view`, natural prompt, `--mode baseline --claude-md none`, 1 trial:

- Score: `1.000`
- Jig usage: `0%`
- Tokens: `162,530`
- Cost: `$0.4746`
- Duration: `50.0s`

This is the clean "agent hand-edits without jig nudges" control.

### Full Baseline Control Sweep

`exp-004` (`--mode baseline --claude-md none`, all 7 scenarios, 1 rep each):

- Trials: `7`
- Mean score: `0.730`
- Jig usage: `0%`
- Mean duration: `37.4s`
- Mean cost: `$0.36`
- Total cost: `$2.51`

### Control-Only Interpretation

- We can claim concrete token/cost/duration savings from the matched smoke control above.
- We can claim a baseline quality floor (`0.730`) from the 7-scenario no-jig control sweep.
- We should not quote broad input/output-token savings against control until we run a matched full-matrix baseline archive with current (`v2`) token fields.

### How Control Deltas Are Computed

For any metric `m` where lower is better (tokens, cost, duration):

- `delta_pct = (1 - treatment_m / control_m) * 100`

Scores are still computed with the primary harness metric:

- `total = assertion_score * negative_score`

## Methodology

1. **Smoke tests first**: Single scenario, single agent, 1 rep to verify harness
2. **Baseline**: `--mode baseline` strips skill/jig references from prompts
3. **Jig mode**: `--mode jig` leaves prompts intact, skills available in sandbox
4. **Prompt tiers**: directed (explicit skill reference) > natural (task description) > ambient (embedded in context)
5. **CLAUDE.md modes**: shared (nudge to use skills) > empty (minimal) > none (no guidance)
6. **Full matrix**: All combinations with 3 reps for statistical significance

## Reproducing

```bash
cd eval

# Smoke test
npx tsx harness/run.ts --scenario add-view --prompt-tier natural --mode baseline --agent claude-code --reps 1

# Full baseline run
npx tsx harness/run.ts --mode baseline --reps 3

# Full jig run
npx tsx harness/run.ts --mode jig --reps 3

# Dry run (see trial count without executing)
npx tsx harness/run.ts --dry-run

# Gradient experiment helper (defaults to SCHEMA_MODE=compat)
bash experiments/run-gradient.sh
```

## Schema Compatibility Modes

Result archives currently exist in both legacy (`v1`) and current (`v2`) shapes.

- `strict` mode: fails on mixed schemas, malformed JSONL, or invalid rows.
- `compat` mode: allows mixed archives, prints schema diagnostics, and computes efficiency metrics only on covered rows.

Examples:

```bash
# Readiness/CI-safe run (default)
npx tsx harness/run.ts --schema-mode strict

# Exploratory archive analysis
npx tsx experiments/analyze-gradient.ts \
  --results results/archive/results-mixed-schema-20260406T114302.jsonl \
  --schema-mode compat

# Force strict mode in the gradient helper when desired
SCHEMA_MODE=strict bash experiments/run-gradient.sh
```

## Archive Hygiene: Split Mixed JSONL

Use the splitter utility to produce schema-homogeneous files from a mixed archive:

```bash
npx tsx experiments/split-results-by-schema.ts \
  --input results/archive/results-mixed-schema-20260406T114302.jsonl \
  --out-dir results/archive
```

This writes:

- `<prefix>.v1-legacy.jsonl`
- `<prefix>.v2-current.jsonl`
