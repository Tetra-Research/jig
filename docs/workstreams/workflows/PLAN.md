# PLAN.md

> Workstream: workflows
> Last updated: 2026-04-04
> Status: Complete

## Objective

Add multi-recipe orchestration to jig (v0.3). A workflow chains multiple recipes into a single invocation with conditional steps (`when`), variable mapping between steps (`vars_map`, `vars`), and configurable error handling (`on_error: stop|continue|report`). This is the prerequisite for libraries (v0.4), which organize workflows over library-namespaced recipes.

The deliverable is: `jig workflow <path> --vars '{...}'` executes an ordered sequence of recipe invocations, with per-step JSON results, shared filesystem state, and predictable error behavior.

## Phases

### Phase 1: Workflow Parsing & CLI Detection
Status: Complete

Build the workflow YAML parser and extend `jig validate` / `jig vars` to auto-detect workflow files.

#### Milestones
- [x] 1.1: Create `src/workflow.rs` with `Workflow`, `WorkflowStep`, `OnError` structs and serde deserialization from YAML
- [x] 1.2: Implement `load_workflow()` — parse YAML, resolve recipe paths relative to workflow dir, validate recipe files exist and are structurally valid
- [x] 1.3: Implement auto-detection in `jig validate`: read YAML, check for `steps` vs `files` key, dispatch to workflow or recipe validation
- [x] 1.4: Implement auto-detection in `jig vars`: same detection logic, output workflow variable declarations
- [x] 1.5: Add workflow validation JSON and human output formats (type, name, variables, steps with recipe/valid/conditional)
- [x] 1.6: Unit tests for parsing: valid workflow, empty steps, missing steps key, both steps+files, bad on_error, missing recipe file, invalid recipe, no variables, optional metadata

#### Validation Criteria
- `jig validate workflow.yaml` auto-detects and validates a workflow with 3+ steps, reporting each recipe's validity
- `jig vars workflow.yaml` outputs the workflow's variable declarations as JSON
- All parse-time error cases produce exit code 1 with structured error messages
- 20+ unit tests covering AC-1.1 through AC-1.20

### Phase 2: Variable Resolution & Conditional Steps
Status: Complete

Implement step variable resolution (shared vars → vars_map → vars overrides) and `when` condition evaluation.

#### Milestones
- [x] 2.1: Implement `resolve_step_variables()` — start with workflow vars, apply vars_map renaming (simultaneous, not chained), apply vars overrides
- [x] 2.2: Implement `evaluate_when()` — render Jinja2 template with workflow vars, apply falsy rules (empty, "false" case-insensitive, "0")
- [x] 2.3: Implement workflow-level variable validation — reuse `variables::validate_variables()` with workflow's `VariableDecl` map
- [x] 2.4: Implement step-level variable validation — validate step's resolved vars against the step recipe's `VariableDecl` map
- [x] 2.5: Unit tests for: vars_map rename semantics, vars_map with nonexistent source, vars_map target collision, vars override, combined vars_map+vars, when truthy/falsy cases, when with Jinja2 control flow, when with undefined variable, simultaneous vars_map

#### Validation Criteria
- `resolve_step_variables()` produces correct output for all documented edge cases (AC-4.1 through AC-4.10)
- `evaluate_when()` correctly classifies: empty → false, "false"/"False"/"FALSE" → false, "0" → false, "true" → true, "yes" → true, non-empty string → true
- Variable validation errors at workflow level halt before any execution (exit 4)
- 15+ unit tests covering FR-3 and FR-4

### Phase 3: Execution Engine
Status: Complete

Build the workflow execution loop that iterates steps, evaluates conditions, resolves variables, and runs recipes — sharing filesystem state (and virtual_files in dry-run) across steps.

#### Milestones
- [x] 3.1: Implement `execute_workflow()` — sequential step loop with condition evaluation, variable resolution, and recipe execution
- [x] 3.2: Extract `run_recipe()` in `workflow.rs` — reusable recipe execution without CLI concerns (rendering, operation execution, error collection)
- [x] 3.3: Implement cross-step `virtual_files` carryover for `--dry-run` mode — a single `ExecutionContext` spans all steps
- [x] 3.4: Implement `on_error` handling: stop (halt on failure), continue (record and proceed, exit 0), report (record and proceed, exit 3)
- [x] 3.5: Implement per-step `on_error` overrides — step-level value takes precedence over workflow-level default
- [x] 3.6: Implement exit code selection via `compute_workflow_exit_code()` in `main.rs`
- [x] 3.7: Unit tests for execution: basic two-step, conditional skip, chain create→inject, chain create→patch, error stop, error continue, error report, step on_error override, dry-run chaining

#### Validation Criteria
- A two-step workflow where step 1 creates a file and step 2 injects into it succeeds in both normal and dry-run mode
- A workflow with `when: "{{ false }}"` skips the step and reports status "skipped"
- `on_error: stop` halts after first failure; `continue` runs all steps; `report` runs all steps but exits 3
- Per-step on_error correctly overrides workflow default
- 20+ unit tests covering FR-5 and FR-6

### Phase 4: CLI Command & Output Formatting
Status: Complete

Add the `jig workflow` subcommand and implement workflow-specific JSON and human output.

