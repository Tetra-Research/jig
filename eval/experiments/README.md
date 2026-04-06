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

## Blog Reference: Current Takeaways (as of 2026-04-06)

This section is the canonical summary to quote in external writing.

Data used for this snapshot:

- `eval/experiments/experiments.jsonl` (`exp-001` to `exp-004`)
- `eval/results/results-core.jsonl` (50 trials; `2026-04-06T16:44:24.645Z` to `2026-04-06T17:06:52.019Z`)
- `eval/results/archive/results-smoke-tests.jsonl` (3 trials)

### Headline

jig performs best when the model can map a request to a specific skill quickly. When that mapping is explicit (directed prompt), we see higher scores, more consistent jig usage, and lower output/cost in several scenarios. The largest remaining gap is discovery under vague prompts.

### Where jig is doing well

From `results-core.jsonl`:

- Directed prompts: `n=13`, mean score `0.923`, jig usage `92.3%` (`12/13` perfect runs).
- Natural prompts: `n=37`, mean score `0.725`, jig usage `21.6%`.
- Directed + jig used: `n=12`, mean score `1.000`.

Scenario breakdown (latest core set):

| Scenario | Directed Mean | Natural Mean | Interpretation |
|---|---:|---:|---|
| `add-endpoint` | 1.000 | 0.852 | Strong fit for explicit skill + short workflow. |
| `add-field` | 1.000 | 0.933 | Quality is high in both modes; directed is more reliable. |
| `add-view` | 0.750 | 0.875 | One directed outlier (`0.0`) pulled mean down; other directed reps passed. |
| `scaffold-test` | 1.000 | 0.222 | Biggest discovery gap: explicit skill works, vague prompt often misses. |

### Discovery is still the bottleneck

From `results-core.jsonl` natural trials:

- When jig was discovered: `n=8`, mean score `1.000`.
- When jig was not discovered: `n=29`, mean score `0.649`.

This supports a clear thesis: once the model chooses jig, task execution is usually strong; the failure mode is finding the right skill under vague prompts.

Gradient-style levels in the same core set:

| Level | Setup | Trials | Mean Score | Jig Used | Mean Tokens | Mean Cost | Mean Duration |
|---|---|---:|---:|---:|---:|---:|---:|
| L2 | skills + shared `CLAUDE.md` + natural prompt | 12 | 0.800 | 66.7% | 272,939 | $0.735 | 69.7s |
| L3 | skills + shared `CLAUDE.md` + directed prompt | 13 | 0.923 | 92.3% | 212,840 | $0.580 | 51.0s |

L3 vs L2 deltas:

- Score: `+0.123`
- Jig usage: `+25.6` percentage points
- Total tokens: `-22.0%`
- Output tokens: `-36.2%`
- Cost: `-21.2%`
- Duration: `-26.9%`

### Where we already see token/cost savings

Smoke comparison (`add-view`, natural prompt, shared `CLAUDE.md`):

- Baseline: `317,608` tokens, `$0.83`, `84.0s`, score `1.0`
- Jig: `241,702` tokens, `$0.65`, `66.2s`, score `1.0`

Savings in this smoke case:

- Tokens: `23.9%` lower
- Cost: `21.4%` lower
- Duration: `21.3%` faster

Important caveat:

- In a no-nudge control (`--mode baseline --claude-md none`) on the same easy scenario, the agent did not use jig and was faster/cheaper (`162,530` tokens, `$0.47`, `50.0s`). This is expected for single easy edits. The value signal for jig appears on consistency and correctness over harder/multi-file workflows, not on every trivial case.
- The full baseline control experiment (`exp-004`) scored `0.730` across 7 trials with `0%` jig usage, and failed `scaffold-test` plus `inject-import`. That is the quality floor we are trying to raise with better discovery and directed skill use.

### Use cases to extrapolate for blog framing

- Brownfield multi-file extensions with known templates (`add-endpoint`, `add-field`) are already strong when skill selection is explicit.
- Greenfield scaffolding (`scaffold-test`) is high upside for jig, but today it is discovery-sensitive under natural language prompts.
- The biggest performance unlock is reducing discovery overhead (prompting + skill descriptions + `CLAUDE.md` nudges), not changing jig's core file mutation mechanics.

### How the eval harness works (what these numbers actually measure)

- Each scenario packages a fixture codebase, expected file outputs, weighted assertions, and negative assertions.
- Per trial, the harness creates an isolated temp sandbox, copies the scenario codebase, optionally strips skills (`--strip-skills`), and configures `CLAUDE.md` mode (`shared|empty|none`).
- `jig` mode leaves prompt references intact; `baseline` mode strips jig-specific prompt text and adds a "do not use jig" baseline context.
- Agents are invoked via CLI config in `agents.yaml`; no internal instrumentation hooks are used.
- Scoring combines weighted assertion pass rate and negative assertions (`total = assertion_score * negative_score`), plus secondary file similarity, jig usage/correctness, tool-call and token/cost efficiency metrics.
- Results are appended as JSONL rows; strict/compat schema modes protect analysis quality when reading mixed historical archives.

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
