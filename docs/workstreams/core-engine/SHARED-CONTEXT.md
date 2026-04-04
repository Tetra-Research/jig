# SHARED-CONTEXT.md

> Workstream: core-engine
> Last updated: 2026-04-02

## Purpose

Deliver the v0.1 jig CLI: recipe parsing, variable validation, Jinja2 template rendering, create and inject file operations, and structured output. This is the minimum feature set that makes jig useful — an LLM can call `jig run recipe.yaml --vars '{...}'` to produce files deterministically instead of re-deriving boilerplate.

## Current State

- Initialized (2026-04-02)
- SPEC.md complete with 7 functional requirements (94 acceptance criteria) and 6 non-functional requirements (14 acceptance criteria)
- PLAN.md complete with 5 phases, 28 milestones
- No Rust code exists yet — greenfield implementation

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Operations as enum (FileOp), not trait objects | Only 2 variants in v0.1. Trait abstraction adds no value without user-defined operations. (D-4) |
| Render-then-execute pipeline | Errors caught before file writes; rendered content always available for error fallback. (D-1) |
| Recipe-relative template paths | Keeps recipes self-contained. --base-dir only affects output paths. (D-2, I-7) |
| No global state | Pure functions with explicit inputs. Trivial testing, determinism guaranteed. (D-5) |
| thiserror for error types | Idiomatic Rust error handling with minimal boilerplate |
| IndexMap for variable ordering | Preserves declaration order for deterministic output (I-1) |
| std::io::IsTerminal for TTY detection | In std since Rust 1.70, no external crate needed |
| `pluralizer` crate for pluralize/singularize | Full English rules without hand-rolling. Acceptable dependency for correctness. |
| `strsim` crate for "did you mean?" hints | minijinja doesn't provide suggestions on undefined vars. Edit distance against declared names serves I-10 (graceful degradation). |
| Register all 13 filters ourselves | Even for filters minijinja provides natively (replace, indent, join). We own exact behavior — upstream changes don't break us. Serves I-1 (determinism). |
| Render all templates upfront | Render ALL before ANY file write. Fail-fast, no partial writes. Templates reference variables not created files. |
| Extra variables pass through silently | LLMs construct variable objects with extra fields. Strictness breaks real usage. |
| Fail-fast on operation errors | Stop on first failure. Later ops may depend on earlier ones (inject targets prior create). |
| 5-phase incremental build | Each phase produces a working CLI that grows: validate/vars → render → run(create) → run(create+inject) → full test suite |

## Patterns Established

- **Acceptance criteria use EARS format** — every AC has a type (Event/State/Ubiquitous/Unwanted) and a traces-to test ID
- **Fixture-based integration tests** — each test case is a directory with recipe.yaml, vars.json, templates/, existing/, expected/. No code changes needed to add tests.
- **Dual output streams** — JSON to stdout for machines, colored text to stderr for humans. Auto-detect via IsTerminal.
- **Exit codes are API** — 0 success, 1 recipe, 2 template, 3 file op, 4 variable. Never changes.

## Known Issues / Tech Debt

- Large file handling for inject reads entire file into memory (fine for source files <10K lines, document as limitation)
- stdin can only serve one purpose per invocation (--vars-stdin consumes it)

## File Ownership

This workstream owns the entire `src/` tree for v0.1:

| File | Phase | Purpose |
|------|-------|---------|
| `Cargo.toml` | 1 | Dependencies and crate config |
| `src/main.rs` | 1-4 | CLI entry point, clap commands |
| `src/error.rs` | 1 | JigError enum, exit code mapping |
| `src/recipe.rs` | 1 | Recipe/VariableDecl/FileOp structs, YAML parsing |
| `src/variables.rs` | 1-2 | Variable types (P1), validation + merging (P2) |
| `src/filters.rs` | 2 | 13 built-in Jinja2 filters |
| `src/renderer.rs` | 2 | minijinja environment, template loading |
| `src/operations/mod.rs` | 3 | Operation dispatch, ExecutionContext, OpResult |
| `src/operations/create.rs` | 3 | File creation logic |
| `src/operations/inject.rs` | 4 | Content injection logic |
| `src/output.rs` | 3 | JSON + human output formatting |
| `tests/integration.rs` | 5 | Fixture-based integration test harness |
| `tests/fixtures/` | 5 | Test fixture directories |
