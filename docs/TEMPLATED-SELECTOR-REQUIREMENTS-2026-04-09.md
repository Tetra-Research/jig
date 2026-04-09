# Templated Selector Requirements (2026-04-09)

Date: 2026-04-09  
Status: Proposed  
Priority: High  
Scope: Product improvement doc for post-compaction implementation planning

## Objective

Remove a core recipe-authoring constraint in `jig`: regex-bearing selector fields currently behave as parse-time literals instead of variable-rendered templates.

This doc defines:

1. the exact current constraint
2. why it matters for real recipe authoring
3. what should change in the product
4. what should not change
5. how to validate the improvement without conflating it with unrelated harness or skill issues

## Executive Summary

Today `jig` supports variable rendering in:

- template bodies
- target paths like `to`, `inject`, `replace`, `patch`
- `skip_if`

But `jig` does **not** support variable rendering in regex-bearing selector fields such as:

- `anchor.pattern`
- `before`
- `after`
- `between.start`
- `between.end`
- `replace.pattern`

That limitation is real in the product, not just in the eval harness.

The result is that authors cannot naturally express the most important general recipe pattern:

- "patch the class named `{{ model_name }}`"
- "patch the function named `{{ function_name }}`"

Instead they must choose between:

1. broad, underspecified anchors such as `^class `
2. hardcoded benchmark-specific anchors such as `^class Entity\(`
3. falling back to manual edits outside the library

This is a meaningful product limitation and should be improved.

There is a closely related follow-on requirement for `anchor.find`.

`anchor.find` is not a regex field, but it is still selector-bearing state because it narrows placement within the matched scope. If `anchor.pattern` is templated but `anchor.find` is not, generalized recipes remain partially hardcoded at the narrowing step.

There is also a second, separate limitation around control-flow-sensitive insertion points such as "insert before return". That issue is real, but it is not the same problem and should be treated as follow-on work after templated selector support.

## Background

Recent head-to-head eval work surfaced two different categories of recipe pain:

1. templated selector fields failed because the parser validates regex fields before variables are rendered
2. some insertion primitives were too weak to express the intended semantic placement cleanly

These surfaced most clearly in the head-to-head benchmark skills for:

- query-layer discipline
- structured logging

In the query-layer scenario, the natural recipe shape was:

- target `{{ models_file }}`
- patch the class named `{{ model_name }}`
- insert `objects = {{ manager_name }}()` inside that class

In the structured-logging scenario, the natural recipe shape was:

- target `{{ target_file }}`
- patch the function named `{{ function_name }}`
- insert log lines at semantically meaningful positions

The first category failed because `{{ ... }}` in regex-bearing selector fields is unsupported today. To make the eval continue, the recipes were temporarily narrowed to static anchors that only fit the benchmark fixtures. That solved the benchmark, but not the product limitation.

## Current Behavior

### Pipeline behavior

The implementation currently splits recipe handling into two phases:

1. **Recipe load / parse / validation**
2. **Variable validation and rendering**

During recipe load, regex-bearing selector fields are validated immediately as literal regex strings.

Relevant implementation points:

- `src/recipe.rs`
  - patch anchors require `anchor.pattern`
  - `validate_regex(...)` is called during recipe parsing
- `src/main.rs`
  - later renders template content, target paths, and `skip_if`
  - does not render selector regexes before execution
- `src/operations/patch.rs`
  - patch execution assumes `anchor.pattern` was already validated and can be compiled directly

This creates the current invariant:

- selector regexes must already be valid literal regexes before any recipe variables are applied

### Practical effect

The following works today:

```yaml
patch: "{{ app }}/models.py"
skip_if: "class {{ model_name }}"
```

The following does not work today:

```yaml
anchor:
  pattern: "^class {{ model_name }}\\("
```

because the parser attempts to compile the regex before `{{ model_name }}` is rendered.

## Product Constraint

The product constraint can be stated precisely:

> `jig` can template content and path-like fields, but it cannot template the regex selectors that tell it where to operate.

