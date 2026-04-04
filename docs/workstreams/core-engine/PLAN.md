# PLAN.md

> Workstream: core-engine
> Last updated: 2026-04-04
> Status: Complete

## Objective

Deliver the minimum viable jig CLI: parse recipes, validate variables, render Jinja2 templates, execute create and inject file operations, and report structured results. This is the v0.1 feature set — the smallest thing that lets an LLM call `jig run` to produce files from a recipe instead of re-deriving boilerplate.

Covers ARCHITECTURE.md Phases A through E (skeleton, rendering, create, inject, integration tests).

## Phases

### Phase 1: Skeleton + Recipe Parsing
Status: Complete
Traces to: FR-1, FR-7 (partial), NFR-4, NFR-5

Bootstrap the Rust crate, wire up clap, and make recipe parsing work end-to-end. After this phase, `jig validate` and `jig vars` are functional commands.

#### Milestones
- [x] 1.1: Cargo.toml with dependencies (serde, serde_yaml, serde_json, clap, thiserror, regex, indexmap)
- [x] 1.2: `src/error.rs` — StructuredError struct (what/where/why/hint), JigError enum wrapping StructuredError with exit code mapping (0-4)
- [x] 1.3: `src/recipe.rs` — Recipe, VariableDecl, VarType, FileOp structs with serde deserialization; template path resolution relative to recipe location; structural validation (missing fields, missing template files); unknown operation type detection with clear error message; custom deserialization for FileOp — use intermediate flat struct with optional fields, then validate and convert to typed enum (reject when more than one of `to`/`inject`/`replace`/`patch` is present, or none is present; if `replace` or `patch` is present, emit AC-1.10 'not supported in v0.1' error, reject when multiple inject modes (after/before/prepend/append) are specified); compile-check regex patterns in after/before fields during recipe validation
- [x] 1.4: `src/variables.rs` — Variable merging and type-checking scaffolding (types imported from recipe.rs, validation logic added in Phase 2)
- [x] 1.5: `src/main.rs` — clap CLI with `validate` and `vars` subcommands, `#[command(version)]`; wire recipe parsing; map errors to exit codes; `jig validate` outputs summary to stderr (variable count, operation types); with `--json`, outputs structured JSON to stdout
- [x] 1.6: Unit tests for recipe parsing — valid recipe, missing fields, malformed YAML, missing template files, optional metadata

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

### Phase 2: Variable Validation + Template Rendering
Status: Complete
Traces to: FR-2, FR-3, FR-7 (partial), NFR-1 (partial)

Full variable pipeline (parse, merge, type-check) and template rendering with all 13 built-in filters. After this phase, `jig render` works.

#### Milestones
- [x] 2.1: `src/variables.rs` — full validation: parse JSON from --vars/--vars-file/--vars-stdin; merge with precedence (defaults < file < stdin < inline); type-check against declarations; required field enforcement; enum validation; array item type validation
- [x] 2.2: `src/filters.rs` — all 13 built-in filters registered with minijinja: snakecase, camelcase, pascalcase, kebabcase, upper, lower, capitalize, replace, pluralize, singularize, quote, indent, join
- [x] 2.3: `src/renderer.rs` — minijinja Environment setup; template loading from recipe-relative paths; filter registration; render with variables context; structured error on undefined variable (with "did you mean?" via edit distance) and syntax errors (with file + line)
- [x] 2.4: Wire `render` subcommand in main.rs with --vars, --vars-file, --vars-stdin, --to options. For `jig render`, create a standalone Environment with filters registered but no template directory — load the template file directly by path via `render_str()` or equivalent
- [x] 2.5: Unit tests for variable validation — every VarType, required missing, default fallback, enum rejection, array item mismatch, merge precedence
- [x] 2.6: Unit tests + insta snapshot tests for all 13 filters and template rendering (conditionals, loops, comments, raw blocks, undefined vars, syntax errors)

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

### Phase 3: Create Operation + Output Formatting
Status: Complete
Traces to: FR-4, FR-6, FR-7 (complete), NFR-2 (partial), NFR-4, NFR-6

File creation with templated output paths, directory auto-creation, skip_if_exists, and dual-stream output (JSON stdout / human stderr). After this phase, `jig run` works for create-only recipes.

