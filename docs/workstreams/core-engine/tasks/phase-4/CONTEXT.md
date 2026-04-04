# Phase 4: Inject Operation

> Workstream: core-engine
> Generated: 2026-04-03
> Source: PLAN.md

## Phase Plan

## Phase 4: Inject Operation
Status: Planned
Traces to: FR-5, NFR-2 (complete), NFR-6

Content injection into existing files via regex anchoring. All injection modes: after, before, prepend, append, with at:first/last match selection and skip_if idempotency. This completes the v0.1 operation set.

#### Milestones
- [ ] 4.1: `src/operations/inject.rs` — read target file (or from virtual_files if target created in same dry-run); skip_if string search (skip_if string search must check virtual_files content when target was created in same dry-run); regex anchor matching (after/before); at:first/at:last match selection; prepend/append modes; render inject path as template; write modified content; inject ops update virtual_files with post-injection content for subsequent operations
- [ ] 4.2: Wire inject dispatch into operations/mod.rs
- [ ] 4.3: Unit tests for every injection mode — after (first match), after (last match), before, prepend, append, skip_if, missing target file error, regex no-match error, templated inject path
- [ ] 4.4: Integration test: recipe with create + inject in same run (create file, then inject into it — tests ordered execution AC-N6.2)

#### Validation Criteria
- after: content on line after first match (AC-5.1)
- before: content on line before first match (AC-5.2)
- prepend: content at start of file (AC-5.3)
- append: content at end of file (AC-5.4)
- at:last uses last match (AC-5.5), at:first (default) uses first (AC-5.6)
- skip_if: skips when string found in file, reports action:"skip" (AC-5.7)
- Regex no-match exits 3 with pattern, file path, hint (AC-5.8)
- Missing target file exits 3 (AC-5.9)
- Inject path renders as template (AC-5.11)
- Create-then-inject in same recipe works (AC-N6.2)
- Second run with skip_if shows all skips — no duplicate content (AC-N2.2)
- Inject success reports action:"inject" with path, location, line count (AC-5.10)
- at field ignored when prepend/append specified; after/before without regex exits 1 (AC-5.12, AC-5.13)
- Invalid regex pattern in after/before exits 1 (AC-5.14)
- Multiple inject modes (after+before etc.) specified exits 1 (AC-5.15)
- --force has no effect on inject operations (AC-5.16)

#### Key Files
- `src/operations/inject.rs`
- `src/operations/mod.rs` (dispatch update)

#### Dependencies
- regex crate

---

## Relevant Acceptance Criteria

Extracted from SPEC.md for: FR-5 NFR-2 NFR-6

### #### FR-5: Inject Operation

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-5.1 | Event | WHEN `after: "regex"` is specified, the system SHALL insert rendered content on the line after the first matching line | TEST-5.1 |
| AC-5.2 | Event | WHEN `before: "regex"` is specified, the system SHALL insert rendered content on the line before the first matching line | TEST-5.2 |
| AC-5.3 | Event | WHEN `prepend: true` is specified, the system SHALL insert rendered content at the very beginning of the file | TEST-5.3 |
| AC-5.4 | Event | WHEN `append: true` is specified, the system SHALL insert rendered content at the very end of the file | TEST-5.4 |
| AC-5.5 | Event | WHEN `at: last` is specified with `after` or `before`, the system SHALL use the last regex match instead of the first | TEST-5.5 |
| AC-5.6 | Event | WHEN `at: first` (default) is specified, the system SHALL use the first regex match | TEST-5.6 |
| AC-5.7 | Event | WHEN `skip_if` is specified, the system SHALL render it as a Jinja2 template with the recipe's variables, then search for the rendered string in the target file. If found, the system SHALL skip the injection and report `"action": "skip"` with a reason | TEST-5.7 |
| AC-5.8 | Unwanted | IF the regex pattern matches zero lines in the target file, the system SHALL exit with code 3 and report the pattern, the file path, and a hint | TEST-5.8 |
| AC-5.9 | Unwanted | IF the target file for injection does not exist, the system SHALL exit with code 3 and report the missing file path | TEST-5.9 |
| AC-5.10 | Event | WHEN an inject operation succeeds, the system SHALL report `"action": "inject"` with the path, location description, and line count (number of lines inserted) | TEST-5.10 |
| AC-5.11 | Event | WHEN the inject path contains Jinja2 expressions, the system SHALL render the path before resolving it | TEST-5.11 |
| AC-5.12 | Ubiquitous | The system SHALL ignore the `at` field when `prepend` or `append` is specified | TEST-5.12 |
| AC-5.13 | Unwanted | IF `after` or `before` is specified without a regex pattern, the system SHALL exit with code 1 | TEST-5.13 |
| AC-5.14 | Unwanted | IF an inject operation's `after` or `before` pattern fails to compile as a valid regex, the system SHALL exit with code 1 during recipe validation, reporting the invalid pattern and the compilation error | TEST-5.14 |
| AC-5.15 | Unwanted | IF an inject operation specifies more than one of after/before/prepend/append, the system SHALL exit with code 1 reporting the conflicting fields | TEST-5.15 |
| AC-5.16 | Ubiquitous | The system SHALL not apply `--force` to inject operations — the `--force` flag only affects create operations (`skip_if_exists` override) | TEST-5.16 |
| AC-5.17 | Unwanted | IF writing the modified file content fails due to permissions, the system SHALL exit with code 3 with the path, permission error, and rendered content | TEST-5.17 |

### #### NFR-2: Idempotent Operations

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N2.1 | Event | WHEN a recipe designed for idempotent execution (all creates use `skip_if_exists: true` and all injects use `skip_if`) is run a second time with the same variables and the same existing files, the system SHALL report all operations as `"action": "skip"` with reasons | TEST-N2.1 |
| AC-N2.2 | Ubiquitous | The system SHALL not produce duplicate content when create uses `skip_if_exists: true` or inject uses `skip_if` | TEST-N2.2 |

### #### NFR-6: Ordered Execution

| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N6.1 | Ubiquitous | The system SHALL execute file operations in the order they appear in the recipe's `files` array | TEST-N6.1 |
| AC-N6.2 | Event | WHEN an inject operation targets a file created by an earlier create operation in the same recipe, the system SHALL find and operate on the newly created file | TEST-N6.2 |

## Execution Context

This phase builds on Phase 3. Assume all prior phase artifacts exist and tests pass.

## Invariants

Refer to `docs/INVARIANTS.md` for project-wide constraints that must be honored.

## Architecture

Refer to `docs/ARCHITECTURE.md` for module boundaries and design decisions.

