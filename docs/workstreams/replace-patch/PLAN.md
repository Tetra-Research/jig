# PLAN.md

> Workstream: replace-patch
> Last updated: 2026-04-04
> Status: Planned

## Objective

Extend jig from greenfield-only (create + inject) to brownfield operations — replacing regions in existing files and inserting content at structurally-determined locations. This is the v0.2 feature set that makes jig useful for the most common real-world task: extending existing code (adding fields, methods, endpoints) across multiple files in a single recipe.

Covers ARCHITECTURE.md Phases F (Replace Operation) and G (Patch Operation + Scope Detection).

## Phases

### Phase 1: Recipe Parsing Extensions
Status: Planned
Traces to: FR-1

Extend `recipe.rs` to parse `replace` and `patch` file operations into fully typed `FileOp` variants. Currently these are accepted by `RawFileOp` but rejected at conversion time (lines 276-291). This phase removes that rejection, adds the new types to `FileOp`, and validates all fields at parse time.

#### Milestones
- [ ] 1.1: Add new types to `src/recipe.rs` — `ReplaceSpec` (Between, Pattern), `Fallback` (Append, Prepend, Skip, Error), `Anchor` struct (pattern, scope, find, position), `ScopeType` enum (8 variants), `Position` enum (7 variants). Add `FileOp::Replace` and `FileOp::Patch` variants.
- [ ] 1.2: Extend `RawFileOp` deserialization to capture replace-specific fields (`between`, `pattern`, `fallback`) and patch-specific fields (`anchor` with `pattern`, `scope`, `find`, `position`). Add `RawAnchor` intermediate struct for serde.
- [ ] 1.3: Implement `convert_file_op` logic for replace: validate between vs pattern (exactly one), validate fallback value, compile-check regex patterns at parse time. For patch: validate anchor.pattern required, compile-check anchor pattern, validate scope type and position type against known variants, default scope to `line` and position to `after`.
- [ ] 1.4: Extend `FileOp::template()` to cover Replace and Patch variants.
- [ ] 1.5: Update `validate_templates()` to check template existence for replace and patch operations.
- [ ] 1.6: Extend `jig validate` and `jig vars` output to include replace/patch operations in their summaries.
- [ ] 1.7: Unit tests for all parsing paths — valid replace (between, pattern, fallback variants), valid patch (all scope/position combinations, with/without find), and error cases (conflicting fields, missing fields, invalid regex, invalid scope/position/fallback strings).

#### Validation Criteria
- Recipes with replace operations parse without error
- Recipes with patch operations parse without error
- All error cases from AC-1.5 through AC-1.16 produce exit code 1 with clear messages
- Regex patterns in between.start, between.end, pattern, and anchor.pattern are compile-checked at parse time
- `jig validate` reports replace/patch operation counts alongside create/inject
- Existing create/inject recipes still parse identically (no regressions)

#### Key Files
- `src/recipe.rs` (major changes — new types, new conversion logic)

#### Dependencies
- None (builds on existing recipe.rs structure)

---

### Phase 2: Replace Operation
Status: Planned
Traces to: FR-2, FR-8 (partial)

Implement the replace operation: match a region in an existing file and swap it with rendered content. Two modes: `between` (marker-delimited region) and `pattern` (regex-matched lines). Four fallback strategies when the match fails.

#### Milestones
- [ ] 2.1: Create `src/operations/replace.rs` — implement `execute()` function following the same signature pattern as create.rs and inject.rs. Core logic: read target file (or virtual_files), apply between or pattern matching, replace the region, write back.
- [ ] 2.2: Implement `between` mode — find start regex match, then find end regex match after start, replace content between them (exclusive of markers). Handle adjacent markers (empty region = insert between).
- [ ] 2.3: Implement `pattern` mode — find all contiguous lines matching regex, replace them with rendered content.
- [ ] 2.4: Implement fallback behavior — when match not found, dispatch to append/prepend/skip/error. Append and prepend write the content; skip returns OpResult::Skip; error returns OpResult::Error with rendered_content.
- [ ] 2.5: Wire replace dispatch into `operations/mod.rs` — add `FileOp::Replace` match arm in `execute_operation`.
- [ ] 2.6: Wire replace into `main.rs` — extend the template preparation loop to handle replace operations (render template content and target path). Add replace to the `PreparedOp` match arms.
- [ ] 2.7: Add verbose output for replace operations — match mode, matched region line range, lines replaced.
- [ ] 2.8: Unit tests — between (normal, empty region, adjacent markers), pattern (single line, multi-line), all four fallbacks, missing target file, end marker not found, dry-run with virtual_files.
- [ ] 2.9: Integration test fixtures — `replace-between`, `replace-between-empty`, `replace-pattern`, `replace-fallback-append`, `replace-fallback-prepend`, `replace-fallback-skip`, `replace-fallback-error`, `error-replace-no-match`, `error-replace-end-missing`.

