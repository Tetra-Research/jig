# VALIDATION.md

> Workstream: core-engine
> Task: phase-5
> Last verified: 2026-04-04

## Phase Validation Criteria

From PLAN.md Phase 5:

### 1. `cargo test` runs all unit + integration + snapshot tests green
**Status: PASS** — 190 tests (176 unit + 2 CLI + 12 integration), all passing.

### 2. Every operation mode has at least one fixture
**Status: PASS**
- Create: `create-simple`, `create-templated-path`, `create-skip-if-exists`, `create-force-overwrite`, `create-directory-creation`
- Inject after: `inject-after`
- Inject before: `inject-before`
- Inject prepend: `inject-prepend`
- Inject append: `inject-append`
- Inject at:last: `inject-at-last`
- Inject skip_if: `inject-skip-if`

### 3. Every error exit code (1-4) has at least one fixture
**Status: PASS**
- Exit 1: `error-malformed-yaml`, `error-missing-template`
- Exit 2: `error-template-syntax`
- Exit 3: `error-missing-target`, `error-regex-no-match`, `error-file-exists`
- Exit 4: `error-missing-vars`, `error-bad-type`

### 4. Adding a new test case requires only a new directory, no code changes
**Status: PASS** — `fixture_tests()` discovers all directories under `tests/fixtures/` at runtime. Adding a new directory with `recipe.yaml` and `vars.json` automatically adds a test case.

### 5. Idempotency fixture: second run produces all skips, no file changes
**Status: PASS** — `combined-idempotency` fixture tested by `idempotency_second_run_all_skips()`. Second run reports all ops as `action: "skip"`, `files_written` empty, `files_skipped` populated.

### 6. Binary has no dynamic dependencies beyond system libc
**Status: PASS** — `binary_no_extra_dynamic_deps()` test verifies via `otool -L` (macOS) / `ldd` (Linux).

### 7. Error fixtures assert that error JSON contains `what`, `where`, `why`, and `hint` fields
**Status: PASS** — `error_fixtures_have_structured_fields()` test verifies all error fixtures. For exit code 3, checks JSON output fields. For exit codes 1, 2, 4, checks stderr structured format.

## Test Inventory

### Integration Test Functions (tests/integration.rs)
| Test | What it validates |
|------|-------------------|
| `fixture_tests` | Auto-discovers and runs all fixture directories |
| `error_fixtures_have_structured_fields` | Error fixtures produce structured error fields, covers exit codes 1-4 |
| `determinism_identical_output_across_runs` | AC-N1.1: byte-identical output across 3 runs |
| `idempotency_second_run_all_skips` | AC-N2.1: second run all skips |
| `snapshot_json_output_create` | Snapshot: JSON create operation output |
| `snapshot_json_output_inject` | Snapshot: JSON inject operation output |
| `snapshot_json_output_skip` | Snapshot: JSON skip output |
| `snapshot_json_output_error` | Snapshot: JSON error output with structured fields |
| `snapshot_error_missing_var` | Snapshot: stderr for missing required variable |
| `snapshot_error_malformed_yaml` | Snapshot: stderr for malformed YAML |
| `snapshot_json_output_combined` | Snapshot: JSON combined create+inject output |
| `binary_no_extra_dynamic_deps` | AC-N3.1: no extra dynamic dependencies |

### Fixture Directories (tests/fixtures/)
| Fixture | Category | Exit Code | Tests |
|---------|----------|-----------|-------|
| `create-simple` | Create | 0 | Simple create, AC-4.1 |
| `create-templated-path` | Create | 0 | Templated path, AC-4.2 |
| `create-skip-if-exists` | Create | 0 | Skip existing, AC-4.4 |
| `create-force-overwrite` | Create | 0 | --force override, AC-4.6 |
| `create-directory-creation` | Create | 0 | Auto dir creation, AC-4.3 |
| `inject-after` | Inject | 0 | After regex, AC-5.1 |
| `inject-before` | Inject | 0 | Before regex, AC-5.2 |
| `inject-prepend` | Inject | 0 | Prepend, AC-5.3 |
| `inject-append` | Inject | 0 | Append, AC-5.4 |
| `inject-at-last` | Inject | 0 | at:last, AC-5.5 |
| `inject-skip-if` | Inject | 0 | skip_if idempotency, AC-5.7 |
| `error-missing-vars` | Error | 4 | Missing required var, AC-2.5 |
| `error-bad-type` | Error | 4 | Type mismatch, AC-2.7 |
| `error-missing-template` | Error | 1 | Missing template file, AC-1.8 |
| `error-missing-target` | Error | 3 | Missing inject target, AC-5.9 |
| `error-regex-no-match` | Error | 3 | Regex no match, AC-5.8 |
| `error-malformed-yaml` | Error | 1 | Bad YAML syntax, AC-1.5 |
| `error-file-exists` | Error | 3 | File conflict, AC-4.5 |
| `error-template-syntax` | Error | 2 | Jinja2 syntax error, AC-3.18 |
| `combined-create-inject` | Combined | 0 | Create then inject, AC-N6.2 |
| `combined-multi-file` | Combined | 0 | Multi-file recipe |
| `combined-idempotency` | Combined | 0 | Idempotent execution, AC-N2.1 |

## Coverage Summary

- Phase validation criteria: 7/7 met
- Fixture coverage: 21 fixtures across create, inject, error, and combined categories
- Snapshot coverage: 7 snapshots (JSON create, inject, skip, error, combined + stderr error formats)
- Exit code coverage: all 5 exit codes (0-4) covered
