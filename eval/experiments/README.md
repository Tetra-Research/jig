# Experiment Log

Structured record of eval runs against the jig agent eval harness. Each experiment tests whether LLM agents discover and correctly use jig skills vs. hand-editing files.

## 2026-04-07: Dedicated Head-to-Head Runner

The preferred experiment path is now the dedicated head-to-head runner:

- `eval/head2head/run.ts`
- `eval/head2head/README.md`
- helper script: `eval/experiments/run-head2head.sh`

This runner removes the old multi-level matrix assumptions and executes only two explicit arms on the same scenario/codebase/prompt:

1. `control` profile
2. `jig` profile

Each arm can supply its own skills and optional `CLAUDE.md` via profile directories.
The runner also captures richer telemetry (including context tokens, model usage map, tool-call breakdown, and raw init/result events).

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

## Strict-Control Snapshot: 2026-04-06

This section is the current source of truth for the "does jig help?" claim.
It is fully control-anchored and based on current (`v2`) result rows.

### Why We Re-Ran

We found an invalid control in earlier gradient runs: a baseline cell path could still run with jig mode defaults.
To fix that, we enforced a strict no-jig baseline and re-ran a matched A/B sweep.

### Harness/Experiment Changes Applied

1. Baseline now explicitly uses `--mode baseline` in the gradient helper.
2. Added `--disable-jig-binary` in `eval/harness/run.ts` and sandbox plumbing.
3. Sandbox now injects a shim `jig` binary that exits non-zero, so accidental jig calls fail in control.
4. Strict control recipe now uses all of:
   - `--mode baseline`
   - `--prompt-tier natural`
   - `--strip-skills`
   - `--claude-md none`
   - `--disable-jig-binary`
5. Treatment arm uses directed jig:
   - `--mode jig`
   - `--prompt-tier directed`
   - `--claude-md shared`

### Dataset Used

- Control check seed: `eval/results/archive/results-gradient-control-check-20260406T170742.jsonl`
- Confirmatory add-view: `eval/results/archive/results-confirmatory-2trial-add-view-20260406T171746.jsonl`
- Main additional batch: `eval/results/archive/results-blog-solid-additional-20260406T172423.jsonl`
- Merged cumulative n=5: `eval/results/archive/results-blog-solid-cumulative-n5-20260406T172423.jsonl`
- Targeted timeout retry (add-field directed): `eval/results/archive/results-timeout-retry-add-field-directed-20260406T175159.jsonl`
- Timeout-retry-adjusted cumulative: `eval/results/archive/results-blog-solid-cumulative-n5-timeout-retry-adjusted-20260406T175159.jsonl`

Final comparison set:

- Scenarios: `add-view`, `add-field`, `add-endpoint`
- Agent: `claude-code`
- Reps: `5` baseline + `5` jig per scenario (`30` total rows)
- Schema mode: `strict`

### Timeout Handling

One timeout occurred in the main batch:

- scenario: `add-field`
- arm: `jig` + `directed`
- row: timeout `true`, `duration_ms ~120000`

We publish three views to avoid hiding this:

1. `as-run` (includes timeout row)
2. `exclude-timeouts` (drops timeout rows)
3. `retry-adjusted` (replaces the timeout row with one targeted retry trial)

The summary below uses `retry-adjusted` so each arm stays balanced at `n=5`.

### Metrics (Explicit)

- `output_tokens`: assistant output tokens only.
- `no_cache_tokens`: `input_tokens + output_tokens` (ignores cache read/create effects).
- `no_read_tokens`: `tokens_used - cache_read_input_tokens` (keeps cache creation, removes cache reads).
- `raw_cost_usd`: provider-reported total cost from row (`cost_usd`).

Note: we do not currently store `input_cost_usd` and `output_cost_usd` separately.

### Plain-English Scorecard (Retry-Adjusted, n=5 Per Arm)

`BETTER` means jig is lower (good). `WORSE` means jig is higher (bad).

