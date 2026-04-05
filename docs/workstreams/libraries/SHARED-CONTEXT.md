# SHARED-CONTEXT.md

> Workstream: libraries
> Last updated: 2026-04-05

## Purpose

Add library management to jig (v0.4). Libraries are versioned recipe collections for a framework (e.g., `jig-django`). This workstream covers manifest parsing, installation (local + git), recipe/workflow discovery, convention mapping, template overrides, project extensions, and the CLI surface for `jig library add|remove|update|list|recipes|info|workflows`.

## Current State

- **Complete** (2026-04-05)
- 402 tests passing (359 unit + 2 CLI + 12 integration + 29 library)
- `cargo clippy` clean
- All review findings from both iterations fixed in code
- Execution integration wired — `jig run django/model/add-field` works end-to-end
- Conventions injected into template rendering context
- Git install supported via `jig library add <git-url>`
- Template overrides and project extensions implemented
- Two iterations: first pass (management layer), second pass (execution + fixes)

## What Was Delivered

| Planned Phase | Status | Notes |
|---------------|--------|-------|
| Phase 0: Tech debt (v0.3 bugs) | **Skipped** | v0.3 critical bugs still open (see Pre-existing below) |
| Phase 1: Manifest & storage | **Complete** | `LibraryManifest` parsing, semver validation, recipe dir checks |
| Phase 2: Local install & CLI | **Complete** | add, remove, update, list with project-local/global, `--force`, `_install_meta.json` |
| Phase 3: Discovery & resolution | **Complete** | Discovery + resolution wired into cmd_run/cmd_workflow/cmd_validate/cmd_vars |
| Phase 4: Conventions | **Complete** | Two-pass rendering, `.jigrc.yaml` override, injected as `{{ conventions.* }}` |
| Phase 5: Git install | **Complete** | URL detection, shallow clone, metadata tracking, error handling |
| Phase 6: Overrides & extensions | **Complete** | `.jig/overrides/` for templates, `.jig/extensions/` for new recipes |

## Decisions Made

### D-L1: Shell out to git CLI for clone/pull
No new binary deps. Git is ubiquitous on dev machines. Only needed at install-time, not runtime. Clear error when git is absent. URL detection via `is_git_url()` checks for `https://`, `git@`, `ssh://`, or `.git` suffix.

### D-L2: Local install copies directory, not symlink
Immutable after install — no surprise mutations from source changes. Symlink dev mode deferred as potential `jig library link`.

### D-L3: Conventions injected as `{{ conventions.models }}` variable namespace
Two-pass design: render convention templates with recipe vars first, then inject rendered paths into template context as a `conventions` map. Conventions added to the variables map before rendering — no refactor of `run_recipe()` needed.

### D-L4: Library-namespaced resolution via slash syntax
`jig run django/model/add-field` — first segment is library name, rest is recipe path. Filesystem paths checked first for backward compatibility (AC-N2.1). Resolution in `resolve_recipe_or_library()` helper in main.rs.

### D-L5: No new error types or exit codes
Library errors map to existing `JigError` variants. Exit codes unchanged (I-5). Exit 3 (FileOperation) for install/remove/update failures. Exit 1 (RecipeValidation) for manifest validation and resolution errors.

### D-L6: `src/library/` module with four submodules
`manifest.rs`, `install.rs`, `discover.rs`, `conventions.rs`. Follows `src/operations/` and `src/scope/` patterns.

### D-L7: No `dirs` crate — uses `HOME` env var directly
Implementation reads `HOME` directly (`install.rs`). Works on macOS/Linux, fragile on Windows. Acceptable for current target platforms.

### D-L8: Manifest workflows validate recipe cross-references at parse time
Workflow steps referencing undeclared recipes are caught when loading the manifest, not at execution time. Prevents runtime surprises.

### D-L9: Project-local shadows global for all operations
`find_installed_library()` and `list_installed()` both check `.jig/libraries/` before `~/.jig/libraries/`. Consistent precedence across all commands.