That is the constraint this doc addresses.

## Why This Matters

### 1. It blocks the core general-purpose patching use case

Many useful recipes need to target a code structure whose name comes from a variable:

- a model class
- a serializer class
- a request/response schema class
- a service function
- a route handler
- a migration symbol

Without templated selectors, these recipes cannot stay both:

- general
- precise

They become general but imprecise, or precise but hardcoded.

### 2. It conflicts with the current product story

The PRD already presents examples that imply templated selector patterns are intended, such as class-targeted anchors using `{{ model }}`.

That means current behavior is not just limited; it is surprising relative to the documented mental model.

### 3. It produces bad incentives for recipe authors

Authors are pushed toward:

- coarse anchors that can match the wrong structure
- benchmark-only static anchors
- manual corrective work by the LLM after a failed run

All three outcomes reduce trust in the library.

### 4. It increases token burn in exactly the wrong place

When an LLM writes the natural templated recipe and the parser rejects it, the model spends tokens:

- diagnosing parser behavior
- rewriting recipes around the limitation
- falling back to hand edits
- verifying whether the library or the recipe is at fault

That is product friction, not productive templating work.

### 5. It weakens eval signal quality

If benchmark recipes must be narrowed to static anchors in order to run, then benchmark results start to measure:

- how well a benchmark-specific recipe was hand-fit to a fixture

instead of:

- how much value the generalized library abstraction provides

This matters for product decisions and for fair comparisons against non-library skills.

## What This Problem Is Not

This doc is **not** primarily about control-flow-aware insertion.

Example:

- `before_close` means "before the closing delimiter or dedent"
- it does **not** mean "before the return"

That is a separate expressiveness issue. It matters, but it is not the same as templated selector rendering.

This doc is also **not** about:

- changing eval harness scoring
- improving skill prompt wording
- fixing shell ergonomics for invoking `jig`

Those are valuable, but they are separate tracks.

## Goals

1. Allow variable rendering in regex-bearing selector fields.
2. Allow variable rendering in `anchor.find` as part of prepared selector state.
3. Preserve strong validation and clear error reporting.
4. Keep recipe behavior deterministic and debuggable.
5. Preserve existing static-selector recipes without breakage.
6. Make generalized recipes materially easier to author.

## Non-Goals

1. Do not redesign the full anchor model in this change.
2. Do not add AST-aware language parsing in this change.
3. Do not solve all control-flow-sensitive insertion cases in this change.
4. Do not silently change semantics of existing position heuristics.
5. Do not make `jig validate` require live execution against a filesystem.

## Required Product Outcome

After this improvement, authors must be able to write recipes like:

```yaml
files:
  - template: manager_attr.py.j2
    patch: "{{ models_file }}"
    anchor:
      pattern: "^class {{ model_name | regex_escape }}\\("
      scope: class_body
      position: before
    skip_if: "objects = {{ manager_name }}()"
```

and:

```yaml
files:
  - template: log_start.py.j2
    patch: "{{ target_file }}"
    anchor:
      pattern: "^def {{ function_name | regex_escape }}\\("
      scope: function_body
      position: before
    skip_if: "\"{{ event_namespace }}.start\""
```

with product-supported behavior, not undocumented fallback behavior.

Authors must also be able to write recipes like:

```yaml
files:
  - template: admin_field.py.j2
    patch: "{{ admin_file }}"
    anchor:
      pattern: "^class {{ model_name | regex_escape }}Admin"
      scope: class_body
      find: "{{ target_attribute }}"
      position: before_close
    skip_if: "{{ field_name }}"
```

where `target_attribute` is rendered before narrowing within the matched class scope.

## Design Requirements

### R1. Selector-bearing fields must support variable rendering

The following fields must support variable rendering:

- `anchor.pattern`
- `anchor.find`
- `before`
- `after`
- `between.start`
- `between.end`
- `replace.pattern`

If additional selector-bearing fields are added later, they should follow the same rendering model, with validation appropriate to their type.

