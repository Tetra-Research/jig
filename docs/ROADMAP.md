# ROADMAP.md

Full delivery plan for jig, bridging the product spec (`jig.md`) to workstream execution. Every feature in the spec is accounted for here — either assigned to a milestone, explicitly deferred, or marked as cross-cutting infrastructure.

Last updated: 2026-04-04

## Current State

| Milestone | Status | Tests | Docs |
|-----------|--------|-------|------|
| v0.1 core-engine | **Done** | 191 | PLAN, SPEC, SHARED-CONTEXT, NARRATIVE |
| v0.2 replace-patch | **Done** | 308 total | PLAN, SPEC, SHARED-CONTEXT, NARRATIVE |
| v0.3 workflows | **Done** | 343 total | PLAN, SPEC, SHARED-CONTEXT, NARRATIVE |
| v0.4–v1.0 | Described in jig.md | — | Nothing beyond one-line mentions in ARCHITECTURE.md |

The engine works. The planning infrastructure works. What's missing is the roadmap connecting the two through the remaining milestones.

---

## Two Tracks

jig has two parallel development tracks with dependencies between them:

```
ENGINE TRACK (the tool itself)          AGENT TRACK (how agents use the tool)
─────────────────────────────           ──────────────────────────────────────
v0.1 core-engine ✅
v0.2 replace-patch ✅
v0.3 workflows ──────────────────────── agent evals (basic scenarios)
v0.4 libraries ──────────────────────── MCP server
v0.5 distribution                       project instructions template
v0.6 Claude Code plugin ◄────────────── requires MCP + libraries
v0.7 scan & check                       agent evals (full scenario suite)
v0.8 infer
v0.9 polyglot / schema-first
v1.0 stable
```

The engine track is sequential — each milestone builds on the previous. The agent track runs in parallel once the engine is usable (after v0.3), and the two converge at v0.6 (Claude Code plugin).

---

## Milestone Details

### v0.3 — Workflows (next up)

**Workstream docs:** Complete (PLAN, SPEC, SHARED-CONTEXT)
**Depends on:** v0.2 (done)
**jig.md reference:** lines 1223–1253, roadmap line 1920

Multi-recipe orchestration. Chain recipes into a single `jig workflow` invocation with conditional steps (`when`), variable mapping (`vars_map`, `vars`), and error handling modes (`stop`, `continue`, `report`).

**Scope (from SPEC):**
- Workflow YAML parsing (distinct from recipe format)
- `jig workflow` CLI command
- Conditional steps via Jinja2 template truthiness
- Variable mapping and overrides between steps
- Three error handling modes
- Per-step result reporting in JSON output
- `jig validate` and `jig vars` auto-detect workflow files
- 25 integration test fixtures

**What this unlocks:** Workflows are the composition primitive. Without them, every multi-file change requires the agent to call `jig run` N times. With them, one `jig workflow` call handles the entire cascade. This is the minimum viable surface for meaningful agent evals.

**Known blockers from v0.2:**
- `Position::Sorted` is a stub (panics) — any workflow step using it will fail
- `write_back` in replace.rs/patch.rs may swallow I/O errors
- 11 of 26 spec-required integration fixtures still missing
- `cmd_run` needs refactoring to extract reusable `run_recipe()` for workflow steps

---

### v0.4 — Libraries

**Workstream docs:** None (needs PLAN + SPEC)
**Depends on:** v0.3 (workflows reference library recipes)
**jig.md reference:** lines 1059–1376, roadmap line 1929

Libraries are versioned recipe collections for a framework. This is where framework opinions live — jig itself stays agnostic.

**Scope (from jig.md):**

*Library manifest:*
- `jig-library.yaml` format: name, version, description, framework, language
- `conventions` block: path templates mapping concerns to file locations
- `recipes` block: flat list of recipe paths + descriptions
- `workflows` block: multi-recipe workflow definitions (uses v0.3 workflow format)