### D-L10: `_install_meta.json` tracks install source
Stored alongside the installed library. Records source path/URL, install type (local/git), timestamp, version. Enables one-arg `jig library update <name>` to re-fetch from original source.

### D-L11: Template overrides via path check, not loader hook
Override check happens before template loading in `cmd_run` — checks `.jig/overrides/<lib>/<recipe>/templates/` and swaps the template path if an override exists. Simpler than hooking into the minijinja loader.

### D-L12: Extensions cannot shadow library recipes
Library recipes always take precedence at the same path. Extensions are only used when a recipe path doesn't exist in the installed library. Listed with `[ext]` marker in human output, `"source": "extension"` in JSON.

## Patterns Established

- **CLI subcommand nesting**: `LibraryAction` enum with 7 variants under `Commands::Library`. Each dispatched from `cmd_library()` in main.rs. JSON and human output for every subcommand.

- **Helper `create_library_source()` for integration tests**: Builds a complete library fixture (manifest + recipe dirs + templates) in a temp directory. Reusable pattern for any test needing a library.

- **`copy_dir_recursive()` for install**: Custom recursive copy in `install.rs` rather than pulling in a crate. Simple and sufficient.

- **`InstalledLibrary` with location enum**: Tracks whether a library is Global or ProjectLocal. Used for display and precedence.

- **`resolve_recipe_or_library()` pattern**: Helper that first checks filesystem, then library resolution. Used by cmd_run, cmd_validate, cmd_vars. Centralizes the precedence logic (AC-N2.1).

- **Convention two-pass rendering**: Convention templates rendered with recipe vars via minijinja, then the rendered strings injected as a `conventions` object into the template context. Reuses existing renderer without special-casing.

- **`_install_meta.json` sidecar pattern**: Metadata stored as a JSON file alongside the installed library directory. Avoids modifying the manifest or library contents.

## Known Issues / Tech Debt

### Pre-existing (from v0.3, still open)
- `write_back` silently swallows write errors in `patch.rs` and `replace.rs` — **Critical**
- `Position::Sorted` is a stub (`todo!()` panic) — **Critical**
- Byte/char index mismatch in `delimiter.rs:87` (multi-byte UTF-8 panics) — **Critical**
- `extract_rendered_from_error` returns error description, not rendered content (v0.3 C1)
- `format_workflow_json` status logic ignores step-level `on_error` overrides (v0.3 M1)
- `cmd_run` and `run_recipe` are divergent copies of rendering pipeline (v0.3 M4)

### From v0.4 (minor, deferred)
- m2: No `--global` flag for remove/update (only add supports `--project`)
- m3: JSON output emits `null` for optional fields instead of omitting them
- m6: `global_libraries_dir` uses `HOME` env var directly — fragile on non-Unix (D-L7)
- Git clone tests don't test actual network operations (URL detection + metadata only)
- No `jig library link` for symlink-based development workflow

## File Ownership

| File | Status | Lines | What It Does |
|------|--------|-------|-------------|
| `src/library/mod.rs` | Complete | ~6 | Module root, exports 4 submodules |
| `src/library/manifest.rs` | Complete | ~343 | `LibraryManifest` parsing, semver validation, recipe dir checks, workflow cross-refs |
| `src/library/install.rs` | Complete | ~689 | add (local+git), remove, update, list, find, recursive copy, metadata, sort |
| `src/library/discover.rs` | Complete | ~451 | list_recipes, recipe_info, list_workflows, resolve_library_recipe, resolve_library_workflow, extension scanning |
| `src/library/conventions.rs` | Complete | ~234 | ProjectConfig for `.jigrc.yaml`, resolve_conventions merging manifest + overrides |
| `src/main.rs` | Modified | +403 | Library resolution in cmd_run/cmd_workflow/cmd_validate/cmd_vars, convention injection, override/extension checks, git install dispatch |
| `src/renderer.rs` | Modified | +32 | Template override resolution support |
| `tests/library.rs` | Complete | ~1289 | 29 integration tests covering full lifecycle, execution, conventions, overrides, extensions |
