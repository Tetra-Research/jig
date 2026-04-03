# jig

A template rendering CLI purpose-built for LLM code generation workflows.

A jig is a manufacturing tool that guides the shape of a part — it doesn't make the part itself, it ensures the part comes out right every time. That's what this tool does for code generation: it turns a template + variables into files, deterministically, so an LLM doesn't have to reinvent boilerplate from scratch on every invocation.

---

## The Problem

LLMs are powerful code generators, but they have a consistency problem. Ask one to "write a unit test for BookingService" three times and you'll get three different file structures, import styles, and naming conventions. The LLM wastes context window and latency *re-deriving* patterns that should be fixed.

The existing solutions don't fit:

- **Hygen** — stale (last commit July 2024), 102 open issues, designed for human-interactive prompts, can't accept structured JSON input, inject can't replace, no multi-file recipes
- **Cookiecutter / Copier** — heavy Python dependencies, project-level scaffolding tools, not designed for fine-grained file generation within an existing codebase
- **Yeoman** — massive framework overhead, generators are full npm packages, way too abstract
- **NX Generators** — coupled to the NX build system and workspace model, can't be used standalone
- **envsubst / sed scripts** — no conditionals, no loops, no injection logic, no recipe concept

What's missing is a tool that:

1. Accepts variables as structured JSON (how LLMs naturally produce data)
2. Renders templates with real control flow (conditionals, loops, filters)
3. Creates new files AND injects into existing files in a single operation
4. Groups multiple file operations into a single recipe
5. Is a single, fast binary with zero runtime dependencies
6. Was designed from day one to be called by an LLM, not a human at a terminal

---

## Core Concepts

### Recipes

A **recipe** is a YAML file that declares what variables are needed and what file operations to perform. It lives alongside the templates it references — there is no central template directory. Templates are co-located with whatever project, plugin, or tool uses them.

```
my-tool/
  templates/
    recipe.yaml
    test.py.j2
    conftest_fixture.j2
```

### Templates

Templates use **Jinja2 syntax** — the most widely known templating language, already familiar to Python developers, Ansible users, and LLMs trained on millions of examples. Templates are plain text files with a `.j2` extension.

### Operations

A recipe produces one or more **operations**, each of which is one of:

- **create** — render a template and write it to a new file path
- **inject** — render a template and insert the result into an existing file at a specific location
- **replace** — render a template and replace a matched region in an existing file

### Variables

Variables are passed as a JSON object. They can be simple strings, numbers, booleans, arrays, or nested objects. The recipe declares which variables are required, their types, and optional defaults.

---

## Recipe Format

```yaml
# recipe.yaml

# Optional metadata
name: unit-test
description: Generate a pytest unit test for a Python class

# Variable declarations
variables:
  module:
    type: string
    required: true
    description: "Dotted Python module path (e.g., hotels.services.booking)"

  class_name:
    type: string
    required: true
    description: "The class under test"

  methods:
    type: array
    items: string
    default: []
    description: "List of method names to generate test stubs for"

  async:
    type: boolean
    default: false
    description: "Whether to generate async test methods"

# File operations, executed in order
files:
  # Create a new test file
  - template: test.py.j2
    to: "tests/{{ module | replace('.', '/') }}/test_{{ class_name | snakecase }}.py"
    skip_if_exists: false  # default: false. If true, skip when target file already exists

  # Inject a fixture into conftest.py
  - template: conftest_fixture.j2
    inject: "tests/conftest.py"
    after: "^# fixtures"          # regex: insert after first matching line
    skip_if: "{{ class_name }}"   # don't inject if this string already exists in the file

  # Inject an import at the top of conftest.py
  - template: conftest_import.j2
    inject: "tests/conftest.py"
    after: "^from .* import"      # insert after the last import line
    at: last                      # "first" (default) or "last" match
    skip_if: "{{ module }}"

  # Replace an entire block in a registry file
  - template: registry_entry.j2
    replace: "config/registry.yaml"
    between:                       # replace everything between these two patterns
      start: "^  # {{ class_name }} start"
      end: "^  # {{ class_name }} end"
    fallback: append               # if the markers don't exist, append to file instead
```

### Variable Types

| Type | JSON | Example |
|------|------|---------|
| `string` | `"value"` | `"BookingService"` |
| `number` | `42` | line counts, ports |
| `boolean` | `true` / `false` | feature flags |
| `array` | `["a", "b"]` | method lists, import lists |
| `object` | `{"k": "v"}` | nested config |
| `enum` | `"unit"` | constrained choices, validated against `values: [unit, integration, e2e]` |

### Injection Modes

| Mode | Frontmatter | Behavior |
|------|-------------|----------|
| **after** | `after: "regex"` | Insert rendered content after the matched line |
| **before** | `before: "regex"` | Insert rendered content before the matched line |
| **prepend** | `prepend: true` | Insert at the very beginning of the file |
| **append** | `append: true` | Insert at the very end of the file |
| **at** | `at: first` or `at: last` | Which match to use when regex matches multiple lines (default: `first`) |
| **skip_if** | `skip_if: "string"` | Skip this injection if the string already exists in the file (idempotency) |

### Replace Mode

| Field | Behavior |
|-------|----------|
| `between.start` | Regex marking the start of the region to replace |
| `between.end` | Regex marking the end of the region to replace |
| `pattern` | Single regex — replaces the entire matched line(s) |
| `fallback` | What to do if the pattern isn't found: `append`, `prepend`, `skip`, or `error` (default: `error`) |

---

## Patches: Extending Existing Code

The real everyday work isn't greenfield scaffolding — it's brownfield extension. You already have the model, the service, the view, the schema. You're adding a field, a method, an endpoint. The shape of each change is completely predictable, it just needs to land in the right spot across 5-6 files.

jig handles this with **patch operations** — templates that describe fragments to insert into existing, previously-generated code. A recipe can mix `create` operations (new files) with `patch` operations (extend existing files) in a single run.

### The Pattern

Consider the most common Django workflow: adding a new field. It touches:

1. **Model** — add the field declaration to the class body
2. **Service** — add the field to create/update method signatures and logic
3. **Schema** — add the field to request/response msgspec structs
4. **View** — maybe update allowed params or serialization
5. **Tests** — add the field to factories, fixtures, and assertions
6. **Admin** — add to `list_display`, `search_fields`, or fieldsets

Each of those is "find the right block in an existing file, add one more entry in the established pattern." It's not creative work — it's mechanical expansion.

### Patch Operation

A patch operation targets an existing file, finds a structural anchor, and inserts rendered content:

```yaml
# recipe.yaml — add-model-field
name: add-model-field
description: Add a field to an existing Django model and propagate to service, schema, view, tests, and admin

variables:
  app:
    type: string
    required: true
    description: "Django app name (e.g., hotels)"
  model:
    type: string
    required: true
    description: "Model class name (e.g., Reservation)"
  field_name:
    type: string
    required: true
    description: "New field name (e.g., loyalty_tier)"
  field_type:
    type: string
    required: true
    description: "Django field type (e.g., CharField)"
  field_args:
    type: string
    default: ""
    description: "Field arguments (e.g., max_length=50, null=True)"
  nullable:
    type: boolean
    default: false
  has_default:
    type: boolean
    default: false
  default_value:
    type: string
    default: ""

files:
  # 1. Add field to the model class
  - template: model_field.j2
    patch: "{{ app }}/models/{{ model | snakecase }}.py"
    anchor:
      pattern: "^class {{ model }}\\("    # find the class definition
      scope: class_body                    # target the body of this class
      position: after_last_field           # insert after the last field declaration
    skip_if: "{{ field_name }}"

  # 2. Add to the service create/update methods
  - template: service_param.j2
    patch: "{{ app }}/services/{{ model | snakecase }}_service.py"
    anchor:
      pattern: "def create\\("
      scope: function_signature            # the parameter list
      position: before_close               # before the closing paren
    skip_if: "{{ field_name }}"

  # 3. Add to the request schema
  - template: schema_field.j2
    patch: "{{ app }}/schemas/{{ model | snakecase }}.py"
    anchor:
      pattern: "^class {{ model }}(Create|Update)Request"
      scope: class_body
      position: after_last_field
    skip_if: "{{ field_name }}"

  # 4. Add to the response schema
  - template: schema_response_field.j2
    patch: "{{ app }}/schemas/{{ model | snakecase }}.py"
    anchor:
      pattern: "^class {{ model }}Response"
      scope: class_body
      position: after_last_field
    skip_if: "{{ field_name }}"

  # 5. Add to the test factory
  - template: factory_field.j2
    patch: "{{ app }}/tests/factories.py"
    anchor:
      pattern: "^class {{ model }}Factory"
      scope: class_body
      position: after_last_field
    skip_if: "{{ field_name }}"

  # 6. Add to admin list_display
  - template: admin_field.j2
    patch: "{{ app }}/admin.py"
    anchor:
      pattern: "^class {{ model }}Admin"
      scope: class_body
      find: "list_display"                 # find this attribute within the class
      position: before_close               # before the closing bracket/paren
    skip_if: "{{ field_name }}"
```

### Anchor System

The anchor system is what makes patches work on files jig has never seen before. Instead of hardcoded line numbers or fragile regex-only matching, anchors combine **structural awareness** with **pattern matching**:

```yaml
anchor:
  pattern: "regex"        # find this line in the file (required)
  scope: <scope_type>     # what region to target relative to the match
  find: "string"          # optionally narrow within the scope
  position: <position>    # where within the scope to insert
```

#### Scopes

Scopes define the structural region that jig operates within, relative to the `pattern` match:

| Scope | Meaning |
|-------|---------|
| `line` | Just the matched line itself (default, same as inject) |
| `block` | The indented block following the matched line (Python-style) |
| `class_body` | The body of the class whose definition matches the pattern |
| `function_body` | The body of the function/method whose definition matches |
| `function_signature` | The parameter list of the function (from `(` to `)`) |
| `braces` | The content between `{` and `}` following the match |
| `brackets` | The content between `[` and `]` following the match |
| `parens` | The content between `(` and `)` following the match |

#### Positions

Positions define where within the scope the rendered content is inserted:

| Position | Meaning |
|----------|---------|
| `before` | Before the first line of the scope |
| `after` | After the last line of the scope |
| `before_close` | Before the closing delimiter (`]`, `)`, `}`, or dedent) |
| `after_last_field` | After the last attribute/field assignment in the scope |
| `after_last_import` | After the last import statement in the scope |
| `after_last_method` | After the last method definition in the scope |
| `sorted` | Insert in alphabetical order among siblings (for things like imports) |

#### `find` Narrowing

When a scope is large (like a whole class body), `find` narrows to a specific attribute or block within it:

```yaml
anchor:
  pattern: "^class ReservationAdmin"
  scope: class_body
  find: "list_display"           # find the list_display line within the class
  position: before_close         # insert before the ] or ) that closes it
```

This finds:
```python
class ReservationAdmin(admin.ModelAdmin):
    list_display = [
        "guest_name",
        "check_in",
        "check_out",
        #            ^ inserts here, before the closing ]
    ]
```

### How Scopes Work Under the Hood

jig doesn't need a full language parser. The scope detection is lightweight and language-agnostic:

1. **Indentation-based scopes** (`block`, `class_body`, `function_body`): Find the matched line's indentation level. The scope is everything indented deeper than that line, until indentation returns to the same or shallower level. Works for Python, YAML, and any indentation-significant language.

2. **Delimiter-based scopes** (`braces`, `brackets`, `parens`, `function_signature`): Count opening/closing delimiters from the matched line. Handles nesting correctly. Works for any C-family language, JSON, TypeScript, etc.

3. **Semantic positions** (`after_last_field`, `after_last_method`, `after_last_import`): Simple heuristics within the scope:
   - `after_last_field`: last line matching `^\s+\w+\s*[:=]` (assignment or type annotation)
   - `after_last_method`: last line matching `^\s+def \w+`
   - `after_last_import`: last line matching `^\s*(from|import) `

These heuristics cover the vast majority of real-world code without needing tree-sitter or an AST parser. They're also easy to debug — jig's `--verbose` mode shows exactly which lines it identified as scope boundaries and where it chose to insert.

### Patch Templates

Patch templates are small fragments, not full files. They render just the content to be inserted:

