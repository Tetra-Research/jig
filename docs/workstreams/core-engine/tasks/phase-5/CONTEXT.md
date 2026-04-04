# Phase 5: Integration Test Framework

> Workstream: core-engine
> Generated: 2026-04-03
> Source: PLAN.md

## Phase Plan

## Phase 5: Integration Test Framework
Status: Planned
Traces to: All FRs and NFRs (validation layer)

Fixture-based integration test harness that makes adding new test cases a matter of adding a directory. Snapshot tests for all output formats. This phase validates the entire v0.1 pipeline end-to-end.

#### Milestones
- [ ] 5.1: Test harness in `tests/integration.rs` — fixture discovery; copy existing/ to temp dir; run jig as subprocess; diff output against expected/; assert JSON output against expected_output.json; assert exit code against expected_exit_code
- [ ] 5.2: Fixtures for create operations — simple create, templated path, skip_if_exists, force overwrite, directory creation
- [ ] 5.3: Fixtures for inject operations — after/before/prepend/append, at:first/at:last, skip_if
- [ ] 5.4: Fixtures for error cases — missing vars, bad type, missing template, missing target file, regex no-match, malformed YAML, file exists without force
- [ ] 5.5: Fixtures for combined operations — create + inject in one recipe, multi-file recipe, idempotency (run twice)
- [ ] 5.6: insta snapshot tests for JSON output format, human output format, error message format
- [ ] 5.7: Determinism test — run same recipe twice, assert byte-identical output (AC-N1.1)

#### Validation Criteria
- `cargo test` runs all unit + integration + snapshot tests green
- Every operation mode has at least one fixture
- Every error exit code (1-4) has at least one fixture
- Adding a new test case requires only a new directory, no code changes. Integration fixtures are auto-discovered from directories (no code changes to add tests). Spec-level unit tests are named functions in `#[cfg(test)]` modules (e.g., `spec::fr1::ac_1_1`). The two layers serve different purposes.
- Idempotency fixture: second run produces all skips, no file changes
- Binary has no dynamic dependencies beyond system libc (verified via `otool -L` on macOS, `ldd` on Linux)
- Every `TEST-*` ID in SPEC.md has a corresponding test function named `spec::fr{N}::ac_{N}_{M}` or `spec::nfr{N}::ac_n{N}_{M}`
- Error fixtures assert that error JSON contains `what`, `where`, `why`, and `hint` fields (at least one fixture per exit code)

#### Key Files
- `tests/integration.rs`
- `tests/fixtures/` (directory tree)

#### Dependencies
- insta crate for snapshot testing
- assert_cmd or similar for subprocess testing (or raw std::process::Command)

## Execution Context

This phase builds on Phase 4. Assume all prior phase artifacts exist and tests pass.

## Invariants

Refer to `docs/INVARIANTS.md` for project-wide constraints that must be honored.

## Architecture

Refer to `docs/ARCHITECTURE.md` for module boundaries and design decisions.

