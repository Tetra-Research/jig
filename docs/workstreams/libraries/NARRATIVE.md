# NARRATIVE.md

> Workstream: libraries
> Last updated: 2026-04-05

## What This Does

The libraries workstream adds a library management system to jig. Libraries are versioned recipe collections for a framework — think `jig-django` containing recipes for models, services, views, schemas, and workflows that chain them together. This workstream covers the full lifecycle: parsing library manifests (`jig-library.yaml`), installing libraries from local directories, listing and inspecting installed libraries, discovering their recipes and workflows, and a convention system for per-project path customization.

The CLI surface is `jig library add|remove|update|list|recipes|info|workflows`. Libraries install to `.jig/libraries/` (project-local, takes precedence) or `~/.jig/libraries/` (global).

## Why It Exists

Without libraries, every project using jig must author its own recipes. Two Django projects that both need "add a model field" would each maintain their own recipe, templates, and workflows. Libraries solve this by packaging framework-specific recipes into installable, versioned collections.

1. **Reuse across projects.** `jig library add ./jig-django` installs a recipe collection. Every Django project on the machine can use the same recipes.

2. **Convention mapping.** Django puts models in `{{ app }}/models/{{ model }}.py`. Rails puts them in `app/models/{{ model }}.rb`. The library's `conventions` block maps abstract concerns ("where do models live?") to framework-specific paths. Projects can override these in `.jigrc.yaml` without forking the library.

3. **Discovery.** `jig library recipes django` lists what's available. `jig library info django/model/add-field` shows the variables and operations. An LLM can query the catalog to decide which recipe to use.

4. **Versioning.** Libraries have a `version` field. `jig library update django` pulls the latest. The installed copy is immutable — no surprise mutations from upstream changes.

Libraries are the ecosystem layer. They're also a prerequisite for the MCP server (which needs `jig_library_recipes` to work), the Claude Code plugin (which wraps a library), and meaningful agent evals (which need a library like jig-django to test against).

## How It Works

### Library Structure

```
jig-django/
  jig-library.yaml          # manifest: name, version, conventions, recipe/workflow listings
  model/
    add-field/
      recipe.yaml            # standard jig recipe
      templates/
        field.j2
    create/
      recipe.yaml
      templates/
        model.j2
  service/
    add-method/
      recipe.yaml
      templates/
        method.j2
```

### Manifest Format

```yaml
name: django
version: "0.1.0"
description: Django recipe collection
framework: django
language: python
conventions:
  models: "{{ app }}/models/{{ model | snakecase }}.py"
  services: "{{ app }}/services/{{ model | snakecase }}_service.py"
recipes:
  model/add-field:
    description: Add a field to an existing Django model
  model/create:
    description: Create a new Django model
  service/add-method:
    description: Add a method to a service class
workflows:
  add-field:
    description: Add field and propagate through stack
    steps:
      - recipe: model/add-field
      - recipe: service/add-method
        when: "{{ update_service }}"
```

### Installation Flow

```
jig library add ./jig-django
  |
  v
Validate jig-library.yaml exists
  |
  v
Parse manifest (name, version, recipes, workflows)
  |
  v
Check not already installed at target scope
  |
  v
Copy directory to .jig/libraries/django/
  |
  v
Report: "Installed django 0.1.0 (3 recipes, 1 workflow)"
```

### What's NOT Wired Yet

The management system is complete but execution integration is missing:

- `jig run django/model/add-field` does NOT work — `resolve_library_recipe()` exists but is never called from `cmd_run`
- `jig workflow django/add-field` does NOT work — same issue with `resolve_library_workflow()`
- Convention injection (`{{ conventions.models }}`) is implemented in `conventions.rs` but never called during template rendering
- Git URL installation not implemented — only local paths work
- Template overrides (`.jig/overrides/`) and extensions (`.jig/extensions/`) not implemented

## Key Design Decisions

### 1. Libraries are immutable copies, not symlinks

`jig library add` copies the entire directory to the installation location. This means the installed library is frozen at the version you installed. Changes to the source don't affect installed copies. A `jig library link` for development mode could be added later.

### 2. Project-local takes precedence over global

If `django` is installed both in `.jig/libraries/` and `~/.jig/libraries/`, the project-local version wins. This lets a project pin a specific version or use a fork without affecting other projects.

### 3. Manifest validates recipe cross-references at parse time

If a workflow step references `model/add-field` but that recipe isn't declared in the `recipes` block, the manifest fails to load. This catches misconfigurations early rather than at execution time.

### 4. Convention system uses two-pass rendering (planned)

Convention templates like `{{ app }}/models/{{ model }}.py` are first rendered with recipe variables, then the rendered paths are injected as `{{ conventions.models }}` into the template context. This reuses the existing renderer — no new path resolution system.

### 5. Slash syntax for library-namespaced resolution (planned)

`django/model/add-field` splits on first `/` to get library name `django` and recipe path `model/add-field`. Filesystem paths take precedence for backward compatibility — if `./django/model/add-field/recipe.yaml` exists on disk, it's used instead of the library.

### 6. No new exit codes

Library errors map to existing codes. Manifest parse failure is exit 1 (validation). Install I/O failure is exit 3 (file operation). Convention variable missing is exit 4 (variable validation). Respects I-5.

## What Remains for v0.4 Completion

The workstream delivered the management infrastructure (~70%) but not the execution integration (~30%). To complete v0.4:

1. **Wire `resolve_library_recipe` into `cmd_run`** — Change `Commands::Run { recipe: PathBuf }` to accept strings, try library resolution before filesystem
2. **Wire `resolve_library_workflow` into `cmd_workflow`** — Same pattern
3. **Inject conventions into `run_recipe()`** — Load `.jigrc.yaml`, resolve conventions, add to var context
4. **Fix C1** — Validate name match in `update_from_path`
5. **Fix M1-M5** — Silent swallow, git URLs (or clear "not supported" message), update API, exit codes, list ordering
6. **Git install** (Phase 5) — URL detection, `git clone --depth 1`, `.jig-source` metadata
7. **Template overrides and extensions** (Phase 6) — `.jig/overrides/`, `.jig/extensions/`