```jinja2
{# model_field.j2 — just the field line #}
    {{ field_name }} = models.{{ field_type }}({{ field_args }}{% if nullable %}, null=True{% endif %}{% if has_default %}, default={{ default_value }}{% endif %})
```

```jinja2
{# service_param.j2 — just the parameter addition #}
        {{ field_name }}: {{ field_type | python_type }}{{ " | None" if nullable else "" }},
```

```jinja2
{# factory_field.j2 — just the factory attribute #}
    {{ field_name }} = {% if field_type == "CharField" %}factory.Faker("word"){% elif field_type == "IntegerField" %}factory.Faker("random_int"){% elif field_type == "BooleanField" %}False{% else %}None{% endif %}
```

### Chaining Recipes: The Cascade Pattern

The real power is chaining recipes together. You define a small, focused recipe for each concern, then compose them:

```yaml
# recipe.yaml — add-field (orchestrator)
name: add-field
description: Add a field to a Django model and cascade through the full stack

includes:
  - add-model-field/recipe.yaml
  - add-service-param/recipe.yaml
  - add-schema-field/recipe.yaml
  - add-admin-field/recipe.yaml
  - add-factory-field/recipe.yaml
  - add-test-assertions/recipe.yaml
```

Each sub-recipe is independently testable and reusable. The `add-schema-field` recipe works whether called from `add-field` or directly.

### Idempotency

Every patch operation is idempotent by default:

- `skip_if` checks whether the content already exists in the target
- Running the same recipe twice with the same variables produces no changes on the second run
- The JSON output reports `"action": "skip"` with a reason for every skipped operation

This matters because an LLM might retry a failed recipe. It should be safe to re-run without duplicating content.

### Fallback to LLM

When jig can't find an anchor or a scope doesn't parse cleanly, it fails gracefully with a structured error:

```json
{
  "action": "error",
  "path": "hotels/models/reservation.py",
  "error": "scope_parse_failed",
  "details": "class_body scope for pattern '^class Reservation\\(' could not determine end of class — indentation is inconsistent at line 45",
  "hint": "Use the LLM's Edit tool to manually insert the field at the appropriate location",
  "rendered_content": "    loyalty_tier = models.CharField(max_length=50, null=True)"
}
```

The key detail: **jig still renders the template and includes the rendered content in the error output**. The LLM doesn't need to re-derive what to insert — it just needs to figure out *where* to put it using its native Edit tool. jig did the deterministic part (rendering), the LLM handles the judgment part (placement).

### Real-World Example: Full Lifecycle

Here's how an LLM-driven skill uses patch recipes to manage the full lifecycle of adding a field:

```
User: "Add a loyalty_tier field to the Reservation model"

Claude:
  1. Reads hotels/models/reservation.py → identifies it's a Django model
  2. Reads the existing fields to understand patterns (CharField style, etc.)
  3. Constructs variables:
     {
       "app": "hotels",
       "model": "Reservation",
       "field_name": "loyalty_tier",
       "field_type": "CharField",
       "field_args": "max_length=50",
       "nullable": true
     }
  4. Runs: jig run add-field/recipe.yaml --vars '...' --json
  5. jig patches 6 files, reports results as JSON
  6. Claude reviews output:
     - 5 operations succeeded
     - 1 skipped (admin field already existed from a previous attempt)
  7. Claude runs: python manage.py makemigrations
  8. Done.
```

The LLM's job was understanding the *intent* ("add loyalty_tier"), extracting the *variables* (type, nullability, app name), and calling jig. jig's job was the mechanical file-by-file insertion. Neither had to do the other's work.

---

## Template Syntax

Standard Jinja2 with a small set of built-in filters useful for code generation:

### Variables
```jinja2
{{ class_name }}
{{ module }}
{{ methods[0] }}
```

### Conditionals
```jinja2
{% if async %}
import pytest_asyncio

@pytest_asyncio.fixture
{% else %}
import pytest

@pytest.fixture
{% endif %}
```

### Loops
```jinja2
{% for method in methods %}
    def test_{{ method }}_returns_expected(self):
        result = self.instance.{{ method }}()
        assert result is not None

{% endfor %}
```

### Filters

Built-in filters for common code transformations:

| Filter | Input | Output |
|--------|-------|--------|
| `snakecase` | `BookingService` | `booking_service` |
| `camelcase` | `booking_service` | `bookingService` |
| `pascalcase` | `booking_service` | `BookingService` |
| `kebabcase` | `BookingService` | `booking-service` |
| `upper` | `booking` | `BOOKING` |
| `lower` | `BOOKING` | `booking` |
| `capitalize` | `booking` | `Booking` |
| `replace` | `a.b.c \| replace('.', '/')` | `a/b/c` |
| `pluralize` | `hotel` | `hotels` |
| `singularize` | `hotels` | `hotel` |
| `quote` | `hello` | `"hello"` |
| `indent` | (multiline string) | Indents each line by N spaces: `\| indent(4)` |
| `join` | `["a","b"] \| join(", ")` | `a, b` |

### Template Comments
```jinja2
{# This comment won't appear in output #}
```

### Raw Blocks (Escape Jinja Syntax)
```jinja2
{% raw %}
{{ this_is_literal_not_a_variable }}
{% endraw %}
```

---

## CLI Interface

### Rendering a Recipe

```bash
# Basic usage — pass variables as JSON
jig run ./templates/recipe.yaml \
  --vars '{"module": "hotels.services.booking", "class_name": "BookingService", "methods": ["create", "cancel"]}'

# Variables from a JSON file
jig run ./templates/recipe.yaml --vars-file context.json

# Variables from stdin (piped from another command or LLM output)
echo '{"module": "hotels.services"}' | jig run ./templates/recipe.yaml --vars-stdin

# Mixed: file + overrides
jig run ./templates/recipe.yaml --vars-file defaults.json --vars '{"class_name": "PaymentService"}'
```

### Dry Run

Preview what would happen without writing anything:

```bash
jig run ./templates/recipe.yaml --vars '...' --dry-run
```

Output:
```
[create] tests/hotels/services/test_booking_service.py (142 lines)
[inject] tests/conftest.py — after "^# fixtures" (3 lines)
[skip]   tests/conftest.py — "BookingService" already present
```

### Rendering a Single Template (No Recipe)

For simple one-off renders without a recipe file:

```bash
jig render ./templates/test.py.j2 --vars '{"class_name": "Foo"}' --to ./tests/test_foo.py

# To stdout (for piping or LLM consumption)
jig render ./templates/test.py.j2 --vars '{"class_name": "Foo"}'
```

### Validating a Recipe

Check that a recipe is well-formed and all referenced templates exist:

```bash
jig validate ./templates/recipe.yaml
```

Output:
```
✓ recipe.yaml — valid
  variables: module (string, required), class_name (string, required), methods (array, optional)
  files: 3 operations (2 create, 1 inject)
  templates: test.py.j2 ✓, conftest_fixture.j2 ✓, conftest_import.j2 ✓
```

### Listing Variables

Show what variables a recipe expects (useful for LLMs constructing the --vars JSON):

```bash
jig vars ./templates/recipe.yaml
```

Output (JSON, machine-readable):
```json
{
  "module": {"type": "string", "required": true, "description": "Dotted Python module path"},
  "class_name": {"type": "string", "required": true, "description": "The class under test"},
  "methods": {"type": "array", "required": false, "default": [], "description": "Method names"},
  "async": {"type": "boolean", "required": false, "default": false}
}
```

### Output Formats

```bash
# Human-readable (default for TTY)
jig run ./recipe.yaml --vars '...'

# JSON output (default when piped, or forced with --json)
jig run ./recipe.yaml --vars '...' --json
```

JSON output for LLM consumption:
```json
{
  "operations": [
    {"action": "create", "path": "tests/test_booking_service.py", "lines": 142},
    {"action": "inject", "path": "tests/conftest.py", "location": "after:^# fixtures", "lines": 3},
    {"action": "skip", "path": "tests/conftest.py", "reason": "skip_if matched: BookingService"}
  ],
  "files_written": ["tests/test_booking_service.py", "tests/conftest.py"],
  "files_skipped": []
}
```

### Global Options

```
--vars JSON          Variables as inline JSON string
--vars-file PATH     Variables from a JSON file
--vars-stdin         Read variables JSON from stdin
--dry-run            Preview operations without writing files
--json               Force JSON output
--quiet              Suppress all output except errors
--force              Overwrite existing files without prompting
--base-dir PATH      Resolve all relative output paths from this directory (default: cwd)
--verbose            Show rendered template content in output
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All operations completed successfully |
| 1 | Recipe validation error (missing variables, bad YAML, missing template files) |
| 2 | Template rendering error (undefined variable, syntax error in template) |
| 3 | File operation error (can't write, can't find injection target, regex didn't match) |
| 4 | Variable validation error (wrong type, missing required variable) |

Deterministic exit codes so an LLM caller can branch on failure type.

---

## Claude Code Plugin Integration

jig is designed to be used from Claude Code skills. A skill that uses templates would look like this:

### Skill Directory Structure

```
my-plugin/
  skills/
    unit-test/
      SKILL.md
      templates/
        recipe.yaml
        test.py.j2
        conftest_fixture.j2
```

Templates live WITH the skill. No central template directory. Each skill owns its templates.

### Skill SKILL.md

```markdown
---
name: unit-test
description: Generate a pytest unit test for a given module and class
allowed-tools: Read Bash Grep Glob Write Edit
argument-hint: <file_path> [class_name]
---

## Steps

1. Read the source file at $0
2. Identify the class at $1 (or the primary class if not specified)
3. Extract: module path, class name, public method names, imports
4. Query the recipe's expected variables:
   ```
   jig vars ${CLAUDE_SKILL_DIR}/templates/recipe.yaml
   ```
5. Run the recipe:
   ```
   jig run ${CLAUDE_SKILL_DIR}/templates/recipe.yaml \
     --vars '{"module": "<extracted>", "class_name": "<extracted>", "methods": [<extracted>]}'
   ```
6. Review jig's JSON output. If any operations were skipped or errored, handle with Edit.
7. Run the generated test to verify it passes.
```

### Why This Works

- **Claude extracts the variables** — this is the part that requires understanding code (reading a source file, identifying classes and methods). LLMs are great at this.
- **jig renders the template** — this is the part that should be deterministic. Same inputs, same output, every time. No creative drift.
- **Claude handles edge cases** — if jig's injection can't find the anchor regex, Claude falls back to its native Edit tool. jig's JSON output tells it exactly what failed and why.

### Publishing as a Claude Code Plugin

jig itself can also ship as a Claude Code plugin, providing documentation and helper skills:

```
jig-plugin/
  .claude-plugin/
    plugin.json
  skills/
    init/
      SKILL.md           # "jig init" — scaffold a new recipe + templates dir in the current skill
    doctor/
      SKILL.md           # "jig doctor" — validate all recipes in a plugin
```

The `plugin.json`:
```json
{
  "name": "jig",
  "version": "1.0.0",
  "description": "Template rendering for Claude Code skills",
  "skills": ["skills/*/SKILL.md"]
}
```

This gives Claude Code users `/jig:init` to scaffold a new recipe inside any skill they're building, and `/jig:doctor` to validate their templates.

---

## Implementation

### Language: Rust

Rust is the right choice for jig:

- **Single static binary** — no runtime dependencies, no Python, no Node, no JVM
- **Fast startup** — critical when called dozens of times per session
- **Cross-platform** — one build matrix covers macOS (arm64, x86_64), Linux (arm64, x86_64), Windows
- **Jinja2 ecosystem** — the `minijinja` crate is a mature, correct Jinja2 implementation used in production by Sentry
- **YAML parsing** — `serde_yaml` is battle-tested
- **Regex** — the `regex` crate is the fastest in any language

### Dependencies

| Crate | Purpose |
|-------|---------|
| `minijinja` | Jinja2 template rendering |
| `serde` + `serde_yaml` + `serde_json` | Recipe and variable parsing |
| `regex` | Injection/replace pattern matching |
| `clap` | CLI argument parsing |
| `heck` | Case conversion filters (snake_case, camelCase, etc.) |
| `owo-colors` | Terminal coloring (human-readable output) |

Total dependency tree: ~20 crates. Binary size: ~3-5 MB.

### Architecture

```
src/
  main.rs              # CLI entry point, argument parsing
  recipe.rs            # Recipe YAML parsing and validation
  variables.rs         # Variable declaration, type checking, JSON input merging
  renderer.rs          # Jinja2 template rendering via minijinja
  operations/
    mod.rs             # Operation trait and dispatch
    create.rs          # Create new files
    inject.rs          # Inject into existing files (line-level)
    patch.rs           # Patch existing files (scope-aware)
    replace.rs         # Replace regions in existing files
  scope/
    mod.rs             # Scope detection dispatch
    indent.rs          # Indentation-based scope detection (Python, YAML)
    delimiter.rs       # Delimiter-based scope detection (braces, brackets, parens)
    position.rs        # Semantic position heuristics (after_last_field, etc.)
  workflow.rs          # Multi-recipe orchestration, conditional steps, variable passing
  library/
    mod.rs             # Library manifest parsing
    install.rs         # Add/remove/update libraries
    discover.rs        # Recipe and workflow discovery
    conventions.rs     # Convention mapping and overrides
  filters.rs           # Custom Jinja2 filters (snakecase, etc.)
  output.rs            # Human-readable and JSON output formatting
  error.rs             # Structured error types with exit codes
