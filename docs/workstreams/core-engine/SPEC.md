# SPEC.md

> Workstream: core-engine
> Last updated: 2026-04-02
> Scope: v0.1 (Phases A-E from ARCHITECTURE.md)

## Overview

The core engine workstream delivers the minimum pipeline that makes jig useful: parse a recipe, validate variables, render templates, execute create and inject operations, and report results. This is the v0.1 feature set — a working CLI that an LLM can call to produce files from recipes.

Out of scope for this workstream: replace operations, patch operations, scope detection, workflows, libraries, scan/infer/check, custom filters via shell, .jigrc.yaml config.

## Requirements

### Functional Requirements

#### FR-1: Recipe Parsing

Deserialize a YAML recipe file into a validated internal representation. Resolve template paths relative to the recipe file location. Reject structurally invalid recipes with clear errors.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-1.1 | Event | WHEN a valid recipe YAML is provided, the system SHALL parse it into a Recipe struct containing name, description, variables, and files | TEST-1.1 |
| AC-1.2 | Event | WHEN a recipe declares variables with type, required, default, description, values (enum), and items (array), the system SHALL parse all fields into the VariableDecl struct | TEST-1.2 |
| AC-1.3 | Event | WHEN a recipe declares `create` file operations with template, to, and skip_if_exists fields, the system SHALL parse them as FileOp::Create | TEST-1.3 |
| AC-1.4 | Event | WHEN a recipe declares `inject` file operations with template, inject, after/before/prepend/append, at, and skip_if fields, the system SHALL parse them as FileOp::Inject | TEST-1.4 |
| AC-1.5 | Unwanted | IF the recipe YAML is malformed (invalid YAML syntax), the system SHALL exit with code 1 and an error message identifying the parse failure location | TEST-1.5 |
| AC-1.6 | Unwanted | IF a required field is missing from the recipe (e.g., files with no template), the system SHALL exit with code 1 and name the missing field | TEST-1.6 |
| AC-1.7 | Event | WHEN a recipe references template files, the system SHALL resolve those paths relative to the recipe file location, not the working directory | TEST-1.7 |
| AC-1.8 | Unwanted | IF a referenced template file does not exist at the resolved path, the system SHALL exit with code 1 and report which template is missing and where it looked | TEST-1.8 |
| AC-1.9 | Event | WHEN the recipe has optional metadata fields (name, description), the system SHALL accept recipes with or without them | TEST-1.9 |
| AC-1.10 | Unwanted | IF the recipe contains an unknown operation type (e.g., `patch`, `replace`), the system SHALL exit with code 1 and report "unknown operation type '<name>' — this operation is not supported in v0.1" | TEST-1.10 |
| AC-1.11 | Event | WHEN the recipe has an empty `files: []` array, the system SHALL exit 0 with an empty operations array | TEST-1.11 |
| AC-1.12 | Event | WHEN the recipe has no `variables` key or an empty `variables` map, the system SHALL accept the recipe as valid | TEST-1.12 |
| AC-1.13 | Unwanted | IF the recipe file does not exist, the system SHALL exit with code 1 naming the missing path | TEST-1.13 |
| AC-1.14 | Unwanted | IF a file operation contains more than one of `to`/`inject`/`replace`/`patch` fields, the system SHALL exit with code 1 reporting the ambiguous operation type | TEST-1.14 |
| AC-1.15 | Unwanted | IF a file operation contains none of `to`/`inject`/`replace`/`patch` fields, the system SHALL exit with code 1 reporting the missing operation type | TEST-1.15 |

#### FR-2: Variable Validation

