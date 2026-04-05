# SHARED-CONTEXT.md

> Workstream: agent-evals
> Last updated: 2026-04-04

## Purpose

Build an evaluation harness that tests whether LLM agents can successfully use jig to make multi-file code changes. This is the feedback loop that turns "is jig easy for agents to use?" into a measurable number, driving design decisions about CLI output, error messages, help text, and recipe naming.

## Current State

- Initialized (2026-04-04)
- jig v0.3 complete (343 tests passing) — all four operations + workflows available
- No `eval/` directory exists yet
- No scenarios, harness code, or results exist

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

## Patterns Established

(None yet — to be populated during implementation)

## Known Issues / Tech Debt

- **No holdout set** — The spec recommends a holdout set of scenarios to prevent overfitting jig's ergonomics to training scenarios. Deferred until we have 10+ scenarios (need enough for a meaningful split).
- **No LLM-as-judge** — The spec describes LLM-as-judge scoring for soft criteria (code style, unnecessary changes). Deferred to a later iteration. Structural assertions are sufficient for the initial eval loop.
- **No MCP mode** — The spec describes running scenarios in MCP mode (agents use MCP server instead of CLI). Deferred until the MCP server exists.
- **No cost tracking** — Token usage extraction from agent output is best-effort. Claude Code's JSON output format may not include token counts in all configurations.

## File Ownership

This workstream owns everything under `eval/`:

```
eval/
  scenarios/                    # test scenarios (fixture codebases + prompts + expected outcomes)
    <scenario-name>/
      scenario.yaml             # scenario definition
      codebase/                 # fixture files (before state)
      expected/                 # expected files (after state)
  harness/
    run.ts                      # CLI entry point and trial orchestrator
    scenarios.ts                # scenario YAML loading and validation
    agents.ts                   # agent configuration and subprocess invocation
    score.ts                    # scoring engine (assertions, file diff, jig usage)
    report.ts                   # result aggregation and report generation
    results.ts                  # JSONL result persistence
    test.ts                     # unit tests for harness components
    test-fixtures/              # test data for harness unit tests
  lib/
    sandbox.ts                  # temp directory setup, codebase copying, cleanup
    diff.ts                     # structural file comparison
    normalize.ts                # whitespace and import order normalization
  agents.yaml                   # agent configurations
  results/
    results.jsonl               # append-only trial log
  log/
    experiments.md              # hypothesis → change → result → surprise journal
```

No files outside `eval/` are owned or modified by this workstream.
