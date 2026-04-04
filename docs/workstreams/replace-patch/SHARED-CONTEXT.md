# SHARED-CONTEXT.md

> Workstream: replace-patch
> Last updated: 2026-04-04

## Purpose

Deliver the v0.2 jig operations: `replace` (swap regions in existing files) and `patch` (scope-aware insertion using anchors). This extends jig from greenfield scaffolding to brownfield code extension — the most common real-world use case. After this workstream, a recipe can add a field to a Django model class, a parameter to a service method, an entry to an admin list_display, and a factory attribute — all in one `jig run` invocation.

## Current State

- **Implementation complete, review findings pending** (2026-04-04)
- 298 tests passing (284 unit + 2 CLI + 12 integration), `cargo clippy` clean
- All four operation types (create, inject, replace, patch) working end-to-end
- Scope detection module (`src/scope/`) fully implemented: indentation-based and delimiter-based
- Code review completed — identified 3 critical, 8 major, 3 minor issues (see PLAN.md "Review Findings")
- **No review findings have been fixed in code.** Commit 6f9b69b only added review artifact docs.
- 11 of 26 spec-required integration fixtures still missing (unit tests cover most behaviors)
- Verbose scope diagnostics (FR-8) not yet implemented — `ScopeDiagnostics` struct doesn't exist

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Scope type specified in recipe, not auto-detected from file extension | Language detection is fragile. The recipe author knows the target language. `scope: class_body` vs `scope: braces` is the right abstraction. (ARCHITECTURE D-3, SPEC NFR-1) |
| Indentation and delimiter scopes are separate code paths | They share no logic — indentation walks whitespace levels, delimiters count balanced pairs. No common abstraction needed. |
| Semantic positions use heuristic regex, not AST parsing | `^\s+\w+\s*[:=]` for fields, `^\s+def \w+` for methods. Covers 90%+ of real code. Tree-sitter is out of scope. When heuristics fail, error includes rendered content for LLM fallback. (ARCHITECTURE D-3, I-10) |
| Position fallback (e.g., field→before_close) rather than error | A recipe for "add field to class body" should work on empty classes too. Erroring on empty scopes makes recipes fragile. Fallback noted in verbose output. |
| Patch auto-adjusts indentation at insertion point | Template authors define relative indentation; jig adjusts base level to match the insertion context. Removes a whole class of "wrong indentation" bugs. |
| `find` auto-detects sub-scopes | If the found line contains an opening delimiter (`[`, `{`, `(`), jig detects the sub-scope automatically. Keeps recipe YAML simple for the common `list_display = [...]` pattern. |
| Replace `between` preserves marker lines | Only content between markers is swapped. Markers are stable anchors for re-runs. Removing them breaks idempotency. |
| Patch uses first regex match (no `at` field) | Predictable default. More specific regex solves the "wrong match" case. Adding `at: first/last` is a future option if needed. |
| `pattern` mode replaces first contiguous block | Not all matches in the file. Predictable behavior; multiple replace operations handle multiple regions. |
| Match insertion point indentation, not file-wide convention | Detect indentation from immediate context (the line at the insertion point), not from file analysis. Always correct locally, even in mixed-indentation files. |

## Patterns Established

- **Scope module is independent and tested in isolation.** `src/scope/` has no dependency on the operations module or the recipe parser. It takes `&[&str]` (file lines) and returns `ScopeResult`. This makes it trivially unit-testable with inline string snippets.
- **All four operations follow the same execute() signature pattern.** Each takes a rendered path, rendered content, operation-specific config, `&mut ExecutionContext`, and `verbose: bool`, and returns `OpResult`. No trait — just consistent function signatures.
- **Error messages for scope failures include rendered content.** Every file operation error (exit code 3) bundles the rendered template output, independent of `--verbose`. This is the I-10 contract: jig never wastes rendering work.
- **Integration test fixtures follow the same directory structure as v0.1.** `recipe.yaml`, `vars.json`, `templates/`, `existing/`, `expected/`, optionally `expected_output.json` and `expected_exit_code`. No harness changes needed.
- **`CharScanner` utility in `delimiter.rs`.** Stateful line/col scanner for walking delimiter-based code. Tracks position, handles string literal detection, comment skipping, and escape sequences. Useful pattern but has a byte/char offset bug that needs fixing.
- **Indentation adjustment is a separate function.** `adjust_indentation()` in `patch.rs` detects base indent from rendered content's first non-empty line, strips it, and re-applies the target context's indent. Preserves relative indentation within templates.
- **`PreparedOp` enum in `main.rs` bridges rendering and execution.** Templates are rendered upfront (D-1), then `PreparedOp` variants carry the rendered content to the operation executor. Replace and Patch variants were added alongside Create and Inject.