```

### Core Flow

```
1. Parse CLI args (clap)
2. Read and validate recipe.yaml (serde_yaml)
3. Read and validate variables JSON (serde_json)
4. Type-check variables against recipe declarations
5. For each file operation:
   a. Render the template with minijinja + variables
   b. Execute the operation (create / inject / replace)
   c. Record the result (success / skip / error)
6. Output results (human or JSON format)
7. Exit with appropriate code
```

---

## Distribution

### Phase 1: Manual GitHub Releases (current)

Build locally, push with `gh release create`. Single-platform (macOS ARM) to start. No CI required.

```bash
cargo build --release
gh release create v0.1.0 target/release/jig --title "v0.1.0" --notes "Initial release"
```

Cross-platform builds and GitHub Actions automation come later when there are users beyond the author.

### Phase 2: Homebrew

Primary distribution channel for macOS and Linux once the CLI is stable enough for others to use.

```ruby
# Formula: jig.rb
class Jig < Formula
  desc "Template renderer for LLM code generation workflows"
  homepage "https://github.com/<org>/jig"
  version "1.0.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/<org>/jig/releases/download/v1.0.0/jig-aarch64-apple-darwin.tar.gz"
      sha256 "..."
    else
      url "https://github.com/<org>/jig/releases/download/v1.0.0/jig-x86_64-apple-darwin.tar.gz"
      sha256 "..."
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/<org>/jig/releases/download/v1.0.0/jig-aarch64-unknown-linux-musl.tar.gz"
      sha256 "..."
    else
      url "https://github.com/<org>/jig/releases/download/v1.0.0/jig-x86_64-unknown-linux-musl.tar.gz"
      sha256 "..."
    end
  end

  def install
    bin.install "jig"
  end

  test do
    system "#{bin}/jig", "--version"
  end
end
```

#### Homebrew Tap

Start with a tap for rapid iteration:

```bash
brew tap <org>/tools
brew install <org>/tools/jig
```

Graduate to homebrew-core once stable (requires 50+ GitHub stars, 30+ days old, passing CI).

### Cargo (crates.io)

For Rust developers and CI environments:

```bash
cargo install jig-cli
```

Crate name `jig-cli` since `jig` is likely taken on crates.io.

### Nix

For Nix-based developer environments (direnv, NixOS):

```nix
# flake.nix
{
  description = "jig — template renderer for LLM workflows";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in {
      packages = forAllSystems (system:
        let pkgs = nixpkgs.legacyPackages.${system};
        in {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "jig";
            version = "1.0.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
          };
        }
      );
    };
}
```

Usage: `nix run github:<org>/jig -- run ./recipe.yaml --vars '...'`

### npm (optional wrapper)

For Node.js-heavy teams that want `npx jig`:

```bash
npx @<org>/jig run ./recipe.yaml --vars '...'
```

This would be a thin npm package that downloads the correct platform binary on postinstall (same pattern as `esbuild`, `turbo`, `biome`). The npm package itself contains no logic — just a binary redirect.

### GitHub Releases

Every tagged release publishes prebuilt binaries:

```
jig-v1.0.0-aarch64-apple-darwin.tar.gz
jig-v1.0.0-x86_64-apple-darwin.tar.gz
jig-v1.0.0-aarch64-unknown-linux-musl.tar.gz
jig-v1.0.0-x86_64-unknown-linux-musl.tar.gz
jig-v1.0.0-x86_64-pc-windows-msvc.zip
```

Statically linked on Linux (musl) so it runs anywhere without glibc version issues.

### Shell Installer

One-line install for README convenience:

```bash
curl -fsSL https://raw.githubusercontent.com/<org>/jig/main/install.sh | sh
```

Detects platform, downloads the right binary, puts it in `~/.local/bin` or `/usr/local/bin`.

---

## Testing Strategy

### Unit Tests

- Recipe parsing: valid YAML, missing fields, type mismatches
- Variable validation: required checks, type coercion, defaults
- Template rendering: all filters, conditionals, loops, edge cases
- Each operation type: create, inject (all modes), replace (all modes)

### Integration Tests

Fixture-based: each test is a directory containing:

```
tests/fixtures/inject-after/
  recipe.yaml              # input recipe
  vars.json                # input variables
  templates/
    fragment.j2            # input template
  existing/
    target.py              # file that exists before jig runs
  expected/
    target.py              # what the file should look like after
```

The test runner:
1. Copies `existing/` to a temp dir
2. Runs `jig run recipe.yaml --vars-file vars.json --base-dir $tmp`
3. Diffs the temp dir against `expected/`

This makes it trivial to add new test cases — just add a directory.

### Snapshot Tests

For template rendering, use `insta` (Rust snapshot testing crate) to catch unintended output changes.

---

## Error Messages

Errors should be clear enough for an LLM to self-correct. Every error includes:

1. **What** went wrong
2. **Where** it happened (file path, line number, variable name)
3. **Why** it's wrong (expected vs. actual)

```
Error: missing required variable "class_name"
  recipe: ./templates/recipe.yaml
  variable: class_name (type: string)
  provided: {"module": "hotels.services"}
  hint: add "class_name" to your --vars JSON
```

```
Error: injection target not found
  file: tests/conftest.py
  pattern: ^# fixtures
  hint: the regex "^# fixtures" matched 0 lines. Check that the anchor comment exists in the file.
```

```
Error: template syntax error
  template: ./templates/test.py.j2
  line: 14
  error: undefined variable "clss_name" (did you mean "class_name"?)
```

---

## Configuration (Optional)

An optional `.jigrc.yaml` in the project root for project-wide defaults:

```yaml
# .jigrc.yaml
base_dir: .                  # default --base-dir
vars_file: .jig/defaults.json  # always merge these variables first
filters:                     # register custom filters (as shell commands)
  copyright_header: "cat .jig/copyright.txt"
```

This is purely optional. jig works without any config file — recipes are fully self-contained.

---

## Custom Filters (Extensibility)

Beyond the built-in filters, users can register custom filters as shell commands:

```yaml
# In .jigrc.yaml or recipe.yaml
filters:
  license_header: "cat ./HEADER.txt"
  format_date: "date -d '{{ value }}' '+%Y-%m-%d'"
```

In a template:
```jinja2
{{ "2024-01-15" | format_date }}
```

This runs the shell command with `{{ value }}` replaced by the filter input. Simple, composable, no plugin API to learn.

---

## Libraries: Reusable Recipe Collections

Individual recipes are useful. But the real leverage comes when someone packages a set of recipes for a specific framework and shares them. That's a **jig library** — a versioned collection of recipes, templates, and patch definitions for a domain.

### What a Library Is

A library is a directory (or published package) of recipes organized by concern:

```
jig-django/
  jig-library.yaml            # library manifest
  model/
    add-field/
      recipe.yaml
      templates/
        model_field.j2
        migration_hint.j2
    add-model/
      recipe.yaml
      templates/
        model.py.j2
        admin.py.j2
        factories.py.j2
    add-index/
      recipe.yaml
      templates/
        index_migration.j2
  service/
    add-method/
      recipe.yaml
      templates/
        service_method.j2
        service_test.j2
    add-service/
      recipe.yaml
      templates/
        service.py.j2
        service_test.py.j2
  view/
    add-endpoint/
      recipe.yaml
      templates/
        view_method.j2
        url_pattern.j2
        schema_request.j2
        schema_response.j2
    add-view/
      recipe.yaml
      templates/
        view.py.j2
        urls.py.j2
        schemas.py.j2
  schema/
    add-field/
      recipe.yaml
      templates/
        msgspec_field.j2
        marshmallow_field.j2
  test/
    add-unit-test/
      recipe.yaml
      templates/ ...
    add-integration-test/
      recipe.yaml
      templates/ ...
    add-factory-field/
      recipe.yaml
      templates/ ...
  admin/
    add-field/
      recipe.yaml
      templates/ ...
    add-admin/
      recipe.yaml
      templates/ ...
```

### Library Manifest

```yaml
# jig-library.yaml
name: django
version: 0.3.0
description: Recipes for Django model/service/view development
framework: django
language: python

# Conventions this library assumes — the LLM reads these to map a project's
# actual layout to the library's expectations
conventions:
  models: "{{ app }}/models/{{ model | snakecase }}.py"
  services: "{{ app }}/services/{{ model | snakecase }}_service.py"
  views: "{{ app }}/views/{{ model | snakecase }}_view.py"
  schemas: "{{ app }}/schemas/{{ model | snakecase }}.py"
  tests: "{{ app }}/tests/test_{{ model | snakecase }}.py"
  factories: "{{ app }}/tests/factories.py"
  admin: "{{ app }}/admin.py"
  urls: "{{ app }}/urls.py"

# Recipes in this library
recipes:
  model/add-field: "Add a field to an existing Django model"
  model/add-model: "Scaffold a new Django model with admin, factory, and tests"
  model/add-index: "Add a database index to an existing model"
  service/add-method: "Add a method to an existing service class"
  service/add-service: "Scaffold a new service class with tests"
  view/add-endpoint: "Add an endpoint to an existing view"
  view/add-view: "Scaffold a new view with URL config and schemas"
  schema/add-field: "Add a field to an existing request/response schema"
  test/add-unit-test: "Generate a unit test for a service method"
  test/add-integration-test: "Generate an integration test for an endpoint"
  test/add-factory-field: "Add a field to a factory class"
  admin/add-field: "Add a field to admin list_display/fieldsets"
  admin/add-admin: "Scaffold admin registration for a model"

# Composable workflows — multi-recipe operations
workflows:
  add-field:
    description: "Add a field across the full stack (model → service → schema → view → admin → tests)"
    steps:
      - recipe: model/add-field
      - recipe: service/add-method
        when: "{{ update_service }}"
      - recipe: schema/add-field
      - recipe: view/add-endpoint
        when: "{{ update_view }}"
      - recipe: test/add-factory-field
      - recipe: admin/add-field

  add-endpoint:
    description: "Add a new API endpoint (view → URL → schema → test)"
    steps:
      - recipe: schema/add-field
        vars_map: { field_name: "request_field" }
      - recipe: view/add-endpoint
      - recipe: test/add-integration-test

  scaffold-resource:
    description: "Create a new model + service + view + admin from scratch"
    steps:
      - recipe: model/add-model
      - recipe: service/add-service
      - recipe: view/add-view
      - recipe: admin/add-admin
```

### Convention Mapping

The `conventions` block is crucial. It tells jig (and the LLM) where this library expects files to live. But real projects aren't always organized exactly the same way.

A project can override conventions in its `.jigrc.yaml`:

```yaml
# .jigrc.yaml
libraries:
  django:
    conventions:
      models: "{{ app }}/models.py"              # single-file models, not per-model files
      services: "{{ app }}/domain/services.py"    # different service location
      schemas: "{{ app }}/api/schemas.py"
