# SHARED-CONTEXT.md

> Workstream: libraries
> Last updated: 2026-04-05

## Purpose

Add library management to jig (v0.4). Libraries are versioned recipe collections for a framework (e.g., `jig-django`). This workstream covers manifest parsing, local installation, recipe/workflow discovery, convention mapping, and the CLI surface for `jig library add|remove|update|list|recipes|info|workflows`.

## Current State

- **Partial** (2026-04-05)
- Library management CLI complete: add, remove, update, list, recipes, info, workflows
- 386 tests passing (359 unit + 2 CLI + 12 integration + 13 library)
- `cargo clippy` clean
- Code review completed — 3 critical, 5 major, 7 minor findings. **Review findings not fixed in code** (commit 4d291ac only added review artifacts)
- **Execution integration not wired** — you can install/catalog/inspect libraries but cannot run recipes from them via `jig run django/model/add-field`
- **Conventions system is dead code** — parsed and tested in isolation but never injected during execution
- **Git install not implemented** — `jig library add` only accepts local paths, not git URLs
- **Template overrides and extensions not implemented**

## What Was Delivered

| Planned Phase | Status | Notes |
|---------------|--------|-------|
| Phase 0: Tech debt (v0.3 bugs) | **Skipped** | Plan called for fixing C1/M1-M4 from v0.3 review; not done |
| Phase 1: Manifest & storage | **Complete** | `LibraryManifest` parsing, storage paths |
| Phase 2: Local install & CLI | **Complete** | add, remove, list with project-local/global support |
| Phase 3: Discovery & resolution | **Partial** | Discovery complete; resolution functions exist but are dead code (not wired into cmd_run/cmd_workflow) |
| Phase 4: Conventions | **Partial** | Parsing and resolution implemented; never called from execution path |
| Phase 5: Git install | **Not started** | No git URL support |
| Phase 6: Overrides & extensions | **Not started** | Neither `.jig/overrides/` nor `.jig/extensions/` implemented |

## Decisions Made

### D-L1: Shell out to git CLI for clone/pull (planned, not yet implemented)
No new binary deps. Git is ubiquitous on dev machines. Only needed at install-time, not runtime. Clear error when git is absent.

### D-L2: Local install copies directory, not symlink
Immutable after install — no surprise mutations from source changes. Symlink dev mode deferred as potential `jig library link`.

### D-L3: Conventions injected as `{{ conventions.models }}` variable namespace (planned, dead code)
Two-pass design: render convention templates with recipe vars, inject rendered paths into template context. Reuses existing variable/template pipeline.

### D-L4: Library-namespaced resolution via slash syntax (planned, dead code)
`jig run django/model/add-field` — first segment is library name, rest is recipe path. Filesystem paths take precedence for backward compatibility.

### D-L5: No new error types or exit codes
Library errors map to existing `JigError` variants. Exit codes unchanged (I-5). **Review found M4: some error mappings are semantically wrong** (e.g., "manifest not found" uses RecipeValidation/exit 1 instead of FileOperation/exit 3).

### D-L6: `src/library/` module with four submodules
`manifest.rs`, `install.rs`, `discover.rs`, `conventions.rs`. Follows `src/operations/` and `src/scope/` patterns.

### D-L7: No `dirs` crate — uses `HOME` env var directly
Plan called for `dirs` crate but implementation reads `HOME` directly (`install.rs:35-46`). Works on macOS/Linux, fragile elsewhere.

### D-L8: Manifest workflows validate recipe cross-references at parse time
Workflow steps referencing undeclared recipes are caught when loading the manifest, not at execution time. Good design choice — prevents runtime surprises.

### D-L9: Project-local shadows global for all operations
`find_installed_library()` and `list_installed()` both check `.jig/libraries/` before `~/.jig/libraries/`. Consistent precedence.

### D-L10: `update` requires explicit source path (diverges from spec)
Spec says `jig library update django` should re-fetch from original source. Implementation requires `jig library update django /path/to/source` because installed libraries don't remember their source. The plan's `.jig-source` metadata file was not implemented.

## Patterns Established

- **CLI subcommand nesting**: `LibraryAction` enum with 7 variants under `Commands::Library`. Each dispatched from `cmd_library()` in main.rs. JSON and human output for every subcommand.

