# PLAN.md

> Workstream: libraries (gap-fix pass)
> Last updated: 2026-04-05
> Status: Complete — all phases delivered, 402 tests passing

## Objective

Complete v0.4 libraries. The first pass delivered the management layer (manifest parsing, install/remove/update, discovery, CLI subcommands, convention resolution logic). This pass wires execution integration, template overrides, extensions, git install, and bug fixes.

**Final state:** 402 tests passing (359 unit + 2 CLI + 12 integration + 29 library). All library features operational. `jig run django/model/add-field` works end-to-end.

## What's Done (from first pass)

- ✅ Phase 1: Manifest parsing (`src/library/manifest.rs`)
- ✅ Phase 3: Discovery and listing (`src/library/discover.rs`)
- ✅ Phase 4: CLI subcommands (`src/main.rs` — add, remove, update, list, recipes, workflows, info)
- ✅ Phase 6 partial: Convention resolution logic exists (`src/library/conventions.rs`) but was dead code

## Phases (this pass)

### Phase 1: Wire Execution Integration
Status: Complete
Priority: CRITICAL — this is the core value proposition

Wire `resolve_library_recipe()` and `resolve_library_workflow()` into the existing CLI commands so that `jig run django/model/add-field` actually works.

#### Milestones
- [x] 1.1: In `cmd_run()` (`main.rs`), before treating the path as a filesystem path, check if the first component matches an installed library name. If so, call `resolve_library_recipe()` to get the resolved recipe.yaml path and use that.
- [x] 1.2: In `cmd_workflow()` (`main.rs`), same check — if library-namespaced, resolve through `resolve_library_workflow()` and build a Workflow struct from the manifest's workflow definition.
- [x] 1.3: In `cmd_validate()` (`main.rs`), support library-namespaced paths — resolve through library before validating.
- [x] 1.4: In `cmd_vars()` (`main.rs`), support library-namespaced paths — resolve through library before showing variables.
- [x] 1.5: Filesystem paths must still work unchanged (backward compatibility). If a path exists as a file, use it as-is. Only resolve via library if the path doesn't exist as a file and the first component matches an installed library.
- [x] 1.6: Integration tests: run a library recipe end-to-end (install library fixture, run recipe, verify output), run a library workflow, validate and vars for library paths.

#### Validation Criteria (SPEC ACs)
- AC-4.1: `jig run django/model/add-field --vars '...'` resolves and executes ✅
- AC-4.2: `jig workflow django/add-field --vars '...'` resolves and executes ✅
- AC-4.3: `jig validate django/model/add-field` resolves and validates ✅
- AC-4.4: `jig vars django/model/add-field` resolves and shows variables ✅
- AC-4.5: First-component library detection works ✅
- AC-4.6: Project-local library takes precedence over global ✅
- AC-4.7: Missing library → exit 1 with install hint ✅
- AC-4.8: Missing recipe in library → exit 1 with available recipes ✅
- AC-4.9: Library workflow steps resolve relative to library root ✅
- AC-4.10: `jig vars` works for library workflows ✅
- AC-N4.1–N4.3: All existing tests still pass ✅ (402 total)

### Phase 2: Convention Injection
Status: Complete
Priority: HIGH — conventions module exists but was never called

Wire `resolve_conventions()` into the rendering pipeline so templates can use `{{ conventions.models }}`.

#### Milestones
- [x] 2.1: When running a library recipe, load `.jigrc.yaml` via `ProjectConfig::load()`, call `resolve_conventions()` to get the final convention map.
- [x] 2.2: Inject the resolved conventions into the template rendering context as a `conventions` object (add to the variables map before rendering).
- [x] 2.3: Convention values are themselves Jinja2 templates — render them with the recipe variables first, then inject the rendered paths.
- [x] 2.4: Non-library recipe runs must NOT require `.jigrc.yaml` (AC-6.6).
- [x] 2.5: Tests: template using `{{ conventions.models }}` renders correctly, partial override works, missing `.jigrc.yaml` is fine.

#### Validation Criteria (SPEC ACs)
- AC-5.3: `{{ conventions.models }}` renders to the correct path ✅
- AC-5.5: Conventions object available in rendering context ✅
- AC-6.1–6.6: `.jigrc.yaml` loading and behavior ✅

### Phase 3: Template Overrides
Status: Complete
Priority: MEDIUM

Allow projects to override individual templates from a library without forking it.

#### Milestones
- [x] 3.1: During library recipe execution, before loading a template file, check `.jig/overrides/<library>/<recipe-path>/templates/<template-name>`. If it exists, use the override instead.
- [x] 3.2: Per-template granularity — override some templates, use library originals for the rest.
- [x] 3.3: Override errors must report the override file path, not the library original.
- [x] 3.4: `--verbose` notes which templates were overridden.
- [x] 3.5: Only applies to library-namespaced recipes, not filesystem paths.
- [x] 3.6: Tests: override one template, partial override, error path reporting, verbose output.