### R2. Rendering must happen before regex compilation

Rendered selector values must be compiled and validated only after variables are available.

The current behavior validates these fields too early. That must change.

For `anchor.find`, rendering must happen during the same preparation stage, but no regex compilation is required.

### R3. Static recipes must remain valid

Existing recipes with literal regex selectors must continue to validate and execute with unchanged semantics.

### R4. Error reporting must remain specific

When a rendered selector is invalid, the error must identify:

1. the recipe field location
2. the rendered value that failed
3. the regex compilation reason
4. a concrete hint for the likely fix

This is especially important once regexes can be assembled from variables.

For rendered `anchor.find` handling, the product should also report:

1. the recipe field location
2. the rendered narrowing value when useful
3. whether the failure was:
   - invalid rendered narrowing input
   - or a normal "find string not found within scope" execution miss

### R5. Validation must still be useful before execution

`jig validate` must remain valuable even when selector fields are templated.

That likely means validation must distinguish between:

- structurally valid recipe schema
- renderable selector expressions
- fully validated rendered regexes when variable values are available

We should not regress to a world where recipes appear valid but fail cryptically at run time.

### R6. Safe literal interpolation must be supported

The product should provide a filter for escaping literal strings for regex insertion.

Recommended filter name:

- `regex_escape`

Without this, templated selector support will encourage dangerous or confusing authoring patterns such as interpolating `Foo+Bar` or `User[Legacy]` directly into regex syntax.

### R7. Prepared operations must carry rendered selector state

The execution layer should operate on already-rendered selector fields, not on the original template strings.

Today prepared operations mainly carry:

- rendered content
- rendered path
- rendered `skip_if`

This improvement likely requires prepared operations to also carry rendered selector data.

That rendered selector data should include both regex-bearing fields and `anchor.find`.

### R8. JSON and human output must preserve debuggability

When useful, run output should expose enough information to understand the resolved target:

- rendered path
- rendered selector pattern
- location/scoping diagnostics when an anchor misses

This is especially important for LLM-assisted workflows.

## Candidate Implementation Shape

This section is not intended as final design law, but it is the most direct path that fits the current architecture.

### Phase A: Separate parsed recipe schema from executable selector state

Keep `Recipe::load(...)` responsible for:

- YAML parsing
- field-shape validation
- enum/variant validation
- required field presence

Do **not** require regex-bearing selector fields to be fully compilable literals at this stage if they contain template expressions.

Possible approach:

1. Store raw selector template strings in the parsed recipe.
2. Defer compilation of regex-bearing fields until a preparation stage after variable validation.

### Phase B: Render selector fields during operation preparation

During the same preparation stage that currently renders:

- template content
- paths
- `skip_if`

also render:

- anchor pattern
- anchor find string
- inject before/after regex
- replace/between regexes

This likely belongs in the `PreparedOp` creation flow in `src/main.rs` and the workflow equivalent in `src/workflow.rs`.

### Phase C: Validate rendered selectors

After rendering selector fields:

1. reject empty selectors where regex is required
2. compile regexes
3. surface errors with recipe field context plus rendered value

This restores early-ish failure while preserving template support.

For `anchor.find`:

1. render before execution
2. reject empty rendered values
3. surface field-specific errors before file mutation when the rendered value is invalid

### Phase D: Execute using rendered selectors

Operations should consume rendered selector state rather than re-reading raw strings from `Recipe`.

That may require:

- a `PreparedAnchor`
- rendered inject modes
- rendered replace specs

`PreparedAnchor` should include rendered `pattern` and rendered `find`.

### Phase E: Add regex-safe authoring support

Implement a `regex_escape` filter and document when to use it.

Rule of thumb:

- if the variable value should match literally, require `regex_escape`
- if the variable value is intended to be regex syntax, allow raw interpolation

## Validation Model

The product should distinguish between three kinds of validity.

### 1. Schema validity

Questions:

- Are the required fields present?
- Is the operation shape legal?
- Are enum values valid?

This should remain available from plain `jig validate`.

