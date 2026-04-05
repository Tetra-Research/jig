# SHARED-CONTEXT.md

> Workstream: workflows
> Last updated: 2026-04-04

## Purpose

Add multi-recipe orchestration to jig (v0.3). Workflows chain multiple recipes into a single `jig workflow` invocation with conditional steps, variable mapping, and configurable error handling. This is the composition layer that enables cross-file operations like "add a field and propagate it through model, service, schema, admin, and tests."

## Current State

- Initialized (2026-04-04)
- SPEC.md written with 8 FRs, 4 NFRs, 83 acceptance criteria, 25 required test fixtures
- PLAN.md written with 5 phases
- No implementation code exists yet — `src/workflow.rs` does not exist
- All dependencies are met: core-engine (v0.1) and replace-patch (v0.2) are complete with 308 tests passing

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

## Patterns Established

*To be populated during implementation.*

## Known Issues / Tech Debt

### Pre-existing (from v0.2 review)
- `src/operations/replace.rs` and `patch.rs` have `write_back` functions that may swallow I/O errors — needs fix before workflows exercise complex replace/patch chains
- `Position::Sorted` is a stub (returns `todo!()`) — any workflow step using `position: sorted` will panic
- 11 of 26 spec-required integration fixtures from replace-patch are still missing

### Anticipated
- `cmd_run` in `main.rs` bundles CLI concerns with recipe execution logic — will need refactoring to extract a reusable `run_recipe()` function for workflow step execution (Phase 3, milestone 3.2)
- vars_map conflict resolution (`{a: c, b: c}`) is unspecified — last entry in IndexMap insertion order wins. Should be documented.

## File Ownership

| File | Phase | Purpose |
|------|-------|---------|
| `src/workflow.rs` | 1-3 | **New.** Workflow parsing, variable resolution, when evaluation, execution engine |
| `src/main.rs` | 1, 4 | **Modified.** Add `Workflow` command variant, `cmd_workflow()`, extend `cmd_validate` and `cmd_vars` with auto-detection |
| `src/output.rs` | 4 | **Modified.** Add `format_workflow_json()` and `format_workflow_human()` |
| `src/recipe.rs` | 1 | **Possibly modified.** May need to expose `load_recipe()` as public for workflow to call, or extract shared YAML detection logic |
| `src/variables.rs` | 2 | **Unchanged.** Reused as-is for workflow variable validation |
| `src/renderer.rs` | 2 | **Unchanged.** Reused for `when` expression rendering |
| `src/operations/mod.rs` | 3 | **Unchanged.** Reused for operation execution within each step |
| `src/error.rs` | — | **Unchanged.** No new error types needed |
| `tests/fixtures/workflow-*` | 5 | **New.** 25 integration test fixture directories |
