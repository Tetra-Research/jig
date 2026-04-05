# PLAN.md

> Workstream: agent-evals
> Last updated: 2026-04-05
> Status: Complete (review findings pending)

## Objective

Build an evaluation harness that measures whether real LLM agents can successfully use jig to make multi-file code changes. The harness runs scenarios (fixture codebase + natural language prompt + expected outcome), invokes agents as subprocesses, scores the results on assertion pass rate and jig usage, and compares against a baseline (same task without jig).

This is the feedback loop that drives jig's ergonomic design decisions. Without it, changes to jig's CLI output, error messages, help text, and MCP tool descriptions are guesswork. With it, they're science.

## Phases

### Phase 1: Harness Skeleton + Scoring Engine
Status: Complete

Build the core infrastructure: scenario loading, sandbox setup, scoring, and result persistence. No agent invocation yet — test with a mock agent script.

#### Milestones
- [x] 1.1: **Scenario parser** — `eval/harness/scenarios.ts` parses scenario YAML, validates required fields, resolves `codebase/` and `expected/` directory paths (FR-1, FR-2)
- [x] 1.2: **Sandbox setup** — `eval/lib/sandbox.ts` copies codebase to temp dir, `git init` + initial commit, ensures `jig` on PATH, cleanup on completion (FR-4)
- [x] 1.3: **Scoring engine** — `eval/harness/score.ts` implements assertion scoring (contains/scope/weight), negative assertion scoring, file correctness scoring (normalize + Jaccard), and jig usage extraction from agent output (FR-5, FR-6, FR-7, FR-13)
- [x] 1.4: **Result persistence** — `eval/harness/results.ts` writes trial results as JSONL, append-only (FR-8)
- [x] 1.5: **Normalize + diff utilities** — `eval/lib/normalize.ts` and `eval/lib/diff.ts` for whitespace normalization and structural file comparison (FR-13)
- [x] 1.6: **Unit tests for scoring** — `eval/harness/test.ts` tests assertion matching, file diff, jig invocation extraction with fixture data

#### Validation Criteria
- Scenario YAML parsing handles all fields from the schema, rejects invalid tiers and missing required fields
- Sandbox creates isolated git-initialized temp dir, cleans up after
- Scoring produces correct assertion_score, file_score, negative_score for hand-crafted test fixtures
- Jig invocation extraction correctly parses `jig run` and `jig workflow` calls from sample agent output
- JSONL write appends without corrupting existing data

### Phase 2: Agent Invocation + Orchestration
Status: Complete

Wire up agent subprocess invocation, the trial loop, and CLI argument parsing.

#### Milestones
- [x] 2.1: **Agent config** — `eval/agents.yaml` + `eval/harness/agents.ts` loads agent configs, invokes via subprocess with timeout, captures stdout/stderr/exitCode/duration (FR-3)
- [x] 2.2: **Trial orchestrator** — `eval/harness/run.ts` iterates scenarios x agents x reps, creates sandbox, invokes agent, scores, writes result (FR-9)
- [x] 2.3: **CLI flags** — `--scenario`, `--agent`, `--reps`, `--tier`, `--dry-run`, `--mode`, `--metrics-only` (FR-9, FR-10)
- [x] 2.4: **Baseline mode** — `eval/harness/baseline.ts` prompt transformation that strips jig references and instructs manual editing (FR-10)
- [x] 2.5: **Progress logging** — one-line stderr output per trial with score and duration (FR-9 AC-9.9)

#### Validation Criteria
- `npx tsx eval/harness/run.ts --dry-run` validates all scenarios and agents, reports what would run
- `npx tsx eval/harness/run.ts --scenario <name> --agent claude-code --reps 1` runs a single trial end-to-end
- Agent timeout kills the process and records a timeout result
- Baseline mode produces a prompt with no jig references

### Phase 3: First Scenarios
Status: Complete

Create the initial set of evaluation scenarios covering the core use cases. Start with easy/medium tiers using jig's existing capabilities (create, inject, replace, patch, workflow).

