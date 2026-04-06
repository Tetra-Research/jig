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
jig run recipe.yaml --vars '{"module":"hotels.services.booking","class_name":"BookingService"}'
```

Same recipe + same variables + same file state yields the same output.

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

## Eval Results and Parseability

Eval harness lives in `eval/` and writes JSONL results.

Scoring mechanism (primary metric):

- `assertion_score = passed_weight / total_weight`
- `negative_score = 1.0` if all negative assertions pass, else `0.0`
- `total = assertion_score * negative_score`

Secondary diagnostics:

- `file_score` (structural similarity), `jig_used`, and `jig_correct` are tracked per trial.

Efficiency accounting:

- `tokens_used = input_tokens + output_tokens + cache_creation_input_tokens + cache_read_input_tokens`
- `cost_usd` and `duration_ms` are recorded per trial.

Control-group snapshot (from `eval/experiments/README.md`, dated 2026-04-06):

- Matched control comparison (`add-view`, natural, shared `CLAUDE.md`, `n=1` per arm):
  - Baseline control: score `1.0`, tokens `317,608`, cost `$0.8279`, duration `84.0s`
  - Jig treatment: score `1.0`, tokens `241,702`, cost `$0.6505`, duration `66.2s`
  - Delta vs control: tokens `-23.9%`, cost `-21.4%`, duration `-21.3%`
- Strict no-jig control (`add-view`, natural, `--mode baseline --claude-md none`, `n=1`): score `1.0`, jig usage `0%`, tokens `162,530`, cost `$0.4746`, duration `50.0s`.
- Full control sweep (`exp-004`, baseline-only, 7 scenarios, `n=7`): mean score `0.730`, jig usage `0%`, mean duration `37.4s`, mean cost `$0.36`.
- Note: current control archive rows are legacy shape and expose total tokens (not split input/output/cache token fields).

Readiness/CI-safe mode (default strict schema checks):

```bash
cd eval
npx tsx harness/run.ts --schema-mode strict
```

Exploratory mixed-archive analysis:

```bash
cd eval
npx tsx experiments/analyze-gradient.ts \
  --results results/archive/results-mixed-schema-20260406T114302.jsonl \
  --schema-mode compat
```

Split mixed archives into schema-homogeneous files:

```bash
cd eval
npx tsx experiments/split-results-by-schema.ts \
  --input results/archive/results-mixed-schema-20260406T114302.jsonl \
  --out-dir results/archive
```

More detail: [`eval/experiments/README.md`](eval/experiments/README.md).
Control-group reference synthesis: [`eval/experiments/README.md#control-group-reference-current-takeaways-as-of-2026-04-06`](eval/experiments/README.md#control-group-reference-current-takeaways-as-of-2026-04-06).

## Docs Map

- Product requirements document: [`PRD.md`](PRD.md)
- System architecture: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Invariants/constraints: [`docs/INVARIANTS.md`](docs/INVARIANTS.md)
- Roadmap and milestone status: [`docs/ROADMAP.md`](docs/ROADMAP.md)
- Manual release runbook: [`docs/RELEASE-MANUAL.md`](docs/RELEASE-MANUAL.md)
- Workstream/autopilot archive pointer: [`docs/workstreams/README.md`](docs/workstreams/README.md)

## Status

- Current project phase: v0.5 manual distribution path active.
- Validation baseline: `cargo test`, `cargo fmt --check`, and `cargo clippy -D warnings` passing.
- Eval harness unit/integration checks passing via `npx tsx eval/harness/test.ts`.