Accept variables as JSON from multiple sources, merge them with defined precedence, and type-check against recipe declarations.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-2.1 | Event | WHEN `--vars` is provided with a JSON string, the system SHALL parse it as the variable input | TEST-2.1 |
| AC-2.2 | Event | WHEN `--vars-file` is provided with a path to a JSON file, the system SHALL read and parse the file as variable input | TEST-2.2 |
| AC-2.3 | Event | WHEN `--vars-stdin` is provided, the system SHALL read JSON from stdin as variable input | TEST-2.3 |
| AC-2.4 | Event | WHEN multiple variable sources are provided, the system SHALL merge with precedence: recipe defaults < vars-file < vars-stdin < inline --vars | TEST-2.4 |
| AC-2.5 | Event | WHEN a variable is declared as `required: true` and no value is provided (after merging), the system SHALL exit with code 4 naming the missing variable and providing a hint | TEST-2.5 |
| AC-2.6 | Event | WHEN a variable is declared with a `default` and no value is provided, the system SHALL use the default value | TEST-2.6 |
| AC-2.7 | Unwanted | IF a provided variable value does not match its declared type (e.g., string given for number, object given for array), the system SHALL exit with code 4 with expected vs actual type | TEST-2.7 |
| AC-2.8 | Event | WHEN a variable is declared as `type: enum` with `values: [a, b, c]`, the system SHALL reject values not in the allowed set with exit code 4 | TEST-2.8 |
| AC-2.9 | Event | WHEN a variable is declared as `type: array` with `items: string`, the system SHALL validate that each array element matches the item type | TEST-2.9 |
| AC-2.10 | Ubiquitous | The system SHALL accept all six variable types: string, number, boolean, array, object, enum | TEST-2.10 |
| AC-2.11 | Ubiquitous | The system SHALL accumulate all variable validation errors and report them together with exit code 4 | TEST-2.11 |
| AC-2.12 | Ubiquitous | The system SHALL accept variable input containing keys not declared in the recipe's variables section without error or warning | TEST-2.12 |
| AC-2.13 | Unwanted | IF `--vars` contains invalid JSON, the system SHALL exit with code 4 with a parse error identifying the location | TEST-2.13 |
| AC-2.14 | Unwanted | IF `--vars-file` points to a nonexistent file, the system SHALL exit with code 4 naming the missing path | TEST-2.14 |
| AC-2.15 | Unwanted | IF `--vars-file` points to a file containing invalid JSON, the system SHALL exit with code 4 with a parse error identifying the file path and the JSON error location | TEST-2.15 |
| AC-2.16 | Event | WHEN no variable sources are provided (no `--vars`, `--vars-file`, or `--vars-stdin`), the system SHALL use an empty object as input and apply recipe defaults | TEST-2.16 |

#### FR-3: Template Rendering

Render Jinja2 templates using minijinja with registered built-in filters. Templates are loaded from recipe-relative paths.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-3.1 | Event | WHEN a template is rendered with a variables context, the system SHALL substitute all `{{ variable }}` expressions with their values | TEST-3.1 |
| AC-3.2 | Event | WHEN a template contains `{% if %}` / `{% else %}` / `{% endif %}` blocks, the system SHALL evaluate them correctly against the variable values | TEST-3.2 |
| AC-3.3 | Event | WHEN a template contains `{% for item in list %}` loops, the system SHALL iterate and render the body for each element | TEST-3.3 |
| AC-3.4 | Ubiquitous | The system SHALL register and support all 13 built-in filters: snakecase, camelcase, pascalcase, kebabcase, upper, lower, capitalize, replace, pluralize, singularize, quote, indent, join | TEST-3.4 |
| AC-3.5 | Event | WHEN `snakecase` is applied to "BookingService", the system SHALL produce "booking_service" | TEST-3.5 |
| AC-3.6 | Event | WHEN `camelcase` is applied to "booking_service", the system SHALL produce "bookingService" | TEST-3.6 |
| AC-3.7 | Event | WHEN `pascalcase` is applied to "booking_service", the system SHALL produce "BookingService" | TEST-3.7 |
| AC-3.8 | Event | WHEN `kebabcase` is applied to "BookingService", the system SHALL produce "booking-service" | TEST-3.8 |
| AC-3.9 | Event | WHEN `replace` is applied as `"a.b.c" \| replace('.', '/')`, the system SHALL produce "a/b/c" | TEST-3.9 |
| AC-3.10 | Event | WHEN `pluralize` is applied to "hotel", the system SHALL produce "hotels" | TEST-3.10 |
| AC-3.11 | Event | WHEN `singularize` is applied to "hotels", the system SHALL produce "hotel" | TEST-3.11 |
| AC-3.12 | Event | WHEN `quote` is applied to "hello", the system SHALL produce `"hello"` (with literal quotes) | TEST-3.12 |
| AC-3.13 | Event | WHEN `indent(4)` is applied to a multiline string, the system SHALL indent each line by 4 spaces, including the first line. Use `indent(4, first=false)` to skip the first line. Note: this diverges from Jinja2 convention where indent() skips the first line by default | TEST-3.13 |
| AC-3.14 | Event | WHEN `join(", ")` is applied to `["a", "b", "c"]`, the system SHALL produce "a, b, c" | TEST-3.14 |
| AC-3.15 | Event | WHEN `{# comment #}` appears in a template, the system SHALL strip it from the output | TEST-3.15 |
| AC-3.16 | Event | WHEN `{% raw %}...{% endraw %}` appears in a template, the system SHALL output the content literally without interpreting Jinja2 syntax | TEST-3.16 |
| AC-3.17 | Unwanted | IF a template references an undefined variable, the system SHALL exit with code 2 and include a "did you mean?" hint when a close match exists among the keys in the provided variable context (Levenshtein distance ≤ 3) | TEST-3.17 |
| AC-3.18 | Unwanted | IF a template has a Jinja2 syntax error, the system SHALL exit with code 2 and report the template file path and line number | TEST-3.18 |