#### Milestones
- [x] 3.1: **Easy tier: scaffold test file** — `eval/scenarios/scaffold-test/` — agent creates a test file using `jig run` with a create recipe (single file, obvious variables)
- [x] 3.2: **Easy tier: inject import** — `eval/scenarios/inject-import/` — agent injects an import into an existing file using `jig run` with an inject recipe
- [x] 3.3: **Medium tier: add model field** — `eval/scenarios/add-field/` — agent uses a recipe (not workflow, since add-field uses a single multi-op recipe) to add a field to a Django-style model and propagate through admin, serializer, factory (multi-file, variable extraction from existing code)
- [x] 3.4: **Medium tier: add endpoint** — `eval/scenarios/add-endpoint/` — agent uses a recipe to add an API endpoint (view, URL, schema, test)
- [x] 3.5: **Discovery tier: find recipe** — `eval/scenarios/discover-recipe/` — agent must inspect multiple recipes in `codebase/recipes/` to find the right one (uses `jig vars`/`jig validate`, not `jig library recipes` since libraries don't exist yet)
- [x] 3.6: **Error-recovery tier: bad anchor** — `eval/scenarios/error-recovery-bad-anchor/` — scenario includes a file where jig's anchor pattern won't match; agent must recover using rendered content from jig's error output

#### Validation Criteria
- Each scenario has valid `scenario.yaml`, `codebase/`, `expected/`, templates, and recipes
- `--dry-run` validates all scenarios
- Running each scenario with `claude-code` produces a non-zero assertion_score (agent can at least partially complete the task)
- Scenarios cover at least 3 different tiers

### Phase 4: Report Generation + Experiment Loop
Status: Partial (report built, first sweep not yet run)

Build the reporting layer and run the first full evaluation sweep.

#### Milestones
- [x] 4.1: **Report generator** — `eval/harness/report.ts` aggregates results from JSONL, computes per-agent/tier/category breakdowns, identifies weakest scenarios, outputs METRIC lines (FR-11)
- [x] 4.2: **Experiment journal template** — `eval/log/experiments.md` with the standard format (FR-12)
- [ ] 4.3: **First full sweep** — run all scenarios x all agents x 5 reps, both jig and baseline modes, generate report
- [ ] 4.4: **Baseline comparison** — analyze jig vs baseline delta in assertion_score and tool_calls

#### Validation Criteria
- Report includes all sections: overall, by-agent, by-tier, by-category, baseline comparison, weakest scenarios
- METRIC lines are parseable and cover the required keys
- First sweep completes without harness errors (agent failures are expected and recorded)

### Outstanding Review Findings

The code review (2026-04-05) identified issues that were partially addressed but several remain open. These should be resolved before running the first full sweep (4.3).

**Critical (must fix before sweep):**
- Timeout trials still go through normal scoring instead of being forced to zero scores (AC-8.6)
- Scoped assertions: `extractScope` expects bare identifier but scenarios use `"class Reservation"` — scope matching silently falls back to whole-file search (AC-1.11, AC-5.2)
- `by_tier` grouping derives tier from tags instead of from the scenario's tier field
- `by_category` is always empty in reports

**Major (should fix before sweep):**
- `category` not enforced as required field in validation (FR-1)
- `--dry-run` doesn't validate agent config schema (FR-9.7, FR-3.1)
- `scoreJigUsage` doesn't extract invocation exit codes or expose `within_expected_range`/`valid_vars` as distinct metrics (FR-6)
- `tokens_used` and `jig_calls` not stored in trial results (FR-7)
- Report baseline comparison uses `total` not `assertion_score + tool_calls` as spec describes
- Sandbox doesn't ensure agent subprocess PATH includes discovered jig binary (FR-4.3)
- `agents.yaml` uses `--print` but spec says `-p`; `--max-turns 25` vs spec's `50`

## Dependencies

- **Depends on:** `core-engine` (v0.1 complete), `replace-patch` (v0.2 complete), `workflows` (v0.3 complete) — all are done. The eval harness needs a working `jig` binary with all four operations and workflow support.
- **Blocks:** Nothing directly, but results from agent evals will inform changes to jig's CLI surface, error messages, and help text across all other workstreams.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Language for harness | TypeScript (via `npx tsx`) | Async subprocess management, JSON/YAML parsing, no build step needed. Matches the spec's design. |
| No test framework | Node.js built-in `assert` | Minimal dependencies (NFR-3). The harness tests are simple enough for direct assertions. |
| Sequential trial execution | One trial at a time | Avoids resource contention between concurrent agents. LLM API rate limits make parallelism counterproductive. |
| Append-only results | JSONL format | Simple, grep-friendly, no corruption risk from concurrent writes or crashes mid-run. Each line is a complete record. |
| Scoring: assertion-based not full-diff | Weighted structural assertions | LLM output varies in whitespace, ordering, comments. Exact diff is too brittle. Assertions test the structural intent. |
| Scenario fixtures: real code, not mocks | Django-style Python fixtures | Agents need realistic code to exercise jig's scope detection and anchor patterns. Fake code would miss the integration. |
| No jig library system yet | Recipes bundled in scenario dirs | Libraries (v0.4) aren't built yet. Scenarios include their own recipes and templates co-located in the fixture. |
| Baseline uses same assertions | Identical scoring for both modes | Fair comparison requires identical success criteria. Only the prompt and available tools differ. |

## Risks / Open Questions

- **Agent output format variability** — Different agents (Claude Code, Codex) produce output in different formats. Jig invocation extraction (FR-6) must handle multiple output styles. Start with Claude Code's `--output-format json` and add parsers as needed.
- **Cost of full sweeps** — Running 15 scenarios x 2 agents x 5 reps x 2 modes = 300 agent invocations. At ~$0.05/invocation that's ~$15/sweep. Need to be intentional about when to run full sweeps vs. targeted single-scenario tests.
- **Scenario design quality** — Bad scenarios (ambiguous prompts, unrealistic fixtures, overly strict assertions) produce noise, not signal. Plan to iterate on scenario design based on early results. The holdout set concept from the spec is deferred until we have enough scenarios (10+).
- **No jig library system** — The spec envisions scenarios that use `jig library recipes django` for discovery. Since libraries (v0.4) don't exist yet, discovery-tier scenarios must work with recipes in the fixture directory and `jig vars` for introspection. Revisit when libraries land.
- **Agent availability** — Codex CLI may not be available or may require different auth. Start with Claude Code only, add Codex when accessible.