## Known Issues / Tech Debt

### From review (unfixed — see PLAN.md "Review Findings" for full list)

- **`write_back` silently swallows write errors** in `patch.rs` and `replace.rs` fallback paths. Uses `let _ = std::fs::write(...)`. Violates AC-7.11, AC-2.14, I-10. **Critical.**
- **`Position::Sorted` is stub-implemented.** `position.rs:170-184` inserts at end-of-scope, not alphabetically. Either implement or reject at parse time. **Critical.**
- **Byte/char index mismatch in `delimiter.rs:87`.** `CharScanner` tracks char positions but Rust slicing uses bytes. Multi-byte UTF-8 panics. **Critical.**
- **`find` narrowing doesn't re-anchor when no sub-scope detected.** Position resolves against original scope, not the found line. Violates AC-6.1.
- **`scope: line` auto-adjusts indentation.** Should behave identically to inject's `after` mode per AC-7.12 — no indentation adjustment.
- **`find_opening` doesn't skip strings/comments.** Only the nesting-depth loop has string/comment awareness.
- **`AfterLastMethod` regex misses Rust qualifiers.** No `const fn`, `unsafe fn`, `pub(crate) fn`, `extern "C" fn`.
- **Location string uses `Debug` not `Display`.** Produces `classbody` instead of `class_body`.
- **Indentation scope uses length comparison, not literal prefix comparison** per AC-3.8.
- **11 of 26 spec-required integration fixtures missing.**
- **Verbose scope diagnostics (FR-8) not implemented.** No `ScopeDiagnostics` struct.

### Pre-existing (from planning)

- **String literal detection is imperfect.** Delimiter scope detection handles single/double/backtick strings but cannot handle multi-line strings (Python triple-quotes, Rust raw strings `r#"..."#`, JS template literals with nested `${}` expressions). These are documented edge cases; the fallback-to-LLM design handles them.
- **Comment detection is basic.** Single-line (`//`, `#`) and multi-line (`/* */`) comments are recognized. Language-specific comment styles (Lua `--[[]]`, HTML `<!-- -->`) are not. Acceptable for the target languages.
- **`after_last_method` requires recursive scope detection.** Finding the end of a method body means running indentation/delimiter scope detection from the method's `def`/`fn` line. This is correct but adds complexity. The alternative (inserting after the `def` line, not the body) would be wrong.
- **No `at: first/last` for patch operations.** Patch always uses the first anchor match. If this becomes a friction point (recipes need the last match), it's a straightforward addition.
- **No workflow orchestration yet (v0.3).** Multi-file "add-field" recipes work, but conditional steps, variable mapping, and error handling modes are deferred.

## File Ownership

This workstream owns:

| File | Phase | Purpose |
|------|-------|---------|
| `src/recipe.rs` | 1 | Extend with FileOp::Replace, FileOp::Patch, and supporting types (shared with core-engine — additive changes only) |
| `src/operations/replace.rs` | 2 | Replace operation execution (new file) |
| `src/operations/patch.rs` | 4 | Patch operation execution (new file) |
| `src/operations/mod.rs` | 2, 4 | Dispatch update for replace and patch (shared — additive) |
| `src/scope/mod.rs` | 3 | Scope detection dispatch and ScopeResult type (new file) |
| `src/scope/indent.rs` | 3 | Indentation-based scope detection (new file) |
| `src/scope/delimiter.rs` | 3 | Delimiter-based scope detection (new file) |
| `src/scope/position.rs` | 3 | Semantic position resolution (new file) |
| `src/main.rs` | 2, 4 | Template preparation for replace/patch ops (shared — additive) |
| `src/output.rs` | 2, 4 | Verbose scope diagnostics (shared — additive) |
| `tests/fixtures/replace-*` | 2, 5 | Replace operation test fixtures (new directories) |
| `tests/fixtures/patch-*` | 4, 5 | Patch operation test fixtures (new directories) |
| `tests/fixtures/error-replace-*` | 2, 5 | Replace error case fixtures (new directories) |
| `tests/fixtures/error-patch-*` | 4, 5 | Patch error case fixtures (new directories) |
| `tests/fixtures/combined-all-ops` | 5 | All four op types in one recipe (new directory) |
| `tests/fixtures/combined-patch-idempotent` | 5 | Patch idempotency fixture (new directory) |
