# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Status

**v0.3 workflows complete.** All four file operations (create, inject, replace, patch) plus multi-recipe workflows with conditional steps, variable mapping, and error handling. 343 tests passing. Next up: libraries (v0.4).

## What jig Is

jig is a template rendering CLI purpose-built for LLM code generation workflows. It takes a recipe (YAML) + variables (JSON) and produces deterministic file operations (create, inject, patch, replace). It is designed to be called by LLMs, not humans at a terminal.

The full specification lives in `jig.md`. Read it before making architectural decisions.

## Language and Build

Rust.

```bash
cargo build              # build
cargo test               # run all tests
cargo test <test_name>   # run a single test
cargo run -- <args>      # run the CLI
cargo clippy             # lint
```

### Planned Dependencies

| Crate | Purpose |
|-------|---------|
| `minijinja` | Jinja2 template rendering |
| `serde` + `serde_yaml` + `serde_json` | Recipe and variable parsing |
| `regex` | Injection/replace pattern matching |
| `clap` | CLI argument parsing |
| `heck` | Case conversion filters (snake_case, camelCase, etc.) |
| `owo-colors` | Terminal coloring |
| `insta` | Snapshot testing |

## Architecture

Planned source layout:

```
src/
  main.rs              # CLI entry point (clap)
  recipe.rs            # Recipe YAML parsing and validation
  variables.rs         # Variable declaration, type checking, JSON merging
  renderer.rs          # Jinja2 rendering via minijinja
  operations/
    mod.rs             # Operation trait and dispatch
    create.rs          # Create new files
    inject.rs          # Inject into existing files (line-level)
    patch.rs           # Patch existing files (scope-aware anchoring)
    replace.rs         # Replace regions in existing files
  scope/
    mod.rs             # Scope detection dispatch
    indent.rs          # Indentation-based (Python, YAML)
    delimiter.rs       # Delimiter-based (braces, brackets, parens)
    position.rs        # Semantic positions (after_last_field, etc.)
  workflow.rs          # Multi-recipe orchestration, conditional steps
  library/
    mod.rs             # Library manifest parsing
    install.rs         # Add/remove/update libraries
    discover.rs        # Recipe and workflow discovery
    conventions.rs     # Convention mapping and overrides
  filters.rs           # Custom Jinja2 filters (snakecase, pluralize, etc.)
  output.rs            # Human-readable and JSON output formatting
  error.rs             # Structured error types with exit codes
```

## Core Concepts

- **Recipe**: YAML file declaring variables and file operations. Lives alongside its templates.
- **Operations**: `create` (new file), `inject` (insert at anchor in existing file), `patch` (scope-aware insert using anchor/scope/position), `replace` (swap region between markers).
- **Anchor system**: Pattern match + scope (class_body, function_body, braces, etc.) + position (after_last_field, before_close, etc.) for structural code insertion without a full parser.
- **Scope detection**: Indentation-based for Python/YAML, delimiter-based for C-family. Lightweight heuristics, not AST.
- **Workflows**: Multi-recipe chains with conditional steps (`when`), variable mapping, and error handling modes.
- **Libraries**: Versioned recipe collections for a framework (e.g., jig-django). Installed globally or per-project, with convention overrides.

## CLI Commands

```
jig run <recipe> --vars '<json>'    # Execute a recipe
jig render <template> --vars '...'  # Render a single template
jig validate <recipe>               # Check recipe is well-formed
jig vars <recipe>                   # Show expected variables (JSON)
jig workflow <name> --vars '...'    # Run a multi-recipe workflow
jig scan <recipe> <path>            # Reverse-extract variables from existing code
jig check <recipe> <path>           # Conformance verification
jig library add|remove|update|list  # Manage recipe libraries
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Recipe validation error |
| 2 | Template rendering error |
| 3 | File operation error |
| 4 | Variable validation error |

## Design Principles

1. JSON in, files out. Non-interactive by default.
2. Deterministic: same recipe + same variables + same files = same output.
3. Idempotent: every operation safe to re-run (`skip_if`, `skip_if_exists`).
4. Transparent failures: errors include what/where/why + rendered content so the LLM can fall back to manual editing.
5. JSON to stdout, human-readable to stderr. Auto-detect TTY vs piped.
6. Templates live with the consumer, not in a central directory.

## Testing Strategy

- **Unit tests**: Recipe parsing, variable validation, template rendering, each operation type.
- **Integration tests**: Fixture directories with `recipe.yaml`, `vars.json`, `templates/`, `existing/`, and `expected/`. Test runner copies existing/ to temp dir, runs jig, diffs against expected/.
- **Snapshot tests**: `insta` crate for template rendering output.
- **Agent evals** (`eval/`): End-to-end tests where real LLM agents invoke jig against fixture codebases, scored on assertion pass rate, jig usage, and efficiency vs. baseline.

## Slash Commands

- `/review` - Adversarial code review (channels Linus, Hickey, Cantrill, Katz, Klabnik)
- `/plan-review <path>` - Adversarial review of planning/design docs for contradictions and gaps
- `/spacex [path]` - Apply The Algorithm to simplify code (Question, Delete, Simplify, Accelerate, Automate)
- `/clean-copy` - Rewrite branch with clean, narrative commit history

## Roadmap Reference

The spec defines a phased roadmap (v0.1 through v1.0+). When implementing, follow the milestone order in `jig.md` — MVP core engine first, then patches, workflows, libraries, distribution, and advanced features (scan, infer, check).
