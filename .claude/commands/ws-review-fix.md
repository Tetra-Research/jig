# Workstream Review Fix

Apply review findings to the code automatically.

## Usage
```
/ws-review-fix <workstream> [task] [--findings <path>] [--max-iter 3]
```

## Arguments
$ARGUMENTS

---

Run the review fix script:

```bash
./scripts/ws-review-fix.sh $ARGUMENTS
```

## How It Works

1. Reads synthesized review findings (or a specific findings file)
2. Feeds Critical + Major + Minor findings to an agent with the SPEC and INVARIANTS as context
3. Agent makes minimal fixes, runs `cargo test`
4. If tests fail, feeds failures to next iteration (same pattern as ws-execute)

## Finding the Findings

Priority order for auto-discovery:
1. `--findings <path>` if specified
2. `code-review-synthesized.md` symlink in reviews/
3. Latest `code-review-synthesized-*.md` by timestamp
4. Latest `code-review-claude-*.md` as fallback

## Full Review Pipeline

```
/ws-review core-engine --synthesize     # 1. Dual-agent review + merge
/ws-review-fix core-engine              # 2. Auto-fix findings
# review diff, commit                   # 3. Human checkpoint
/ws-review core-engine --synthesize     # 4. Re-review to confirm
```
