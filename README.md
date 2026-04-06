# jig

Deterministic file generation for LLM-assisted coding workflows.

`jig` takes a YAML recipe plus JSON variables and applies reproducible file operations (`create`, `inject`, `replace`, `patch`) so agents stop re-deriving boilerplate and team conventions on every run.

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

## Why Use It With Agents

- Deterministic outputs: removes formatting/style drift for repeatable tasks.
- Idempotent operations: retries don’t duplicate edits when `skip_if`/`skip_if_exists` is used.
- Structured failures: errors include machine-readable `what`, `where`, `why`, and `hint` fields.
- Tool boundary clarity: LLM handles intent and variable extraction; `jig` handles mechanical file edits.

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
