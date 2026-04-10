# Recipe Schema Reference

## Top-level structure

```yaml
name: optional-string        # human label for the recipe
description: optional-string  # what pattern this codifies

variables:
  var_name:
    type: string | number | boolean | array | object | enum
    required: true | false    # default: false
    default: any-json-value   # applied before user values
    description: optional     # helps LLMs understand the variable
    values: [a, b, c]         # required when type: enum
    items: string | number    # element type when type: array

files:
  - # one entry per file operation (see operations.md)
```

## Variable types

| Type | JSON shape | Notes |
|------|-----------|-------|
| `string` | `"value"` | Workhorse type. Handles code snippets, paths, names. |
| `number` | `42` or `3.14` | Integer or float. |
| `boolean` | `true`/`false` | Use for feature flags in templates. |
| `array` | `["a","b"]` | If `items` declared, all elements must match that type. |
| `object` | `{"k":"v"}` | Arbitrary nested JSON. |
| `enum` | `"value"` | Must be one of declared `values: [...]`. |

## Variable resolution order

1. `default:` from recipe (lowest priority)
2. `--vars-file` values
3. `--vars-stdin` values
4. `--vars` inline JSON (highest priority)

All are merged at the JSON level. Missing `required: true` variables cause exit code 4.

Extra JSON keys not declared in `variables:` pass through to templates — useful for helper values.

## Minimal examples

### Create a new file

```yaml
variables:
  module:
    type: string
    required: true

files:
  - template: templates/test.py.j2
    to: "tests/test_{{ module | snakecase }}.py"
    skip_if_exists: false
```

### Inject after a line match

```yaml
variables:
  model_name:
    type: string
    required: true
  models_file:
    type: string
    default: models.py

files:
  - template: templates/import_line.py.j2
    inject: "{{ models_file }}"
    after: '^from django\.db import models'
    skip_if: "class {{ model_name }}"
```

### Patch into a class body

```yaml
variables:
  model_name:
    type: string
    required: true
  field_name:
    type: string
    required: true
  field_type:
    type: string
    required: true

files:
  - template: templates/field.py.j2
    patch: "models.py"
    anchor:
      pattern: "^class {{ model_name | regex_escape }}\\("
      scope: class_body
      position: after_last_field
    skip_if: "{{ field_name }}"
```

### Replace between markers

```yaml
variables:
  config_content:
    type: string
    required: true

files:
  - template: templates/config_block.j2
    replace: "config.yaml"
    between:
      start: "# --- BEGIN MANAGED ---"
      end: "# --- END MANAGED ---"
```