```

This lets the same library work across projects with different directory structures. The override is simple key-value — remap the path pattern, keep everything else.

### Workflows: Multi-Recipe Operations

A workflow chains multiple recipes together. This is the answer to "I want to add a field and have it cascade through the whole stack."

```bash
# Run a full-stack field addition
jig workflow django/add-field \
  --vars '{"app": "hotels", "model": "Reservation", "field_name": "loyalty_tier", "field_type": "CharField", "field_args": "max_length=50", "nullable": true, "update_service": true, "update_view": false}'
```

Workflows support:

| Feature | Syntax | Behavior |
|---------|--------|----------|
| **Conditional steps** | `when: "{{ expr }}"` | Skip this recipe if the expression is falsy |
| **Variable mapping** | `vars_map: {a: b}` | Rename variables when passing to a sub-recipe |
| **Variable overrides** | `vars: {key: value}` | Override specific variables for a sub-recipe |
| **Shared variables** | (default) | All top-level variables are passed through to every step |
| **Stop on error** | `on_error: stop` (default) | Stop the workflow if any recipe fails |
| **Continue on error** | `on_error: continue` | Skip the failed recipe and continue |
| **Report partial** | `on_error: report` | Continue but mark the workflow as partial in output |

### Workflows vs. Includes

The earlier `includes` concept (in the Patches section) is now subsumed by workflows. The difference:

- **`includes`** was static — always runs all sub-recipes
- **Workflows** are conditional, configurable, and report per-step results

Workflows are the right abstraction. `includes` is removed.

### Installing Libraries

```bash
# From a git repository
jig library add https://github.com/someone/jig-django

# From a local directory (development / private libraries)
jig library add ./path/to/jig-django

# List installed libraries
jig library list

# Update a library
jig library update django

# Remove a library
jig library remove django
```

Libraries are installed to `~/.jig/libraries/` (global) or `.jig/libraries/` (project-local). Project-local takes precedence.

### Library Discovery

```bash
# List all recipes in an installed library
jig library recipes django

# Show details for a specific recipe
jig library info django/model/add-field

# List all workflows
jig library workflows django
```

### Built-In vs. Community Libraries

jig ships with zero built-in libraries. The tool is framework-agnostic. Libraries are where framework opinions live:

| Library | Domain |
|---------|--------|
| `jig-django` | Django models, views, services, admin, DRF, msgspec |
| `jig-fastapi` | FastAPI routes, Pydantic models, dependency injection |
| `jig-rails` | Rails models, controllers, views, migrations, RSpec |
| `jig-nextjs` | Next.js pages, API routes, components, server actions |
| `jig-vue` | Vue 3 components, composables, Pinia stores, Vitest |
| `jig-react` | React components, hooks, tests, Storybook stories |
| `jig-express` | Express routes, middleware, controllers, Joi schemas |
| `jig-rust` | Rust structs, traits, impls, tests, Cargo features |
| `jig-go` | Go structs, handlers, middleware, table-driven tests |
| `jig-flutter` | Dart widgets, BLoC providers, repository pattern |

Anyone can publish a library. There's no registry — it's just a git repo with a `jig-library.yaml`.

### Libraries as Claude Code Plugins

A jig library can simultaneously be a Claude Code plugin. The plugin provides skills that know how to use the library's recipes:

```
jig-django/
  jig-library.yaml              # jig library manifest
  .claude-plugin/
    plugin.json                  # Claude Code plugin manifest
  skills/
    add-field/
      SKILL.md                   # "Read the model, extract context, run jig workflow django/add-field"
    add-endpoint/
      SKILL.md                   # "Read the view, extract context, run jig workflow django/add-endpoint"
    scaffold-resource/
      SKILL.md                   # "Ask the user for model name + fields, run jig workflow django/scaffold-resource"
  model/
    add-field/
      recipe.yaml
      templates/ ...
  service/ ...
  view/ ...
```

The skills wrap the recipes with LLM intelligence:
- The **skill** reads code, understands context, extracts variables, and decides which workflow to run
- The **recipe/workflow** does the deterministic file generation and patching
- The **library** provides the full collection, installable as one unit

This means a team can `jig library add jig-django` to get the CLI recipes, AND install the Claude Code plugin to get the LLM-powered skills that invoke them.

### Project-Specific Overrides and Extensions

Teams can extend a library with their own recipes without forking it:

```
my-project/
  .jig/
    libraries/
      django/          # installed library (read-only)
    overrides/
      django/
        model/
          add-field/
            templates/
              model_field.j2    # override just this one template
    extensions/
      django/
        model/
          add-audit-fields/
            recipe.yaml         # new recipe that extends the library
            templates/ ...
```

- **Overrides** replace a single template in a library recipe (your team uses a different field style)
- **Extensions** add new recipes namespaced under the library (your team has audit field patterns unique to your codebase)

Both are project-local and version-controlled. The upstream library can be updated without conflicts.

### The Vision: Framework Ecosystem

The end state is:

1. **jig** is the engine — small, fast, framework-agnostic
2. **Libraries** are where the framework knowledge lives — community-maintained, versioned, overridable
3. **Claude Code plugins** wrap libraries with LLM intelligence — the skill reads code and decides what to do, the recipe handles the mechanical work
4. **Project overrides** customize libraries for your team's conventions — no forks, no divergence

A new developer joins a Django team. They install the jig-django Claude Code plugin. Now they can say "add a field called loyalty_tier to Reservation" and get a consistent, team-compliant, multi-file change — without the LLM having to learn the team's patterns from scratch every time.

---

## Thinking Bigger: What Else Composes?

The Django example is the most obvious case, but the pattern applies anywhere you have layered architecture with predictable cross-cutting changes:

### Frontend (Vue/React/Angular)

Adding a new prop to a component:
- Component file (add prop declaration)
- Parent components (pass the new prop at call sites)
- Types file (add to interface)
- Tests (add to mock props, add assertions)
- Storybook (add to stories, add knob)

Adding a new store field:
- Store (add state, getter, action, mutation)
- Components (add computed property or composable usage)
- API layer (add to request/response types)
- Tests (add to store tests, component tests)

### API Layer

Adding a new endpoint:
- Route definition
- Controller/handler
- Request validation schema
- Response serialization schema
- Middleware registration
- OpenAPI/Swagger docs
- Client SDK regeneration
- Integration tests

### Infrastructure

Adding a new service:
- Dockerfile
- Docker Compose service
- Process Compose config
- Environment variables
- Health check endpoint
- CI pipeline step
- Deployment config

### Mobile (Flutter/React Native)

Adding a new screen:
- Screen widget/component
- Navigation/routing registration
- State management (BLoC/Redux/provider)
- API repository method
- DTO/model class
- Tests (widget test, unit test, integration test)

Every one of these is a **workflow** — a predictable cascade of small, mechanical changes across files. jig libraries capture these patterns once. The LLM provides the judgment (what to call it, what type it should be, where it fits). jig provides the execution (render, patch, create, inject).

---

## The Feedback Loop: Scan, Infer, Check

The core `jig run` flow is one-directional: variables → template → files. But the most powerful workflows are cycles. These features close the loop so that jig can read existing code, learn patterns, and verify conformance — not just generate.

### Scan: Reading Code Backwards Into Variables

`jig scan` reverses the recipe. Instead of "given these variables, produce this file," it asks "given this file, what variables would have produced it?"

```bash
jig scan django/model ./hotels/models/reservation.py
```

Output:
```json
{
  "recipe": "django/model/add-model",
  "confidence": 0.92,
  "variables": {
    "app": "hotels",
    "model": "Reservation",
    "fields": [
      {"name": "guest_name", "type": "CharField", "args": "max_length=255"},
      {"name": "check_in", "type": "DateField", "args": ""},
      {"name": "check_out", "type": "DateField", "args": ""},
      {"name": "status", "type": "CharField", "args": "max_length=20, choices=STATUS_CHOICES"},
      {"name": "total_amount", "type": "DecimalField", "args": "max_digits=10, decimal_places=2"}
    ],
    "mixins": ["TimeStampedModel", "SoftDeleteModel"],
    "meta": {"ordering": ["-created_at"], "db_table": "hotels_reservation"}
  },
  "unrecognized": [
    {"line": 34, "content": "objects = ReservationQuerySet.as_manager()", "note": "custom manager — not captured by recipe variables"}
  ]
}
```

#### How Scan Works

1. jig reads the target file
2. It matches the file against a library's recipe using the recipe's anchor patterns and scope definitions
3. For each template in the recipe, it reverse-matches: finds the structural elements that the template would have generated and extracts the variable values
4. Fields it can't explain are returned in `unrecognized` — these are project-specific customizations that go beyond the recipe

#### Why Scan Matters

Scan is the bridge between "I have existing code" and "I want to extend it with jig." Without scan, the LLM has to read the file, understand the structure, and construct the variables JSON manually. With scan, the LLM gets a structured representation of the existing code that maps directly to recipe variables.

The workflow becomes:

```
1. jig scan django/model ./hotels/models/reservation.py    → get current state as variables
2. LLM modifies the variables (adds a field to the array)
3. jig run django/model/add-field --vars '...'              → apply the change
```

Step 2 is where the LLM adds value — understanding user intent, choosing the right field type, naming things well. Steps 1 and 3 are mechanical.

#### Scan for Discovery

Scan also works at the directory level to discover what's in a project:

```bash
jig scan django ./hotels/
```

Output:
```json
{
  "models": [
    {"file": "models/reservation.py", "class": "Reservation", "fields": 12},
    {"file": "models/payment.py", "class": "Payment", "fields": 8},
    {"file": "models/guest.py", "class": "Guest", "fields": 15}
  ],
  "services": [
    {"file": "services/reservation_service.py", "class": "ReservationService", "methods": 6},
    {"file": "services/payment_service.py", "class": "PaymentService", "methods": 4}
  ],
  "views": [
    {"file": "views/reservation_view.py", "class": "ReservationView", "endpoints": 5}
  ],
  "coverage": {
    "models_with_services": ["Reservation", "Payment"],
    "models_without_services": ["Guest"],
    "models_with_admin": ["Reservation", "Guest"],
    "models_without_admin": ["Payment"]
  }
}
```

This gives the LLM (or the developer) a project map — which models exist, which have services, which are missing admin registration. It's the project graph, derived from the code, structured by the library's understanding of what "complete" looks like.

### Infer: Learning Recipes From Examples

Creating recipes by hand is the highest-friction part of jig. `jig infer` dramatically lowers that cost by learning from examples.

#### From Before/After Pairs

```bash
# Show jig what adding a field looks like
jig infer \
  --before hotels/models/reservation.py.before \
  --after  hotels/models/reservation.py.after \
  --name "model-field"
```

Output:
```yaml
# Inferred recipe (draft)
name: model-field
description: "Add a field to a Django model class"
confidence: 0.87

variables:
  field_name:
    type: string
    inferred_from: "loyalty_tier"  # the actual value found in the diff
  field_type:
    type: string
    inferred_from: "CharField"
  field_args:
    type: string
    inferred_from: "max_length=50, null=True"

files:
  - template: model_field.j2     # auto-generated template
    patch: "{{ target_file }}"   # placeholder — needs convention mapping
    anchor:
      pattern: "^class \\w+\\("
      scope: class_body
      position: after_last_field
    skip_if: "{{ field_name }}"

# Auto-generated template:
# --- model_field.j2 ---
#     {{ field_name }} = models.{{ field_type }}({{ field_args }})
```

jig diffs the before/after, identifies the structural change (a line was added inside a class body, after the existing field declarations), extracts the variable parts (field name, type, args), and proposes a recipe + template.

#### From Multiple Examples

The more examples you give, the better the inference:

```bash
jig infer \
  --example hotels/models/reservation.py:before,after \
  --example hotels/models/payment.py:before,after \
  --example hotels/models/guest.py:before,after \
  --name "model-field"