#### FR-4: Create Operation

Render a template and write it to a new file path. Support templated output paths, automatic directory creation, and skip-if-exists idempotency.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-4.1 | Event | WHEN a create operation executes, the system SHALL render the template and write the result to the path specified in `to` | TEST-4.1 |
| AC-4.2 | Event | WHEN the `to` path contains Jinja2 expressions (e.g., `tests/{{ module \| replace('.', '/') }}/test_{{ class_name \| snakecase }}.py`), the system SHALL render the path before writing | TEST-4.2 |
| AC-4.3 | Event | WHEN parent directories in the `to` path do not exist, the system SHALL create them automatically | TEST-4.3 |
| AC-4.4 | Event | WHEN `skip_if_exists: true` and the target file already exists, the system SHALL skip the operation and report `"action": "skip"` with a reason | TEST-4.4 |
| AC-4.5 | Unwanted | IF `skip_if_exists: false` (default) and the target file already exists and `--force` is not set, the system SHALL exit with code 3 reporting the conflict | TEST-4.5 |
| AC-4.6 | Event | WHEN `--force` is set and the target file already exists, the system SHALL overwrite the file regardless of skip_if_exists | TEST-4.6 |
| AC-4.7 | Event | WHEN `--base-dir` is set, the system SHALL resolve `to` paths relative to the base directory instead of the working directory | TEST-4.7 |
| AC-4.8 | Event | WHEN a create operation succeeds, the system SHALL report `"action": "create"` with the path and line count (number of lines in the written file) | TEST-4.8 |
| AC-4.9 | Unwanted | IF a filesystem write fails due to permissions, the system SHALL exit with code 3 with the path and permission error | TEST-4.9 |
| AC-4.10 | Unwanted | IF `--base-dir` specifies a directory that does not exist, the system SHALL exit with code 3 naming the missing directory | TEST-4.10 |

#### FR-5: Inject Operation

