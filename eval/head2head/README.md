# Head-to-Head Skill Runner

Purpose: run a strict A/B comparison where the only intentional change is the skill profile.

Reference notes for upcoming skill work:
- `DJANGO_PATTERN_PLAYBOOK.md` (captured 2026-04-08)

- Arm `control`: your control skill profile.
- Arm `jig`: your jig-backed skill profile.
- Same scenario codebase, same prompt, same agent, same rep index.

The runner writes:

- Trial rows: `results/head2head-results.jsonl` (`schema_version=head2head_v1`)
- Pair rows: `results/head2head-pairs.jsonl` (`schema_version=head2head_pair_v1`)
- Per-trial artifacts: `results/head2head-artifacts/...`

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