#### Validation Criteria
- `between` mode preserves marker lines, replaces content between (AC-2.1, AC-2.7)
- `pattern` mode replaces matched lines entirely (AC-2.2)
- All four fallbacks work correctly (AC-2.3 through AC-2.6)
- Missing target file exits 3 with rendered content (AC-2.10)
- Missing end marker exits 3 with rendered content (AC-2.11)
- Dry-run mode uses and updates virtual_files (AC-2.13)
- Replace works in a recipe alongside create and inject operations
- All integration fixtures pass

#### Key Files
- `src/operations/replace.rs` (new)
- `src/operations/mod.rs` (dispatch update)
- `src/main.rs` (preparation update)
- `src/output.rs` (minor — replace action formatting, verbose diagnostics)
- `tests/fixtures/replace-*` (new)

#### Dependencies
- Phase 1 (recipe parsing for FileOp::Replace)

---

### Phase 3: Scope Detection
Status: Planned
Traces to: FR-3, FR-4, FR-5

Implement the scope detection module — the structural analysis layer that makes patch operations "smart." Two detection strategies (indentation-based, delimiter-based) and seven semantic position heuristics.

#### Milestones
- [ ] 3.1: Create `src/scope/mod.rs` — `ScopeResult` struct, `detect_scope()` dispatch function that routes to indent or delimiter based on `ScopeType`. Export the public API.
- [ ] 3.2: Create `src/scope/indent.rs` — indentation-based scope detection for `Block`, `ClassBody`, `FunctionBody`. Algorithm: find anchor indentation, walk forward collecting deeper-indented lines, stop when indentation returns to anchor level or shallower. Handle blank lines (don't terminate scope), multi-line class declarations (find the colon), empty scopes.
- [ ] 3.3: Create `src/scope/delimiter.rs` — delimiter-based scope detection for `Braces`, `Brackets`, `Parens`, `FunctionSignature`. Algorithm: find opening delimiter on/after anchor, count nesting depth with string literal and comment awareness, close when depth returns to zero. Handle nested delimiters, string escaping, empty scopes.
- [ ] 3.4: Create `src/scope/position.rs` — semantic position resolution within a scope. Implement all 7 positions: `Before`, `After`, `BeforeClose`, `AfterLastField`, `AfterLastMethod`, `AfterLastImport`, `Sorted`. Include fallback logic (field→before_close, method→before_close, import→before). Return `PositionResult` with insertion line, detected indentation, and fallback note.
- [ ] 3.5: Implement `find_within_scope()` — search for a string within scope boundaries, detect if the found line opens a sub-scope (heuristic: line contains `[`, `{`, or `(` as the last non-whitespace character, or assignment with delimiter), return `FindResult` with sub-scope if applicable.
- [ ] 3.6: Unit tests for indentation scopes — Python class body, Python function body, YAML nested block, empty scope, blank lines within scope, nested classes, nested functions, multi-line class declaration, decorator handling.
- [ ] 3.7: Unit tests for delimiter scopes — Rust struct braces, TypeScript interface braces, Go function braces, Python list brackets, function signature parens, nested delimiters, delimiters in strings, delimiters in comments, escaped delimiters, empty scope, missing delimiter errors.
- [ ] 3.8: Unit tests for position resolution — each of the 7 positions tested against realistic code blocks, fallback behavior when pattern not found, sorted insertion order.
- [ ] 3.9: Unit tests for find narrowing — find within class body, find with sub-scope detection (list_display = [...]), find not found error.

#### Validation Criteria
- Python class body correctly identified (AC-3.2): `class Foo:` through dedent
- Python function body correctly identified (AC-3.3): `def bar():` through dedent
- Blank lines don't terminate indentation scope (AC-3.4)
- Nested delimiters tracked correctly (AC-4.5)
- String literals don't affect delimiter counting (AC-4.6)
- Comments don't affect delimiter counting (AC-4.7)
- `after_last_field` finds `^\s+\w+\s*[:=]` pattern (AC-5.4)
- `after_last_method` finds complete method body end (AC-5.5, AC-5.11)
- `sorted` inserts in alphabetical order (AC-5.7)
- Fallbacks work: field→before_close, method→before_close, import→before (AC-5.8-5.10)
- Find narrows to sub-scope when found line opens one (AC-6.2)
- All scope types work across languages without configuration (AC-N1.3)
- Deterministic: same input = same scope boundaries (AC-N4.1)

#### Key Files
- `src/scope/mod.rs` (new)
- `src/scope/indent.rs` (new)
- `src/scope/delimiter.rs` (new)
- `src/scope/position.rs` (new)

#### Dependencies
- None (scope module is independent — tested in isolation before wiring to patch)

---

### Phase 4: Patch Operation
Status: Planned
Traces to: FR-6, FR-7, FR-8 (complete)

Implement the patch operation: anchor-based, scope-aware content insertion. This wires together the scope detection module (Phase 3) with the file operation pipeline, adds indentation matching, and completes verbose diagnostics.

#### Milestones
- [ ] 4.1: Create `src/operations/patch.rs` — implement `execute()` function. Core pipeline: read target file → skip_if check → find anchor via regex → detect scope → apply find narrowing (if any) → resolve position → match indentation → insert rendered content → write back.
- [ ] 4.2: Implement indentation matching — detect the indentation level at the insertion point from surrounding lines, adjust the rendered content's base indentation to match. Handle the case where the rendered template already has its own indentation (preserve relative indentation within the template, adjust only the base level).
- [ ] 4.3: Implement `scope: line` as the degenerate case — just insert after the anchor line, identical to inject's `after` mode. This ensures patch can always fall back to the simplest behavior.
- [ ] 4.4: Wire patch dispatch into `operations/mod.rs` — add `FileOp::Patch` match arm in `execute_operation`.
- [ ] 4.5: Wire patch into `main.rs` — extend the template preparation loop to handle patch operations (render template content, target path, and skip_if). Add patch to the `PreparedOp` match arms.
- [ ] 4.6: Complete verbose diagnostics for patch and replace — `ScopeDiagnostics` struct in output, displayed in both JSON and human modes. Show anchor line, scope range, insertion point, find match, position fallback.
- [ ] 4.7: Unit tests — full pipeline (anchor→scope→position→insert), skip_if, scope: line, indentation matching, anchor not found, scope detection failure, find narrowing, dry-run with virtual_files.
- [ ] 4.8: Integration test fixtures — `patch-class-body-field`, `patch-function-body`, `patch-braces`, `patch-brackets`, `patch-function-signature`, `patch-find-narrowing`, `patch-skip-if`, `patch-sorted`, `patch-scope-line`, `patch-indent-matching`, `patch-nested-delimiters`, `patch-string-delimiters`, `error-patch-no-anchor`, `error-patch-unbalanced`, `error-patch-find-missing`.

#### Validation Criteria
- Full patch pipeline works end-to-end: anchor→scope→position→insert (AC-7.1)
- skip_if prevents duplicate content (AC-7.2, AC-N3.1, AC-N3.3)
- Indentation matching adjusts inserted content (AC-7.7)
- scope: line behaves like inject after (AC-7.12)
- Scope detection errors include rendered content (AC-7.5, AC-N2.1)
- Find narrowing works with sub-scopes (AC-6.2, AC-6.4)
- Verbose output shows complete scope diagnostics (AC-8.1, AC-8.2, AC-8.5)
- All integration fixtures pass
- Existing create/inject tests still pass (no regressions)

#### Key Files
- `src/operations/patch.rs` (new)
- `src/operations/mod.rs` (dispatch update)
- `src/main.rs` (preparation update)
- `src/output.rs` (verbose scope diagnostics)
- `tests/fixtures/patch-*` (new)
- `tests/fixtures/error-patch-*` (new)

#### Dependencies
- Phase 1 (recipe parsing for FileOp::Patch)
- Phase 3 (scope detection module)

---

### Phase 5: Integration Testing + Full Validation
Status: Planned
Traces to: All FRs and NFRs (validation layer)

Comprehensive integration testing across all four operation types. Multi-operation recipes, idempotency, determinism, cross-language scope detection, and the full fixture suite.

#### Milestones
- [ ] 5.1: Create `combined-all-ops` fixture — a single recipe that uses create, inject, replace, and patch in sequence, demonstrating all four operation types working together.
- [ ] 5.2: Create `combined-patch-idempotent` fixture — a patch-heavy recipe run twice, second run should show all skips.
- [ ] 5.3: Cross-language scope detection fixtures — Python class body, Rust struct braces, TypeScript interface braces, Go function braces, YAML nested block. Each verifies scope detection against real-world code patterns.
- [ ] 5.4: Determinism test — run the same patch/replace recipe twice, assert byte-identical output.
- [ ] 5.5: Error recovery test — every error case produces structured error with rendered_content, enabling LLM fallback.
- [ ] 5.6: Snapshot tests — insta snapshots for verbose scope diagnostics (JSON and human output), new error message formats.
- [ ] 5.7: Verify `cargo clippy` passes clean and `cargo test` passes all tests (existing 191 + new).

#### Validation Criteria
- All four operation types work in a single recipe (ordered execution preserved)
- Idempotent recipes produce all skips on second run
- Same inputs = same output across runs (determinism)
- Every error case from the Error Handling table in SPEC.md is covered by at least one fixture
- All scope types tested against at least two different languages
- `cargo test` green, `cargo clippy` clean
- Total test count significantly above 191 (target: 300+)

#### Key Files
- `tests/fixtures/combined-*` (new)
- `tests/fixtures/patch-*-python`, `patch-*-rust`, etc. (new, language-specific)
- `tests/integration.rs` (may need minor updates for new fixture types)

#### Dependencies
- Phase 2 (replace operation)
- Phase 4 (patch operation)

## Dependencies

- **Depends on:** core-engine workstream (v0.1 — complete). Specifically: recipe.rs parsing infrastructure, operations/mod.rs dispatch, output.rs formatting, error.rs types, the existing integration test harness.
- **Blocks:** workflows (v0.3) — workflows chain recipes that include patch/replace operations. Libraries (v0.4) — library recipes use patch operations extensively (the "add-model-field" example from the spec).

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Scope type in recipe, not auto-detected from file extension | Recipe author specifies `scope: class_body` or `scope: braces` explicitly | Language detection is fragile and adds complexity. The recipe author knows the target language. Scope type is the right abstraction level. (ARCHITECTURE D-3, NFR-1 AC-N1.3) |
| Indentation vs delimiter are separate code paths | `scope/indent.rs` and `scope/delimiter.rs` as distinct modules | They share no logic. Indentation walks lines measuring whitespace. Delimiters count balanced pairs. Forcing a common abstraction would be artificial. |
| Semantic positions are heuristic, not parsed | Regex patterns like `^\s+\w+\s*[:=]` for field detection | Full parsing (tree-sitter) is out of scope. Heuristics cover 90%+ of real code. When they fail, the error includes rendered content for fallback. (ARCHITECTURE D-3, I-10) |
| Position fallback rather than error | `after_last_field` falls back to `before_close` if no fields found | A recipe that works on a class with 5 fields should also work on an empty class. Erroring on empty scopes would make recipes fragile. The fallback is noted in verbose output. |
| Indentation matching on insert | Patch auto-adjusts rendered content indentation to match insertion context | Template authors shouldn't need to know the exact indentation of every insertion point. Templates define relative indentation; jig adjusts the base level. |
| `find` narrowing auto-detects sub-scopes | If the `find` line contains an opening delimiter, jig detects the sub-scope | This enables the `list_display = [...]` pattern from the spec without requiring the recipe author to specify a second scope type. Keeps recipe YAML simple. |
| Replace preserves markers (between mode) | Start and end marker lines are kept; only content between is swapped | Markers serve as stable anchors for future re-runs. Removing them would break idempotency. The spec is explicit about this. |
| First match for anchor pattern | Patch uses first regex match (like inject's `at: first` default) | Predictable behavior. If the user needs a specific match, they write a more specific regex. Adding `at: first/last` to patch is a future option if needed. |

## Risks / Open Questions

- **Risk: Indentation heuristics fail on unusual code.** Python with mixed indentation, or code with unusual formatting, may confuse the indentation-based scope detection. Mitigation: clear error messages with rendered content, verbose mode showing scope boundaries, and the fallback-to-LLM design (I-10). Accept that edge cases exist and fail gracefully rather than trying to handle every case.

- **Risk: String literal detection in delimiter scopes is imperfect.** Multi-line strings (Python triple-quotes, Rust raw strings, JavaScript template literals with nested expressions) are hard to detect without full parsing. Mitigation: handle the common cases (single/double quoted, backtick), document the limitation, rely on the error-with-rendered-content fallback for edge cases.

- **Risk: `after_last_method` body detection.** Finding the end of a method's body (not just the `def` line) requires running scope detection recursively. This is correct but adds complexity. Mitigation: implement it — the alternative (inserting after the `def` line) would be wrong in most cases.

- **Open question: Should `pattern` mode in replace match a single contiguous block or all matches?** The spec says "the entire matched line(s)" which is ambiguous. Decision: match the first contiguous block of lines matching the pattern. If users need all occurrences, they can use multiple replace operations. This keeps behavior predictable.

- **Open question: Indentation matching precision.** Should the patch operation detect whether the project uses 2-space or 4-space indentation and adjust? Or just match the immediate context? Decision: match the immediate context (indentation of the line at the insertion point). This is simpler and always correct for the local context, even in files with mixed indentation.

## Execution Order

```
Phase 1 ──► Phase 2 ──► Phase 4 ──► Phase 5
recipe       replace      patch       integration
parsing                               tests
         ╲              ╱
          ► Phase 3 ──►
            scope
            detection
```

Phase 3 (scope detection) is independent of Phase 2 (replace) — they can be developed in parallel. Phase 4 (patch) depends on both Phase 1 (parsing) and Phase 3 (scope). Phase 5 validates everything together.

Within each phase, milestones are sequential — each builds on the previous one.
