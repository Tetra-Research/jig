# Workstream Autopilot

Full unattended pipeline: init → plan → execute → review → consolidate → PR.

Run this and walk away. Come back to a PR.

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

1. **Init** — Creates workstream if it doesn't exist (`ws-init`)
2. **Populate** — Agent fills SPEC.md + PLAN.md from project spec (`jig.md`)
3. **Plan** — Dual-agent planning + synthesis (`ws-plan --synthesize`)
4. **Execute** — Iterative implementation with validation (`ws-execute`)
5. **Review** — Dual-agent review cycle: review → synthesize → fix (`ws-review-cycle`)
6. **Consolidate** — Update durable docs with learnings (`ws-consolidate`)
7. **PR** — Push branch and open pull request

Smart defaults: skips init if workstream exists, skips planning if synthesized plan exists.
Each stage commits its results. Failed execution opens a draft PR with partial work.

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--max-iter` | 3 | Fix iterations (execution + review) |
| `--rounds` | 2 | Review cycle rounds |
| `--agent` | claude | Agent for execution |
| `--review-agent` | both | Agent for reviews |
| `--plan-agent` | both | Agent for planning |
| `--branch` | auto | Custom branch name |
| `--base` | main | Base branch for PR |
| `--skip-init` | | Skip init + spec population |
| `--skip-plan` | | Skip planning |
| `--skip-execute` | | Skip execution |
| `--skip-review` | | Skip review cycle |

## Examples

```bash
# Brand new workstream — full pipeline from scratch
./scripts/ws-autopilot.sh replace-patch

# Existing workstream, specific phases
./scripts/ws-autopilot.sh core-engine phase-6 phase-7 --max-iter 3

# Just execute + review (planning already done)
./scripts/ws-autopilot.sh core-engine --skip-init --skip-plan

# Review + consolidate only (code already written)
./scripts/ws-autopilot.sh core-engine --skip-execute --rounds 3
```