Render a template and insert the result into an existing file at a location determined by regex pattern matching. Support idempotency via skip_if.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-5.1 | Event | WHEN `after: "regex"` is specified, the system SHALL insert rendered content on the line after the first matching line | TEST-5.1 |
| AC-5.2 | Event | WHEN `before: "regex"` is specified, the system SHALL insert rendered content on the line before the first matching line | TEST-5.2 |
| AC-5.3 | Event | WHEN `prepend: true` is specified, the system SHALL insert rendered content at the very beginning of the file | TEST-5.3 |
| AC-5.4 | Event | WHEN `append: true` is specified, the system SHALL insert rendered content at the very end of the file | TEST-5.4 |
| AC-5.5 | Event | WHEN `at: last` is specified with `after` or `before`, the system SHALL use the last regex match instead of the first | TEST-5.5 |
| AC-5.6 | Event | WHEN `at: first` (default) is specified, the system SHALL use the first regex match | TEST-5.6 |
| AC-5.7 | Event | WHEN `skip_if` is specified, the system SHALL render it as a Jinja2 template with the recipe's variables, then search for the rendered string in the target file. If found, the system SHALL skip the injection and report `"action": "skip"` with a reason | TEST-5.7 |
| AC-5.8 | Unwanted | IF the regex pattern matches zero lines in the target file, the system SHALL exit with code 3 and report the pattern, the file path, and a hint | TEST-5.8 |
| AC-5.9 | Unwanted | IF the target file for injection does not exist, the system SHALL exit with code 3 and report the missing file path | TEST-5.9 |
| AC-5.10 | Event | WHEN an inject operation succeeds, the system SHALL report `"action": "inject"` with the path, location description, and line count (number of lines inserted) | TEST-5.10 |
| AC-5.11 | Event | WHEN the inject path contains Jinja2 expressions, the system SHALL render the path before resolving it | TEST-5.11 |
| AC-5.12 | Ubiquitous | The system SHALL ignore the `at` field when `prepend` or `append` is specified | TEST-5.12 |
| AC-5.13 | Unwanted | IF `after` or `before` is specified without a regex pattern, the system SHALL exit with code 1 | TEST-5.13 |
| AC-5.14 | Unwanted | IF an inject operation's `after` or `before` pattern fails to compile as a valid regex, the system SHALL exit with code 1 during recipe validation, reporting the invalid pattern and the compilation error | TEST-5.14 |
| AC-5.15 | Unwanted | IF an inject operation specifies more than one of after/before/prepend/append, the system SHALL exit with code 1 reporting the conflicting fields | TEST-5.15 |
| AC-5.16 | Ubiquitous | The system SHALL not apply `--force` to inject operations — the `--force` flag only affects create operations (`skip_if_exists` override) | TEST-5.16 |
| AC-5.17 | Unwanted | IF writing the modified file content fails due to permissions, the system SHALL exit with code 3 with the path, permission error, and rendered content | TEST-5.17 |

#### FR-6: Output Formatting

Report operation results as JSON (stdout, for LLM callers) or human-readable text (stderr, for terminal users). Auto-detect the appropriate mode.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-6.1 | State | WHILE stdout is a TTY, the system SHALL output human-readable colored text to stderr and produce no stdout output | TEST-6.1 |
| AC-6.2 | State | WHILE stdout is piped (not a TTY), the system SHALL output JSON to stdout and produce no stderr output except errors | TEST-6.2 |
| AC-6.3 | Event | WHEN `--json` is specified, the system SHALL force JSON output to stdout regardless of TTY detection | TEST-6.3 |
| AC-6.4 | Event | WHEN `--quiet` is specified, the system SHALL suppress all stderr output. Stdout behavior is determined by `--json` independently | TEST-6.4 |
| AC-6.5 | Event | WHEN JSON output is produced, the system SHALL include an `operations` array with action, path, lines (for success), reason (for skip), or error details (for error) for each operation | TEST-6.5 |
| AC-6.6 | Event | WHEN JSON output is produced, the system SHALL include `files_written` and `files_skipped` summary arrays containing unique file paths. A path appears in `files_written` if any operation wrote to it. A path appears in `files_skipped` only if all operations targeting it were skipped. In dry-run mode, `files_written` lists paths that would have been written. Paths SHALL appear in the order they were first encountered during operation execution. | TEST-6.6 |
| AC-6.7 | Event | WHEN `--verbose` is specified, the system SHALL include rendered template content in the output | TEST-6.7 |
| AC-6.8 | Event | WHEN `--dry-run` is specified, the system SHALL produce identical output format but write no files to disk. Create operations record their output paths and rendered content in a virtual file state. Inject operations targeting a path created earlier in the same dry-run SHALL behave as if the file exists with the rendered content from the create operation. Inject operations SHALL update the virtual file state with post-injection content so subsequent operations in the same dry-run see the cumulative result. WHEN `--dry-run` and `--force` are both specified, create operations SHALL report `action: create` for existing files, reflecting what `--force` would do. | TEST-6.8 |
| AC-6.9 | Ubiquitous | The system SHALL resolve flag interactions as: `--quiet` suppresses stderr only and has no effect on stdout or JSON content; `--verbose` adds rendered content to both human stderr and JSON stdout independently; in human mode with `--quiet`, `--verbose` has no visible effect (stderr is suppressed); in JSON mode with `--quiet`, `--verbose` still adds rendered content to JSON stdout | TEST-6.9 |
| AC-6.10 | Event | WHEN a file operation fails, the system SHALL stop execution immediately and not execute subsequent operations. The `operations` array SHALL contain results for executed operations only | TEST-6.10 |
| AC-6.11 | Ubiquitous | The system SHALL include a top-level `dry_run` boolean field in JSON output reflecting whether `--dry-run` was specified | TEST-6.11 |

