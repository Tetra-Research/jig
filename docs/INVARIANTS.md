# INVARIANTS.md

Project-wide constraints. Every workstream must respect these. If a change would violate an invariant, it requires explicit discussion and sign-off before proceeding.

## I-1: Deterministic Output

Same recipe + same variables + same existing files = same output. Always. No randomness, no timestamps, no machine-specific behavior in rendered output. Tests can assert exact equality.

## I-2: Idempotent Operations

Every file operation *supports* idempotent re-runs via `skip_if_exists` (create) and `skip_if` (inject). When a recipe is designed for idempotent execution (all creates use `skip_if_exists: true` and all injects use `skip_if`), running the same recipe twice with the same variables produces no changes on the second run. JSON output reports `"action": "skip"` with a reason for every skipped operation.

## I-3: JSON In, Files Out

Variables are structured JSON. Output is files on disk. The CLI is non-interactive — no prompts, no wizards, no "press y to continue." An LLM (or script) can call jig without any human in the loop.

## I-4: Structured Errors with Rendered Content

When an operation fails (anchor not found, scope parse fails, file missing), jig still renders the template and includes the rendered content in the error output. The caller never has to re-derive what to insert — only where to put it. Errors include what/where/why + a hint.

## I-5: Stable Exit Codes

Exit codes are API. They do not change between minor versions.

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Recipe validation error |
| 2 | Template rendering error |
| 3 | File operation error |
| 4 | Variable validation error |

## I-6: Single Static Binary, Zero Runtime Dependencies

No Python, no Node, no JVM. One binary, runs anywhere. Startup must be fast — jig may be called dozens of times in a single LLM session.

## I-7: Templates Live With the Consumer

There is no central template directory. Templates are co-located with the recipe that uses them. A recipe + its templates form a self-contained unit that can be moved, versioned, or shared independently.

## I-8: Dual Output Streams

For `jig run`: In TTY mode, human-readable colored output to stderr, nothing to stdout. In piped mode, JSON to stdout, nothing to stderr (except errors). `--json` forces JSON to stdout regardless of TTY. `--quiet` suppresses stderr only; stdout behavior is determined by `--json` independently. `--verbose` adds rendered template content to output. Note: `jig vars` and `jig render` write to stdout by default regardless of TTY mode. `jig render --to <path>` redirects output to a file.

## I-9: Operations Are Ordered

File operations in a recipe execute in declaration order. Later operations can depend on files created or modified by earlier operations in the same recipe run.

## I-10: Graceful Degradation to LLM

jig handles the deterministic part (rendering). When structural analysis fails (can't find anchor, can't parse scope), it fails with enough information for the LLM to fall back to its native Edit tool. jig never silently produces wrong output — it either succeeds or fails clearly.
