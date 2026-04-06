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
```
