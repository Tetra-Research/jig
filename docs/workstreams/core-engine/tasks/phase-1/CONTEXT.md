# Phase 1: Skeleton + Recipe Parsing

> Workstream: core-engine
> Generated: 2026-04-03
> Source: PLAN.md

## Phase Plan

## Phase 1: Skeleton + Recipe Parsing
Status: Planned
Traces to: FR-1, FR-7 (partial), NFR-4, NFR-5

Bootstrap the Rust crate, wire up clap, and make recipe parsing work end-to-end. After this phase, `jig validate` and `jig vars` are functional commands.

#### Milestones
- [ ] 1.1: Cargo.toml with dependencies (serde, serde_yaml, serde_json, clap, thiserror, regex, indexmap)
- [ ] 1.2: `src/error.rs` — StructuredError struct (what/where/why/hint), JigError enum wrapping StructuredError with exit code mapping (0-4)
- [ ] 1.3: `src/recipe.rs` — Recipe, VariableDecl, VarType, FileOp structs with serde deserialization; template path resolution relative to recipe location; structural validation (missing fields, missing template files); unknown operation type detection with clear error message; custom deserialization for FileOp — use intermediate flat struct with optional fields, then validate and convert to typed enum (reject when more than one of `to`/`inject`/`replace`/`patch` is present, or none is present; if `replace` or `patch` is present, emit AC-1.10 'not supported in v0.1' error, reject when multiple inject modes (after/before/prepend/append) are specified); compile-check regex patterns in after/before fields during recipe validation
- [ ] 1.4: `src/variables.rs` — Variable merging and type-checking scaffolding (types imported from recipe.rs, validation logic added in Phase 2)
- [ ] 1.5: `src/main.rs` — clap CLI with `validate` and `vars` subcommands, `#[command(version)]`; wire recipe parsing; map errors to exit codes; `jig validate` outputs summary to stderr (variable count, operation types); with `--json`, outputs structured JSON to stdout
- [ ] 1.6: Unit tests for recipe parsing — valid recipe, missing fields, malformed YAML, missing template files, optional metadata

#### Validation Criteria
- `jig validate recipe.yaml` parses the example recipe from jig.md and exits 0
- `jig validate bad.yaml` exits 1 with a clear error naming the problem
- `jig vars recipe.yaml` outputs JSON matching the SPEC schema (type, required, default, description)
- Template paths resolve relative to recipe file, not cwd
- AC-1.1 through AC-1.15 have corresponding unit tests (AC-1.11 empty files array tested in Phase 3 when `jig run` exists)
- `jig validate` output includes variable names and operation types
- AC-N5.1 exit codes are correct for recipe validation errors

#### Key Files
- `Cargo.toml`
- `src/main.rs`
- `src/recipe.rs`
- `src/variables.rs` (types only)
- `src/error.rs`

#### Dependencies
- regex crate (needed at parse time for compile-checking after/before patterns)

---

## Relevant Acceptance Criteria

Extracted from SPEC.md for: FR-1 FR-7 NFR-4 NFR-5

### #### FR-1: Recipe Parsing

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-1.1 | Event | WHEN a valid recipe YAML is provided, the system SHALL parse it into a Recipe struct containing name, description, variables, and files | TEST-1.1 |
| AC-1.2 | Event | WHEN a recipe declares variables with type, required, default, description, values (enum), and items (array), the system SHALL parse all fields into the VariableDecl struct | TEST-1.2 |
| AC-1.3 | Event | WHEN a recipe declares `create` file operations with template, to, and skip_if_exists fields, the system SHALL parse them as FileOp::Create | TEST-1.3 |
| AC-1.4 | Event | WHEN a recipe declares `inject` file operations with template, inject, after/before/prepend/append, at, and skip_if fields, the system SHALL parse them as FileOp::Inject | TEST-1.4 |
| AC-1.5 | Unwanted | IF the recipe YAML is malformed (invalid YAML syntax), the system SHALL exit with code 1 and an error message identifying the parse failure location | TEST-1.5 |
| AC-1.6 | Unwanted | IF a required field is missing from the recipe (e.g., files with no template), the system SHALL exit with code 1 and name the missing field | TEST-1.6 |
| AC-1.7 | Event | WHEN a recipe references template files, the system SHALL resolve those paths relative to the recipe file location, not the working directory | TEST-1.7 |
| AC-1.8 | Unwanted | IF a referenced template file does not exist at the resolved path, the system SHALL exit with code 1 and report which template is missing and where it looked | TEST-1.8 |
| AC-1.9 | Event | WHEN the recipe has optional metadata fields (name, description), the system SHALL accept recipes with or without them | TEST-1.9 |
| AC-1.10 | Unwanted | IF the recipe contains an unknown operation type (e.g., `patch`, `replace`), the system SHALL exit with code 1 and report "unknown operation type '<name>' — this operation is not supported in v0.1" | TEST-1.10 |
| AC-1.11 | Event | WHEN the recipe has an empty `files: []` array, the system SHALL exit 0 with an empty operations array | TEST-1.11 |
| AC-1.12 | Event | WHEN the recipe has no `variables` key or an empty `variables` map, the system SHALL accept the recipe as valid | TEST-1.12 |
| AC-1.13 | Unwanted | IF the recipe file does not exist, the system SHALL exit with code 1 naming the missing path | TEST-1.13 |
| AC-1.14 | Unwanted | IF a file operation contains more than one of `to`/`inject`/`replace`/`patch` fields, the system SHALL exit with code 1 reporting the ambiguous operation type | TEST-1.14 |
| AC-1.15 | Unwanted | IF a file operation contains none of `to`/`inject`/`replace`/`patch` fields, the system SHALL exit with code 1 reporting the missing operation type | TEST-1.15 |

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

### #### NFR-4: Structured Errors with Rendered Content

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N4.1 | Ubiquitous | The system SHALL include what, where, why, and hint fields in every error message | TEST-N4.1 |
| AC-N4.2 | Event | WHEN a file operation fails (exit code 3), the system SHALL include the rendered template content in the error output so the caller can fall back to manual editing. This is independent of `--verbose` — rendered content in errors is always present | TEST-N4.2 |
| AC-N4.3 | Event | WHEN a template rendering error occurs, the system SHALL report the template file path and the line number of the error | TEST-N4.3 |
| AC-N4.4 | Event | WHEN a variable validation error occurs, the system SHALL report the variable name, expected type, and actual value provided | TEST-N4.4 |

### #### NFR-5: Stable Exit Codes

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N5.1 | Ubiquitous | The system SHALL exit with code 0 on success, 1 for recipe validation errors, 2 for template rendering errors, 3 for file operation errors, 4 for variable validation errors | TEST-N5.1 |
| AC-N5.2 | Ubiquitous | The system SHALL use the exit code corresponding to the first pipeline stage that fails: recipe validation (1) before variable validation (4) before template rendering (2) before file operations (3) | TEST-N5.2 |

## Execution Context

This is the first phase. No prior artifacts exist — bootstrap from scratch.

## Invariants

Refer to `docs/INVARIANTS.md` for project-wide constraints that must be honored.

## Architecture

Refer to `docs/ARCHITECTURE.md` for module boundaries and design decisions.