#### FR-7: CLI Interface

Provide four subcommands for v0.1: `validate`, `vars`, `render`, and `run`. Accept global options for variable sources, output control, and execution modes.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-7.1 | Event | WHEN `jig validate <recipe>` is invoked, the system SHALL parse the recipe and report whether it is valid, listing variables and operations | TEST-7.1 |
| AC-7.2 | Event | WHEN `jig vars <recipe>` is invoked, the system SHALL output the expected variables as a JSON object with type, required, default, and description fields | TEST-7.2 |
| AC-7.3 | Event | WHEN `jig render <template> --vars '<json>'` is invoked, the system SHALL render the template with the given variables and output the result to stdout. Note: `jig render` operates without recipe context — variable type validation is not available, but "did you mean?" hints work against the provided variable keys. | TEST-7.3 |
| AC-7.4 | Event | WHEN `jig render` is invoked with `--to <path>`, the system SHALL write the rendered output to the specified file instead of stdout | TEST-7.4 |
| AC-7.5 | Event | WHEN `jig run <recipe> --vars '<json>'` is invoked, the system SHALL execute all file operations in the recipe in declaration order | TEST-7.5 |
| AC-7.6 | Ubiquitous | The system SHALL accept global options: `--vars`, `--vars-file`, `--vars-stdin`, `--dry-run`, `--json`, `--quiet`, `--force`, `--base-dir`, `--verbose`, `--version` | TEST-7.6 |
| AC-7.7 | Event | WHEN `--version` is specified, the system SHALL print the version string and exit with code 0 | TEST-7.7 |

### Non-Functional Requirements

#### NFR-1: Deterministic Output

Same inputs must always produce the same outputs, with no randomness or environment-dependent behavior in rendered content.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N1.1 | Ubiquitous | The system SHALL produce byte-identical output when given the same recipe, variables, and existing files across multiple runs | TEST-N1.1 |
| AC-N1.2 | Ubiquitous | The system SHALL not include timestamps, random values, or machine-specific identifiers in rendered output | TEST-N1.2 |

#### NFR-2: Idempotent Operations

Running the same recipe twice with the same variables must produce no changes on the second run.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N2.1 | Event | WHEN a recipe designed for idempotent execution (all creates use `skip_if_exists: true` and all injects use `skip_if`) is run a second time with the same variables and the same existing files, the system SHALL report all operations as `"action": "skip"` with reasons | TEST-N2.1 |
| AC-N2.2 | Ubiquitous | The system SHALL not produce duplicate content when create uses `skip_if_exists: true` or inject uses `skip_if` | TEST-N2.2 |

#### NFR-3: Single Static Binary

No runtime dependencies beyond the system C library. One binary that runs on macOS and Linux without Python, Node, JVM, or additional shared libraries.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N3.1 | Ubiquitous | The system SHALL compile to a single static binary with no runtime dependencies beyond the system C library | TEST-N3.1 |
| AC-N3.2 | Ubiquitous | The system SHALL not require any external programs, interpreters, or shared libraries beyond the system C library to run | TEST-N3.2 |

#### NFR-4: Structured Errors with Rendered Content

