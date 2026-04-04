# Phase 3: Create Operation + Output Formatting

> Workstream: core-engine
> Generated: 2026-04-03
> Source: PLAN.md

## Phase Plan

## Phase 3: Create Operation + Output Formatting
Status: Planned
Traces to: FR-4, FR-6, FR-7 (complete), NFR-2 (partial), NFR-4, NFR-6

File creation with templated output paths, directory auto-creation, skip_if_exists, and dual-stream output (JSON stdout / human stderr). After this phase, `jig run` works for create-only recipes.

#### Milestones
- [ ] 3.1: `src/operations/mod.rs` — ExecutionContext struct (base_dir, dry_run, force, virtual_files for dry-run state); OpResult enum (Success with action/path/lines/location, Skip, Error); operation dispatch (create only initially). In dry-run mode, create ops populate virtual_files instead of writing to disk.
- [ ] 3.2: `src/operations/create.rs` — render `to` path as template; create parent directories; write rendered content; skip_if_exists logic; --force override; --base-dir path resolution
- [ ] 3.3: `src/output.rs` — OutputMode enum (Json/Human/Quiet); TTY auto-detection; JSON serialization of operations array + files_written/files_skipped; track per-path write/skip status for files_written and files_skipped arrays; colored human output to stderr; --verbose includes rendered content
- [ ] 3.4: Wire `run` subcommand in main.rs with --dry-run, --json, --quiet, --force, --base-dir, --verbose; render all template contents AND templated path fields (`to`, `inject`, `skip_if`) into a Vec before executing any operation — rendering failures abort before any file is touched; execute operations in declaration order; fail-fast on operation errors — stop execution on first failure
- [ ] 3.5: Unit tests for create operation — happy path, skip_if_exists, force overwrite, templated paths, directory creation, file-already-exists error
- [ ] 3.6: Unit tests for output formatting — JSON schema matches SPEC, human output, quiet mode, verbose mode, dry-run

#### Validation Criteria
- `jig run recipe.yaml --vars '...'` creates files at templated paths (AC-4.1, AC-4.2)
- Parent directories created automatically (AC-4.3)
- skip_if_exists: true skips existing files with action:"skip" (AC-4.4)
- Default (skip_if_exists: false) errors on existing file without --force (AC-4.5)
- --force overwrites regardless (AC-4.6)
- --base-dir changes output root (AC-4.7)
- --dry-run produces output but writes nothing (AC-6.8)
- JSON output when piped, human output when TTY (AC-6.1, AC-6.2)
- --json forces JSON (AC-6.3), --quiet suppresses non-errors (AC-6.4)
- Operations execute in declaration order (AC-N6.1)
- Second run with skip_if_exists: true reports all skips (AC-N2.1)

#### Key Files
- `src/operations/mod.rs`
- `src/operations/create.rs`
- `src/output.rs`
- `src/main.rs` (run subcommand)

#### Dependencies
- owo-colors crate for terminal coloring

---

## Relevant Acceptance Criteria

Extracted from SPEC.md for: FR-4 FR-6 FR-7 NFR-2 NFR-4 NFR-6

### #### FR-4: Create Operation

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-4.1 | Event | WHEN a create operation executes, the system SHALL render the template and write the result to the path specified in `to` | TEST-4.1 |
| AC-4.2 | Event | WHEN the `to` path contains Jinja2 expressions (e.g., `tests/{{ module \| replace('.', '/') }}/test_{{ class_name \| snakecase }}.py`), the system SHALL render the path before writing | TEST-4.2 |
| AC-4.3 | Event | WHEN parent directories in the `to` path do not exist, the system SHALL create them automatically | TEST-4.3 |
| AC-4.4 | Event | WHEN `skip_if_exists: true` and the target file already exists, the system SHALL skip the operation and report `"action": "skip"` with a reason | TEST-4.4 |
| AC-4.5 | Unwanted | IF `skip_if_exists: false` (default) and the target file already exists and `--force` is not set, the system SHALL exit with code 3 reporting the conflict | TEST-4.5 |
| AC-4.6 | Event | WHEN `--force` is set and the target file already exists, the system SHALL overwrite the file regardless of skip_if_exists | TEST-4.6 |
| AC-4.7 | Event | WHEN `--base-dir` is set, the system SHALL resolve `to` paths relative to the base directory instead of the working directory | TEST-4.7 |
| AC-4.8 | Event | WHEN a create operation succeeds, the system SHALL report `"action": "create"` with the path and line count (number of lines in the written file) | TEST-4.8 |
| AC-4.9 | Unwanted | IF a filesystem write fails due to permissions, the system SHALL exit with code 3 with the path and permission error | TEST-4.9 |
| AC-4.10 | Unwanted | IF `--base-dir` specifies a directory that does not exist, the system SHALL exit with code 3 naming the missing directory | TEST-4.10 |