*CLI commands:*
- `jig library add <url|path>` — install from git repo or local directory
- `jig library remove <name>` — uninstall
- `jig library update <name>` — pull latest
- `jig library list` — show installed libraries
- `jig library recipes <name>` — list all recipes in a library
- `jig library info <name>/<recipe>` — show recipe details
- `jig library workflows <name>` — list workflows

*Installation:*
- Global: `~/.jig/libraries/`
- Project-local: `.jig/libraries/` (takes precedence)

*Convention overrides:*
- `.jigrc.yaml` per-project convention remapping
- Override just the path patterns, keep everything else

*Project extensions:*
- `.jig/overrides/<library>/<recipe>/templates/` — replace a single template without forking
- `.jig/extensions/<library>/<recipe>/` — add new recipes namespaced under a library

**What this unlocks:** Libraries make jig useful beyond a single project. They're also a prerequisite for the MCP server (which needs `jig_library_recipes` to work), the Claude Code plugin (which wraps a library), and meaningful agent evals (which need a library like jig-django to test against).

**Open questions for SPEC:**
- Does `jig library add` from git clone the whole repo or just the jig-library.yaml + recipe dirs?
- Versioning strategy: git tags? semver in manifest? lock file?
- How does `jig run` resolve a library recipe path (e.g., `jig run django/model/add-field`)? Is it `jig run --library django model/add-field` or path-based?
- Does `jig workflow` accept library-qualified workflow names directly?

---

### v0.5 — Distribution

**Workstream docs:** None (needs PLAN)
**Depends on:** v0.4 (distribution should include library install capability)
**jig.md reference:** lines 806–950, roadmap line 1937

Package and ship jig so others can install it.

**Scope (from jig.md):**

| Channel | Mechanism |
|---------|-----------|
| GitHub Releases | Cross-compiled binaries (macOS arm64/x86_64, Linux arm64/x86_64/musl, Windows x86_64) |
| Homebrew | Tap first (`brew tap <org>/tools`), graduate to homebrew-core at 50+ stars |
| Cargo | `cargo install jig-cli` on crates.io |
| Nix | Flake with `buildRustPackage` |
| npm | Thin binary wrapper (`npx @<org>/jig`), platform-specific postinstall (esbuild pattern) |
| Shell installer | `curl -fsSL .../install.sh \| sh` — detects platform, downloads binary |

*CI/CD:*
- GitHub Actions workflow for cross-compilation
- Release automation on git tag push
- Binary signing (optional)
- SHA256 checksums in release notes

**What this unlocks:** External users. Everything before v0.5 is author-only usage. This is also when the npm wrapper becomes available, which matters because the MCP server ships as `npx @jig/mcp-server` and needs to find the `jig` binary.

**Open questions for PLAN:**
- Minimum viable distribution: just GitHub Releases + Homebrew tap? Or all channels at once?
- Does the npm wrapper ship in this milestone or with the MCP server?

---

### v0.6 — Claude Code Plugin

**Workstream docs:** None (needs PLAN + SPEC)
**Depends on:** v0.4 (libraries), MCP server (agent track)
**jig.md reference:** lines 650–730, 1307–1376, roadmap line 1946

The deepest integration: jig as a Claude Code plugin bundling MCP tools, skills, and hooks.

**Scope (from jig.md):**

*jig plugin itself:*
- `.claude-plugin/plugin.json` manifest
- `/jig:init` skill — scaffold recipe + templates dir inside a skill
- `/jig:doctor` skill — validate all recipes in a plugin

*jig-django as reference library + plugin:*
- First community library: recipes for model, service, view, schema, admin, test, factory
- Workflows: `add-field`, `add-endpoint`, `scaffold-resource`
- Claude Code skills wrapping each workflow: read code → extract variables → call jig
- Dual-publish: installable as `jig library add` AND as a Claude Code plugin