Errors must include what/where/why/hint. File operation errors must include the rendered content so callers can fall back.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N4.1 | Ubiquitous | The system SHALL include what, where, why, and hint fields in every error message | TEST-N4.1 |
| AC-N4.2 | Event | WHEN a file operation fails (exit code 3), the system SHALL include the rendered template content in the error output so the caller can fall back to manual editing. This is independent of `--verbose` — rendered content in errors is always present | TEST-N4.2 |
| AC-N4.3 | Event | WHEN a template rendering error occurs, the system SHALL report the template file path and the line number of the error | TEST-N4.3 |
| AC-N4.4 | Event | WHEN a variable validation error occurs, the system SHALL report the variable name, expected type, and actual value provided | TEST-N4.4 |

#### NFR-5: Stable Exit Codes

Exit codes are API. They must be deterministic and match the defined mapping.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N5.1 | Ubiquitous | The system SHALL exit with code 0 on success, 1 for recipe validation errors, 2 for template rendering errors, 3 for file operation errors, 4 for variable validation errors | TEST-N5.1 |
| AC-N5.2 | Ubiquitous | The system SHALL use the exit code corresponding to the first pipeline stage that fails: recipe validation (1) before variable validation (4) before template rendering (2) before file operations (3) | TEST-N5.2 |

#### NFR-6: Ordered Execution

File operations must execute in declaration order so later operations can depend on files created by earlier ones.

**Acceptance Criteria (EARS):**
| ID | Type | Criterion | Traces To |
|----|------|-----------|-----------|
| AC-N6.1 | Ubiquitous | The system SHALL execute file operations in the order they appear in the recipe's `files` array | TEST-N6.1 |
| AC-N6.2 | Event | WHEN an inject operation targets a file created by an earlier create operation in the same recipe, the system SHALL find and operate on the newly created file | TEST-N6.2 |

## Interfaces

### Public API (CLI)

```
jig validate <recipe>
jig vars <recipe>
jig render <template> [--to <path>] [--vars '<json>']
jig run <recipe> [--vars '<json>'] [options]

Options:
  --vars <json>        Inline variables as JSON string
  --vars-file <path>   Variables from a JSON file
  --vars-stdin         Read variables from stdin
  --dry-run            Preview without writing
  --json               Force JSON output
  --quiet              Suppress stderr output
  --force              Overwrite existing files
  --base-dir <path>    Resolve output paths from this directory
  --verbose            Include rendered content in output
  --version            Print version and exit
```

### JSON Output Schema

```json
{
  "dry_run": false,
  "operations": [
    {"action": "create", "path": "...", "lines": 42},
    {"action": "inject", "path": "...", "location": "after:^# fixtures", "lines": 3},
    {"action": "skip", "path": "...", "reason": "skip_if matched: BookingService"},
    {"action": "error", "path": "...", "what": "...", "where": "...", "why": "...", "hint": "...", "rendered_content": "..."}
  ],
  "files_written": ["..."],
  "files_skipped": ["..."]
}
```

### Internal Interfaces

```rust
// Recipe → Variable Validator
fn validate_variables(decls: &IndexMap<String, VariableDecl>, provided: Value) -> Result<Value, JigError>;

// Recipe + Variables → Renderer
fn render_template(env: &Environment, template_name: &str, vars: &Value) -> Result<String, JigError>;

// Rendered content + FileOp → Operation Executor
fn execute_operation(op: &FileOp, rendered: &str, ctx: &ExecutionContext) -> OpResult;

// Vec<OpResult> → Output Formatter
fn format_output(results: &[OpResult], mode: OutputMode) -> String;
```

## Data Model

```rust
struct Recipe {
    name: Option<String>,
    description: Option<String>,
    variables: IndexMap<String, VariableDecl>,
    files: Vec<FileOp>,
}

struct VariableDecl {
    var_type: VarType,          // string, number, boolean, array, object, enum
    required: bool,             // default: false
    default: Option<Value>,
    description: Option<String>,
    values: Option<Vec<String>>, // for enum type
    items: Option<VarType>,      // for array type
}

enum FileOp {
    Create { template: String, to: String, skip_if_exists: bool },
    Inject { template: String, inject: String, mode: InjectMode, skip_if: Option<String> },
}

enum InjectMode {
    After { pattern: String, at: MatchPosition },
    Before { pattern: String, at: MatchPosition },
    Prepend,
    Append,
}

enum MatchPosition { First, Last }

enum OpResult {
    Success { action: &'static str, path: PathBuf, lines: usize, location: Option<String>, rendered_content: Option<String> },
    Skip { path: PathBuf, reason: String, rendered_content: Option<String> },
    Error { path: PathBuf, error: StructuredError, rendered_content: String },
}

struct StructuredError {
    what: String,      // what happened
    where_: String,    // file, variable, or line reference
    why: String,       // expected vs actual, or root cause
    hint: String,      // actionable suggestion for the caller
}

enum JigError {
    RecipeValidation(StructuredError),     // exit 1
    TemplateRendering(StructuredError),    // exit 2
    FileOperation(StructuredError),        // exit 3
    VariableValidation(StructuredError),   // exit 4
}
```

