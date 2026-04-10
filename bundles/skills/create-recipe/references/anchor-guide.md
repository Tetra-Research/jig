# Anchor, Scope, and Position Guide

Anchors are the targeting system for `patch` operations. They combine three concepts:

1. **Pattern** — regex that finds the anchor line
2. **Scope** — structural region around the anchor
3. **Position** — where within the scope to insert

## Patterns

Patterns are standard regex matched against each line in the file. The first matching line wins.

**Rules:**
- Always pipe interpolated variables through `regex_escape`: `{{ model_name | regex_escape }}`
- Anchor to distinctive syntax — class declarations, function signatures, export statements
- Start patterns with `^` when anchoring to line beginnings

**Good patterns:**
```yaml
pattern: "^class {{ model_name | regex_escape }}\\("          # Python class
pattern: "^def {{ function_name | regex_escape }}\\("          # Python function
pattern: "^export const {{ symbol | regex_escape }} = z\\.object\\("  # Zod schema
pattern: "^router\\."                                          # Express router
```

**Bad patterns:**
```yaml
pattern: "class"           # matches every class in the file
pattern: "{{ name }}"      # unescaped — dots/parens in name break regex
pattern: "def.*:"          # too broad, matches many functions
```

## Scope types

| Scope | Detection | Use when |
|-------|-----------|----------|
| `line` | Just the anchor line | Single-line edits |
| `block` | Indent-based (deeper-indented lines after anchor) | Generic indented block |
| `class_body` | Indent-based (body of class declaration) | Adding fields, methods to a class |
| `function_body` | Indent-based (body of function/method) | Adding statements to a function |
| `function_signature` | Balanced `(...)` after anchor | Adding parameters |
| `braces` | Balanced `{...}` | Object literals, JS/TS blocks |
| `brackets` | Balanced `[...]` | Array literals |
| `parens` | Balanced `(...)` | Function calls, tuples |

**Indent-based scopes** (`block`, `class_body`, `function_body`): best for Python, YAML, and any language where structure is conveyed by indentation. The scope extends from the anchor to the last deeper-indented line.

**Delimiter-based scopes** (`braces`, `brackets`, `parens`, `function_signature`): best for C-family languages (JS, TS, Rust, Go, Java). The scope tracks nesting depth and handles string literals and comments.

## Position types

| Position | Inserts at | Best for |
|----------|-----------|----------|
| `before` | Start of scope | Prepending to a block |
| `after` | End of scope | Appending to a block |
| `before_close` | Before closing delimiter | Adding entries to object/array literals |
| `after_last_field` | After last `\w+\s*[:=]` pattern | Adding fields to structs/classes |
| `after_last_method` | After last `fn`/`def` body | Adding methods to classes |
| `after_last_import` | After last `from`/`import` line | Adding imports |
| `sorted` | Alphabetically sorted by first line | Maintaining sorted lists |

**Fallback behavior:** if a position can't resolve (e.g., `after_last_field` in an empty class), it falls back to `before_close`. Check `position_fallback` in `--verbose` output.

## The `find` field

Narrows the scope by searching for a substring within it. If the anchor scope is an entire class and you only want to insert near a specific attribute, `find` lets you zero in:

```yaml
anchor:
  pattern: "^class {{ model_name | regex_escape }}\\("
  scope: class_body
  find: "objects ="
  position: after
```

This finds the class body, then within it finds the `objects =` line, and inserts after it.

## Real-world examples

### Add a field to a Python model class

```yaml
anchor:
  pattern: "^class {{ model_name | regex_escape }}\\("
  scope: class_body
  position: after_last_field
```
Finds the class declaration, scopes to its indented body, inserts after the last assignment-like line.

### Add a field to a Zod schema (TypeScript)

```yaml
anchor:
  pattern: "^export const {{ schema_symbol | regex_escape }} = z\\.object\\("
  scope: braces
  position: before_close
```
Finds the schema declaration, scopes to the `{...}` block, inserts before `})`.

### Add a log statement at the start of a function

```yaml
anchor:
  pattern: "^def {{ function_name | regex_escape }}\\("
  scope: function_body
  position: before
```
Finds the function declaration, scopes to its body, inserts at the very beginning.

### Add a parameter to a function signature

```yaml
anchor:
  pattern: "^def {{ function_name | regex_escape }}\\("
  scope: parens
  position: before_close
```
Finds the function, scopes to the `(...)` parameter list, inserts before the closing paren.

### Add a method to a class (after existing methods)

```yaml
anchor:
  pattern: "^class {{ class_name | regex_escape }}\\("
  scope: class_body
  position: after_last_method
```
Finds the class, scopes to its body, inserts after the last method's full body.

## Indentation handling

Patch operations automatically adjust indentation:
1. The template's base indent is detected from its first non-empty line
2. All lines are shifted relative to the target scope's indent level
3. You don't need to manually indent templates to match the target file

Write templates at their natural indentation. jig adjusts on insertion.
