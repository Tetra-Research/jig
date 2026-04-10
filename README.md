# jig

Deterministic file generation for LLM-native coding workflows.

`jig` is an execution layer for agentic coding loops, not a generic scaffolding generator. The model reads code, extracts variables, and decides intent; `jig` applies reproducible file operations (`create`, `inject`, `replace`, `patch`) so the mechanical edits are deterministic across retries.

Traditional templating tools usually assume human-interactive generation and central template catalogs. `jig` is designed for skill-local ownership: put recipes and workflows directly in the skill that uses them. A single skill can own multiple recipes plus a multi-step workflow.

## Why We Built Jig

LLMs are strong at reasoning about intent and weak at repeating the same structural edits consistently in existing codebases. We built `jig` to split those responsibilities cleanly:

- LLM: understand code, choose workflow, extract variables, handle novel edge cases.
- `jig`: render deterministic edits, apply idempotent operations, return structured failures an LLM can recover from.

## Why This Is Not Just Another Templating Library

| Dimension | Traditional Templating Libraries | jig |
|---|---|---|
| Primary target | Human-driven scaffolding | LLM-driven write/edit loops |
| Typical scope | New-project bootstrap | Brownfield multi-file edits + scaffolding |
| Input model | Interactive prompts / ad-hoc config | Structured JSON variables |
| Template ownership | Centralized generator registry | Skill-local recipes/workflows |
| Retry behavior | Often re-renders blindly | Idempotent operations (`skip_if`, `skip_if_exists`) |
| Error contract | Human logs | Machine-parseable `what`/`where`/`why`/`hint` + deterministic exit codes |
| Composition | One generator at a time | Multi-step workflows with per-step control |

## Success Criteria (How We Judge Value)

- Agents can discover the right recipe/workflow under realistic prompts.
- Once `jig` is chosen, multi-file correctness is high and repeatable.
- Structured output is parseable for harness scoring and archive analysis.
- For non-trivial workflows, tool calls/tokens/cost trend down versus manual editing.

## Install (Manual Release Channel)

Latest release:

```bash
curl -fsSL https://raw.githubusercontent.com/Tetra-Research/jig/main/install.sh | sh
```

Pin a version:

```bash
curl -fsSL https://raw.githubusercontent.com/Tetra-Research/jig/main/install.sh | sh -s -- --version v0.1.0
```

Installer defaults:

- Installs to `~/.local/bin` (override with `--install-dir` or `JIG_INSTALL_DIR`)
- Pulls binaries from GitHub Releases in `Tetra-Research/jig`
- Verifies signed `SHA256SUMS`, then verifies artifact checksum

Release process details: [`docs/RELEASE-MANUAL.md`](docs/RELEASE-MANUAL.md).

## Quick Start

Minimal recipe:

```yaml
name: add-test
variables:
  module:
    type: string
    required: true
  class_name:
    type: string
    required: true

files:
  - template: test.py.j2
    to: "tests/{{ module | replace('.', '/') }}/test_{{ class_name | snakecase }}.py"
```

Run:

```bash
jig run recipe.yaml --vars '{"module":"app.services.core_service","class_name":"CoreService"}'
```

Same recipe + same variables + same file state yields the same output.

## Agent Integration

`jig` ships bundled skills that teach coding agents how to author recipes and workflows. Install them with a single command:

```bash
jig agent install claude
jig agent install codex
jig agent install opencode
```

This copies skills into the agent's skill directory (e.g., `.claude/skills/`, `.codex/skills/`). If the repo has only one agent marker, the agent argument is optional — `jig` infers it.

Bundled skills:

- **create-recipe** — guides the agent through designing variables, choosing operations, writing anchors/templates, and validating a new recipe.
- **create-workflow** — guides the agent through chaining multiple recipes into a multi-step workflow with conditional execution and variable mapping.

Manage installed skills:

