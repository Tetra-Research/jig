# SHARED-CONTEXT.md

> Workstream: workflows
> Last updated: 2026-04-04

## Purpose

Add multi-recipe orchestration to jig (v0.3). Workflows chain multiple recipes into a single `jig workflow` invocation with conditional steps, variable mapping, and configurable error handling. This is the composition layer that enables cross-file operations like "add a field and propagate it through model, service, schema, admin, and tests."

## Current State

- **Complete** (2026-04-04)
- All 5 phases implemented: parsing, variable resolution, execution engine, CLI command, integration tests
- 343 tests passing (329 unit + 2 CLI + 12 integration), `cargo clippy` clean
- 25 integration fixtures (18 success-path, 7 error-case)
- 37 unit tests in `src/workflow.rs` covering parsing, variable resolution, when evaluation, execution
- Code review completed — 1 critical, 4 major, 6 minor findings identified. **Review findings not yet fixed in code** (commit 76d826f only added review artifacts)

## Decisions Made

### D-1: Workflow files are standalone YAML, distinct from recipes
Workflows have `steps:`, recipes have `files:`. Auto-detection in `jig validate` and `jig vars` checks for these keys. A file with both is an error. This keeps the format clean and avoids overloading the recipe concept.

### D-2: Recipe paths resolve relative to the workflow file
Consistent with I-7 (templates live with the consumer). A workflow + its recipe subdirectories form a self-contained bundle that can be moved or shared.

### D-3: `when` uses Jinja2 rendering + string truthiness
No new expression language. Render the `when` template with workflow-level variables. The rendered string is falsy if it's empty, "false" (case-insensitive), or "0". Everything else is truthy. This covers minijinja's boolean rendering ("false") and zero.

### D-4: `when` evaluates against workflow-level variables (not step-level)
The condition determines "should this step run at all" — a workflow-level concern. vars_map and vars modify variables for the recipe, not for the condition. This prevents circular dependencies.

### D-5: vars_map is rename, not copy
`{field_name: request_field}` means the step sees `request_field` but NOT `field_name`. The spec says "rename." If you need both, add the original to `vars` explicitly.

### D-6: vars_map renamings are simultaneous
`{a: b, b: c}` means original `a` → `b` and original `b` → `c`. Not chained (`a` → `b` → `c`). Prevents order-dependent surprises.

### D-7: No new exit codes
Workflow errors map to existing codes: 1 (validation), 2 (rendering), 3 (file ops), 4 (variables). No exit code 5+. Respects I-5 (stable exit codes).

### D-8: Single ExecutionContext spans all steps
One `base_dir`, one `force` flag, one `virtual_files` map across the entire workflow. Enables dry-run chaining where step N+1 reads files "created" by step N.

### D-9: Step variable validation happens at execution time, not upfront
Conditional steps might reference variables that only exist when the condition is true. Upfront validation would produce false errors for skipped steps.

### D-10: No nested workflows, no inter-step variable passing
Steps reference recipes only (not other workflows). Variables flow from workflow to steps, not between steps. Steps communicate through the filesystem. Keeps v0.3 scope manageable.

### D-11: Duplicate `run_recipe()` rather than refactor `cmd_run` (implementation decision)
Rather than refactoring the working `cmd_run` pipeline in `main.rs`, implementation created a parallel `run_recipe()` in `workflow.rs` that duplicates the rendering logic. This avoided risk of breaking the existing recipe execution path during workflow development, but creates maintenance burden (review finding M4). Should be consolidated before v0.4.

### D-12: Duplicate vars_map targets rejected at parse time (implementation decision)
`load_workflow()` validates that a single step's `vars_map` doesn't map two source keys to the same target. This was an open question in planning (what happens with `{a: c, b: c}`?). Rather than last-write-wins, it's now a validation error at parse time.

## Patterns Established

- **`run_recipe()` extracted as independent function in `workflow.rs`.** Rather than refactoring `cmd_run`, the implementation created a parallel `run_recipe()` function (`workflow.rs:551-619`) that handles recipe loading, template rendering, and operation execution without CLI concerns. This avoids touching the working `cmd_run` code path but creates code duplication (see review finding M4).

- **`detect_file_type()` as the auto-detection entry point.** A single function (`workflow.rs:151-198`) reads YAML, checks for `steps` vs `files` keys, and returns `FileType::Workflow` or `FileType::Recipe`. Used in three places: `cmd_validate`, `cmd_vars`, and `cmd_workflow` (guard against recipe files). This pattern should be reused by libraries (v0.4) if they add a new file type.

- **`compute_workflow_exit_code()` centralizes exit code logic.** Moved exit code computation out of the execution loop into a dedicated function in `main.rs` (lines 582-607). Takes the workflow's on_error mode and the step results, returns the correct exit code. Prevents exit-code logic from leaking into the execution engine.

- **Workflow fixtures use step subdirectories.** Each workflow fixture has `step1/`, `step2/`, etc. directories containing full recipe bundles (recipe.yaml + templates/). The workflow.yaml references these via relative paths. This mirrors the intended real-world layout.