### 2. Renderability

Questions:

- Do the selector templates reference defined variables?
- Can they be rendered given the declared variables?

That includes `anchor.find`, not just regex-bearing selector fields.

Depending on the current templating model, this may be checked partially or completely.

### 3. Concrete regex validity

Questions:

- After variables are supplied, do the rendered selectors compile as regex?

This may require:

- a `validate --vars ...` mode
- or richer reporting during `run`

The exact CLI shape can be decided in implementation, but the product needs a clean story here.

## CLI and UX Requirements

### Validation UX

Preferred outcome:

- `jig validate recipe.yaml` checks schema-level validity and reports selector fields that are templated and therefore deferred
- `jig validate recipe.yaml --vars '{...}'` fully validates rendered selector regexes

If a lighter first version is needed, the minimum acceptable behavior is:

- `jig run` must produce precise rendered-regex errors
- `jig validate` must not misleadingly imply that a templated selector is already fully regex-validated

### Error message UX

Error messages for rendered selector failures should look roughly like:

- which recipe field failed
- what the rendered selector became
- why regex compilation failed
- whether `regex_escape` is the likely fix

Example failure classes:

- unescaped metacharacters
- empty rendered selector
- invalid grouping or brackets
- empty rendered `anchor.find`

## Backward Compatibility Requirements

1. Static selector recipes must continue to work unchanged.
2. Existing command lines for `jig run` must continue to work unchanged.
3. Existing command lines for `jig validate` must continue to work unchanged.
4. New behavior should be additive unless a previous error path was clearly incorrect or misleading.

## Documentation Requirements

The product docs should be updated in these places:

1. `PRD.md`
   - align examples with actual supported behavior
   - if templated selectors are implemented, keep the existing examples and make them explicit

2. `docs/ARCHITECTURE.md`
   - update the pipeline description to show selector rendering and validation timing
   - document prepared rendered selector state

3. Authoring examples
   - include at least one class-targeted patch using `regex_escape`
   - include at least one function-targeted patch using a templated anchor

4. Error docs or troubleshooting notes
   - explain when to use `regex_escape`
   - explain the difference between literal regex intent and literal string matching intent

## Testing Requirements

### Unit tests

Add unit coverage for:

1. templated `anchor.pattern` renders successfully with provided vars
2. templated `anchor.find` renders successfully with provided vars
3. templated `before` renders successfully with provided vars
4. templated `after` renders successfully with provided vars
5. templated `between.start` and `between.end` render successfully
6. invalid rendered regex fails with clear error
7. empty rendered regex fails with clear error
8. empty rendered `anchor.find` fails with clear error
9. `regex_escape` makes literal interpolation safe

### Integration tests

Add end-to-end recipe tests for:

1. class-body patch targeted by templated class name
2. class-body patch targeted by templated class name plus templated `anchor.find`
3. function-body patch targeted by templated function name
4. inject-before targeted by templated regex
5. replace/between targeted by templated regex
6. validation behavior with and without vars

### Regression tests

Preserve coverage for:

1. existing static regex selectors
2. existing static `anchor.find` behavior
3. path rendering behavior
4. `skip_if` rendering behavior
5. current fail-fast execution semantics

## Acceptance Criteria

The acceptance criteria below use EARS-style wording where practical.