```bash
jig agent list                # show bundled skills
jig agent update [agent]      # replace installed skills with the current bundle
jig agent remove [agent]      # remove jig-managed skills
jig agent doctor [agent]      # inspect install state, version drift, missing/extra skills
```

Options: `--to <path>` targets a different project root; `--force` replaces existing jig-managed installs.

## Examples

The repo includes self-contained examples under [`examples/`](examples/README.md).

Python:

- [`add-service-test`](examples/add-service-test/README.md) — deterministic pytest scaffolding with stable inputs and autospec mocks
- [`structured-logging-contract`](examples/structured-logging-contract/README.md) — consistent start/done logging across service functions
- [`view-contract-enforcer`](examples/view-contract-enforcer/README.md) — request/response schema, view, URL wiring, and tests in one shot
- [`query-layer-discipline`](examples/query-layer-discipline/README.md) — read paths through QuerySet + Manager + selector layers
- [`schema-migration-safety`](examples/schema-migration-safety/README.md) — two-step migrations (add nullable + backfill, then enforce)

TypeScript:

- [`typescript-create-model`](examples/typescript-create-model) — Zod schema + inferred TypeScript type
- [`typescript-update-model`](examples/typescript-update-model) — add fields to an existing Zod schema
- [`typescript-create-endpoint-workflow`](examples/typescript-create-endpoint-workflow) — multi-step workflow: schema, handler, import, route registration
- [`typescript-update-endpoint-workflow`](examples/typescript-update-endpoint-workflow) — add an update handler alongside an existing create handler

Each example includes:

- a runnable `recipe.yaml` (or `workflow.yaml`)
- a concrete `vars.json`
- `before/` and `after/` file trees
- the templates used by the recipe

## Workflows

Workflows chain multiple recipes into a sequential pipeline. Each step runs one recipe, optionally gated by a condition:

```yaml
name: create-endpoint
variables:
  route_name:
    type: string
    required: true
  route_path:
    type: string
    required: true

steps:
  - recipe: schema/recipe.yaml
  - recipe: handler/recipe.yaml
  - recipe: import/recipe.yaml
  - recipe: register/recipe.yaml
    when: "{{ register_route }}"
    on_error: continue
```

Run:

```bash
jig workflow workflow.yaml --vars '{"route_name":"projects","route_path":"/projects"}'
```

Steps share the workflow's variable context. Use `vars_map` to rename variables between workflow and recipe scopes, and `on_error: stop | continue | report` to control failure handling.

## Libraries

Libraries are versioned collections of recipes and workflows for a framework or stack:

```bash
jig library add path/to/library      # install from local path
jig library add https://...git       # install from git
jig library list                      # show installed libraries
jig library recipes <library>         # list recipes in a library
jig library info <library>/<recipe>   # show recipe details (variables, operations)
jig library workflows <library>       # list workflows in a library
jig library update <library>          # reinstall from source
jig library remove <library>          # uninstall
```

Run a library recipe with qualified syntax:

```bash
jig run django/model/add-field --vars '{"app":"core","field_name":"created_at"}'
```

Libraries support conventions (default variable values per project) via `jig.yaml` manifests and per-project overrides in `.jigrc.yaml`.

## Skill-Local by Design

You can colocate `jig` assets with the skill that calls them:

```text
my-plugin/
  skills/
    add-field/
      SKILL.md
      templates/
        model/recipe.yaml
        service/recipe.yaml
        schema/recipe.yaml
        workflow.yaml
```

This keeps automation close to the workflow context instead of forcing one central recipe registry.

## Why Use It With Agents

- LLM-native boundary: the model handles reasoning and variable extraction; `jig` handles deterministic file mutations.
- Skill-native packaging: recipes/workflows live in the skill directory, so automation ships with the skill itself.
- Deterministic outputs: removes formatting/style drift for repeatable tasks.
- Idempotent operations: retries don’t duplicate edits when `skip_if`/`skip_if_exists` is used.
- Structured failures: errors include machine-readable `what`, `where`, `why`, and `hint` fields.
- Multi-step execution: one workflow can chain many recipe steps, each doing create/inject/replace/patch operations.