- **`RawWorkflow` intermediate struct for deserialization.** Serde deserializes into `RawWorkflow` (unresolved paths), then `load_workflow()` resolves paths and validates recipes to produce the final `Workflow` struct. This separates parsing from validation — same pattern as `Recipe::load()`.

- **Simultaneous vars_map via snapshot-remove-insert.** `resolve_step_variables()` (lines 361-401) snapshots original values, removes mapped keys, then inserts renamed values. This prevents chaining without needing a separate temporary map.

## Known Issues / Tech Debt

### From v0.3 code review (unfixed)

**Critical:**
- **C1: `extract_rendered_from_error` returns error description, not rendered template content.** `workflow.rs:621-628` — `se.what.clone()` returns the human-readable error message as `rendered_content` in JSON output. The spec requires the actual rendered template so LLM callers can fall back to manual editing. Fix: extract from `OpResult::Error` in `partial_results` instead.

**Major:**
- **M1: `format_workflow_json` status logic breaks with per-step `on_error` overrides.** `output.rs:300-302` — Only checks workflow-level `on_error`, not step-level. When a step uses `on_error: report` but workflow default is `stop`, JSON status becomes `"error"` instead of `"partial"` despite exit code 3. Fix: derive status from exit_code, not on_error mode.
- **M2: Workflow steps silently accept workflow YAML files as recipes.** `workflow.rs:279` — `Recipe::load` succeeds on workflow files because `files` defaults to `[]` via `#[serde(default)]` and `steps` is silently ignored. A typo referencing another workflow succeeds with 0 operations. Fix: call `detect_file_type` on each step's recipe and reject `FileType::Workflow`.
- **M3: Workflow early-error JSON drops multi-error variable validation.** `main.rs:477` — `e.structured_error()` returns only the first error. JSON output loses all but the first validation error; human stderr correctly shows all. Fix: use `e.structured_errors()` and emit all.
- **M4: `cmd_run` and `run_recipe` are divergent copies of the same rendering pipeline.** `main.rs:335-427` vs `workflow.rs:551-618` — Template rendering, path rendering, and skip_if rendering logic is duplicated with subtly different error handling. Fix: refactor `cmd_run` to call `run_recipe`.

**Minor:**
- **m1: Determinism unit test is trivially weak.** `workflow.rs:1215-1233` — Compares `steps.len()` stringified, not actual output. Integration test is much stronger; this unit test adds no value.
- **m2: `load_workflow` reports only first validation error.** `workflow.rs:303-306` — Errors are collected but only the first is returned. Should report all.
- **m3: No integration test for AC-7.8** (recipe passed to `jig workflow`). Code handles it but no regression fixture exists.
- **m4: Early error JSON creates misleading output.** `main.rs:473-507` — Pre-execution errors produce `"steps": [{"recipe": ""}]` with empty recipe string.
- **m5: Dry-run workflow human output says "wrote" instead of "would write".** `output.rs:406,436` — `format_human(operations, false, verbose)` hardcodes `false` for `dry_run` instead of forwarding actual value.
- **m6: `resolve_step_variables` panics on non-object JSON.** `workflow.rs:369,394` — `.unwrap()` on `as_object_mut()`. Safe in practice (internal callers always provide objects) but the function is `pub`.

### Pre-existing (from v0.2, still open)
- `write_back` silently swallows write errors in `patch.rs` and `replace.rs` — **Critical**
- `Position::Sorted` is a stub (`todo!()` panic) — **Critical**
- Byte/char index mismatch in `delimiter.rs:87` (multi-byte UTF-8 panics) — **Critical**
- 11 of 26 spec-required integration fixtures from replace-patch still missing

## File Ownership

| File | Phase | What Changed |
|------|-------|--------------|
| `src/workflow.rs` | 1-3 | **New** (1234 lines). Workflow parsing, variable resolution, when evaluation, execution engine, `run_recipe()` |
| `src/main.rs` | 1, 4 | **Modified.** Added `Workflow` command variant, `cmd_workflow()`, `compute_workflow_exit_code()`, auto-detection in `cmd_validate` and `cmd_vars` |
| `src/output.rs` | 4 | **Modified.** Added `format_workflow_json()`, `format_workflow_human()`, `build_workflow_validate_json()` |
| `src/recipe.rs` | — | **Unchanged.** `Recipe::load()` used directly from `workflow.rs`; no public API changes needed |
| `src/variables.rs` | — | **Unchanged.** Reused as-is for workflow variable validation |
| `src/renderer.rs` | — | **Unchanged.** Reused for `when` expression rendering |
| `src/operations/mod.rs` | — | **Unchanged.** Reused for operation execution within each step |
| `src/error.rs` | — | **Unchanged.** No new error types needed |
| `tests/fixtures/workflow-*` | 5 | **New.** 18 success-path fixture directories |
| `tests/fixtures/error-workflow-*` | 5 | **New.** 7 error-case fixture directories |
| `tests/integration.rs` | 5 | **Modified.** Added workflow fixture support, determinism and idempotency tests |
