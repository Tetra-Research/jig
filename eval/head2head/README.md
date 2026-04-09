# Head-to-Head Skill Runner

Purpose: run a strict A/B comparison where the only intentional change is the skill profile.

Reference notes for upcoming skill work:
- `DJANGO_PATTERN_PLAYBOOK.md` (captured 2026-04-08)
- `HEAD2HEAD_SKILL_PAIRS.md` (active pair map)

- Arm `control`: your control skill profile.
- Arm `jig`: your jig-backed skill profile.
- Same scenario codebase, same prompt, same agent, same rep index.

The runner writes:

- Trial rows: `results/head2head-results.jsonl` (`schema_version=head2head_v1`)
- Pair rows: `results/head2head-pairs.jsonl` (`schema_version=head2head_pair_v1`)
- Per-trial artifacts: `results/head2head-artifacts/...` including prompt/stdout/stderr plus `git diff`, changed-file manifests, and changed-file snapshots

## Execution Model

Current behavior:

- The runner is serial today.
- It executes one trial at a time.
- A full run is `scenario_count x reps x 2 arms`.
- With the current `h2h-*` set, a default full run is `5 scenarios x 1 rep x 2 arms = 10 trials`.

Why this is safe to parallelize:

- Each trial uses its own temp sandbox.
- Each trial writes artifacts into its own unique directory.
- Each agent invocation is an isolated child process.

The practical bottlenecks are not local filesystem conflicts. They are provider-side throughput, rate limits, and experiment noise from running too many comparisons at once.

## Parallelism Notes

Planned direction:

- Add bounded concurrency rather than unbounded `Promise.all(...)`.
- Parallelize at the `scenario x rep` pair level first.
- Keep control and jig arms sequential within a pair by default.
- Optionally allow concurrent arms inside a pair when speed matters more than experimental cleanliness.

Recommended future flags:

- `--max-parallel-pairs <n>`: run up to `n` scenario/rep pairs at once.
- `--pair-mode sequential|concurrent`: choose whether control/jig inside a pair run one after the other or at the same time.
- `--shuffle-pairs`: reduce fixed ordering bias across long runs.
- retry controls for transient provider failures or timeouts.

Recommended default when this is implemented:

- Use a small bounded pool such as `2-4` parallel pairs.
- Default to `--pair-mode sequential` for cleaner A/B comparisons.
- Treat higher concurrency as a throughput optimization, not a scientifically neutral change.

## Profile Format

Each arm points to a profile directory using `--control-profile` and `--jig-profile`.
Supported profile layouts:

1. `overlay/` directory:
`overlay/` is copied directly onto the sandbox root.
Use this when you need full control over `.claude/skills`, `CLAUDE.md`, or any extra files.

2. Simple layout:
- `CLAUDE.md` (optional, copied to sandbox root)
- `skills/<skill-name>/...` (optional, copied to `.claude/skills`)

The runner defaults to clean-slate mode, which removes any `.claude/` and `CLAUDE.md` from scenario codebase before applying a profile.

## Usage

From `eval/`:

```bash
npx tsx head2head/run.ts \
  --scenario add-field-v9-template-first \
  --agent claude-code \
  --reps 3 \
  --control-profile head2head/profiles/control \
  --jig-profile head2head/profiles/jig \
  --thinking-mode
```

## Important Flags

- `--prompt-source natural|directed|ambient|legacy_prompt|custom` (default: `natural`)
- `--prompt-file <path>` or `--prompt-text "<text>"` (required when `--prompt-source custom`)
- `--thinking-mode`: injects a required line before first tool call so you can capture intent.
- `--thinking-prefix <prefix>` (default: `HEAD2HEAD_THINKING:`)
- `--preserve-codebase-claude`: disables clean-slate removal.
- `--results <path>` and `--pairs <path>` for custom JSONL outputs.
- `--artifacts-dir <path>` and `--no-capture-artifacts`.
- `--dry-run`.

## Telemetry Captured

Per trial the runner captures, when available from stream-json:

- score/assertion outputs
- duration (`duration_ms`, `duration_api_ms`)
- tool-call counts and tool-call breakdown by name
- jig command detections
- input/output/cache tokens
- `context_tokens` (`input + cache creation + cache read`)
- `tokens_used`
- `cost_usd`
- model/service tier/model usage map
- permission denial count
- raw init/result events for post-hoc analysis
- optional `thinking_text` extracted from your configured prefix
