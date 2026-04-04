# Phase 2: Variable Validation + Template Rendering

> Workstream: core-engine
> Generated: 2026-04-03
> Source: PLAN.md

## Phase Plan

## Phase 2: Variable Validation + Template Rendering
Status: Planned
Traces to: FR-2, FR-3, FR-7 (partial), NFR-1 (partial)

Full variable pipeline (parse, merge, type-check) and template rendering with all 13 built-in filters. After this phase, `jig render` works.

#### Milestones
- [ ] 2.1: `src/variables.rs` — full validation: parse JSON from --vars/--vars-file/--vars-stdin; merge with precedence (defaults < file < stdin < inline); type-check against declarations; required field enforcement; enum validation; array item type validation
- [ ] 2.2: `src/filters.rs` — all 13 built-in filters registered with minijinja: snakecase, camelcase, pascalcase, kebabcase, upper, lower, capitalize, replace, pluralize, singularize, quote, indent, join
- [ ] 2.3: `src/renderer.rs` — minijinja Environment setup; template loading from recipe-relative paths; filter registration; render with variables context; structured error on undefined variable (with "did you mean?" via edit distance) and syntax errors (with file + line)
- [ ] 2.4: Wire `render` subcommand in main.rs with --vars, --vars-file, --vars-stdin, --to options. For `jig render`, create a standalone Environment with filters registered but no template directory — load the template file directly by path via `render_str()` or equivalent
- [ ] 2.5: Unit tests for variable validation — every VarType, required missing, default fallback, enum rejection, array item mismatch, merge precedence
- [ ] 2.6: Unit tests + insta snapshot tests for all 13 filters and template rendering (conditionals, loops, comments, raw blocks, undefined vars, syntax errors)

#### Validation Criteria
- All 13 filters produce correct output per AC-3.4 through AC-3.14
- `jig render template.j2 --vars '{"class_name": "BookingService"}'` renders to stdout
- Type mismatch exits 4 with expected vs actual type (AC-2.7)
- Missing required variable exits 4 with variable name and hint (AC-2.5)
- Merge precedence: inline --vars wins over --vars-stdin wins over --vars-file wins over defaults (AC-2.4)
- Undefined template variable exits 2 with "did you mean?" hint (AC-3.17)
- Template syntax error exits 2 with file path and line number (AC-3.18)
- Same inputs produce byte-identical output across runs (AC-N1.1)
- `jig render template.j2 --vars '...' --to output.txt` writes to file instead of stdout (AC-7.4)
- Multiple validation errors accumulated and reported together (AC-2.11)

#### Key Files
- `src/variables.rs` (full implementation)
- `src/filters.rs`
- `src/renderer.rs`

#### Dependencies
- heck crate for case conversions
- minijinja crate for template rendering

---

## Relevant Acceptance Criteria

Extracted from SPEC.md for: FR-2 FR-3 FR-7 NFR-1

### #### FR-2: Variable Validation

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-2.1 | Event | WHEN `--vars` is provided with a JSON string, the system SHALL parse it as the variable input | TEST-2.1 |
| AC-2.2 | Event | WHEN `--vars-file` is provided with a path to a JSON file, the system SHALL read and parse the file as variable input | TEST-2.2 |
| AC-2.3 | Event | WHEN `--vars-stdin` is provided, the system SHALL read JSON from stdin as variable input | TEST-2.3 |
| AC-2.4 | Event | WHEN multiple variable sources are provided, the system SHALL merge with precedence: recipe defaults < vars-file < vars-stdin < inline --vars | TEST-2.4 |
| AC-2.5 | Event | WHEN a variable is declared as `required: true` and no value is provided (after merging), the system SHALL exit with code 4 naming the missing variable and providing a hint | TEST-2.5 |
| AC-2.6 | Event | WHEN a variable is declared with a `default` and no value is provided, the system SHALL use the default value | TEST-2.6 |
| AC-2.7 | Unwanted | IF a provided variable value does not match its declared type (e.g., string given for number, object given for array), the system SHALL exit with code 4 with expected vs actual type | TEST-2.7 |
| AC-2.8 | Event | WHEN a variable is declared as `type: enum` with `values: [a, b, c]`, the system SHALL reject values not in the allowed set with exit code 4 | TEST-2.8 |
| AC-2.9 | Event | WHEN a variable is declared as `type: array` with `items: string`, the system SHALL validate that each array element matches the item type | TEST-2.9 |
| AC-2.10 | Ubiquitous | The system SHALL accept all six variable types: string, number, boolean, array, object, enum | TEST-2.10 |
| AC-2.11 | Ubiquitous | The system SHALL accumulate all variable validation errors and report them together with exit code 4 | TEST-2.11 |
| AC-2.12 | Ubiquitous | The system SHALL accept variable input containing keys not declared in the recipe's variables section without error or warning | TEST-2.12 |
| AC-2.13 | Unwanted | IF `--vars` contains invalid JSON, the system SHALL exit with code 4 with a parse error identifying the location | TEST-2.13 |
| AC-2.14 | Unwanted | IF `--vars-file` points to a nonexistent file, the system SHALL exit with code 4 naming the missing path | TEST-2.14 |
| AC-2.15 | Unwanted | IF `--vars-file` points to a file containing invalid JSON, the system SHALL exit with code 4 with a parse error identifying the file path and the JSON error location | TEST-2.15 |
| AC-2.16 | Event | WHEN no variable sources are provided (no `--vars`, `--vars-file`, or `--vars-stdin`), the system SHALL use an empty object as input and apply recipe defaults | TEST-2.16 |

