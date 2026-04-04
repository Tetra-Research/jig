# Workstream Autopilot

Full unattended pipeline: execute → review cycle → consolidate → PR.

## Usage
```
/ws-autopilot <workstream> [task...] [--max-iter 3] [--rounds 2]
```

## Arguments
$ARGUMENTS

---

Run the autopilot script:

```bash
./scripts/ws-autopilot.sh $ARGUMENTS
```

## How It Works

1. **Branch** — Creates `autopilot/<workstream>/<task>` feature branch
2. **Execute** — Runs `ws-execute` with iterative fix loop
3. **Review** — Runs `ws-review-cycle` (dual-agent review → synthesize → fix)
4. **Consolidate** — Runs `ws-consolidate` to update durable docs
5. **PR** — Pushes branch and opens a pull request with pipeline results

Each stage commits its results. If execution fails, a draft PR is opened with partial work.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--max-iter` | 3 | Fix iterations per step (execution + review) |
| `--rounds` | 2 | Review cycle rounds |
| `--agent` | claude | Agent for execution |
| `--review-agent` | both | Agent for reviews (claude, codex, both) |
| `--branch` | auto | Custom branch name |
| `--base` | main | Base branch for PR |
| `--skip-execute` | | Start at review (code already written) |
| `--skip-review` | | Skip review cycle |

## Examples

```bash
# Full pipeline for next phases
./scripts/ws-autopilot.sh core-engine phase-6 phase-7 --max-iter 3

# Review + consolidate only (code already done)
./scripts/ws-autopilot.sh core-engine --skip-execute --rounds 3

# Single task, claude-only everything
./scripts/ws-autopilot.sh core-engine phase-6 --agent claude --review-agent claude
```
