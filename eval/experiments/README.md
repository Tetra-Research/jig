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

## Findings

### Discovery tax is a prompting problem, not a utility problem (2026-04-06)

The L2→L3 gap (0.890 vs 0.935) is almost entirely explained by whether the agent connects its task to jig, not by whether jig is useful once invoked. Evidence:

- **When agents find jig, they use it correctly.** L2 jig usage is 67% and rising. When jig is used, scores match L3.
- **scaffold-test is the clearest signal.** L0: 0.00, L1: 0.25, L2: 0.50, L3: 1.00. The `create-test` recipe works perfectly — the variable is whether the agent thinks to look for it given a natural prompt like "create a test file."
- **Strengthening the CLAUDE.md nudge moved L2 from 0.844 → 0.890** without changing any recipes or agent behavior. The discover skill description listing concrete tasks ("scaffold a test, add an endpoint") improved triggering.
- **Input token cost at L2 (243K) exceeds L3 (216K)** — the agent still explores before finding the right recipe. L3 skips exploration because the prompt names the skill directly.

**Implication for the blog post:** jig's value proposition (deterministic, assertion-verifiable output) holds across all levels. The remaining gap is a skill-discovery UX problem — how quickly the agent maps a natural-language task to the right recipe. Better skill descriptions and CLAUDE.md framing close most of it; the last ~5% requires explicit prompting.

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
