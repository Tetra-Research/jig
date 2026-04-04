# VALIDATION.md

> Workstream: core-engine
> Task: phase-5
> Last verified: 2026-04-03

## Phase Validation Criteria

From PLAN.md Phase 5:

- `cargo test` runs all unit + integration + snapshot tests green
- Every operation mode has at least one fixture
- Every error exit code (1-4) has at least one fixture
- Adding a new test case requires only a new directory, no code changes. Integration fixtures are auto-discovered from directories (no code changes to add tests). Spec-level unit tests are named functions in `#[cfg(test)]` modules (e.g., `spec::fr1::ac_1_1`). The two layers serve different purposes.
- Idempotency fixture: second run produces all skips, no file changes
- Binary has no dynamic dependencies beyond system libc (verified via `otool -L` on macOS, `ldd` on Linux)
- Every `TEST-*` ID in SPEC.md has a corresponding test function named `spec::fr{N}::ac_{N}_{M}` or `spec::nfr{N}::ac_n{N}_{M}`
- Error fixtures assert that error JSON contains `what`, `where`, `why`, and `hint` fields (at least one fixture per exit code)

## Spec Requirements -> Tests

| Criterion | EARS Type | Source | Test | Status |
|-----------|-----------|--------|------|--------|
| <!-- AC-N.N --> | <!-- type --> | SPEC.md | `spec::...` | PENDING |

## Coverage Summary

- Spec criteria: 0/0 covered
- Phase validation criteria: 0/19 covered

## Gaps

All criteria need test implementations.