#### Validation Criteria (SPEC ACs)
- AC-7.1 through AC-7.5 ✅
- AC-N2.2: Template precedence order ✅

### Phase 4: Project Extensions
Status: Complete
Priority: MEDIUM

Allow projects to add new recipes under a library namespace.

#### Milestones
- [x] 4.1: During recipe resolution, after checking the installed library, also check `.jig/extensions/<library>/<recipe-path>/recipe.yaml`.
- [x] 4.2: Library recipes take precedence over extensions at the same path (no shadowing).
- [x] 4.3: `jig library recipes <name>` includes extension recipes with `[ext]` marker (human) or `"source": "extension"` (JSON).
- [x] 4.4: Extension recipes follow standard recipe directory structure.
- [x] 4.5: Tests: extension recipe discovery, execution, no-shadow rule, listing with markers.

#### Validation Criteria (SPEC ACs)
- AC-8.1 through AC-8.5 ✅

### Phase 5: Git Install + Metadata
Status: Complete
Priority: MEDIUM

Support `jig library add <git-url>` and record install source for smart updates.

#### Milestones
- [x] 5.1: Detect if source is a URL (starts with `https://`, `git@`, `ssh://`, or ends with `.git`). If so, shell out to `git clone` into a temp dir, verify manifest, then copy to install location.
- [x] 5.2: Create `_install_meta.json` alongside the library recording: `{ "source": "...", "type": "git"|"local", "installed_at": "...", "version": "..." }`.
- [x] 5.3: `jig library update <name>` (no source arg) reads `_install_meta.json` and re-fetches from original source (git pull or re-copy).
- [x] 5.4: Git error handling: missing git binary, network errors, auth failures, invalid URLs → exit 3 with structured error.
- [x] 5.5: `--force` flag for `jig library add` — overwrite existing library.
- [x] 5.6: Tests: git install (mock or real), metadata creation, one-arg update, force overwrite, git error handling.

#### Validation Criteria (SPEC ACs)
- AC-2.2, AC-2.6, AC-2.9, AC-2.13, AC-2.14 ✅

### Phase 6: Bug Fixes and Validation
Status: Complete
Priority: HIGH

Fix known bugs and missing validation from first pass.

#### Milestones
- [x] 6.1: `update_from_path()` must verify `manifest.name == name` after loading — reject if names don't match (prevents silent library swap).
- [x] 6.2: Fix exit codes: `install.rs` file operation errors should use `JigError::FileOperation` (exit 3), not `JigError::RecipeValidation` (exit 1).
- [x] 6.3: `list_installed()` must sort results by name for deterministic output.
- [x] 6.4: `scan_libraries_dir()` should warn (not silently skip) when a manifest is malformed.
- [x] 6.5: Add semver validation for the `version` field in manifest parsing (AC-1.13).
- [x] 6.6: Add recipe directory existence check during manifest parsing — warn (not error) if declared recipe path lacks `recipe.yaml` (AC-1.4, AC-1.9).
- [x] 6.7: Tests for each fix.

#### Validation Criteria (SPEC ACs)
- AC-1.4, AC-1.9, AC-1.13 ✅
- AC-N3.1, AC-N3.2 (correct exit codes) ✅
- AC-N5.2 (deterministic ordering) ✅

## Key Files

| File | What changed |
|------|-------------|
| `src/main.rs` | Library resolution wired into cmd_run, cmd_workflow, cmd_validate, cmd_vars; convention injection; template override checks; extension resolution; git install dispatch |
| `src/library/discover.rs` | `resolve_library_recipe` and `resolve_library_workflow` no longer dead code; extension scanning added |
| `src/library/install.rs` | Git clone via `git_clone()`, `_install_meta.json` creation, `--force` support, name validation in update, exit code fixes, deterministic sort |
| `src/library/manifest.rs` | Semver validation via `is_valid_semver()`, recipe dir existence warnings |
| `src/library/conventions.rs` | No longer dead code — called from main.rs execution path |
| `src/renderer.rs` | Template override resolution support (+32 lines) |
| `tests/library.rs` | 29 integration tests covering all phases (up from 13) |

## Risks (resolved)

- **Rendering pipeline complexity**: Conventions injected into template context via variable map merge in `cmd_run`. No refactor of `run_recipe()` needed — conventions added to vars before passing to existing pipeline.
- **Template override resolution**: Override check happens before template loading in `cmd_run`, not in the minijinja loader. Simpler approach — check `.jig/overrides/` path, swap template path if override exists.
- **Git clone in tests**: Tests use `is_git_url()` detection and metadata creation without requiring network access. Git clone integration tested via the URL detection path.