#### Milestones
- [x] 3.1: `src/operations/mod.rs` — ExecutionContext struct (base_dir, dry_run, force, virtual_files for dry-run state); OpResult enum (Success with action/path/lines/location, Skip, Error); operation dispatch (create only initially). In dry-run mode, create ops populate virtual_files instead of writing to disk.
- [x] 3.2: `src/operations/create.rs` — render `to` path as template; create parent directories; write rendered content; skip_if_exists logic; --force override; --base-dir path resolution
- [x] 3.3: `src/output.rs` — OutputMode enum (Json/Human/Quiet); TTY auto-detection; JSON serialization of operations array + files_written/files_skipped; track per-path write/skip status for files_written and files_skipped arrays; colored human output to stderr; --verbose includes rendered content
- [x] 3.4: Wire `run` subcommand in main.rs with --dry-run, --json, --quiet, --force, --base-dir, --verbose; render all template contents AND templated path fields (`to`, `inject`, `skip_if`) into a Vec before executing any operation — rendering failures abort before any file is touched; execute operations in declaration order; fail-fast on operation errors — stop execution on first failure
- [x] 3.5: Unit tests for create operation — happy path, skip_if_exists, force overwrite, templated paths, directory creation, file-already-exists error
- [x] 3.6: Unit tests for output formatting — JSON schema matches SPEC, human output, quiet mode, verbose mode, dry-run

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

### Phase 4: Inject Operation
Status: Complete
Traces to: FR-5, NFR-2 (complete), NFR-6

Content injection into existing files via regex anchoring. All injection modes: after, before, prepend, append, with at:first/last match selection and skip_if idempotency. This completes the v0.1 operation set.

#### Milestones
- [x] 4.1: `src/operations/inject.rs` — read target file (or from virtual_files if target created in same dry-run); skip_if string search (skip_if string search must check virtual_files content when target was created in same dry-run); regex anchor matching (after/before); at:first/at:last match selection; prepend/append modes; render inject path as template; write modified content; inject ops update virtual_files with post-injection content for subsequent operations
- [x] 4.2: Wire inject dispatch into operations/mod.rs
- [x] 4.3: Unit tests for every injection mode — after (first match), after (last match), before, prepend, append, skip_if, missing target file error, regex no-match error, templated inject path
- [x] 4.4: Integration test: recipe with create + inject in same run (create file, then inject into it — tests ordered execution AC-N6.2)

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

### Phase 5: Integration Test Framework
Status: Complete
Traces to: All FRs and NFRs (validation layer)

Fixture-based integration test harness that makes adding new test cases a matter of adding a directory. Snapshot tests for all output formats. This phase validates the entire v0.1 pipeline end-to-end.

#### Milestones
- [x] 5.1: Test harness in `tests/integration.rs` — fixture discovery; copy existing/ to temp dir; run jig as subprocess; diff output against expected/; assert JSON output against expected_output.json; assert exit code against expected_exit_code
- [x] 5.2: Fixtures for create operations — simple create, templated path, skip_if_exists, force overwrite, directory creation
- [x] 5.3: Fixtures for inject operations — after/before/prepend/append, at:first/at:last, skip_if
- [x] 5.4: Fixtures for error cases — missing vars, bad type, missing template, missing target file, regex no-match, malformed YAML, file exists without force
- [x] 5.5: Fixtures for combined operations — create + inject in one recipe, multi-file recipe, idempotency (run twice)
- [x] 5.6: insta snapshot tests for JSON output format, human output format, error message format
- [x] 5.7: Determinism test — run same recipe twice, assert byte-identical output (AC-N1.1)

#### Validation Criteria
- `cargo test` runs all unit + integration + snapshot tests green
- Every operation mode has at least one fixture
- Every error exit code (1-4) has at least one fixture
- Adding a new test case requires only a new directory, no code changes. Integration fixtures are auto-discovered from directories (no code changes to add tests). Spec-level unit tests are named functions in `#[cfg(test)]` modules (e.g., `spec::fr1::ac_1_1`). The two layers serve different purposes.
- Idempotency fixture: second run produces all skips, no file changes
- Binary has no dynamic dependencies beyond system libc (verified via `otool -L` on macOS, `ldd` on Linux)
- Every `TEST-*` ID in SPEC.md has a corresponding test function named `spec::fr{N}::ac_{N}_{M}` or `spec::nfr{N}::ac_n{N}_{M}`
- Error fixtures assert that error JSON contains `what`, `where`, `why`, and `hint` fields (at least one fixture per exit code)

#### Key Files
- `tests/integration.rs`
- `tests/fixtures/` (directory tree)

#### Dependencies
- insta crate for snapshot testing
- assert_cmd or similar for subprocess testing (or raw std::process::Command)