1. **When** a recipe contains a templated `anchor.pattern` and valid variables are provided, **the system shall** render the selector and execute the patch against the rendered target.
2. **When** a recipe contains a templated `anchor.find` and valid variables are provided, **the system shall** render the narrowing string before scope narrowing and use the rendered value during patch placement.
3. **When** a recipe contains a templated `before` or `after` selector and valid variables are provided, **the system shall** render the selector and use the rendered regex during injection.
4. **When** a recipe contains templated `between.start`, `between.end`, or `replace.pattern` fields and valid variables are provided, **the system shall** render and validate those selectors before execution.
5. **When** a rendered selector regex is invalid, **the system shall** fail before file mutation and report the recipe field, rendered selector, and regex compilation error.
6. **When** a rendered selector becomes empty, **the system shall** fail before file mutation with an explicit empty-selector error.
7. **When** a rendered `anchor.find` becomes empty, **the system shall** fail before file mutation with an explicit empty-narrowing error.
8. **When** a static-selector recipe that works today is executed after this change, **the system shall** preserve the existing behavior.
9. **When** a recipe interpolates a literal identifier into a selector using `regex_escape`, **the system shall** treat regex metacharacters in that identifier literally.
10. **If** `jig validate` is run without concrete variables for a recipe that contains templated selectors, **then the system shall** avoid falsely claiming that the selector regexes have already been fully validated.
11. **When** `jig validate` is run with concrete variables for a recipe that contains templated selectors, **the system shall** validate the rendered selectors and report any selector-specific failures clearly.

## Recommended Implementation Order

1. Refactor preparation model to support rendered selector state.
2. Add rendered-selector validation in the prepare/run path.
3. Extend prepared-anchor handling to include rendered `anchor.find`.
4. Add `regex_escape`.
5. Add `validate --vars` or equivalent selector-aware validation flow.
6. Update docs and examples.
7. Rerun the head-to-head scenarios that previously required static selector narrowing.

## Risks

### Risk 1: Validation becomes less clear

If implementation only defers errors to runtime without improving diagnostics, the product may technically support templated selectors while becoming harder to debug.

Mitigation:

- implement explicit rendered-selector validation
- improve structured errors

### Risk 2: Regex escaping becomes a new footgun

If users interpolate raw values into regexes without guidance, they will produce confusing failures.

Mitigation:

- add `regex_escape`
- document it prominently
- mention it in invalid-regex hints

### Risk 3: Partial implementation only fixes patch anchors

If we implement templated `anchor.pattern` but leave `anchor.find`, `before`, `after`, and `between` behind, recipe authoring remains inconsistent.

Mitigation:

- implement selector rendering across all regex-bearing selector fields in one coherent pass

### Risk 4: Internal data model becomes split and messy

If both raw and rendered selector state are used inconsistently, debugging will get harder.

Mitigation:

- establish a clear rule:
  - parsed recipe stores raw selector templates
  - prepared operations carry rendered selector state
  - execution operates only on prepared state

## Out of Scope but Closely Related Follow-On

After templated selectors are implemented, we should evaluate whether to improve control-flow-aware patch semantics such as:

- `before_first_return`
- `before_all_returns`
- `after_last_assignment`

That is a separate product decision. It should not block the selector templating work.

## Open Questions

1. Should plain `jig validate` report templated selectors as "deferred validation" or "conditionally valid"?
2. Should `regex_escape` be mandatory in examples whenever the intent is literal name matching?
3. Should rendered selector patterns be included in JSON output on success, or only on verbose/debug paths?
4. Should workflow validation support full rendered-selector validation per step when vars are supplied at workflow level?

## Proposed File Touches

Likely implementation surfaces:

- `src/recipe.rs`
- `src/main.rs`
- `src/workflow.rs`
- `src/operations/mod.rs`
- `src/operations/inject.rs`
- `src/operations/replace.rs`
- `src/operations/patch.rs`
- `src/renderer.rs`
- `src/filters.rs`
- `docs/ARCHITECTURE.md`
- `PRD.md`

## Recommended Decision

Implement templated selector support as a first-class product capability.

Rationale:

1. it resolves a real limitation rather than a benchmark artifact
2. it aligns implementation with the documented product model
3. it enables generalized recipes without broad unsafe anchors
4. it reduces LLM token waste caused by fighting the tool instead of using it

## Appendix: Short-Term Benchmark Guidance

Until this improvement lands:

1. benchmark-specific skills may use static anchors when the benchmark fixture is intentionally fixed
2. those recipes should be treated as narrowed benchmark scaffolding, not evidence that the general product problem is solved
3. future benchmark analysis should avoid overstating wins achieved by hardcoded selector anchoring
