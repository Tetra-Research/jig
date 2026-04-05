# PLAN.md

> Workstream: workflows
> Last updated: 2026-04-04
> Status: Planned

## Objective

Add multi-recipe orchestration to jig (v0.3). A workflow chains multiple recipes into a single invocation with conditional steps (`when`), variable mapping between steps (`vars_map`, `vars`), and configurable error handling (`on_error: stop|continue|report`). This is the prerequisite for libraries (v0.4), which organize workflows over library-namespaced recipes.

The deliverable is: `jig workflow <path> --vars '{...}'` executes an ordered sequence of recipe invocations, with per-step JSON results, shared filesystem state, and predictable error behavior.

## Phases

### Phase 1: Workflow Parsing & CLI Detection
Status: Planned

Build the workflow YAML parser and extend `jig validate` / `jig vars` to auto-detect workflow files.

#### Milestones
- [ ] 1.1: Create `src/workflow.rs` with `Workflow`, `WorkflowStep`, `OnError` structs and serde deserialization from YAML
- [ ] 1.2: Implement `load_workflow()` — parse YAML, resolve recipe paths relative to workflow dir, validate recipe files exist and are structurally valid
- [ ] 1.3: Implement auto-detection in `jig validate`: read YAML, check for `steps` vs `files` key, dispatch to workflow or recipe validation
- [ ] 1.4: Implement auto-detection in `jig vars`: same detection logic, output workflow variable declarations
- [ ] 1.5: Add workflow validation JSON and human output formats (type, name, variables, steps with recipe/valid/conditional)
- [ ] 1.6: Unit tests for parsing: valid workflow, empty steps, missing steps key, both steps+files, bad on_error, missing recipe file, invalid recipe, no variables, optional metadata

#### Validation Criteria
- `jig validate workflow.yaml` auto-detects and validates a workflow with 3+ steps, reporting each recipe's validity
- `jig vars workflow.yaml` outputs the workflow's variable declarations as JSON
- All parse-time error cases produce exit code 1 with structured error messages
- 20+ unit tests covering AC-1.1 through AC-1.20

### Phase 2: Variable Resolution & Conditional Steps
Status: Planned

Implement step variable resolution (shared vars → vars_map → vars overrides) and `when` condition evaluation.

#### Milestones
- [ ] 2.1: Implement `resolve_step_variables()` — start with workflow vars, apply vars_map renaming (simultaneous, not chained), apply vars overrides
- [ ] 2.2: Implement `evaluate_when()` — render Jinja2 template with workflow vars, apply falsy rules (empty, "false" case-insensitive, "0")
- [ ] 2.3: Implement workflow-level variable validation — reuse `variables::validate_variables()` with workflow's `VariableDecl` map
- [ ] 2.4: Implement step-level variable validation — validate step's resolved vars against the step recipe's `VariableDecl` map
- [ ] 2.5: Unit tests for: vars_map rename semantics, vars_map with nonexistent source, vars_map target collision, vars override, combined vars_map+vars, when truthy/falsy cases, when with Jinja2 control flow, when with undefined variable, simultaneous vars_map

#### Validation Criteria
- `resolve_step_variables()` produces correct output for all documented edge cases (AC-4.1 through AC-4.10)
- `evaluate_when()` correctly classifies: empty → false, "false"/"False"/"FALSE" → false, "0" → false, "true" → true, "yes" → true, non-empty string → true
- Variable validation errors at workflow level halt before any execution (exit 4)
- 15+ unit tests covering FR-3 and FR-4

### Phase 3: Execution Engine
Status: Planned

Build the workflow execution loop that iterates steps, evaluates conditions, resolves variables, and runs recipes — sharing filesystem state (and virtual_files in dry-run) across steps.

#### Milestones
- [ ] 3.1: Implement `execute_workflow()` — sequential step loop with condition evaluation, variable resolution, and recipe execution
- [ ] 3.2: Integrate with existing `cmd_run` logic — reuse recipe loading, template rendering, and operation execution from `main.rs` (extract shared logic if needed)
- [ ] 3.3: Implement cross-step `virtual_files` carryover for `--dry-run` mode — a single `ExecutionContext` spans all steps
- [ ] 3.4: Implement `on_error` handling: stop (halt on failure), continue (record and proceed, exit 0), report (record and proceed, exit 3)
- [ ] 3.5: Implement per-step `on_error` overrides — step-level value takes precedence over workflow-level default
- [ ] 3.6: Implement exit code selection: all success → 0, stop mode failure → step's exit code, continue → 0, report with failures → 3
- [ ] 3.7: Unit tests for execution: basic two-step, conditional skip, chain create→inject, chain create→patch, error stop, error continue, error report, step on_error override, dry-run chaining