## Dependencies

- **Depends on:** None (first workstream, greenfield)
- **Blocks:** replace-operation (v0.2), patch-operation (v0.2), workflows (v0.3), libraries (v0.4)

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Operations as enum, not trait objects | `FileOp` enum with match dispatch | Only 2 variants in v0.1 (create, inject). Trait abstraction adds complexity for no benefit until user-defined operations exist, which isn't planned. (ARCHITECTURE D-4) |
| Render before execute | Template rendering happens before operation execution | Errors caught before any file touched; rendered content always available for error messages (ARCHITECTURE D-1, I-4, I-10) |
| Recipe-relative template resolution | Template paths relative to recipe file, not cwd | Keeps recipes self-contained and portable (ARCHITECTURE D-2, I-7). --base-dir only affects output paths. |
| No global state | Every function takes inputs as arguments | Trivial testing, guaranteed determinism (ARCHITECTURE D-5, I-1) |
| thiserror for error types | Use thiserror derive macros for JigError | Idiomatic Rust error handling with minimal boilerplate, good Display/Error trait impls |
| IndexMap for variables | Preserve declaration order in variable maps | Deterministic iteration order for vars output and error messages (I-1) |
| Inline unit tests + fixture integration tests | #[cfg(test)] modules in each source file; tests/fixtures/ for end-to-end | Unit tests co-located with code for fast iteration; fixture tests for pipeline validation without code changes |
| pluralize/singularize approach | Use the `pluralizer` crate | Full English pluralization rules without hand-rolling. Acceptable dependency for correctness. |
| "did you mean?" for undefined vars | `strsim` crate (Levenshtein distance) against declared variable names | minijinja doesn't provide hints. Hand-rolling edit distance serves I-10 (graceful degradation) — the error tells the caller exactly what to fix. |
| Filter ownership | Register all 13 filters ourselves, even for ones minijinja provides (replace, indent, join) | We control exact behavior. No surprises from upstream minijinja changes. Determinism (I-1) over convenience. |
| Render all templates upfront | Render ALL templates before ANY file write | Fail-fast, no partial writes. Templates reference variables not created files. (D-1) |
| Extra variables pass through | Don't error or warn on undeclared variables in input | LLMs construct variable objects with extra fields. Strictness here breaks real usage. |
| Fail-fast on operation errors | Stop on first operation failure | Later operations may depend on earlier ones (e.g., inject targets a prior create). (I-9, AC-N6.2) |

## Risks / Open Questions

All resolved — see Resolved Questions below.

## Resolved Questions

- **pluralize/singularize**: Using `pluralizer` crate. Full English rules, no hand-rolling.
- **"did you mean?"**: Using `strsim` crate for Levenshtein distance against declared var names. minijinja doesn't provide this. Serves I-10.
- **minijinja filter compatibility**: Register all 13 filters ourselves. We own the behavior, upstream changes don't affect us. Serves I-1.
- **TTY detection**: `std::io::IsTerminal` (Rust 1.70+) works correctly. No external crate needed.
- **stdin variable reading**: Documented as single-purpose per invocation. Non-interactive design (I-3) makes this a non-issue.
- **Large file handling for inject**: Reads entire file into memory. Acceptable for source files. Documented as known limitation.
- **pluralizer irregular plurals**: Works for common irregulars (person→people, child→children). Acceptable coverage.
- **minijinja filter override**: Overriding built-in filters works. All 13 registered as custom filters.

## Completion Summary

- **Completed**: 2026-04-04
- **Tests**: 191 total (177 unit + 2 CLI integration + 12 fixture integration)
- **Review**: Dual-agent review cycle (Claude + Codex) came back clean — 0 Critical, 0 Major findings
- **All 5 phases delivered**: validate, vars, render, run (create + inject), full integration test suite
- **CLI commands working**: `jig validate`, `jig vars`, `jig render`, `jig run`

## Execution Order

```
Phase 1 ──► Phase 2 ──► Phase 3 ──► Phase 4 ──► Phase 5
skeleton     vars+render  create+output inject     integration
                                                    tests
```

Each phase produces a working, testable artifact. No phase depends on "wiring up later." The CLI grows incrementally:
- After Phase 1: `jig validate`, `jig vars`
- After Phase 2: + `jig render`
- After Phase 3: + `jig run` (create only)
- After Phase 4: + `jig run` (create + inject) — **v0.1 complete**
- After Phase 5: Full test coverage validates the release