- **Helper `create_library_source()` for integration tests**: Builds a complete library fixture (manifest + recipe dirs + templates) in a temp directory. Reusable pattern for any test needing a library.

- **`copy_dir_recursive()` for install**: Custom recursive copy in `install.rs` rather than pulling in a crate. Simple and sufficient.

- **`InstalledLibrary` with location enum**: Tracks whether a library is Global or ProjectLocal. Used for display and precedence.

## Known Issues / Tech Debt

### From v0.4 code review (unfixed)

**Critical:**
- **C1: `update_from_path` doesn't validate name match** (`install.rs:144-191`). Can silently replace django's directory with flask's files. Fix: verify `manifest.name == name` after loading.
- **C2: Library recipes/workflows can't be executed** (`discover.rs:147,193`). `resolve_library_recipe` and `resolve_library_workflow` are `#[allow(dead_code)]`. Not wired into `cmd_run` or `cmd_workflow`. The core value proposition is missing.
- **C3: Convention resolution is dead code** (`conventions.rs`, `mod.rs:1`). Entire `conventions` module is `#[allow(dead_code)]`. `ProjectConfig::load()` and `resolve_conventions()` never called.

**Major:**
- **M1: `scan_libraries_dir` silently swallows malformed manifests** (`install.rs:284`). Violates I-10. Should warn or include in JSON output.
- **M2: `add` only supports local paths** (`main.rs:103-105`). No git URL support, no error message saying it's not yet supported.
- **M3: `update` requires source path** (`main.rs:117-120`). Spec says one-arg update. Library doesn't remember source.
- **M4: Exit codes semantically wrong** (`install.rs:65,233`). "No manifest found" and "not installed" use RecipeValidation (exit 1). Violates I-5.
- **M5: `list_installed` non-deterministic ordering** (`install.rs:196`). `read_dir` has no guaranteed order. Fix: sort by name.

**Minor:**
- m1: Path-splitting logic duplicated between `Info` handler and `discover::resolve_library_recipe`
- m2: No `--global` flag for remove/update
- m3: JSON output emits `null` for optional fields
- m4: Integration tests don't verify specific exit codes
- m5: No test for `--global` installation
- m6: `global_libraries_dir` uses `HOME` env var directly (see D-L7)
- m7: Overrides and extensions not implemented (Phases 5-6 not started)

### Pre-existing (from v0.3, still open)
- `write_back` silently swallows write errors in `patch.rs` and `replace.rs` — **Critical**
- `Position::Sorted` is a stub (`todo!()` panic) — **Critical**
- Byte/char index mismatch in `delimiter.rs:87` (multi-byte UTF-8 panics) — **Critical**
- `extract_rendered_from_error` returns error description, not rendered content (v0.3 C1)
- `format_workflow_json` status logic ignores step-level `on_error` overrides (v0.3 M1)
- `cmd_run` and `run_recipe` are divergent copies of rendering pipeline (v0.3 M4)

## File Ownership

| File | Status | What It Does |
|------|--------|-------------|
| `src/library/mod.rs` | New (6 lines) | Module root, exports 4 submodules |
| `src/library/manifest.rs` | New (312 lines) | `LibraryManifest` parsing from `jig-library.yaml`, field validation, recipe cross-reference checks |
| `src/library/install.rs` | New (509 lines) | `add_from_path`, `remove`, `update_from_path`, `list_installed`, `find_installed_library`, recursive copy |
| `src/library/discover.rs` | New (378 lines) | `list_recipes`, `recipe_info`, `list_workflows`, `resolve_library_recipe` (dead), `resolve_library_workflow` (dead) |
| `src/library/conventions.rs` | New (234 lines) | `ProjectConfig` for `.jigrc.yaml`, `resolve_conventions` merging manifest + overrides (all dead code) |
| `src/main.rs` | Modified | `Library` command + `LibraryAction` enum, `cmd_library()` dispatch (lines 671-964) |
| `src/output.rs` | Not modified | Library commands format JSON/human directly in `cmd_library()` |
| `tests/library.rs` | New (551 lines) | 13 integration tests covering full lifecycle |