### #### FR-3: Template Rendering

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-3.1 | Event | WHEN a template is rendered with a variables context, the system SHALL substitute all `{{ variable }}` expressions with their values | TEST-3.1 |
| AC-3.2 | Event | WHEN a template contains `{% if %}` / `{% else %}` / `{% endif %}` blocks, the system SHALL evaluate them correctly against the variable values | TEST-3.2 |
| AC-3.3 | Event | WHEN a template contains `{% for item in list %}` loops, the system SHALL iterate and render the body for each element | TEST-3.3 |
| AC-3.4 | Ubiquitous | The system SHALL register and support all 13 built-in filters: snakecase, camelcase, pascalcase, kebabcase, upper, lower, capitalize, replace, pluralize, singularize, quote, indent, join | TEST-3.4 |
| AC-3.5 | Event | WHEN `snakecase` is applied to "BookingService", the system SHALL produce "booking_service" | TEST-3.5 |
| AC-3.6 | Event | WHEN `camelcase` is applied to "booking_service", the system SHALL produce "bookingService" | TEST-3.6 |
| AC-3.7 | Event | WHEN `pascalcase` is applied to "booking_service", the system SHALL produce "BookingService" | TEST-3.7 |
| AC-3.8 | Event | WHEN `kebabcase` is applied to "BookingService", the system SHALL produce "booking-service" | TEST-3.8 |
| AC-3.9 | Event | WHEN `replace` is applied as `"a.b.c" \| replace('.', '/')`, the system SHALL produce "a/b/c" | TEST-3.9 |
| AC-3.10 | Event | WHEN `pluralize` is applied to "hotel", the system SHALL produce "hotels" | TEST-3.10 |
| AC-3.11 | Event | WHEN `singularize` is applied to "hotels", the system SHALL produce "hotel" | TEST-3.11 |
| AC-3.12 | Event | WHEN `quote` is applied to "hello", the system SHALL produce `"hello"` (with literal quotes) | TEST-3.12 |
| AC-3.13 | Event | WHEN `indent(4)` is applied to a multiline string, the system SHALL indent each line by 4 spaces, including the first line. Use `indent(4, first=false)` to skip the first line. Note: this diverges from Jinja2 convention where indent() skips the first line by default | TEST-3.13 |
| AC-3.14 | Event | WHEN `join(", ")` is applied to `["a", "b", "c"]`, the system SHALL produce "a, b, c" | TEST-3.14 |
| AC-3.15 | Event | WHEN `{# comment #}` appears in a template, the system SHALL strip it from the output | TEST-3.15 |
| AC-3.16 | Event | WHEN `{% raw %}...{% endraw %}` appears in a template, the system SHALL output the content literally without interpreting Jinja2 syntax | TEST-3.16 |
| AC-3.17 | Unwanted | IF a template references an undefined variable, the system SHALL exit with code 2 and include a "did you mean?" hint when a close match exists among the keys in the provided variable context (Levenshtein distance ≤ 3) | TEST-3.17 |
| AC-3.18 | Unwanted | IF a template has a Jinja2 syntax error, the system SHALL exit with code 2 and report the template file path and line number | TEST-3.18 |

### #### FR-7: CLI Interface

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-7.1 | Event | WHEN `jig validate <recipe>` is invoked, the system SHALL parse the recipe and report whether it is valid, listing variables and operations | TEST-7.1 |
| AC-7.2 | Event | WHEN `jig vars <recipe>` is invoked, the system SHALL output the expected variables as a JSON object with type, required, default, and description fields | TEST-7.2 |
| AC-7.3 | Event | WHEN `jig render <template> --vars '<json>'` is invoked, the system SHALL render the template with the given variables and output the result to stdout. Note: `jig render` operates without recipe context — variable type validation is not available, but "did you mean?" hints work against the provided variable keys. | TEST-7.3 |
| AC-7.4 | Event | WHEN `jig render` is invoked with `--to <path>`, the system SHALL write the rendered output to the specified file instead of stdout | TEST-7.4 |
| AC-7.5 | Event | WHEN `jig run <recipe> --vars '<json>'` is invoked, the system SHALL execute all file operations in the recipe in declaration order | TEST-7.5 |
| AC-7.6 | Ubiquitous | The system SHALL accept global options: `--vars`, `--vars-file`, `--vars-stdin`, `--dry-run`, `--json`, `--quiet`, `--force`, `--base-dir`, `--verbose`, `--version` | TEST-7.6 |
| AC-7.7 | Event | WHEN `--version` is specified, the system SHALL print the version string and exit with code 0 | TEST-7.7 |

### #### NFR-1: Deterministic Output

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N1.1 | Ubiquitous | The system SHALL produce byte-identical output when given the same recipe, variables, and existing files across multiple runs | TEST-N1.1 |
| AC-N1.2 | Ubiquitous | The system SHALL not include timestamps, random values, or machine-specific identifiers in rendered output | TEST-N1.2 |

## Execution Context

This phase builds on Phase 1. Assume all prior phase artifacts exist and tests pass.

## Invariants

Refer to `docs/INVARIANTS.md` for project-wide constraints that must be honored.

## Architecture

Refer to `docs/ARCHITECTURE.md` for module boundaries and design decisions.