| Scope | Output Tokens | No-Cache Tokens (`input+output`) | No-Read Tokens (`tokens_used-cache_read`) | Raw Cost USD |
|---|---|---|---|---|
| OVERALL | `2507.9 -> 1702.7` (`32.1% BETTER`) | `2583.4 -> 1745.1` (`32.5% BETTER`) | `12705.3 -> 10372.7` (`18.4% BETTER`) | `$0.7398 -> $0.6431` (`13.1% BETTER`) |
| add-endpoint | `3335.2 -> 1301.8` (`61.0% BETTER`) | `3430.8 -> 1342.2` (`60.9% BETTER`) | `14019.4 -> 8873.0` (`36.7% BETTER`) | `$0.9586 -> $0.4987` (`48.0% BETTER`) |
| add-view | `1407.4 -> 822.8` (`41.5% BETTER`) | `1458.0 -> 853.0` (`41.5% BETTER`) | `12296.0 -> 7341.4` (`40.3% BETTER`) | `$0.5153 -> $0.3804` (`26.2% BETTER`) |
| add-field | `2781.2 -> 2983.4` (`7.3% WORSE`) | `2861.4 -> 3040.0` (`6.2% WORSE`) | `11800.6 -> 14903.8` (`26.3% WORSE`) | `$0.7456 -> $1.0500` (`40.8% WORSE`) |

### Interpretation We Can Defend Today

1. Jig is clearly better on `add-view` and `add-endpoint`.
2. Jig is clearly worse on `add-field` right now.
3. Net across all three scenarios, jig still wins on output tokens and raw cost.
4. This is not only a cache artifact: `add-field` remains worse even in cache-neutral token views.

### How We Compute Delta vs Baseline

For any "lower is better" metric `m`:

- `reduction_pct = (1 - jig_m / baseline_m) * 100`

So:

- positive `%` = `BETTER` (jig lower)
- negative `%` = `WORSE` (jig higher)

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

# Disable per-trial artifact capture (stdout/stderr/prompt logs)
npx tsx harness/run.ts --scenario add-view --reps 1 --no-capture-artifacts

# Customize artifact output directory
npx tsx harness/run.ts --scenario add-view --reps 1 --artifacts-dir results/artifacts-custom

# Full baseline run
npx tsx harness/run.ts --mode baseline --reps 3

# Full jig run
npx tsx harness/run.ts --mode jig --reps 3

# Explicit results path (recommended for archiving)
npx tsx harness/run.ts --mode baseline --reps 1 --results results/archive/results-baseline-$(date +%Y%m%dT%H%M%S).jsonl

# Dry run (see trial count without executing)
npx tsx harness/run.ts --dry-run

# Gradient experiment helper (defaults to SCHEMA_MODE=compat)
bash experiments/run-gradient.sh

# Full discovery matrix (30 cells across mode/prompt/CLAUDE.md/skills)
REPS=1 AGENT=claude-code JOBS=8 SCHEMA_MODE=strict bash experiments/run-discovery-matrix.sh

# Include scheduler control pair (sequential JOBS=1 and parallel JOBS=8)
REPS=1 AGENT=claude-code JOBS=8 INCLUDE_PARALLEL_CONTROL=1 SCHEMA_MODE=strict bash experiments/run-discovery-matrix.sh
```

## Thorough Parallel Matrix

`experiments/run-discovery-matrix.sh` runs a full discovery-factor matrix with sharded per-cell outputs.

Dimensions:

- `mode`: `baseline`, `jig`
- `prompt-tier`: `natural`, `ambient` (and `directed` for `jig`)
- `claude-md`: `none`, `empty`, `shared`
- `strip-skills`: `false`, `true`

This yields `30` cells total:

- Baseline cells: `2 prompt tiers * 3 claude-md * 2 skill states = 12`
- Jig cells: `3 prompt tiers * 3 claude-md * 2 skill states = 18`

Each cell writes to its own JSONL shard under `results/tmp/...`, then the script merges shards into a single archive in `results/archive/` and emits a manifest TSV describing the matrix cell metadata.

Parallelization controls:

- `JOBS=<n>` sets concurrent cell workers (`xargs -P`).
- `INCLUDE_PARALLEL_CONTROL=1` runs a built-in schedule-control pair:
  - Sequential pass (`JOBS=1`)
  - Parallel pass (`JOBS=<configured>`)

This is the recommended way to quantify whether scheduler parallelism changes scores, token/cost metrics, or failure/timeout behavior.

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

# Use an explicit output path for gradient runs
RESULTS_PATH=results/archive/results-gradient-$(date +%Y%m%dT%H%M%S).jsonl SCHEMA_MODE=strict bash experiments/run-gradient.sh
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
