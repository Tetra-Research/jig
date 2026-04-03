# Workstream Execute

Run iterative execution with fresh-context retries (Ralph-loop pattern).

## Usage
```
/ws-execute <workstream> [task] [--agent claude|codex] [--max-iter 5]
```

## Arguments
$ARGUMENTS

---

Run the execution script:

```bash
./scripts/ws-execute.sh $ARGUMENTS
```

## How It Works

The script runs an agent in a **fresh-context iteration loop**:

1. Build execution prompt from synthesized plan + task context + VALIDATION.md
2. For each iteration (fresh subprocess, no accumulated context):
   - Run agent with prompt + any previous failure context
   - Save output to `exec/iteration-N-timestamp.md`
   - Run `validate.sh` to check pass/fail
   - If passes → COMPLETE
   - If fails → extract failures, feed to next iteration
3. Write execution summary

## Key Design

- **Fresh context per iteration** — each run starts clean, preventing context confusion
- **Failure forwarding** — each iteration sees "the previous attempt failed because X"
- **validate.sh as gate** — reuses existing validation (tests + VALIDATION.md)

## After Execution

- If COMPLETE: `/ws-review <ws> [task]`
- If INCOMPLETE: review iteration outputs, fix issues manually, re-run