## Rendering Lifecycle

The following string fields are rendered as Jinja2 templates with the recipe's variables before use:
- `to` (create operation output path)
- `inject` (inject operation target path)  
- `skip_if` (inject idempotency check — rendered, then searched in target file)
- `template` contents (the template file itself)

The following fields are NOT rendered:
- `skip_if_exists` (boolean, not a template)
- `after`, `before` (regex patterns, used literally)
- `at` (enum value: first/last)

All template fields are rendered upfront before any operation executes (see D-1).

## Error Handling

| Error Category | Exit Code | Examples |
|----------------|-----------|----------|
| Recipe validation | 1 | Malformed YAML, missing fields, missing template files |
| Template rendering | 2 | Undefined variable, Jinja2 syntax error |
| File operation | 3 | Target file missing (inject), file exists (create), regex no match |
| Variable validation | 4 | Missing required, wrong type, invalid enum value |

Every error includes: what happened, where (file/variable/line), why (expected vs actual), and a hint for the caller.

File operation errors additionally include `rendered_content` so the LLM caller can fall back to manual editing with the Edit tool.

### Partial Write Recovery

Render-all-upfront prevents partial writes from template rendering failures — if any template has a syntax error, no files are touched. However, execution failures can still produce partial writes (e.g., create succeeds for op 1, inject fails for op 2). Recovery options: for create-related partial writes, re-run with `--force` to overwrite. For inject failures, use the rendered content from the error output to manually apply the operation. (`--force` only affects create operations.)

## Testing Strategy

- **Spec tests:** One or more tests per AC-* criterion above, namespaced as `spec::fr{N}::ac_{N}_{M}`
- **Invariant tests:** Determinism (run twice, compare output), idempotency (run twice, second run all skips), exit code stability
- **Integration fixtures:** Each fixture directory contains recipe.yaml, vars.json, templates/, existing/ (optional), expected/, expected_output.json (optional), expected_exit_code (optional)
- **Snapshot tests:** `insta` for rendered template output, JSON output format, and error messages

## Requirement Traceability

| Requirement | Criteria | Test | Status |
|-------------|----------|------|--------|
| FR-1 | AC-1.1 through AC-1.15 | spec::fr1::* | PENDING |
| FR-2 | AC-2.1 through AC-2.16 | spec::fr2::* | PENDING |
| FR-3 | AC-3.1 through AC-3.18 | spec::fr3::* | PENDING |
| FR-4 | AC-4.1 through AC-4.10 | spec::fr4::* | PENDING |
| FR-5 | AC-5.1 through AC-5.17 | spec::fr5::* | PENDING |
| FR-6 | AC-6.1 through AC-6.11 | spec::fr6::* | PENDING |
| FR-7 | AC-7.1 through AC-7.7 | spec::fr7::* | PENDING |
| NFR-1 | AC-N1.1, AC-N1.2 | spec::nfr1::* | PENDING |
| NFR-2 | AC-N2.1, AC-N2.2 | spec::nfr2::* | PENDING |
| NFR-3 | AC-N3.1, AC-N3.2 | spec::nfr3::* | PENDING |
| NFR-4 | AC-N4.1 through AC-N4.4 | spec::nfr4::* | PENDING |
| NFR-5 | AC-N5.1, AC-N5.2 | spec::nfr5::* | PENDING |
| NFR-6 | AC-N6.1, AC-N6.2 | spec::nfr6::* | PENDING |