#### Validation Criteria
- A two-step workflow where step 1 creates a file and step 2 injects into it succeeds in both normal and dry-run mode
- A workflow with `when: "{{ false }}"` skips the step and reports status "skipped"
- `on_error: stop` halts after first failure; `continue` runs all steps; `report` runs all steps but exits 3
- Per-step on_error correctly overrides workflow default
- 20+ unit tests covering FR-5 and FR-6

### Phase 4: CLI Command & Output Formatting
Status: Planned

Add the `jig workflow` subcommand and implement workflow-specific JSON and human output.

#### Milestones
- [ ] 4.1: Add `Workflow` variant to `Commands` enum in `main.rs` — accepts path + same global options as `Run`
- [ ] 4.2: Implement `cmd_workflow()` in `main.rs` — parse workflow, validate vars, execute, format output, return exit code
- [ ] 4.3: Implement `format_workflow_json()` in `output.rs` — per-step results, aggregate files_written/files_skipped, top-level workflow/on_error/status/dry_run fields
- [ ] 4.4: Implement `format_workflow_human()` in `output.rs` — step headers ("Step 1/3: ..."), nested operation lines, summary line
- [ ] 4.5: Handle edge case: `jig workflow` passed a recipe file → exit 1 with helpful message pointing to `jig run`
- [ ] 4.6: Wire `--verbose` to include rendered content across all steps in both JSON and human output
- [ ] 4.7: CLI integration tests: basic workflow run, JSON output format, human output format, --dry-run, --quiet

#### Validation Criteria
- `jig workflow workflow.yaml --vars '...' --json` produces correct per-step JSON output matching the schema in SPEC.md
- Human output shows step progress headers, nested operation results, and summary line
- `jig workflow recipe.yaml` produces a clear error directing to `jig run`
- 10+ tests covering FR-7 and FR-8

### Phase 5: Integration Testing
Status: Planned

Build fixture-based integration tests covering the full workflow feature set, including error cases and cross-step chaining.

#### Milestones
- [ ] 5.1: Create fixture infrastructure — workflow fixtures follow the same pattern as recipe fixtures, with a `workflow.yaml` at the root and step recipe subdirectories
- [ ] 5.2: Implement 14 success-path fixtures: basic, three-steps, when-true, when-false, when-complex, vars-map, vars-override, vars-map-and-override, chain-create-inject, chain-create-patch, empty-steps, no-variables, idempotent, dry-run-chain
- [ ] 5.3: Implement 3 error-handling fixtures: on-error-stop, on-error-continue, on-error-report, step-on-error
- [ ] 5.4: Implement 7 error-case fixtures: missing-recipe, bad-yaml, missing-steps, ambiguous, bad-vars, when-undef, step-missing-var
- [ ] 5.5: Run full test suite (`cargo test`), verify all pass, update test count in CLAUDE.md

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

## Risks / Open Questions

- **Risk: Refactoring `cmd_run` for reuse.** The current `cmd_run` in `main.rs` handles recipe loading, variable validation, rendering, execution, and output formatting in one function. Workflow execution needs to call the recipe-execution part without the CLI-level concerns. May require extracting a `run_recipe()` function. Mitigation: do this extraction in Phase 3 milestone 3.2.
- **Risk: `virtual_files` carryover complexity.** In dry-run mode, each step's create operations populate virtual_files. Inject/patch/replace operations in later steps need to read from virtual_files for files that haven't been written to disk. The current ExecutionContext already supports this within a single recipe, but cross-step carryover needs testing for edge cases (step 1 creates, step 2 injects into virtual file, step 3 patches the same virtual file). Mitigation: dedicated fixture `workflow-dry-run-chain`.
- **Open question: Should `vars` values support Jinja2 rendering?** Currently specified as literal JSON values. Rendering them would allow `vars: { path: "{{ app }}/models" }` but adds complexity. Decision: no for v0.3. If needed, the template author can compute the value in the recipe's template instead.
- **Open question: Should simultaneous vars_map detect conflicts?** If `{a: c, b: c}` maps both `a` and `b` to `c`, which wins? Currently unspecified. Decision: last entry in insertion order wins (IndexMap guarantees order). Document this.