### #### FR-6: Output Formatting

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-6.1 | State | WHILE stdout is a TTY, the system SHALL output human-readable colored text to stderr and produce no stdout output | TEST-6.1 |
| AC-6.2 | State | WHILE stdout is piped (not a TTY), the system SHALL output JSON to stdout and produce no stderr output except errors | TEST-6.2 |
| AC-6.3 | Event | WHEN `--json` is specified, the system SHALL force JSON output to stdout regardless of TTY detection | TEST-6.3 |
| AC-6.4 | Event | WHEN `--quiet` is specified, the system SHALL suppress all stderr output. Stdout behavior is determined by `--json` independently | TEST-6.4 |
| AC-6.5 | Event | WHEN JSON output is produced, the system SHALL include an `operations` array with action, path, lines (for success), reason (for skip), or error details (for error) for each operation | TEST-6.5 |
| AC-6.6 | Event | WHEN JSON output is produced, the system SHALL include `files_written` and `files_skipped` summary arrays containing unique file paths. A path appears in `files_written` if any operation wrote to it. A path appears in `files_skipped` only if all operations targeting it were skipped. In dry-run mode, `files_written` lists paths that would have been written. Paths SHALL appear in the order they were first encountered during operation execution. | TEST-6.6 |
| AC-6.7 | Event | WHEN `--verbose` is specified, the system SHALL include rendered template content in the output | TEST-6.7 |
| AC-6.8 | Event | WHEN `--dry-run` is specified, the system SHALL produce identical output format but write no files to disk. Create operations record their output paths and rendered content in a virtual file state. Inject operations targeting a path created earlier in the same dry-run SHALL behave as if the file exists with the rendered content from the create operation. Inject operations SHALL update the virtual file state with post-injection content so subsequent operations in the same dry-run see the cumulative result. WHEN `--dry-run` and `--force` are both specified, create operations SHALL report `action: create` for existing files, reflecting what `--force` would do. | TEST-6.8 |
| AC-6.9 | Ubiquitous | The system SHALL resolve flag interactions as: `--quiet` suppresses stderr only and has no effect on stdout or JSON content; `--verbose` adds rendered content to both human stderr and JSON stdout independently; in human mode with `--quiet`, `--verbose` has no visible effect (stderr is suppressed); in JSON mode with `--quiet`, `--verbose` still adds rendered content to JSON stdout | TEST-6.9 |
| AC-6.10 | Event | WHEN a file operation fails, the system SHALL stop execution immediately and not execute subsequent operations. The `operations` array SHALL contain results for executed operations only | TEST-6.10 |
| AC-6.11 | Ubiquitous | The system SHALL include a top-level `dry_run` boolean field in JSON output reflecting whether `--dry-run` was specified | TEST-6.11 |

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

### #### NFR-2: Idempotent Operations

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N2.1 | Event | WHEN a recipe designed for idempotent execution (all creates use `skip_if_exists: true` and all injects use `skip_if`) is run a second time with the same variables and the same existing files, the system SHALL report all operations as `"action": "skip"` with reasons | TEST-N2.1 |
| AC-N2.2 | Ubiquitous | The system SHALL not produce duplicate content when create uses `skip_if_exists: true` or inject uses `skip_if` | TEST-N2.2 |

### #### NFR-4: Structured Errors with Rendered Content

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N4.1 | Ubiquitous | The system SHALL include what, where, why, and hint fields in every error message | TEST-N4.1 |
| AC-N4.2 | Event | WHEN a file operation fails (exit code 3), the system SHALL include the rendered template content in the error output so the caller can fall back to manual editing. This is independent of `--verbose` — rendered content in errors is always present | TEST-N4.2 |
| AC-N4.3 | Event | WHEN a template rendering error occurs, the system SHALL report the template file path and the line number of the error | TEST-N4.3 |
| AC-N4.4 | Event | WHEN a variable validation error occurs, the system SHALL report the variable name, expected type, and actual value provided | TEST-N4.4 |

### #### NFR-6: Ordered Execution

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N6.1 | Ubiquitous | The system SHALL execute file operations in the order they appear in the recipe's `files` array | TEST-N6.1 |
| AC-N6.2 | Event | WHEN an inject operation targets a file created by an earlier create operation in the same recipe, the system SHALL find and operate on the newly created file | TEST-N6.2 |

## Execution Context

This phase builds on Phase 2. Assume all prior phase artifacts exist and tests pass.

## Invariants

Refer to `docs/INVARIANTS.md` for project-wide constraints that must be honored.

## Architecture

Refer to `docs/ARCHITECTURE.md` for module boundaries and design decisions.

