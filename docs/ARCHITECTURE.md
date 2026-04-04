# ARCHITECTURE.md

Target architecture for jig, the iterative build strategy to get there, and the evaluation criteria at each gate. This document grounds all workstream planning.

## System Overview

jig is a linear pipeline: parse CLI args, load recipe, validate variables, render templates, execute file operations, format output, exit. Every stage is independently testable. No stage has side effects except operation execution (which writes files).

```
CLI Input
  |
  v
+------------------+
| CLI Parser       |  args, flags, var sources
| (clap)           |
+------------------+
  |
  v
+------------------+
| Recipe Loader    |  YAML -> Recipe struct
| (serde_yaml)     |
+------------------+
  |
  v
+------------------+
| Variable         |  JSON input -> typed, validated Variables
| Validator        |  merge: defaults < file < stdin < inline
| (serde_json)     |
+------------------+
  |
  v
+------------------+
| Template         |  template + variables -> rendered string
| Renderer         |  custom filters registered here
| (minijinja)      |
+------------------+
  |
  v
+------------------+
| Operation        |  rendered content + file operation spec
| Executor         |  -> file writes (or dry-run recording)
+------------------+
  |
  v
+------------------+
| Output           |  results -> JSON (stdout) or human (stderr)
| Formatter        |  TTY auto-detect, --json, --quiet
+------------------+
  |
  v
Exit Code (0-4)
```

## Module Map

### `src/main.rs` — Entry Point