```

With three examples, jig can:
- Distinguish the variable parts (they change across examples) from the fixed parts (they stay the same)
- Infer types more accurately (saw CharField twice and IntegerField once → field_type is a string, not a constant)
- Detect optional parts (field_args was empty in one example → it's optional with a default of "")

#### From Git History

The most powerful mode — jig reads your git history to find repeated patterns:

```bash
jig infer --from-git --pattern "Add * field to *" --limit 10
```

jig searches commit messages matching the pattern, extracts the diffs, groups them by structural similarity, and proposes recipes for the most common patterns. It's literally learning from your team's past behavior.

#### Infer for Workflows

`jig infer` can also learn multi-file patterns:

```bash
# Show jig a commit that touched multiple files
jig infer --from-commit abc123f
```

If the commit added a field to the model, service, schema, and test factory, jig proposes a workflow with one recipe per file, all sharing the same variables. The entire "add a field across the stack" workflow is inferred from a single past commit.

#### The Human-in-the-Loop

Inferred recipes are always drafts. jig writes them to a `_drafts/` directory and marks them with `confidence` scores. The developer (or LLM) reviews, adjusts variable names, fixes edge cases, and promotes the draft to a real recipe:

```bash
# Review the draft
jig infer review _drafts/model-field/

# Promote to a real recipe
jig infer promote _drafts/model-field/ --to model/add-field/
```

The expectation is that inferred recipes are 70-90% right. The last 10-30% is human judgment — naming variables well, handling edge cases, adding skip_if guards. But getting 70% for free is a massive speedup over writing from scratch.

### Check: Conformance Verification

If jig knows the recipe for "how a model should look," it can verify that existing models conform. This is the audit direction — templates as structural lint rules.

```bash
jig check django/model ./hotels/models/*.py
```

Output:
```
hotels/models/reservation.py
  ✓ class structure matches model recipe
  ✓ has TimeStampedModel mixin
  ✓ has Meta class with ordering
  ✗ missing __str__ method (expected by convention)

hotels/models/payment.py
  ✓ class structure matches model recipe
  ✗ missing TimeStampedModel mixin (expected by convention)
  ✗ missing admin registration in hotels/admin.py

hotels/models/legacy_rate.py
  ~ partial match (confidence: 0.6)
  ✗ uses old-style CharField without explicit max_length
  ✗ missing factory in tests/factories.py
  ✗ missing service class

Summary: 3 models checked, 1 conformant, 2 with issues
```

#### Check Levels

```bash
# Check a single file against a recipe
jig check django/model ./hotels/models/reservation.py

# Check all models in a directory
jig check django/model ./hotels/models/

# Check the full stack for a resource
jig check django/scaffold-resource --resource Reservation --app hotels

# Check everything the library knows about
jig check django ./hotels/
```

#### Check Output for LLMs

```bash
jig check django/model ./hotels/models/ --json
```

```json
{
  "results": [
    {
      "file": "hotels/models/reservation.py",
      "recipe": "django/model/add-model",
      "conformant": true,
      "issues": [
        {"severity": "warn", "rule": "missing_str_method", "message": "No __str__ method defined", "fix_recipe": "django/model/add-str-method"}
      ]
    },
    {
      "file": "hotels/models/payment.py",
      "recipe": "django/model/add-model",
      "conformant": false,
      "issues": [
        {"severity": "error", "rule": "missing_mixin", "message": "Missing TimeStampedModel mixin", "fix_recipe": null},
        {"severity": "warn", "rule": "missing_admin", "message": "No admin registration found", "fix_recipe": "django/admin/add-admin"}
      ]
    }
  ]
}
```

The `fix_recipe` field is key — when an issue has a known recipe that would fix it, jig tells you. The LLM can read the check output and automatically run the fix recipes:

```
1. jig check django ./hotels/ --json        → find conformance issues
2. LLM reads issues, filters actionable ones
3. jig run django/admin/add-admin --vars ... → fix the missing admin
4. jig run django/model/add-str-method ...   → fix the missing __str__
```

Scan → Check → Fix is a self-healing loop.

#### Check as CI Gate

```bash
# In CI: fail if any model is non-conformant
jig check django/model ./*/models/ --strict --exit-on-failure
```

This turns your team's conventions into enforceable rules — not through lint config files, but through the same recipe templates that generate the code. The source of truth is the recipe.

### Polyglot Workflows: Crossing the Stack Boundary

A single user action often spans multiple frameworks. Adding a field to a Django model also means updating TypeScript types in the frontend, adding a column to the Vue table component, and maybe updating the API client SDK.

Polyglot workflows span multiple libraries:

```yaml
# In a project-level .jig/workflows/add-field-fullstack.yaml
name: add-field-fullstack
description: Add a field from database to UI

variables:
  app: { type: string, required: true }
  model: { type: string, required: true }
  field_name: { type: string, required: true }
  field_type: { type: string, required: true }
  field_args: { type: string, default: "" }
  nullable: { type: boolean, default: false }
  ts_type: { type: string, required: true, description: "TypeScript type for the field" }
  show_in_table: { type: boolean, default: true }
  show_in_form: { type: boolean, default: true }

steps:
  # Backend (jig-django library)
  - library: django
    workflow: add-field
    vars:
      app: "{{ app }}"
      model: "{{ model }}"
      field_name: "{{ field_name }}"
      field_type: "{{ field_type }}"
      field_args: "{{ field_args }}"
      nullable: "{{ nullable }}"

  # Frontend types (jig-vue library)
  - library: vue
    recipe: types/add-field
    vars:
      interface: "{{ model }}"
      field: "{{ field_name }}"
      type: "{{ ts_type }}"
      optional: "{{ nullable }}"

  # Frontend table column (jig-vue library)
  - library: vue
    recipe: component/add-column
    when: "{{ show_in_table }}"
    vars:
      component: "{{ model }}Table"
      field: "{{ field_name }}"
      label: "{{ field_name | capitalize | replace('_', ' ') }}"

  # Frontend form field (jig-vue library)
  - library: vue
    recipe: component/add-form-field
    when: "{{ show_in_form }}"
    vars:
      component: "{{ model }}Form"
      field: "{{ field_name }}"
      type: "{{ ts_type }}"
      required: "{{ not nullable }}"
```

One command:
```bash
jig workflow add-field-fullstack \
  --vars '{"app":"hotels","model":"Reservation","field_name":"loyalty_tier","field_type":"CharField","field_args":"max_length=50","nullable":true,"ts_type":"string | null","show_in_table":true,"show_in_form":true}'
```

Touches: model, service, schema, admin, factory, tests (Django) + TypeScript interface, table component, form component (Vue). All from a single invocation with a single variables JSON.

### Schema-First Generation

Instead of hand-specifying variables, derive them from an existing schema definition:

```bash
# From an OpenAPI spec — generate all backend + frontend for a resource
jig from-schema openapi ./api-spec.yaml \
  --resource Reservation \
  --workflow scaffold-resource-fullstack

# From a database table — reverse-engineer models from an existing DB
jig from-schema sql --connection $DATABASE_URL \
  --table reservations \
  --recipe django/model/add-model

# From a protobuf definition
jig from-schema proto ./reservation.proto \
  --recipe go/scaffold-handler

# From a GraphQL schema
jig from-schema graphql ./schema.graphql \
  --type Reservation \
  --workflow nextjs/scaffold-page
```

#### Schema Type Mapping

Each library defines how schema types map to framework types:

```yaml
# In jig-library.yaml
type_mappings:
  openapi_to_django:
    string: { type: "CharField", args: "max_length=255" }
    string[format=email]: { type: "EmailField", args: "" }
    string[format=date]: { type: "DateField", args: "" }
    string[format=date-time]: { type: "DateTimeField", args: "" }
    string[format=uuid]: { type: "UUIDField", args: "default=uuid.uuid4" }
    string[enum]: { type: "CharField", args: "max_length=50, choices=CHOICES" }
    integer: { type: "IntegerField", args: "" }
    integer[format=int64]: { type: "BigIntegerField", args: "" }
    number: { type: "DecimalField", args: "max_digits=10, decimal_places=2" }
    boolean: { type: "BooleanField", args: "default=False" }
    array: { type: "JSONField", args: "default=list" }
    object: { type: "JSONField", args: "default=dict" }

  openapi_to_typescript:
    string: "string"
    string[format=date]: "string"
    string[format=date-time]: "string"
    integer: "number"
    number: "number"
    boolean: "boolean"
    array: "Array<unknown>"
    object: "Record<string, unknown>"
```

With this mapping, `jig from-schema` reads the spec, resolves types for both Django and TypeScript, and passes the right variables to the right recipes. The schema is the single source of truth; jig translates it into framework-specific code.

### Observing the LLM: Recipe Discovery from Behavior

This is the meta-feature — a Claude Code skill that watches what the LLM does manually and proposes recipes to automate it.

#### How It Works

A Claude Code hook runs after each tool call, logging what files were created/modified and what content was inserted. After a session (or on demand), a skill analyzes the log:

```bash
# After a coding session
/jig:discover
```

Output:
```
I noticed 3 repeated patterns in your recent sessions:

