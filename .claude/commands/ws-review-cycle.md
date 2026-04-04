# Workstream Review Cycle

Full review pipeline: review → synthesize → fix, repeated N rounds.

## Usage
```
/ws-review-cycle <workstream> [task] [--rounds 3] [--max-iter 3]
```

## Arguments
$ARGUMENTS

---

Run the review cycle script:

```bash
./scripts/ws-review-cycle.sh $ARGUMENTS
```

## How It Works

Each round:
1. **Review** — Claude + Codex review code against SPEC and INVARIANTS (parallel)
2. **Synthesize** — Third agent merges findings, deduplicates, ranks by severity
3. **Fix** — Agent applies fixes (up to `--max-iter` attempts per round)
4. **Loop** — If fixes applied, next round re-reviews the updated code

Stops early if a review comes back clean (no Critical or Major findings).

## Options

| Flag | Default | Description |
|------|---------|-------------|
| `--rounds` | 3 | Number of review→fix cycles |
| `--max-iter` | 3 | Max fix iterations per round |
| `--agent` | both | Review agents (claude, codex, or both) |

## Examples

```bash
# Full review cycle, 3 rounds
./scripts/ws-review-cycle.sh core-engine --rounds 3

# Scoped to a phase, 2 rounds with 2 fix iterations each
./scripts/ws-review-cycle.sh core-engine phase-3 --rounds 2 --max-iter 2

# Claude-only review (no Codex)
./scripts/ws-review-cycle.sh core-engine --agent claude --rounds 2
```