#### Milestones
- [x] 4.1: Add `Workflow` variant to `Commands` enum in `main.rs` — accepts path + same global options as `Run`
- [x] 4.2: Implement `cmd_workflow()` in `main.rs` — parse workflow, validate vars, execute, format output, return exit code
- [x] 4.3: Implement `format_workflow_json()` in `output.rs` — per-step results, aggregate files_written/files_skipped, top-level workflow/on_error/status/dry_run fields
- [x] 4.4: Implement `format_workflow_human()` in `output.rs` — step headers ("Step 1/3: ..."), nested operation lines, summary line
- [x] 4.5: Handle edge case: `jig workflow` passed a recipe file → exit 1 with helpful message pointing to `jig run`
- [x] 4.6: Wire `--verbose` to include rendered content across all steps in both JSON and human output
- [x] 4.7: CLI integration tests: basic workflow run, JSON output format, human output format, --dry-run, --quiet

#### Validation Criteria
- `jig workflow workflow.yaml --vars '...' --json` produces correct per-step JSON output matching the schema in SPEC.md
- Human output shows step progress headers, nested operation results, and summary line
- `jig workflow recipe.yaml` produces a clear error directing to `jig run`
- 10+ tests covering FR-7 and FR-8

### Phase 5: Integration Testing
Status: Complete

Build fixture-based integration tests covering the full workflow feature set, including error cases and cross-step chaining.

#### Milestones
- [x] 5.1: Create fixture infrastructure — workflow fixtures follow the same pattern as recipe fixtures, with a `workflow.yaml` at the root and step recipe subdirectories
- [x] 5.2: Implement 14 success-path fixtures: basic, three-steps, when-true, when-false, when-complex, vars-map, vars-override, vars-map-and-override, chain-create-inject, chain-create-patch, empty-steps, no-variables, idempotent, dry-run-chain
- [x] 5.3: Implement 4 error-handling fixtures: on-error-stop, on-error-continue, on-error-report, step-on-error
- [x] 5.4: Implement 7 error-case fixtures: missing-recipe, bad-yaml, missing-steps, ambiguous, bad-vars, when-undef, step-missing-var
- [x] 5.5: Run full test suite (`cargo test`), verify all pass, update test count in CLAUDE.md — 343 tests passing

#### Validation Criteria
- All 25 spec-required fixtures exist and pass
- `cargo test` passes with 0 failures
- `cargo clippy` clean
- Test count updated in CLAUDE.md project status

## Dependencies

- **Depends on:** core-engine (v0.1, complete), replace-patch (v0.2, complete). Workflows orchestrate all four operation types — they must all work.
- **Blocks:** libraries (v0.4) — libraries define workflows over library-namespaced recipes and need the workflow execution engine.

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Workflow file format | Standalone YAML with `steps` key | Distinguishable from recipes (which have `files`). Clean auto-detection. Libraries (v0.4) will reference these. |
| Recipe path resolution | Relative to workflow file directory | Consistent with I-7 (templates live with the consumer). Keeps workflow + recipe bundles self-contained. |
| `when` evaluation | Render Jinja2, check string truthiness | Simple, predictable. Reuses existing renderer. No new expression language. |
| Falsy values | Empty, "false" (case-insensitive), "0" | Covers minijinja boolean rendering ("false"), zero, and empty. Minimal set — easy to document and remember. |
| `when` variable scope | Workflow-level variables (before vars_map/vars) | Prevents circular dependency where step vars affect step execution. Condition is about "should this step run at all" — a workflow-level concern. |
| vars_map semantics | Rename (not copy) | Spec says "rename." Avoids ambiguity when recipe has different meaning for original name. Copy achievable via `vars` override. |
| vars_map application | Simultaneous (not chained) | `{a: b, b: c}` means original `a` → `b` and original `b` → `c`, not `a` → `b` → `c`. Prevents order-dependent surprises. |
| No new exit codes | Reuse 0-4 | Workflow errors map to existing categories (validation, rendering, file ops, variables). Adding exit code 5+ would break I-5 stability guarantee without clear benefit. |
| No nested workflows | Steps reference recipes only | Keeps v0.3 scope manageable. Nested workflows can be added later if needed. Flat is better than nested. |
| No inter-step variable passing | Steps share filesystem, not variables | Variables flow from workflow to steps. Steps communicate through the filesystem (one creates a file, the next reads/patches it). Keeps the model simple. |
| Execution context sharing | Single ExecutionContext across steps | Enables dry-run virtual_files carryover. One base_dir, one force flag, one set of virtual files. |
| Step variable validation timing | At execution time (not upfront) | Conditional steps might have variables that only make sense when the step runs. Validating skipped steps would produce false errors. |

## Risks / Open Questions (Resolved)

- **~~Risk: Refactoring `cmd_run` for reuse.~~** Resolved. Created `run_recipe()` in `workflow.rs` (lines 551-619) that handles recipe loading, rendering, and execution without CLI concerns. `cmd_run` in `main.rs` was NOT refactored — both contain similar rendering pipelines. See review finding M4 in Known Issues.
- **~~Risk: `virtual_files` carryover complexity.~~** Resolved. Single `ExecutionContext` spans all steps. Tested via `workflow-dry-run-chain` fixture (step 1 creates, step 2 injects into virtual file).
- **~~Open question: `vars` values support Jinja2?~~** Decision: no for v0.3. Literal JSON values only.
- **~~Open question: vars_map conflict detection?~~** Decision: `load_workflow()` now validates for duplicate targets within a single step's vars_map and rejects them at parse time. Cross-step conflicts are not checked (different steps are independent).