*Skill structure (from jig.md):*
```
jig-django/
  jig-library.yaml              # library manifest
  .claude-plugin/plugin.json    # Claude Code plugin manifest
  skills/
    add-field/SKILL.md           # reads model, extracts context, runs jig workflow
    add-endpoint/SKILL.md
    scaffold-resource/SKILL.md
  model/add-field/recipe.yaml    # actual recipes
  service/add-method/recipe.yaml
  ...
```

**What this unlocks:** The full vision — a developer says "add a loyalty_tier field to Reservation" and gets a consistent, multi-file, team-compliant change. The skill handles the intelligence (reading code, extracting variables), jig handles the mechanics (rendering, patching).

**Open questions for SPEC:**
- Does the jig plugin provide the MCP server, or is it registered separately?
- Hook integration: what triggers `jig check` automatically? Post-edit on model files?
- How does dual-publish versioning work? Library version matches plugin version?

---

### v0.7 — Scan & Check

**Workstream docs:** None (needs PLAN + SPEC)
**Depends on:** v0.4 (needs library recipes to scan/check against)
**jig.md reference:** lines 1435–1720, roadmap line 1961

Reverse operations that close the loop: instead of variables → files, go files → variables (scan) and files → conformance report (check).

**Scope (from jig.md):**

*`jig scan`:*
- Reverse a recipe: given an existing file, extract the variables that would have produced it
- File-level scan: `jig scan django/model ./hotels/models/reservation.py`
- Directory-level scan: `jig scan django ./hotels/` — project map showing coverage
- Output: JSON with `variables`, `confidence`, `unrecognized` (lines the recipe can't explain)
- Enables the workflow: scan → LLM modifies variables → run recipe

*`jig check`:*
- Conformance verification: `jig check django/model ./hotels/models/*.py`
- Issue reporting with severity levels (error, warn)
- `fix_recipe` references: when an issue has a known recipe fix, jig tells you
- Directory-level checking across all library concerns
- `--strict` and `--exit-on-failure` flags for CI gating
- JSON output for LLM consumption

**What this unlocks:** Scan → Check → Fix is a self-healing loop. The LLM can run `jig check`, read the issues, and automatically invoke the fix recipes. Check as CI gate turns team conventions into enforceable rules — the recipe IS the source of truth.

**Open questions for SPEC:**
- Scan algorithm: template reverse-matching is complex. What's the minimum viable approach? Regex extraction from anchor patterns? Or full template inversion?
- Confidence scoring: what's the threshold for "matched"?
- How does scan handle files that deviate significantly from the recipe?

---

### v0.8 — Infer

**Workstream docs:** None (needs PLAN + SPEC)
**Depends on:** v0.7 (scan provides the foundation for pattern recognition)
**jig.md reference:** lines 1526–1625, roadmap line 1968

Learn recipes from examples instead of writing them by hand.

**Scope (from jig.md):**
- `jig infer --before <file> --after <file>` — learn from a single before/after pair
- `jig infer --example <file>:before,after` (multiple) — variable detection improves with more examples
- `jig infer --from-git --pattern "Add * field to *"` — learn from commit history
- `jig infer --from-commit <sha>` — learn multi-file workflow from a single commit
- Draft/review/promote workflow: inferred recipes go to `_drafts/`, reviewed, then promoted
- Inferred recipes are expected to be 70-90% correct (the last 10-30% is human judgment)

**What this unlocks:** The biggest friction reduction. Instead of hand-writing recipes, developers (or the LLM) infer them from the team's actual behavior. Combined with the observation engine (post-1.0), this makes recipe creation nearly zero-effort.

---

### v0.9 — Polyglot & Schema-First

**Workstream docs:** None (needs PLAN + SPEC)
**Depends on:** v0.4 (libraries), v0.3 (workflows)
**jig.md reference:** lines 1722–1851, roadmap line 1975

Cross the framework boundary. A single command touches Django backend + Vue frontend.

**Scope (from jig.md):**
- Cross-library workflows: steps reference different libraries (`library: django`, `library: vue`)
- `jig from-schema openapi|sql|proto|graphql` command: derive variables from schema definitions
- Type mapping definitions in library manifests (`openapi_to_django`, `openapi_to_typescript`)
- Schema-to-variables resolution: read the spec, resolve types, pass to recipes

---

### v1.0 — Stable

**Workstream docs:** None
**Depends on:** All previous milestones at a quality bar
**jig.md reference:** roadmap line 1981

Semver stability guarantees on:
- Recipe YAML format
- Anchor/scope system
- Library manifest format
- CLI interface and JSON output
- Exit codes

Also: comprehensive documentation site, homebrew-core publication, 3+ community libraries.

---

## Agent Track

These are not milestones in the engine sense — they're infrastructure that develops in parallel with the engine track.

### MCP Server

**Workstream docs:** None (needs PLAN)
**Can start after:** v0.2 (core engine exists), but full value requires v0.4 (libraries)
**jig.md reference:** lines 2042–2243

A thin stdio wrapper (~100-200 lines) that exposes jig commands as typed MCP tools. Agents get automatic tool discovery instead of parsing `--help`.

**Architecture:**
```
Agent (Claude Code, Codex, Cursor, etc.)
  ↕ MCP stdio transport (JSON-RPC over stdin/stdout)
jig-mcp-server
  ↕ spawns subprocess
jig CLI binary
```

**6 tools defined in jig.md:**

| MCP Tool | Maps to CLI |
|----------|------------|
| `jig_run` | `jig run <recipe> --vars '<json>' --json` |
| `jig_vars` | `jig vars <recipe>` |
| `jig_scan` | `jig scan <recipe> <path> --json` |
| `jig_check` | `jig check <recipe> <path> --json` |
| `jig_workflow` | `jig workflow <workflow> --vars '<json>' --json` |
| `jig_library_recipes` | `jig library recipes <library> --json` |

**Implementation options (from jig.md):**
1. TypeScript via `@modelcontextprotocol/sdk` (~100 lines, distributed via npm as `npx @jig/mcp-server`) — most pragmatic
2. Rust binary alongside jig (~200 lines) — shares release pipeline
3. Python via `mcp` SDK — minimal code

**Phasing:**
- Phase 1 (after v0.3): `jig_run`, `jig_vars`, `jig_workflow` — enough for agents to use existing recipes
- Phase 2 (after v0.4): add `jig_library_recipes` — agents can discover library recipes
- Phase 3 (after v0.7): add `jig_scan`, `jig_check` — full tool surface

**Per-tool config (from jig.md):**
- Claude Code: `.mcp.json` or `~/.claude/settings.json`
- Codex: `~/.codex/config.toml`
- Cursor: `.cursor/mcp.json`
- Windsurf: `~/.codeium/windsurf/mcp_config.json`

**Open questions:**
- Ship as separate npm package or bundled with jig binary?
- Version independently or lock to jig version?
- Does Phase 1 ship with v0.3 or v0.4?

---

### Project Instructions Template

**Workstream docs:** None (trivial — just a markdown snippet)
**Can start after:** v0.3 (when `jig workflow` exists)
**jig.md reference:** lines 2244–2278

A CLAUDE.md / AGENTS.md / .cursorrules snippet telling agents about available jig commands:

```markdown
## Code Generation with jig

This project uses jig for template-based code generation. When creating or extending
models, services, views, or tests, prefer jig recipes over manual code generation.

Available workflows:
- `jig workflow django/add-field --vars '...'`
- `jig workflow django/add-endpoint --vars '...'`

To see what variables a recipe needs: `jig vars <recipe>`
Always use --json flag and review the output before proceeding.
```

This costs nothing, works immediately with 6/11 agentic tools, and doesn't need its own workstream. Ship it as part of v0.6 (plugin) or earlier as a docs artifact.

---

### Agent Eval System

**Workstream docs:** None (needs PLAN + SPEC — this is substantial infrastructure)
**Can start after:** v0.3 (needs workflows for meaningful scenarios), full value at v0.4+ (needs libraries)
**jig.md reference:** lines 2315–2904 (590 lines — the most detailed section in the spec)

The eval system measures whether agents can actually *use* jig. It's the scientific method applied to CLI ergonomics.

**What it tests (axis 2 — agent usability, not mechanical correctness):**
- Can agents discover the right recipe?
- Can agents extract variables from existing code?
- Do agents construct valid `--vars` JSON?
- Do agents recover from jig errors?
- Do agents know when to fall back to manual editing?
- How does success vary across agent models?

**Architecture (from jig.md):**
```
eval/
  scenarios/          # fixture codebases + prompts + assertions
  harness/            # TypeScript: run.ts, agents.ts, score.ts, report.ts
  results/            # results.jsonl (append-only trial log)
  log/                # experiments.md (hypothesis journal)
  lib/                # sandbox.ts, diff.ts, normalize.ts
```

**Scoring dimensions:**
1. **Assertion pass rate** (primary) — weighted structural checks, 0-1
2. **File correctness** — structural diff with Jaccard fallback, 0-1
3. **Negative assertions** — no syntax errors / duplicates, binary 0 or 1
4. **Jig usage** — did the agent use jig or bypass it?
5. **Efficiency** — tool calls, tokens, wall-clock time

**Key design features:**
- Baseline comparison: same scenarios with jig vs. without jig (isolates tool value)
- Experiment loop: hypothesis → change → run → score → log → decide
- Holdout set: training scenarios (iterated freely) vs. holdout scenarios (periodic generalization check)
- Scenario tiers: easy, medium, hard, discovery, error-recovery
- MCP vs. CLI mode comparison
- LLM-as-judge for soft criteria
- Cost tracking per trial

**Phasing:**

| Phase | When | Scope |
|-------|------|-------|
| Eval-1 | After v0.3 | Harness skeleton + 3-5 basic scenarios (single-recipe, no library). Tests: can agents call `jig run` and `jig workflow` correctly? Baseline comparison. |
| Eval-2 | After v0.4 | Add library scenarios (10-15 total). Tests: recipe discovery, variable extraction from existing code, workflow invocation. Full scoring. |
| Eval-3 | After v0.6 | Add MCP mode, plugin scenarios, discovery tier. Holdout set. Full experiment loop operational. |
| Eval-4 | After v0.7 | Add scan/check scenarios. Error-recovery tier. The eval now covers the full tool surface. |

**Why start early:** The eval system doesn't just measure — it drives design decisions. Discovering that agents fail to construct `--vars` JSON correctly *before* v1.0 means we can fix it. Discovering it after v1.0 means we're stuck with a semver-locked interface.

**Open questions for PLAN:**
- TypeScript is specified for the harness. Confirm this vs. Rust/Python.
- Where do fixture codebases come from? Hand-written? Extracted from real projects?
- Agent cost budget per eval run. At $0.04/trial, 450 trials = ~$18/run.

---

### Observation Engine (post-1.0)

**jig.md reference:** lines 1852–1897

A Claude Code hook that logs edit patterns across sessions and proposes recipes from repeated behavior. `/jig:discover` analyzes the log and suggests what to automate.

Not on the critical path. Depends on v0.8 (infer) to be useful — the observation engine detects patterns, infer creates the recipe drafts.

---

## Cross-Cutting Concerns

These don't belong to a single milestone but span multiple:

### `.jigrc.yaml` Configuration

**jig.md reference:** lines 1023–1055

Project-wide defaults: `base_dir`, `vars_file`, custom shell filters. Convention overrides for libraries land in the same file.

| Feature | Lands in |
|---------|----------|
| `base_dir`, `vars_file` defaults | v0.3 or v0.4 (whenever config file loading is added) |
| Convention overrides (`libraries.django.conventions`) | v0.4 (libraries) |
| Custom filters via shell commands | Post-1.0 or v0.4 (low priority) |

### `includes` → Workflows Migration

**jig.md reference:** lines 1246–1252

The spec explicitly states that the `includes` concept from the patches section is "subsumed by workflows" and removed. Workflows are the composition primitive. No `includes` support should be built.

### Verbose Scope Diagnostics

**Spec reference:** v0.2 SPEC, currently incomplete

`--verbose` showing scope boundaries, insertion points, and indentation detection is spec'd in v0.2 but implementation is partial. Should be completed as tech debt before v0.3 exercises it through workflows.

---

## Dependency Graph

```
v0.1 ✅ ─► v0.2 ✅ ─► v0.3 workflows ─► v0.4 libraries ─► v0.5 distribution
                           │                   │                  │
                           │                   ├─► MCP server ────┤
                           │                   │                  │
                           ├─► eval (basic) ───┤                  │
                           │                   │                  │
                           │                   ├─► eval (full) ◄──┤
                           │                   │                  │
                           │                   └─► v0.6 plugin ◄──┘
                           │                         │
                           │                   v0.7 scan/check ─► v0.8 infer
                           │                         │
                           │                   eval (complete) ◄──┘
                           │
                           └─► project instructions template (trivial)

v0.9 polyglot ◄── v0.4 libraries + v0.3 workflows
v1.0 stable ◄── everything above at quality bar
```

Key dependencies:
- **MCP server** needs v0.4 (libraries) for `jig_library_recipes` tool, but Phase 1 (`jig_run`, `jig_vars`, `jig_workflow`) can ship with v0.3
- **Agent evals** need v0.3 (workflows) minimum, v0.4 (libraries) for meaningful scenarios
- **Plugin (v0.6)** needs MCP server + v0.4 libraries + first library (jig-django)
- **Scan/Check (v0.7)** needs v0.4 libraries to know what to scan against
- **Infer (v0.8)** needs v0.7 scan as foundation
- **Distribution (v0.5)** is technically independent but should ship after v0.4 so the distributed binary includes library support

---

## What Needs Docs Before Work Starts

| Next workstream | Needs | Priority |
|----------------|-------|----------|
| v0.3 workflows | **Ready.** PLAN + SPEC exist. | Now |
| Agent evals (basic) | PLAN needed. Scope: harness skeleton, 3-5 scenarios, scoring, baseline comparison. | After v0.3 starts |
| MCP server (Phase 1) | PLAN needed. Scope: TypeScript stdio wrapper, 3 tools (run, vars, workflow). | After v0.3 done |
| v0.4 libraries | PLAN + SPEC needed. Most complex remaining spec work — manifest format, install logic, convention overrides, extensions. | After v0.3 done |
| v0.5 distribution | PLAN needed. Mostly CI/packaging, not code design. | After v0.4 done |
| v0.6 plugin | PLAN + SPEC needed. Depends on MCP + libraries existing first. | After v0.4 + MCP done |
| v0.7 scan/check | PLAN + SPEC needed. Algorithm design for template inversion. | After v0.4 done |

---

## Post-1.0 Features (Captured, Not Planned)

From jig.md, for completeness. None of these have workstream docs or timeline commitments:

| Feature | jig.md ref | Notes |
|---------|-----------|-------|
| Observation engine | lines 1852–1897 | Claude Code hook + `/jig:discover` skill |
| Recipe hooks (`pre_run`, `post_run`) | line 1991 | Run formatters/linters on generated files |
| Interactive mode | line 1992 | Human-prompted variable input (low priority) |
| Watch mode | line 1993 | Re-render on template change (authoring tool) |
| Tree-sitter integration | line 1994 | Optional AST-aware scoping (precision upgrade) |
| Library registry | line 1995 | Searchable index like crates.io for jig libraries |
| Diff preview (`--diff`) | line 1996 | Unified diff output instead of writing files |
| Undo (`jig undo`) | line 1997 | Revert last run via `.jig/history/` log |
| Template linting (`jig lint`) | line 1998 | Catch unused variables, unreachable conditionals, missing skip_if |
