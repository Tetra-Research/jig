# SHARED-CONTEXT.md

> Workstream: agent-evals
> Last updated: 2026-04-05

## Purpose

Build an evaluation harness that tests whether LLM agents can successfully use jig to make multi-file code changes. This is the feedback loop that turns "is jig easy for agents to use?" into a measurable number, driving design decisions about CLI output, error messages, help text, and recipe naming.

## Current State

- Harness implemented (2026-04-05) — all TypeScript code, 6 scenarios, test suite passing
- jig v0.3 complete (343 tests passing) — all four operations + workflows available
- Review complete (2026-04-05) — critical and major findings documented, partial fixes applied
- First full sweep not yet run (pending review finding fixes)
- 655-line test suite covers scenario loading, scoring, sandbox, agents, baseline, reporting

## Decisions Made

### D-1: TypeScript harness, not Rust
The eval harness is TypeScript (run via `npx tsx`), not Rust. Rationale: the harness is glue code (subprocess management, YAML/JSON parsing, file diffing) not performance-critical code. TypeScript is faster to iterate on, has native async subprocess support, and avoids coupling the eval system to jig's build.

### D-2: Black-box CLI testing only
The harness invokes `jig` as a compiled binary via subprocess, the same way an agent would. It does not import Rust modules or link against jig's library. This ensures the eval tests the actual agent experience, not an internal API.

### D-3: Assertion-based scoring over exact diff
Scoring uses weighted structural assertions ("does file X contain string Y within scope Z?") rather than exact file diffing. LLM output varies in whitespace, comments, import ordering, and argument order. Assertions test the structural intent; exact diff is too brittle. Jaccard file similarity is computed as a secondary metric but not the primary score.

### D-4: Sequential trial execution
Trials run one at a time, not in parallel. LLM API rate limits and resource contention make parallelism counterproductive. A full sweep of 300 trials takes ~1-2 hours at sequential pace, which is acceptable for an overnight eval run.

### D-5: Recipes co-located in scenario fixtures
Since jig libraries (v0.4) don't exist yet, each scenario includes its own recipes and templates inside the fixture's `codebase/` directory. The agent discovers them via `jig vars` and `jig validate`, not `jig library recipes`. This decision will be revisited when libraries land.

### D-6: JSONL for results persistence
Trial results are appended as JSON lines to `eval/results/results.jsonl`. JSONL is simple, grep-friendly, and crash-safe (each line is a complete record). No database, no structured file format that requires atomic writes.

### D-7: Start with Claude Code agents only
The initial implementation targets Claude Code (opus and sonnet) as the only agents. Codex CLI support is deferred until agent availability is confirmed. The agent config system is designed to support arbitrary CLI agents, so adding new agents is a config change, not a code change.

### D-8: Sandbox jig detection via PATH probing
The sandbox tries `which jig`, then `target/release/jig`, then `target/debug/jig` from the project root. This avoids requiring jig to be globally installed during development. However, the discovered path is not injected into the agent subprocess PATH — a gap flagged in review (FR-4.3).

### D-9: Shared types module
All TypeScript interfaces live in `eval/harness/types.ts` — a single source of truth for Scenario, TrialResult, TrialScore, AgentConfig, etc. Harness modules import from types.ts rather than defining their own.

### D-10: Composite score = assertion_score * negative_score
File score is computed and stored but intentionally excluded from the composite total. Rationale: file score (Jaccard similarity) penalizes cosmetically different but functionally correct output, while assertions test structural intent. The total score is the headline number; file_score is a secondary diagnostic.

### D-11: Agent config uses --print not -p
`agents.yaml` uses `--print` (the long form) instead of `-p` from the spec. Both are equivalent for Claude Code. `--max-turns` was set to 25 instead of the spec's 50 — this was a cost-conscious implementation choice, may need adjustment based on sweep results.

## Patterns Established

### P-1: Scenario as self-contained directory
Each scenario is a directory containing `scenario.yaml`, `codebase/`, `expected/`, and recipes under `codebase/recipes/`. No external dependencies. Adding a scenario means adding a directory.

### P-2: Indentation-based scope extraction for Python
`score.ts:extractScope` uses indentation heuristics to extract class/function bodies from Python files. It finds the anchor line (e.g., `class Reservation`), then collects all subsequent lines with deeper indentation. This is lightweight but fragile — review found it doesn't handle the `"class Reservation"` format used in scenario assertions (expects bare identifier).

