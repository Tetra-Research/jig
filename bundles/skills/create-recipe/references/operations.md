# Operations Reference

## create

Creates a new file from a rendered template.

```yaml
- template: templates/model.ts.j2
  to: "src/models/{{ model_name | snakecase }}.ts"
  skip_if_exists: true
```

**Fields:**
- `template` (required): path to Jinja template, relative to recipe directory
- `to` (required): output file path (Jinja-rendered, so variables work in paths)
- `skip_if_exists` (optional, default: false): if true, skip when the file already exists

**Behavior:**
- Creates parent directories automatically
- `skip_if_exists: false` errors if file exists (unless `--force`)
- In a multi-operation recipe, created files are available to subsequent operations via virtual file state

**Use for:** new files — models, tests, migrations, config files, modules.

---

## inject

Inserts rendered content at a line-matched position in an existing file.

```yaml
- template: templates/import_line.py.j2
  inject: "{{ target_file }}"
  after: '^(from|import) '
  at: last
  skip_if: 'from .selectors import'
```

**Fields:**
- `template` (required): Jinja template
- `inject` (required): target file path
- `after` (string, regex): insert after the matching line
- `before` (string, regex): insert before the matching line
- `prepend` (bool): insert at file start
- `append` (bool): insert at file end
- `at` (optional): `first` (default) or `last` — which match to use when multiple lines match
- `skip_if` (string, literal): skip if this substring exists in the file

**Behavior:**
- Only ONE of `after`/`before`/`prepend`/`append` per operation
- `at: last` is critical for import blocks — finds the last import line, not the first
- `skip_if` is a literal substring search, NOT regex

**Use for:** adding imports, registering routes, appending config entries — any insertion where the position is defined by a line match.

---

## patch

Inserts rendered content into a structural region of an existing file. This is the sophisticated operation — it combines regex anchoring with scope detection and positional insertion.

```yaml
- template: templates/field.py.j2
  patch: "models.py"
  anchor:
    pattern: "^class {{ model_name | regex_escape }}\\("
    scope: class_body
    position: after_last_field
  skip_if: "{{ field_name }} ="
```

**Fields:**
- `template` (required): Jinja template
- `patch` (required): target file path
- `anchor` (required): object with:
  - `pattern` (required, regex): finds the anchor line in the file
  - `scope` (required): structural region around the anchor (see anchor-guide.md)
  - `position` (required): where within the scope to insert (see anchor-guide.md)
  - `find` (optional, string): narrow scope by searching for a substring within it
- `skip_if` (string, literal): skip if this substring exists in the file

**Behavior:**
- Anchor pattern matches the FIRST matching line in the file
- Scope detection determines the structural extent (indent-based for Python/YAML, delimiter-based for braces/parens)
- Position resolves to an exact insertion line within the scope
- Rendered content is re-indented to match the target context
- If `position` can't resolve (e.g., `after_last_field` with no fields), falls back to `before_close`

**Use for:** adding fields to classes, methods to services, entries to object literals, parameters to function signatures — any insertion where position depends on code structure, not just a line match.

---

## replace

Replaces a region of an existing file with rendered content.

```yaml
- template: templates/updated_config.j2
  replace: "config.yaml"
  between:
    start: "# --- BEGIN MANAGED ---"
    end: "# --- END MANAGED ---"
  fallback: append
```

**Fields:**
- `template` (required): Jinja template
- `replace` (required): target file path
- `between` (object): replace content between two marker lines
  - `start` (regex): start marker (preserved in output)
  - `end` (regex): end marker (preserved in output)
- `pattern` (regex): alternative to `between` — replace all matching lines (inclusive)
- `fallback` (optional): `error` (default), `append`, `prepend`, `skip` — what to do when the pattern/markers aren't found

**Behavior:**
- With `between:`, the start and end marker lines are preserved — only the content between them is replaced
- With `pattern:`, all contiguous matching lines are replaced
- `fallback: append` is useful for first-run when markers don't exist yet

**Use for:** regenerating managed config blocks, updating version strings, replacing generated sections between markers.