## Release Trust Model

- Official release authenticity is enforced through signed checksums and installer verification.
- The installer refuses non-official repos unless `JIG_ALLOW_UNOFFICIAL_REPO=1` is set.
- Public source remains buildable by anyone; this protects release authenticity, not install exclusivity.

## Evaluation

The preferred evaluation path is the dedicated head-to-head runner in [`eval/head2head/`](eval/head2head/README.md).

This runner compares two explicit arms on the same scenario:

- `control`: plain-language skill spec
- `jig`: recipe/template-backed skill

The current head-to-head pair set is documented in [`eval/head2head/HEAD2HEAD_SKILL_PAIRS.md`](eval/head2head/HEAD2HEAD_SKILL_PAIRS.md) and focuses on routine backend patterns:

- deterministic service tests
- query-layer discipline
- rollout-safe schema migrations
- structured logging contracts
- view request/response contracts

Per trial, the head-to-head runner captures:

- correctness score and file-score diagnostics
- duration
- tool-call counts
- input, output, cache, context, and total token usage
- cost
- raw init/result events
- optional thinking text when enabled

### Current Result Snapshot

Current replicated head-to-head baseline:

- run: [`eval/results/head2head-pairs-r25-20260409.jsonl`](eval/results/head2head-pairs-r25-20260409.jsonl)
- scope: `5 scenarios x 3 reps x 2 arms = 30 trials`
- outcome: both arms passed all `15/15` scenario/rep pairs

Aggregate jig deltas versus control across the full run:

- score delta: `0.000`
- total tokens: `-1,463,345`
- total cost: `-$4.0582`
- total duration: `-193,621 ms`
- tool calls: `-85`

Average per pair:

- about `97.6k` fewer tokens
- about `$0.27` cheaper
- about `12.9s` faster
- about `5.7` fewer tool calls

Read we can defend today:

- for `4/5` routine backend patterns, jig was cheaper and faster at equal correctness
- `structured-logging-contract` remained the honest exception: correctness parity, but neutral-to-worse efficiency in some runs
- the current claim is intentionally narrow: `jig` helps most on routine, shape-constrained backend edits

Relevant review notes:

- adversarial harness review: [`eval/results/head2head-r11-20260409-adversarial-review.md`](eval/results/head2head-r11-20260409-adversarial-review.md)
- structured-logging multi-run review: [`eval/results/head2head-structured-logging-r20-r24-review-20260409.md`](eval/results/head2head-structured-logging-r20-r24-review-20260409.md)

Run the head-to-head suite:

```bash
cd eval
npx tsx head2head/run.ts \
  --scenario h2h-deterministic-service-test,h2h-query-layer-discipline,h2h-schema-migration-safety,h2h-structured-logging-contract,h2h-view-contract-enforcer \
  --reps 3 \
  --control-profile head2head/profiles/control \
  --jig-profile head2head/profiles/jig \
  --prompt-source directed \
  --thinking-mode
```

## Docs Map

- Public example index: [`examples/README.md`](examples/README.md)
- Product requirements document: [`PRD.md`](PRD.md)
- System architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Invariants/constraints: [`docs/INVARIANTS.md`](docs/INVARIANTS.md)
- Roadmap and milestone status: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Manual release runbook: [`docs/RELEASE-MANUAL.md`](docs/RELEASE-MANUAL.md)
- Public site scaffold: [`site/README.md`](site/README.md)
- Workstream/autopilot archive pointer: [`docs/workstreams/README.md`](docs/workstreams/README.md)

## Status

- Current project phase: v0.5 manual distribution path active.
- Validation baseline: `cargo test`, `cargo fmt --check`, and `cargo clippy -D warnings` passing.
- Eval checks passing via `npx tsx eval/harness/test.ts` and `npx tsx eval/head2head/test.ts`.