1. "Add model field" (seen 4 times)
   Files touched: models/*.py, services/*.py, schemas/*.py, tests/factories.py
   Confidence: 0.91
   → Run `jig infer promote` to create recipe

2. "Add API endpoint" (seen 2 times)
   Files touched: views/*.py, urls.py, schemas/*.py, tests/test_*.py
   Confidence: 0.78
   → Needs one more example for high confidence

3. "Add admin permission" (seen 3 times)
   Files touched: admin.py, permissions.py
   Confidence: 0.85
   → Run `jig infer promote` to create recipe
```

#### The Learning Curve Is Zero

The developer never has to decide "I should create a recipe for this." They just work normally. jig observes, detects patterns, and proposes recipes when it has enough examples. The team's conventions emerge from their actual behavior, not from a spec document someone has to write and maintain.

#### Privacy and Scope

The observation log is project-local (`.jig/observations/`) and never leaves the machine. It records structural patterns (file paths, change shapes), not content. It can be disabled with a single flag in `.jigrc.yaml`:

```yaml
observe: false  # disable pattern observation
```

---

## Roadmap

### v0.1 — MVP: Core Engine
- Recipe YAML parsing
- Jinja2 rendering (minijinja) with built-in filters
- Create operation (new files)
- Inject operation (after, before, append, prepend, skip_if)
- JSON variables via --vars, --vars-file, --vars-stdin
- Dry-run mode
- JSON and human-readable output
- Deterministic exit codes

### v0.2 — Brownfield: Patches
- Patch operation with anchor system
- Scope detection (indentation-based and delimiter-based)
- Semantic positions (after_last_field, after_last_method, etc.)
- Replace operation (between, pattern, fallback)
- `find` narrowing within scopes
- `skip_if` idempotency for patches
- `--verbose` mode showing scope boundaries and insertion points

### v0.3 — Composition: Workflows
- Workflow definitions in recipe YAML
- Conditional steps (`when` expressions)
- Variable mapping and overrides between steps
- Error handling modes (stop, continue, report)
- `jig workflow` command
- Per-step result reporting in JSON output

### v0.4 — Ecosystem: Libraries
- Library manifest format (`jig-library.yaml`)
- `jig library add/remove/update/list` commands
- Convention mapping and project-level overrides
- Library recipe discovery (`jig library recipes <name>`)
- Project-local extensions directory
- Template overrides (replace a single template without forking)

### v0.5 — Distribution
- GitHub Actions release pipeline (cross-compile all targets)
- Homebrew tap formula
- `cargo install jig-cli`
- Nix flake
- npm binary wrapper (platform-specific postinstall, like esbuild)
- Shell installer script
- README with usage examples

### v0.6 — Claude Code Plugin
- Plugin with `/jig:init` skill (scaffold recipe + templates in a skill directory)
- Plugin with `/jig:doctor` skill (validate all recipes in a plugin)
- Reference documentation for skill authors
- `jig-django` library as first community library + Claude Code plugin dual-publish
- Example workflows: add-field, add-endpoint, scaffold-resource

### v1.0 — Stable
- Semver stability guarantee on recipe format, anchor/scope system, and library manifest
- Semver stability guarantee on CLI interface and JSON output
- Semver stability guarantee on exit codes
- Comprehensive documentation site
- Published to homebrew-core
- At least 3 community libraries (django, vue/react, rails or fastapi)

### v0.7 — Scan and Check
- `jig scan` — reverse a recipe to extract variables from existing code
- Directory-level scan for project mapping
- `jig check` — conformance verification against recipes
- `fix_recipe` references in check output
- `--strict` and `--exit-on-failure` for CI gating

### v0.8 — Infer
- `jig infer` from before/after file pairs
- Multi-example inference with variable detection
- `jig infer --from-git` to learn from commit history
- Multi-file workflow inference from single commits
- Draft review and promote workflow

### v0.9 — Polyglot and Schema-First
- Cross-library workflows (django + vue in a single workflow)
- `jig from-schema openapi|sql|proto|graphql` command
- Type mapping definitions in library manifests
- Schema-to-variables resolution

### v1.0 — Stable
- Semver stability guarantee on recipe format, anchor/scope system, and library manifest
- Semver stability guarantee on CLI interface and JSON output
- Semver stability guarantee on exit codes
- Comprehensive documentation site
- Published to homebrew-core
- At least 3 community libraries (django, vue/react, rails or fastapi)

### Future (Post-1.0)

- **Observation engine** — Claude Code hook that logs edit patterns and proposes recipes from repeated behavior
- **Hooks** — `pre_run` and `post_run` shell commands in recipes (e.g., run `ruff format` on patched files, `prettier --write` on generated TypeScript)
- **Interactive mode** — for human users who want to be prompted for variables (not the priority, but nice to have)
- **Watch mode** — re-render when templates change (useful when authoring templates)
- **Tree-sitter integration** — optional, for projects that need precise AST-aware scoping beyond indentation/delimiter heuristics
- **Library registry** — a searchable index of community libraries (like crates.io or npm, but for jig libraries)
- **Diff preview** — `--diff` flag that outputs unified diffs instead of writing files, for review before applying
- **Undo** — `jig undo` to revert the last run (backed by a `.jig/history/` log of operations and original file contents)
- **Template linting** — `jig lint` to catch common template mistakes (unused variables, unreachable conditionals, missing skip_if for patches)

---

## Cross-Compatibility: Working With Every Agentic Coding Tool

jig is not Claude Code-specific. It's designed to work with every LLM-powered coding agent — Claude Code, Codex, OpenCode, Cursor, Windsurf, Aider, Continue, Cline, GitHub Copilot, Zed AI, Amp, and whatever comes next.

### The Universal Interface: CLI + JSON stdout

Every agentic coding tool in 2026 can invoke a CLI via a shell/bash tool and read its stdout. This is the lowest common denominator. It works today, on all 11 major tools, with zero integration effort.

| Tool | Shell/Bash | MCP Client | Plugin System |
|------|-----------|------------|---------------|
| Claude Code | Yes | Yes | Skills, Hooks |
| Codex CLI (OpenAI) | Yes (sandboxed) | Yes | AGENTS.md |
| OpenCode | Yes | Yes | Agents, Skills |
| Cursor | Yes | Yes | .cursorrules |
| Windsurf | Yes | Yes | None |
| Aider | Limited (/run) | No (community workarounds) | None |
| Continue | Yes (Agent mode) | Yes | Slash cmds |
| Cline | Yes | Yes | MCP marketplace |
| GitHub Copilot | Yes | Yes | AGENTS.md |
| Zed AI | Yes | Yes | Tool Profiles |
| Amp (Sourcegraph) | Yes | Yes | Skills w/ bundled MCP |

**Every single tool can call `jig run` and parse the JSON output.** This is the foundation.

### Design Rules for Agent-Friendly CLIs

1. **JSON to stdout, human-readable to stderr.** Every tool reads stdout. The LLM parses JSON reliably. Progress messages go to stderr so they don't pollute structured output.

2. **Meaningful exit codes.** Not just 0/1 — distinct codes for validation errors, template errors, file operation errors, variable errors. Agents branch on exit codes without parsing error messages.

3. **Non-interactive by default.** Never prompt for input. Never hang waiting for stdin. If stdin is not a TTY, fail with a clear error. Agents can't answer interactive prompts.

4. **`--json` flag or auto-detect non-TTY.** When stdout is piped (not a terminal), automatically switch to JSON output. Or use `--json` to force it. Agents always get structured data.

5. **Single-invocation design.** Windsurf limits 20 tool calls per prompt. Cursor recommends under 40 total tools. Design commands that do one complete unit of work per invocation — don't require multi-step call sequences.

6. **Schema introspection via CLI.** `jig vars <recipe>` outputs the expected input schema as JSON. Agents can call this to discover what variables are needed, then construct the `--vars` JSON correctly without guessing.

7. **`--help` with noun-verb structure.** Agents explore CLIs by running `--help`. A tree structure (`jig run`, `jig scan`, `jig check`, `jig library list`) is navigable by an LLM in 1-2 `--help` calls.

### MCP Server: Structured Tool Integration

10 of 11 tools support MCP (Model Context Protocol) natively. An MCP server wraps the CLI with typed tool definitions, so agents don't need to parse `--help` or construct CLI flags — they call structured tools with JSON parameters and get JSON responses.

#### Architecture

The MCP server is a thin stdio wrapper. It does not reimplement jig's logic — it shells out to the `jig` binary:

```
Agent (Claude Code, Codex, Cursor, etc.)
  ↕ MCP stdio transport (JSON-RPC over stdin/stdout)
jig-mcp-server
  ↕ spawns subprocess
jig CLI binary
  ↕ reads/writes files
filesystem
```

#### How stdio MCP Works

MCP stdio transport is the simplest form: the agent launches the MCP server as a child process and communicates over stdin/stdout using JSON-RPC.

```
Agent starts: jig-mcp-server (as child process)
  Agent writes to server's stdin:  {"jsonrpc":"2.0","method":"tools/list","id":1}
  Server writes to its stdout:     {"jsonrpc":"2.0","result":{"tools":[...]},"id":1}

  Agent writes to server's stdin:  {"jsonrpc":"2.0","method":"tools/call","params":{"name":"jig_run","arguments":{...}},"id":2}
  Server internally runs:          jig run ./recipe.yaml --vars '...' --json
  Server writes to its stdout:     {"jsonrpc":"2.0","result":{"content":[{"type":"text","text":"..."}]},"id":2}
```

No HTTP, no WebSocket, no ports. Just pipes. The agent manages the server's lifecycle — starts it at session begin, kills it at session end.

#### MCP Tool Definitions

```json
{
  "tools": [
    {
      "name": "jig_run",
      "description": "Run a jig recipe to create/patch/inject files from templates",
      "inputSchema": {
        "type": "object",
        "properties": {
          "recipe": {"type": "string", "description": "Path to recipe.yaml"},
          "vars": {"type": "object", "description": "Template variables as JSON object"},
          "dry_run": {"type": "boolean", "default": false},
          "base_dir": {"type": "string", "description": "Base directory for output paths"}
        },
        "required": ["recipe", "vars"]
      }
    },
    {
      "name": "jig_vars",
      "description": "List the variables a recipe expects, with types and descriptions",
      "inputSchema": {
        "type": "object",
        "properties": {
          "recipe": {"type": "string", "description": "Path to recipe.yaml"}
        },
        "required": ["recipe"]
      }
    },
    {
      "name": "jig_scan",
      "description": "Scan an existing file and extract variables that match a recipe",
      "inputSchema": {
        "type": "object",
        "properties": {
          "recipe": {"type": "string", "description": "Library recipe (e.g., django/model)"},
          "path": {"type": "string", "description": "File or directory to scan"}
        },
        "required": ["recipe", "path"]
      }
    },
    {
      "name": "jig_check",
      "description": "Check existing files for conformance against a recipe",
      "inputSchema": {
        "type": "object",
        "properties": {
          "recipe": {"type": "string", "description": "Library recipe to check against"},
          "path": {"type": "string", "description": "File or directory to check"},
          "strict": {"type": "boolean", "default": false}
        },
        "required": ["recipe", "path"]
      }
    },
    {
      "name": "jig_workflow",
      "description": "Run a multi-step workflow that chains recipes together",
      "inputSchema": {
        "type": "object",
        "properties": {
          "workflow": {"type": "string", "description": "Library workflow (e.g., django/add-field)"},
          "vars": {"type": "object", "description": "Workflow variables"}
        },
        "required": ["workflow", "vars"]
      }
    },
    {
      "name": "jig_library_recipes",
      "description": "List all available recipes in an installed library",
      "inputSchema": {
        "type": "object",
        "properties": {
          "library": {"type": "string", "description": "Library name (e.g., django)"}
        },
        "required": ["library"]
      }
    }
  ]
}
```

Each tool call translates to a CLI invocation internally:
- `jig_run` → `jig run <recipe> --vars '<json>' --json`
- `jig_vars` → `jig vars <recipe>`
- `jig_scan` → `jig scan <recipe> <path> --json`
- `jig_check` → `jig check <recipe> <path> --json`
- `jig_workflow` → `jig workflow <workflow> --vars '<json>' --json`
- `jig_library_recipes` → `jig library recipes <library> --json`

#### MCP Server Implementation

The MCP server is a separate, tiny binary (or script). It can be:

1. **A Rust binary** — built alongside jig, shares the same release pipeline. ~200 lines of MCP protocol handling + subprocess spawning.
2. **A TypeScript/Node script** — using the `@modelcontextprotocol/sdk` package. Even simpler. Many MCP servers are written this way. Distributed via npm.
3. **A Python script** — using the `mcp` Python SDK. Minimal code.

The TypeScript approach is probably the most pragmatic — the MCP SDK is mature, the server is ~100 lines, and it's distributed via npm where most MCP servers live:

```bash
npx @jig/mcp-server
```

The server finds the `jig` binary on PATH and wraps it.

#### MCP Configuration Per Tool

Each tool has its own config file for registering MCP servers:

```jsonc
// Claude Code: .mcp.json (project) or ~/.claude/settings.json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig/mcp-server"],
      "env": {}
    }
  }
}
```

```toml
# Codex CLI: ~/.codex/config.toml
[mcp_servers.jig]
command = "npx"
args = ["@jig/mcp-server"]
```

```jsonc
// Cursor: .cursor/mcp.json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig/mcp-server"]
    }
  }
}
```

```jsonc
// Windsurf: ~/.codeium/windsurf/mcp_config.json
{
  "mcpServers": {
    "jig": {
      "command": "npx",
      "args": ["@jig/mcp-server"]
    }
  }
}
```

Same pattern for Cline, Continue, Zed, Amp. The config syntax varies slightly but the content is identical: command + args to launch the stdio server.

#### Why MCP Over Just CLI

| Aspect | CLI via Bash | MCP Server |
|--------|-------------|------------|
| **Discovery** | Agent runs `jig --help`, parses text | Agent gets typed tool list automatically |
| **Parameter validation** | Agent constructs flags, might get them wrong | Agent fills a JSON schema, validated before execution |
| **Return type** | Raw text that agent must parse | Structured JSON with content type annotations |
| **Setup cost** | Zero — just needs jig on PATH | One-time config file entry |
| **Works everywhere** | 11/11 tools | 10/11 tools (not Aider) |

MCP is strictly better for tools that support it. CLI is the fallback that works everywhere. Ship both.

### Project-Level Instructions: CLAUDE.md / AGENTS.md / .cursorrules

For guided usage without MCP, project-level instruction files tell the agent about jig:

```markdown
<!-- In CLAUDE.md, AGENTS.md, or .cursorrules -->

## Code Generation with jig

This project uses jig for template-based code generation. When creating or extending
models, services, views, or tests, prefer jig recipes over manual code generation.

Available workflows:
- `jig workflow django/add-field --vars '...'` — add a field across the full stack
- `jig workflow django/add-endpoint --vars '...'` — add an API endpoint
- `jig workflow django/scaffold-resource --vars '...'` — scaffold a new resource

To see what variables a recipe needs: `jig vars <recipe>`
To scan an existing file: `jig scan django/model <path>`

Always use --json flag and review the output before proceeding.
```

This costs nothing, works immediately, and is compatible with every tool that reads project instructions.

### The Compatibility Strategy

Three layers, each reaching more tools:

```
Layer 1: CLI + JSON stdout     → works with 11/11 tools (universal)
Layer 2: MCP stdio server      → works with 10/11 tools (typed, discoverable)
Layer 3: Project instructions  → works with 6/11 tools (guided, contextual)
```

All three layers ship together. The CLI is the core. The MCP server is a 100-line wrapper. The project instructions are a markdown snippet. Total effort to support all agentic coding tools: the CLI itself + ~200 lines of MCP glue.

### Claude Code Plugin: The High-Fidelity Integration

For Claude Code specifically, the deepest integration is a plugin that bundles:

1. **The MCP server** — for typed tool calls
2. **Skills** — for intelligent workflows that combine jig with Claude's native tools
3. **Hooks** — for automatic behavior (e.g., run `jig check` after every model file edit)

But this is additive. Teams using Codex or Cursor get 90% of the value from the CLI + MCP server alone. The Claude Code plugin adds the last 10% — LLM-powered intelligence around when and how to invoke jig.

---

## Design Principles

1. **Templates live with the consumer.** No central template directory. A skill, a project, a team — each owns their templates alongside their code. Libraries distribute collections, but they install locally.

2. **JSON in, files out.** The interface is structured data → rendered files. No interactive prompts, no wizard flows. An LLM produces JSON naturally; jig consumes it.

3. **Deterministic.** Same recipe + same variables + same existing files = same output, every time. No randomness, no creativity, no drift.

4. **Greenfield and brownfield.** Creating new files and extending existing files are equally first-class. Most real work is brownfield — adding a field, a method, an endpoint — and the tool must make that as easy as scaffolding from scratch.

5. **Transparent failures.** Every error tells you what, where, and why. Exit codes are semantic. JSON output includes skip reasons. When a patch can't find its anchor, jig still returns the rendered content so the LLM can place it manually. The deterministic work (rendering) is never wasted by a placement failure.

6. **Composable with LLM tools.** jig handles what templates handle well (boilerplate, structure, repetition). It does NOT try to handle what LLMs handle well (understanding code semantics, choosing the right insertion point in novel files, adapting to unexpected structures). When jig's scope detection fails, the LLM falls back to its native edit tool — and jig's output tells it exactly what to fix.

7. **Layered ecosystem.** The engine is framework-agnostic. Libraries add framework knowledge. Claude Code plugins add LLM intelligence. Project overrides add team conventions. Each layer is optional, composable, and independently versioned.

8. **Idempotent by default.** Every operation can be run twice safely. Patches check `skip_if`. Creates check `skip_if_exists`. Workflows report what was skipped and why. An LLM retrying a failed run should never duplicate content.

9. **Small.** One binary. Twenty crates. Five-second install. If jig itself needs a tutorial, it's too complicated.

---

## Agent Eval System

jig's mechanical correctness is testable with unit and integration tests. But jig's actual value proposition — that it makes LLM-driven code generation more consistent and reliable — is only testable by putting real agents in front of it and measuring whether they succeed. The eval system tests the full loop: agent reads a codebase, understands an intent, invokes jig with the right variables, and produces the correct multi-file change. This is fundamentally different from testing jig's internals. It tests the **ergonomic surface** — whether agents can actually hold this tool.

### What We're Measuring

Two distinct axes, both critical:

1. **Mechanical correctness** — Did jig produce the right files given the right variables? (deterministic, tested by unit/integration tests)
2. **Agent usability** — Can an agent figure out the right variables, invoke jig correctly, handle errors, and produce the intended change? (stochastic, tested by the eval system)

The eval system targets axis 2 exclusively. It answers questions like:
- Can the agent discover what recipe to use?
- Can the agent extract the right variables from existing code?
- Does the agent construct valid `--vars` JSON?
- Does the agent recover when jig returns an error?
- Does the agent know when to fall back to manual editing?
- How does success rate vary across agents (Claude, GPT, Codex, etc.)?
- How does success rate change as we modify jig's CLI output, error messages, help text, or MCP tool descriptions?

These are the questions that drive design decisions. If agents consistently fail to construct the `--vars` JSON correctly, that's a signal to simplify the variable interface. If agents never discover `jig vars`, that's a signal to improve discoverability. The eval system turns "is this tool easy for agents to use?" into a number.

### Architecture

```
eval/
  scenarios/                    # test scenarios (the "queries")
    add-field/
      scenario.yaml             # intent, expected outcome, scoring criteria
      codebase/                 # small fixture codebase (the "before" state)
        hotels/
          models/reservation.py
          services/reservation_service.py
          schemas/reservation.py
          admin.py
          tests/factories.py
      expected/                 # the "after" state (ground truth)
        hotels/
          models/reservation.py
          services/reservation_service.py
          schemas/reservation.py
          admin.py
          tests/factories.py
    add-endpoint/
      scenario.yaml
      codebase/ ...
      expected/ ...
    scaffold-model/
      scenario.yaml
      codebase/ ...
      expected/ ...
  harness/
    run.ts                      # main orchestrator — runs all scenarios × agents × reps
    agents.ts                   # agent invocation layer (claude -p, codex, etc.)
    score.ts                    # scoring engine — diffs actual vs expected
    report.ts                   # aggregation and human-readable output
  results/
    results.jsonl               # append-only trial log (one JSON object per trial)
  log/
    experiments.md              # hypothesis → change → result → surprise journal
  lib/
    sandbox.ts                  # temp directory setup, codebase copying, cleanup
    diff.ts                     # structural diff engine (not just text diff)
    normalize.ts                # whitespace, import order, trailing newline normalization
```

### Scenarios

A scenario is the unit of evaluation. It defines:

1. **A small, self-contained codebase** — the fixture. Just enough code to be realistic (a Django app with 2-3 models, services, views). Small enough that an agent can read the relevant files within a single context window.

2. **A natural language prompt** — the instruction given to the agent. Written the way a developer would actually phrase it: "Add a `loyalty_tier` CharField (max_length=50, nullable) to the Reservation model and propagate it through the stack."

3. **Expected outcomes** — what the codebase should look like after the agent is done. Both the files themselves (for diffing) and structural assertions (for flexible scoring).

```yaml
# eval/scenarios/add-field/scenario.yaml

name: add-field-loyalty-tier
description: Add a nullable CharField to an existing Django model and cascade through service, schema, admin, and tests
tier: medium
category: brownfield-extension

# The prompt given to the agent
prompt: |
  Add a `loyalty_tier` field to the Reservation model in the hotels app.
  It should be a CharField with max_length=50 and nullable.
  Propagate the field through the service layer, request/response schemas,
  admin list_display, and the test factory.
  
  Use jig to make these changes. The jig-django library is installed.
  Run: jig library recipes django — to see available recipes.

# Context given to the agent (prepended to prompt)
context: |
  You are working in a Django project. The `jig` CLI is installed and
  the `jig-django` library is available. Prefer using jig recipes over
  manual code edits for structural changes.

# Which files the agent is expected to modify
expected_files_modified:
  - hotels/models/reservation.py
  - hotels/services/reservation_service.py
  - hotels/schemas/reservation.py
  - hotels/admin.py
  - hotels/tests/factories.py

# Structural assertions (flexible scoring — order-independent, whitespace-tolerant)
assertions:
  - file: hotels/models/reservation.py
    contains: "loyalty_tier = models.CharField(max_length=50, null=True)"
    scope: "class Reservation"
    weight: 1.0

  - file: hotels/services/reservation_service.py
    contains: "loyalty_tier"
    scope: "def create("
    weight: 0.8

  - file: hotels/schemas/reservation.py
    contains: "loyalty_tier"
    scope: "class ReservationCreateRequest"
    weight: 0.8

  - file: hotels/schemas/reservation.py
    contains: "loyalty_tier"
    scope: "class ReservationResponse"
    weight: 0.8

  - file: hotels/admin.py
    contains: "loyalty_tier"
    scope: "list_display"
    weight: 0.6

  - file: hotels/tests/factories.py
    contains: "loyalty_tier"
    scope: "class ReservationFactory"
    weight: 0.6

# Negative assertions (things that should NOT happen)
negative_assertions:
  - file: hotels/models/reservation.py
    not_contains: "loyalty_tier.*loyalty_tier"    # no duplicate field
    description: "Field should not be duplicated"

  - any_file:
    not_contains: "SyntaxError|IndentationError"
    description: "No syntax errors introduced"

# Metadata for analysis
tags: [brownfield, multi-file, django, field-addition, jig-workflow]
estimated_jig_commands: 1   # ideally one workflow call
max_jig_commands: 3         # acceptable if broken into individual recipes
```

### Scenario Tiers

| Tier | Description | Example |
|------|-------------|---------|
| **easy** | Single file, single recipe, obvious variables | "Create a new test file for BookingService" |
| **medium** | Multi-file, workflow or chained recipes, variable extraction from existing code | "Add a field to Reservation and propagate" |
| **hard** | Ambiguous intent, error recovery required, cross-library workflow | "Refactor the room pricing to support seasonal rates" |
| **discovery** | Agent must find the right recipe without being told which one | "Make the Guest model admin-browsable" |
| **error-recovery** | Scenario includes a deliberate obstacle (missing anchor, bad template) | "Add a field to a model with non-standard structure" |

### Scenario Categories

Scenarios are also tagged by what they test about jig's ergonomic surface:

| Category | Tests |
|----------|-------|
| `recipe-discovery` | Can the agent find the right recipe? (`jig library recipes`, `jig vars`) |
| `variable-extraction` | Can the agent read existing code and construct correct `--vars` JSON? |
| `workflow-invocation` | Can the agent call a multi-step workflow correctly? |
| `error-handling` | Can the agent recover from jig errors (missing anchor, bad variables)? |
| `fallback` | Does the agent fall back to manual editing when jig can't handle it? |
| `idempotency` | Does the agent handle "already exists" gracefully? |
| `multi-tool` | Can the agent combine jig with other tools (running tests, formatting)? |

### Agent Invocation

Agents are invoked via CLI subprocess. Each agent gets:

1. A fresh copy of the fixture codebase in a temp directory
2. The scenario prompt (with context prepended)
3. A working `jig` binary on PATH with the relevant library installed
4. A time limit (default: 120 seconds)

```typescript
// eval/harness/agents.ts

interface AgentConfig {
  name: string;
  command: string;           // CLI command to invoke
  args: string[];            // base arguments
  env?: Record<string, string>;
  timeout_ms: number;
  supports_mcp: boolean;     // whether to also test via MCP
}

const AGENTS: AgentConfig[] = [
  {
    name: "claude-code",
    command: "claude",
    args: ["-p", "--output-format", "json", "--max-turns", "50"],
    timeout_ms: 120_000,
    supports_mcp: true,
  },
  {
    name: "claude-code-sonnet",
    command: "claude",
    args: ["-p", "--output-format", "json", "--max-turns", "50", "--model", "sonnet"],
    timeout_ms: 120_000,
    supports_mcp: true,
  },
  {
    name: "codex",
    command: "codex",
    args: ["--approval-mode", "full-auto", "-q"],
    timeout_ms: 120_000,
    supports_mcp: true,
  },
];

async function invokeAgent(
  agent: AgentConfig,
  prompt: string,
  workDir: string,
): Promise<AgentResult> {
  const fullPrompt = prompt;
  const proc = spawn(agent.command, [...agent.args, fullPrompt], {
    cwd: workDir,
    env: { ...process.env, ...agent.env, HOME: process.env.HOME },
    timeout: agent.timeout_ms,
  });

  let stdout = "";
  let stderr = "";
  proc.stdout.on("data", (d) => stdout += d);
  proc.stderr.on("data", (d) => stderr += d);

  const exitCode = await new Promise<number>((resolve) => {
    proc.on("close", resolve);
    proc.on("error", () => resolve(-1));
  });

  return {
    agent: agent.name,
    exitCode,
    stdout,
    stderr,
    durationMs: Date.now() - startTime,
  };
}
```

The key constraint: **agents are invoked exactly how a real user would invoke them.** No special instrumentation inside the agent. No hooks into the agent's decision-making. We observe only inputs (prompt + codebase) and outputs (modified files + exit code). This keeps the eval honest — it measures the actual user experience.

### Scoring

Scoring happens after the agent finishes, by comparing the modified codebase against the expected state. Five dimensions:

#### 1. File Correctness (structural diff)

For each file in `expected_files_modified`, compute a structural similarity score:

```typescript
function scoreFile(actual: string, expected: string): number {
  // Normalize: strip trailing whitespace, normalize newlines, sort imports
  const a = normalize(actual);
  const e = normalize(expected);
  
  if (a === e) return 1.0;
  
  // Line-level Jaccard similarity as fallback
  const aLines = new Set(a.split("\n").map(l => l.trim()).filter(Boolean));
  const eLines = new Set(e.split("\n").map(l => l.trim()).filter(Boolean));
  const intersection = new Set([...aLines].filter(x => eLines.has(x)));
  const union = new Set([...aLines, ...eLines]);
  return intersection.size / union.size;
}
```

This isn't just exact match — it handles the inherent variability of LLM output. The field might be `loyalty_tier = models.CharField(max_length=50, null=True)` or `loyalty_tier = models.CharField(null=True, max_length=50)`. Both are correct. Structural diff catches this; exact text diff doesn't.

#### 2. Assertion Pass Rate

Each assertion in the scenario is checked independently:

```typescript
function scoreAssertions(scenario: Scenario, workDir: string): AssertionResult[] {
  return scenario.assertions.map(a => {
    const content = readFile(path.join(workDir, a.file));
    
    // If scope is specified, extract just that scope
    const region = a.scope ? extractScope(content, a.scope) : content;
    
    const passed = region.includes(a.contains);
    return { assertion: a, passed, weight: a.weight };
  });
}
```

Weighted assertion pass rate: `sum(passed * weight) / sum(weight)`. This is the primary score.

#### 3. Negative Assertion Check

Binary: did the agent introduce anything it shouldn't have? Syntax errors, duplicate fields, broken imports. Any negative assertion failure is a hard penalty.

#### 4. Jig Usage Score

Did the agent actually use jig, or did it bypass the tool and edit files manually?

```typescript
function scoreJigUsage(agentOutput: string, scenario: Scenario): JigUsageScore {
  const jigCalls = extractJigInvocations(agentOutput);
  
  return {
    used_jig: jigCalls.length > 0,
    call_count: jigCalls.length,
    within_expected_range: jigCalls.length >= 1 && jigCalls.length <= scenario.max_jig_commands,
    correct_recipe: jigCalls.some(c => c.recipe === scenario.expected_recipe),
    valid_vars: jigCalls.every(c => isValidJson(c.vars)),
  };
}
```

This dimension is unique to jig's eval. We're not just testing "did the agent produce the right code" — we're testing "did the agent use the tool to produce the right code." An agent that manually edits all six files correctly has achieved the code change but has failed the ergonomic test. The whole point of jig is that the agent shouldn't have to do that.

#### 5. Efficiency Score

How many tool calls, tokens, and seconds did the agent use?

```typescript
function scoreEfficiency(result: AgentResult): EfficiencyScore {
  return {
    duration_ms: result.durationMs,
    tool_calls: countToolCalls(result.stdout),
    jig_calls: countJigCalls(result.stdout),
    tokens_used: extractTokenUsage(result.stdout),  // if available from agent output
  };
}
```

This isn't pass/fail — it's a distribution metric. If jig reduces median tool calls from 24 to 6 across a scenario suite, that's the efficiency story.

#### Composite Score

```typescript
interface TrialScore {
  assertion_score: number;    // 0-1, weighted assertion pass rate
  file_score: number;         // 0-1, mean structural similarity across expected files
  negative_score: number;     // 0 or 1, binary (any negative assertion failure = 0)
  jig_used: boolean;          // did the agent invoke jig at all
  jig_correct: boolean;       // did jig invocations use valid recipes/vars
  total: number;              // composite: assertion_score * negative_score (primary metric)
}
```

### Trial Result Format

Every trial appends one JSON line to `results/results.jsonl`:

```json
{
  "scenario": "add-field-loyalty-tier",
  "agent": "claude-code",
  "rep": 3,
  "timestamp": "2026-04-05T14:32:01.000Z",
  "duration_ms": 18400,
  "scores": {
    "assertion_score": 0.92,
    "file_score": 0.88,
    "negative_score": 1.0,
    "jig_used": true,
    "jig_correct": true,
    "total": 0.92
  },
  "assertions": [
    {"assertion": "models/reservation.py contains loyalty_tier", "passed": true},
    {"assertion": "admin.py contains loyalty_tier in list_display", "passed": false}
  ],
  "jig_invocations": [
    {"command": "jig workflow django/add-field", "vars": "{...}", "exit_code": 0}
  ],
  "agent_exit_code": 0,
  "agent_tool_calls": 8,
  "tags": ["brownfield", "multi-file", "django"]
}
```

### Running the Harness

```bash
# Run all scenarios × all agents × 5 reps
npx tsx eval/harness/run.ts --reps 5

# Single scenario, single agent
npx tsx eval/harness/run.ts --scenario add-field-loyalty-tier --agent claude-code --reps 10

# Only metrics output (for piping to analysis)
npx tsx eval/harness/run.ts --metrics-only

# Dry run (validate scenarios and agents without executing)
npx tsx eval/harness/run.ts --dry-run

# Run with MCP integration (agents use MCP server instead of CLI)
npx tsx eval/harness/run.ts --mode mcp

# Run the baseline: same scenarios but agents are told NOT to use jig
npx tsx eval/harness/run.ts --mode baseline
```

### The Baseline Comparison

The most important comparison: **with jig vs. without jig.**

Every scenario is run in two modes:

1. **`jig` mode** — the prompt tells the agent to use jig, the library is installed, recipes are available.
2. **`baseline` mode** — the prompt gives the same intent ("add a loyalty_tier field to Reservation and propagate") but no mention of jig. The agent uses its native tools (Read, Edit, Write) to make the changes manually.

This isolates jig's value. If agents score 0.95 with jig and 0.72 without jig on the same scenarios, that's a concrete measurement of what the tool provides. If agents score 0.95 both ways, jig isn't helping — back to the drawing board.

The baseline also measures **efficiency**: even if correctness is similar, if jig mode uses 6 tool calls vs. 24 in baseline mode, that's a real improvement in token cost and latency.

### Experiment Loop

The eval system exists to drive design decisions. The loop:

1. **Hypothesis** — "Agents fail to construct `--vars` JSON because the variable types aren't obvious from `jig vars` output. If we add example values to the `jig vars` output, agents will construct valid JSON more often."

2. **Change** — Modify jig's `vars` command to include example values in its JSON output.

3. **Run** — Execute the eval suite. Compare against previous results.

4. **Score** — Did assertion_score or jig_correct improve? Did any scenario regress?

5. **Log** — Record in `log/experiments.md`:
   ```markdown
   ## Experiment 4 — Add example values to `jig vars` output
   **Hypothesis:** Agents fail to construct valid --vars JSON because they lack 
   concrete examples. Adding example values to `jig vars` output should improve
   jig_correct rate.
   **What changed:** Modified `jig vars` to include an `example` field for each
   variable in the JSON output.
   **Result:** jig_correct 0.78→0.91 (+13 pts). assertion_score 0.85→0.89 (+4 pts).
   variable-extraction scenarios improved most. Kept.
   **Surprise:** The biggest improvement was on `discovery` tier scenarios — agents
   used the example values to infer the recipe's purpose, not just its input format.
   ```

6. **Decide** — Keep or revert the change to jig.

### What the Experiments Actually Optimize

The eval loop doesn't optimize jig's template engine or recipe format — those are tested mechanically. It optimizes the **agent-facing surface**:

| Surface | Example experiments |
|---------|-------------------|
| **CLI help text** | Does rewording `jig run --help` change agent success rate? |
| **Error messages** | Does including the rendered content in error output help agents recover? |
| **`jig vars` output** | Do example values help? Do descriptions matter? |
| **MCP tool descriptions** | Does a longer `description` field improve recipe selection? |
| **JSON output structure** | Does flattening the output JSON reduce agent confusion? |
| **Recipe naming** | Does `model/add-field` vs `add-model-field` affect discoverability? |
| **Project instructions** | What CLAUDE.md/AGENTS.md phrasing produces the best jig adoption? |
| **Workflow vs. individual recipes** | Do agents succeed more with one workflow call or multiple recipe calls? |

These are all decisions that feel like bike-shedding in the abstract but have measurable impact on agent success rates. The eval system turns them into science.

### Aggregation and Reporting

```
╔════════════════════════════════════════════╗
║         JIG AGENT EVAL REPORT              ║
╚════════════════════════════════════════════╝

Trials: 450 (15 scenarios × 3 agents × 10 reps)

── Overall ──
  assertion_score:  0.891
  jig_used:         0.847  (381/450 trials)
  jig_correct:      0.793  (357/450 trials)

── By Agent ──
  claude-code:        0.923
  claude-code-sonnet: 0.876
  codex:              0.874

── By Tier ──
  easy:               0.961
  medium:             0.894
  hard:               0.812
  discovery:          0.782
  error-recovery:     0.734

── By Category ──
  recipe-discovery:     0.856
  variable-extraction:  0.812
  workflow-invocation:  0.934
  error-handling:       0.745
  fallback:             0.890

── Baseline Comparison ──
  jig mode:      0.891 (mean assertion_score)
  baseline mode:  0.743 (mean assertion_score)
  delta:          +0.148
  jig tool calls: 2.1 (mean)
  baseline tool calls: 18.7 (mean)

── Weakest Scenarios ──
  error-recovery-bad-anchor    0.520
  discovery-ambiguous-intent   0.610
  hard-seasonal-pricing        0.680

METRIC overall_assertion=0.891
METRIC jig_used_pct=0.847
METRIC baseline_delta=0.148
METRIC claude_code_score=0.923
METRIC easy_score=0.961
METRIC hard_score=0.812
```

The `METRIC` lines are machine-parseable for tracking trends across experiments.

### Holdout Set

To prevent overfitting jig's ergonomics to the eval scenarios:

- **Training scenarios** (12-15): used during the experiment loop, iterated on freely.
- **Holdout scenarios** (5-8): never looked at during iteration. Run periodically (every ~10 experiments) to check generalization.

If training scores rise but holdout scores stall or drop, we're overfitting jig's interface to the specific phrasing of training prompts. Revert to last holdout-validated checkpoint.

### Scenario Fixture Design

Fixtures must be small but realistic. Design principles:

1. **Minimal viable codebase.** 2-3 models, 1 service, 1 view, 1 admin, 1 test factory. Enough structure for multi-file operations; small enough for agents to read in a few tool calls.

2. **Conventions match the library.** The fixture codebase follows jig-django's expected conventions (file paths, class naming, import style). This is what jig is designed for.

3. **One deliberate imperfection per error-recovery scenario.** A non-standard class structure, a missing anchor comment, an unusual import style. Tests whether jig's error messages help agents recover.

4. **Frozen dependencies.** No real Django installation. The fixture is just Python files with the right structure. Agents don't run the code — they edit it. This avoids test flakiness from dependency issues.

5. **Git-initialized.** Each fixture temp dir is `git init`-ed so agents can use `git diff` to review their changes (some agents do this reflexively).

### MCP vs. CLI Mode

Every scenario runs in both modes:

- **CLI mode** — agent invokes `jig run ...` via bash. Tests the raw CLI experience.
- **MCP mode** — agent has the jig MCP server registered. Tests the structured tool experience.

Comparing scores across modes answers: "Is the MCP server actually better than the CLI for agents?" If CLI and MCP score the same, the MCP server adds complexity without value. If MCP scores significantly higher, it validates the investment.

### LLM-as-Judge Validation

Some assertions are hard to express structurally. For these, an LLM-as-judge pass evaluates soft criteria:

```yaml
# In scenario.yaml
llm_judge_criteria:
  - "The generated code follows the existing style conventions of the codebase"
  - "No unnecessary files were created or modified"
  - "The agent's jig invocation used the most appropriate recipe (workflow preferred over individual recipes)"
```

The judge is a separate LLM call (not the agent being tested) that reads the before/after diff and the criteria, then scores each criterion 0-1. This catches quality issues that structural assertions miss — like an agent that produces technically correct code but in a wildly different style than the rest of the codebase.

LLM-as-judge scores are logged alongside structural scores but reported separately. They're useful for diagnosis but too noisy to be the primary metric.

### Cost Tracking

Every trial logs token usage (when available from agent output) and wall-clock time. Aggregated:

```
── Cost Summary ──
  Mean tokens/trial (jig mode):      12,400
  Mean tokens/trial (baseline mode): 38,200
  Mean duration (jig mode):          14.2s
  Mean duration (baseline mode):     41.8s
  Estimated cost/trial (jig mode):   $0.037
  Estimated cost/trial (baseline):   $0.114
```

This is the economic argument for jig: if it reduces token usage by 3x on multi-file changes, it pays for itself immediately at scale.