Owns: CLI argument parsing via clap. Wires the pipeline together.
Depends on: every other module (it's the composition root).

Responsibilities:
- Define CLI commands: `run`, `render`, `validate`, `vars`
- Parse `--vars`, `--vars-file`, `--vars-stdin`, `--dry-run`, `--json`, `--quiet`, `--force`, `--base-dir`, `--verbose`
- Call pipeline stages in order
- Map errors to exit codes

### `src/recipe.rs` — Recipe Parsing

Owns: `Recipe`, `VariableDecl`, `FileOp` structs. YAML deserialization.
Depends on: serde, serde_yaml.

Responsibilities:
- Deserialize `recipe.yaml` into `Recipe` struct
- Validate structural correctness (required fields present, template files exist relative to recipe)
- Resolve template paths relative to the recipe file location (I-7: templates live with the consumer)
- Distinguish operation types: create vs inject vs replace vs patch

Key types:
```rust
struct Recipe {
    name: Option<String>,
    description: Option<String>,
    variables: IndexMap<String, VariableDecl>,
    files: Vec<FileOp>,
}

struct VariableDecl {
    var_type: VarType,       // string, number, boolean, array, object, enum
    required: bool,
    default: Option<Value>,
    description: Option<String>,
    values: Option<Vec<String>>,  // for enum type
    items: Option<VarType>,       // for array type
}

enum FileOp {
    Create { template: String, to: String, skip_if_exists: bool },
    Inject { template: String, inject: String, mode: InjectMode, skip_if: Option<String> },
    Replace { template: String, replace: String, spec: ReplaceSpec, fallback: Fallback },
    Patch { template: String, patch: String, anchor: Anchor, skip_if: Option<String> },
}
```

### `src/variables.rs` — Variable Validation

Owns: Variable merging, type checking, default application.
Depends on: serde_json, recipe types.

Responsibilities:
- Parse JSON from multiple sources (inline string, file, stdin)
- Merge with precedence: recipe defaults < vars-file < vars-stdin < inline --vars
- Type-check each value against its `VariableDecl`
- Validate required fields are present
- Validate enum values against allowed set
- Validate array item types
- Produce clear error messages with hints (I-4)

### `src/renderer.rs` — Template Rendering

Owns: minijinja `Environment` setup, filter registration, template loading.
Depends on: minijinja, heck (for case conversion filters).

Responsibilities:
- Create minijinja `Environment` with templates loaded from recipe-relative paths
- Register built-in filters: `snakecase`, `camelcase`, `pascalcase`, `kebabcase`, `upper`, `lower`, `capitalize`, `replace`, `pluralize`, `singularize`, `quote`, `indent`, `join`
- Render a template with a variables context
- Return `Result<String>` — never silently produce wrong output (I-10)

Design note: The renderer is stateless per invocation. Create environment, register filters, load templates, render, return. No caching needed at v0.1 scale.

### `src/operations/mod.rs` — Operation Dispatch

Owns: operation dispatch, dry-run tracking.
Depends on: operation implementations, renderer output.

```rust
struct ExecutionContext {
    base_dir: PathBuf,
    dry_run: bool,
    force: bool,
    virtual_files: HashMap<PathBuf, String>,  // populated by create ops in dry-run mode
}

enum OpResult {
    Success { action: &'static str, path: PathBuf, lines: usize, location: Option<String>, rendered_content: Option<String> },
    Skip { path: PathBuf, reason: String, rendered_content: Option<String> },
    Error { path: PathBuf, error: StructuredError, rendered_content: String },
}

// Note: `verbose` is an OutputMode concern passed to the formatter, not part of ExecutionContext.

// FileOp enum match dispatch — no trait needed for v0.1 (D-4)
fn execute_operation(op: &FileOp, rendered: &str, ctx: &ExecutionContext) -> OpResult { ... }
```

Operations execute in declaration order (I-9). Each operation receives the already-rendered content — rendering happens before execution, not during.

### `src/operations/create.rs` — Create Operation

Owns: New file creation logic.
Depends on: std::fs.

Responsibilities:
- Render the `to` path as a template (it can contain `{{ variables }}`)
- Create parent directories as needed
- Write rendered content to the target path
- `skip_if_exists`: if true and target exists, return `OpResult::Skip`
- If not `--force` and target exists and `skip_if_exists` is false, return error

### `src/operations/inject.rs` — Inject Operation

Owns: Content injection into existing files.
Depends on: regex.

Responsibilities:
- Read the target file
- `skip_if`: search for string in file content, skip if found (I-2: idempotency)
- Find the anchor line using regex pattern
- `at: first` (default) or `at: last` — which match to use
- Insert rendered content `after` or `before` the anchor line
- `prepend`: insert at beginning of file
- `append`: insert at end of file
- Write the modified content back

### `src/operations/replace.rs` — Replace Operation

Owns: Region replacement in existing files.
Depends on: regex.

Responsibilities:
- `between`: find start pattern, find end pattern, replace everything between them (exclusive of markers)
- `pattern`: find matching line(s), replace them entirely
- `fallback`: what to do when pattern not found — `append`, `prepend`, `skip`, `error` (default)
- Preserve line endings and file encoding

### `src/operations/patch.rs` — Patch Operation

Owns: Scope-aware content insertion. Delegates scope detection.
Depends on: scope module, regex.

Responsibilities:
- Find anchor pattern in file
- Determine scope boundaries using the scope module
- Apply `find` narrowing within the scope
- Insert rendered content at the specified position
- `skip_if`: idempotency check within the scope

### `src/scope/mod.rs` — Scope Detection Dispatch

Owns: Scope type classification, dispatch to indent vs delimiter strategies.
Depends on: indent, delimiter, position submodules.

```rust
enum ScopeType {
    Line,              // just the matched line
    Block,             // indent-based block
    ClassBody,         // indent-based class body
    FunctionBody,      // indent-based function body
    FunctionSignature, // delimiter: ( to )
    Braces,            // delimiter: { to }
    Brackets,          // delimiter: [ to ]
    Parens,            // delimiter: ( to )
}
```

### `src/scope/indent.rs` — Indentation-Based Scope Detection

Owns: Scope detection for indentation-significant languages (Python, YAML).

Algorithm:
1. Find the anchor line's indentation level
2. Walk forward: scope includes all lines indented deeper than the anchor
3. Scope ends when indentation returns to the same or shallower level
4. Handle blank lines within the scope (they don't end it)

### `src/scope/delimiter.rs` — Delimiter-Based Scope Detection

Owns: Scope detection for delimiter-based languages (C-family, JSON, TypeScript).

Algorithm:
1. From the anchor line, find the opening delimiter
2. Count nesting depth (handle nested delimiters)
3. Scope is everything between opening and closing delimiter (exclusive)
4. Handle string literals (don't count delimiters inside strings)

### `src/scope/position.rs` — Semantic Position Heuristics

Owns: Position determination within a scope.

| Position | Heuristic |
|----------|-----------|
| `before` | First line of scope |
| `after` | Last line of scope |
| `before_close` | Line before closing delimiter or dedent |
| `after_last_field` | Last line matching `^\s+\w+\s*[:=]` |
| `after_last_method` | Last line matching `^\s+def \w+` |
| `after_last_import` | Last line matching `^\s*(from\|import) ` |
| `sorted` | Alphabetical insertion among siblings |

### `src/filters.rs` — Custom Jinja2 Filters

Owns: Filter function implementations and registration.
Depends on: heck (case conversions), minijinja (filter API).

Filters are pure functions: `fn(value: &str, args...) -> String`. No side effects, no I/O. This module is the thinnest — most filters are one-liners wrapping `heck` or standard string operations.

### `src/output.rs` — Output Formatting

Owns: Dual-stream output (I-8), TTY detection.
Depends on: serde_json, owo-colors.

Responsibilities:
- Collect `Vec<OpResult>` from operation execution
- JSON mode: serialize results to stdout
- Human mode: colored, summarized output to stderr
- Auto-detect: if stdout is a TTY, use human mode; if piped, use JSON
- `--json`: force JSON
- `--quiet`: suppress all stderr output
- `--verbose`: include rendered content in output

### `src/error.rs` — Structured Error Types

Owns: Error types, exit code mapping, error formatting.
Depends on: nothing (leaf module).

```rust
struct StructuredError {
    what: String,
    where_: String,
    why: String,
    hint: String,
}

enum JigError {
    RecipeValidation(StructuredError),     // exit 1
    TemplateRendering(StructuredError),    // exit 2
    FileOperation(StructuredError),        // exit 3
    VariableValidation(StructuredError),   // exit 4
}
```

Every error includes: what, where, why, hint (I-4). File operation errors include the rendered content so the caller can fall back to manual editing (I-10).

### `src/workflow.rs` — Multi-Recipe Orchestration (v0.3+)

Owns: Workflow definition, step execution, conditional logic, variable mapping.
Depends on: recipe, variables, renderer, operations.

Not built until v0.3. Included here for architectural completeness.

### `src/library/` — Library Management (v0.4+)

Owns: Library manifest parsing, installation, discovery, convention mapping.
Depends on: recipe, filesystem, git (for remote installs).

Not built until v0.4. Included here for architectural completeness.

## Dependency Map

```
                     main.rs
                    /   |   \
                   /    |    \
              recipe  variables  output
                |       |         |
                v       v         v
             renderer  (merges)  error
                |
                v
           operations/mod
           /    |    \    \
       create inject replace patch
                              |
                              v
                          scope/mod
                         /    |    \
                    indent  delim  position
```

External crate usage:
- clap: main.rs only
- serde + serde_yaml: recipe.rs
- serde_json: variables.rs, output.rs
- minijinja: renderer.rs
- regex: inject.rs, replace.rs, patch.rs, scope/*.rs
- heck: filters.rs
- owo-colors: output.rs
- thiserror: error.rs
- indexmap: recipe.rs, variables.rs
- pluralizer: filters.rs
- strsim: renderer.rs (did-you-mean hints)
- insta: tests (snapshot testing)

## Key Design Decisions

### D-1: Render-Then-Execute

All templates are rendered before any operation executes. The pipeline renders every template in the recipe upfront, then executes operations sequentially. This means:
- A rendering error in any template prevents all file writes
- The rendered content is always available for error messages (I-4)
- Operations are simpler — they just deal with strings and files
- Note: execution failures (e.g., inject target missing) can still produce partial writes if earlier operations succeeded. Recovery: re-run with `--force` or use rendered content from error output for manual editing.

### D-2: Recipe-Relative Template Resolution

Template paths in recipes are relative to the recipe file, not the working directory. This keeps recipes self-contained (I-7). The `--base-dir` flag only affects output paths (where files are written), not template paths (where templates are read from).

### D-3: Regex for Anchoring, Not Parsing

Anchors use regex, not AST parsing. This is deliberate:
- Language-agnostic (same tool works for Python, TypeScript, Go, Rust)
- No parser dependencies (keeps binary small, I-6)
- Failure is explicit (regex didn't match = clear error, not a silent wrong parse)
- Scope detection adds just enough structure without full parsing

### D-4: Operations Are Values, Not Traits (Initially)

For v0.1, operations can be a simple enum with match dispatch rather than a trait-object pattern. The trait abstraction is only needed if we add user-defined operations, which isn't planned. Keep it simple — an enum with four variants covers all cases through v0.4.

### D-5: No Global State

No singletons, no global config objects, no lazy_static. Every function takes its inputs as arguments. This makes testing trivial and ensures determinism (I-1).

## Iterative Build Strategy

Each phase produces a working, testable, demoable artifact. Nothing is "wired up later." Each phase can be evaluated independently.

### Phase A: Skeleton + Recipe Parsing

**Build:**
- `Cargo.toml` with dependencies: serde, serde_yaml, serde_json, clap
- `src/main.rs` — clap CLI with `validate` and `vars` subcommands
- `src/recipe.rs` — Recipe struct, YAML deserialization, structural validation
- `src/error.rs` — error types and exit codes
- `src/variables.rs` — VariableDecl types (no validation logic yet, just the types)

**Demo:**
```bash
jig validate recipe.yaml          # prints validation result, exits 0 or 1
jig vars recipe.yaml              # prints expected variables as JSON
```

**Evaluate:**
- Can parse the example recipe from the PRD
- Rejects invalid YAML with exit code 1 and a clear message
- Template file existence is checked relative to recipe location
- `jig vars` output matches the PRD's expected format
- Unit tests: valid recipe, missing fields, bad types, missing templates

### Phase B: Variable Validation + Template Rendering

**Build:**
- `src/variables.rs` — full validation: type checking, required fields, defaults, merging
- `src/renderer.rs` — minijinja environment, template loading
- `src/filters.rs` — all built-in filters
- Add `render` subcommand to CLI
- Add `--vars`, `--vars-file`, `--vars-stdin` to CLI

**Demo:**
```bash
jig render template.j2 --vars '{"class_name": "Foo"}'     # rendered output to stdout
jig render template.j2 --vars '{"bad": 123}' 2>&1         # type error, exit 4
```

**Evaluate:**
- All 13 built-in filters produce correct output
- Variable type checking catches mismatches (string vs number, missing required, bad enum value)
- Merge precedence works: defaults < file < stdin < inline
- Template syntax errors produce exit code 2 with line number
- Undefined variables produce a helpful error with "did you mean?" hint
- Snapshot tests (insta) for rendered output across filter combinations

### Phase C: Create Operation + Output Formatting

**Build:**
- `src/operations/mod.rs` — operation dispatch (create only)
- `src/operations/create.rs` — file creation with directory creation, skip_if_exists
- `src/output.rs` — JSON and human-readable output, TTY detection
- Add `run` subcommand to CLI
- Add `--dry-run`, `--json`, `--quiet`, `--force`, `--base-dir`, `--verbose`

**Demo:**
```bash
jig run recipe.yaml --vars '...'                 # creates files
jig run recipe.yaml --vars '...' --dry-run       # preview without writing
jig run recipe.yaml --vars '...' --json | jq .   # machine-readable output
jig run recipe.yaml --vars '...'                 # second run: skip_if_exists works
```

**Evaluate:**
- Template paths in `to` field render correctly (`tests/{{ module | replace('.', '/') }}/...`)
- Parent directories are created automatically
- `skip_if_exists` prevents overwriting
- `--force` overrides skip_if_exists
- JSON output matches the format in the PRD
- Human output is colored and readable
- Dry-run produces identical JSON output but writes no files
- Second run with same vars shows all skips (I-2)
- Integration test fixtures: recipe + vars + expected output directory

### Phase D: Inject Operation

**Build:**
- `src/operations/inject.rs` — all injection modes
- Wire inject dispatch into operations/mod.rs
- regex dependency added

**Demo:**
```bash
# Recipe with create + inject operations
jig run recipe.yaml --vars '...'         # creates file, injects into existing
jig run recipe.yaml --vars '...'         # second run: skip_if prevents duplication
```

**Evaluate:**
- `after` regex: content appears on the line after the match
- `before` regex: content appears on the line before the match
- `prepend`: content at start of file
- `append`: content at end of file
- `at: first` vs `at: last`: correct match selection when regex matches multiple lines
- `skip_if`: string search prevents duplicate injection (I-2)
- Regex match failure produces exit code 3 with the pattern and file path in the error
- Integration test fixtures for each injection mode
- **This completes the v0.1 feature set**

### Phase E: Integration Test Framework

**Build:**
- `tests/` directory with fixture-based integration test harness
- Fixture format: `recipe.yaml` + `vars.json` + `templates/` + `existing/` + `expected/`
- Test runner: copy existing/ to temp, run jig, diff against expected/
- Snapshot tests for all output formats
- Error case fixtures (missing vars, bad regex, missing template, missing target file)

**Evaluate:**
- Every operation mode has at least one fixture
- Every error case has a fixture that asserts exit code + error message content
- `cargo test` runs all unit tests + integration tests + snapshot tests
- Adding a new test case is just adding a directory (no code changes)

### Phase F: Replace Operation (v0.2 start)

**Build:**
- `src/operations/replace.rs` — between, pattern, fallback modes

**Evaluate:**
- `between` start/end: content between markers is replaced
- `pattern`: matched lines are replaced
- `fallback: append/prepend/skip/error` each work correctly
- Markers themselves are preserved (only content between them changes)

### Phase G: Patch Operation + Scope Detection (v0.2 completion)

**Build:**
- `src/scope/mod.rs` — scope type dispatch
- `src/scope/indent.rs` — indentation-based scope detection
- `src/scope/delimiter.rs` — delimiter-based scope detection
- `src/scope/position.rs` — semantic position heuristics
- `src/operations/patch.rs` — anchor + scope + position + find

**Evaluate:**
- Indentation scope: correctly finds class/function body boundaries in Python code
- Delimiter scope: correctly finds brace/bracket/paren boundaries in TypeScript/Rust
- Nested delimiters handled correctly
- String literals don't confuse delimiter counting
- Each semantic position (after_last_field, after_last_method, etc.) tested with real code snippets
- `find` narrowing works within a scope
- `--verbose` shows scope boundaries and insertion point
- Full patch recipe from the PRD (add-model-field) works against a Django-style fixture
- Scope parse failure produces structured error with rendered content (I-4, I-10)

### Phase H: Workflows (v0.3)

**Build:**
- `src/workflow.rs` — workflow definition, step execution
- `workflow` subcommand in CLI
- Conditional steps, variable mapping, error handling modes

**Evaluate:**
- Multi-recipe workflows execute in order
- `when` expressions skip/include steps correctly
- `vars_map` renames variables between steps
- `on_error: stop/continue/report` behaves correctly
- Per-step results in JSON output

### Phase I: Libraries (v0.4)

**Build:**
- `src/library/mod.rs` — manifest parsing
- `src/library/install.rs` — add/remove/update
- `src/library/discover.rs` — recipe listing
- `src/library/conventions.rs` — convention mapping and overrides
- `library` subcommand in CLI

**Evaluate:**
- Install from local directory works
- Install from git URL works
- Convention overrides in .jigrc.yaml apply correctly
- `jig library recipes <name>` lists all recipes
- Project-local extensions and template overrides work

## Testing Strategy

Three layers, matching the pipeline architecture:

| Layer | What | Tool | Location |
|-------|------|------|----------|
| **Unit** | Individual functions: parsing, validation, filter output, scope detection | `#[cfg(test)]` inline | Each `src/*.rs` |
| **Integration** | Full pipeline: recipe + vars in, files out, diff against expected | Fixture directories | `tests/fixtures/` |
| **Snapshot** | Rendered template output, JSON output format, error messages | `insta` crate | `tests/snapshots/` |

### Fixture Directory Convention

```
tests/fixtures/<test-name>/
  recipe.yaml              # input recipe
  vars.json                # input variables
  templates/               # template files referenced by recipe
    *.j2
  existing/                # files that exist before jig runs (for inject/replace/patch)
    *.py, *.ts, etc.
  expected/                # what the output directory should look like after
    *.py, *.ts, etc.
  expected_output.json     # optional: assert on jig's JSON stdout
  expected_exit_code       # optional: file containing just "0", "1", etc.
```

Test runner:
1. Copy `existing/` to a temp directory
2. Run `jig run recipe.yaml --vars-file vars.json --base-dir $tmp --json`
3. Diff temp directory against `expected/`
4. If `expected_output.json` exists, assert JSON output matches
5. If `expected_exit_code` exists, assert exit code matches

## What This Architecture Does NOT Cover

These are explicitly out of scope for the architecture and will be addressed in later workstreams:

- **Scan, Infer, Check** (v0.7-v0.8) — reverse operations, pattern learning, conformance
- **Schema-first generation** (v0.9) — OpenAPI/SQL/proto/GraphQL input
- **Observation engine** (post-1.0) — Claude Code hook for pattern discovery
- **Custom filters via shell commands** — .jigrc.yaml `filters:` block
- **Distribution** (v0.5) — CI, Homebrew, npm wrapper, Nix flake
- **Claude Code plugin** (v0.6) — /jig:init, /jig:doctor skills
- **Tree-sitter integration** (post-1.0) — optional AST-aware scoping
