# jig

Deterministic file generation for LLM-native coding workflows.

`jig` is built to sit inside an LLM's write/edit loop. The model reads code, extracts variables, and decides intent; `jig` applies reproducible file operations (`create`, `inject`, `replace`, `patch`) so the mechanical edits are deterministic.

Unlike templating tools that centralize recipes in one global store, `jig` is designed for skill-local ownership: put recipes and workflows directly in the skill that uses them. A single skill can own multiple recipes plus a multi-step workflow.

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