### P-3: Deterministic scenario ordering
`loadAllScenarios` returns scenarios sorted lexicographically by directory name. This ensures trial order is reproducible across runs (NFR-2).

### P-4: Baseline prompt transformation via regex stripping
`baseline.ts` strips lines matching jig-related patterns (jig run, jig workflow, recipe references, --vars) and prepends a "use native tools only" context. Simple regex approach — effective but may be too aggressive or too lenient for some prompts.

### P-5: Test suite uses Node.js assert, no framework
The 655-line `test.ts` uses built-in `assert` with a hand-rolled runner (named test functions, try/catch, pass/fail counting). Matches NFR-3 (minimal dependencies). Tests cover all harness modules.

## Known Issues / Tech Debt

- **Scope extraction fragile** — `extractScope` in `score.ts` expects bare identifiers but scenario assertions use `"class Reservation"` format. Scope matching silently falls back to whole-file search, producing false-positive assertion passes. Must fix before first sweep.
- **Timeout scoring not zeroed** — Timed-out trials go through normal scoring instead of being forced to zero (AC-8.6 violation). If the agent made partial changes before timeout, scores may be misleadingly non-zero.
- **Tier/category not in TrialResult** — `by_tier` grouping in reports derives tier from tags (unreliable). `by_category` is always empty. The TrialResult type needs explicit tier and category fields populated from the scenario.
- **Report reads all historical results** — End-of-run report aggregates all JSONL results, not just the current execution. This can skew interpretation when comparing against prior runs.
- **No holdout set** — The spec recommends a holdout set of scenarios to prevent overfitting jig's ergonomics to training scenarios. Deferred until we have 10+ scenarios (need enough for a meaningful split).
- **No LLM-as-judge** — The spec describes LLM-as-judge scoring for soft criteria (code style, unnecessary changes). Deferred to a later iteration. Structural assertions are sufficient for the initial eval loop.
- **No MCP mode** — The spec describes running scenarios in MCP mode (agents use MCP server instead of CLI). Deferred until the MCP server exists.
- **No cost tracking** — Token usage extraction from agent output is best-effort. Claude Code's JSON output format may not include token counts in all configurations.
- **Jig binary not on agent PATH** — Sandbox detects jig binary location but doesn't inject it into the agent subprocess PATH. Agent must find jig independently.
- **Scenario parse failures can abort run** — A malformed `scenario.yaml` throws instead of log-and-skip, which can halt the entire harness instead of gracefully skipping one scenario.

## File Ownership

This workstream owns everything under `eval/`:

```
eval/
  scenarios/                    # 6 scenarios across 4 tiers
    scaffold-test/              # easy: create test file
    inject-import/              # easy: inject import statement
    add-field/                  # medium: add model field + propagate
    add-endpoint/               # medium: add API endpoint (view, URL, schema, test)
    discover-recipe/            # discovery: find correct recipe from multiple options
    error-recovery-bad-anchor/  # error-recovery: recover from failed anchor match
  harness/
    types.ts                    # shared TypeScript interfaces
    run.ts                      # CLI entry point and trial orchestrator
    scenarios.ts                # scenario YAML loading and validation
    agents.ts                   # agent configuration and subprocess invocation
    score.ts                    # scoring engine (assertions, file diff, jig usage)
    baseline.ts                 # baseline mode prompt transformation
    report.ts                   # result aggregation and report generation
    results.ts                  # JSONL result persistence
    test.ts                     # unit tests for harness components (655 lines)
    test-fixtures/              # test data for harness unit tests
  lib/
    sandbox.ts                  # temp directory setup, codebase copying, cleanup
    diff.ts                     # structural file comparison (Jaccard)
    normalize.ts                # whitespace and line ending normalization
  agents.yaml                   # agent configurations (claude-code, claude-code-sonnet)
  package.json                  # dependencies: yaml, tsx
  tsconfig.json                 # TypeScript config (strict, ES2022, ESNext modules)
  results/
    results.jsonl               # append-only trial log (not yet populated)
  log/
    experiments.md              # hypothesis → change → result → surprise journal
```

No files outside `eval/` are owned or modified by this workstream.
