# Workflow Schema Reference

## Top-level structure

```yaml
name: optional-string
description: optional-string

variables:
  var_name:
    type: string | number | boolean | array | object | enum
    required: true | false
    default: any-json-value
    description: optional
    values: [...]    # for enum
    items: string    # for array

steps:
  - recipe: path/to/recipe.yaml
    when: "{{ jinja_expression }}"     # optional condition
    vars_map:                           # optional variable mapping
      recipe_var: workflow_var
    vars:                               # optional inline overrides
      extra_var: "value"
    on_error: stop | continue | report  # optional, default: stop

on_error: stop | continue | report      # workflow-level default
```

## Steps

Steps are executed sequentially in declaration order. Each step runs one recipe with `jig run`.

### `recipe` (required)

Path to a recipe YAML, relative to the workflow file's directory.

### `when` (optional)

A Jinja expression rendered with the workflow's variable context. The step runs if the result is truthy (non-empty, not `"false"`, not `"0"`).

```yaml
when: "{{ include_tests }}"
when: "{{ framework == 'express' }}"
when: "{{ fields | length > 0 }}"
```

### `vars_map` (optional)

Maps workflow variable names to recipe variable names when they differ. Keys are the recipe's variable names, values are the workflow's variable names.

```yaml
# Workflow has `route_name`, recipe expects `module_name`
vars_map:
  module_name: route_name
```

Without `vars_map`, workflow variables are passed through by name.

### `vars` (optional)

Inline variable overrides for this step. These take highest precedence for the step.

```yaml
vars:
  output_dir: "tests/"
  verbose: true
```

### `on_error` (optional)

What to do when this step fails:

- `stop` (default) — halt the entire workflow immediately
- `continue` — log the error, proceed to next step
- `report` — log the error, include in final output, proceed

## Variable flow

```
workflow --vars JSON
  │
  ├─ workflow.variables (validation + defaults)
  │
  ├─ step.vars_map (rename)
  ├─ step.vars (override)
  │
  └─ recipe.variables (validation + defaults)
      │
      └─ template rendering
```

All steps see the original workflow variables. Steps cannot produce outputs or modify the shared variable context.

## Invocation

```bash
# Run a workflow
jig workflow path/to/workflow.yaml --vars '{"key":"value"}'

# With a vars file
jig workflow path/to/workflow.yaml --vars-file vars.json

# Dry-run
jig workflow path/to/workflow.yaml --vars '...' --dry-run --json

# Verbose (includes rendered_content and scope diagnostics)
jig workflow path/to/workflow.yaml --vars '...' --verbose --json
```

## Output shape

```json
{
  "dry_run": false,
  "steps": [
    {
      "recipe": "step1/recipe.yaml",
      "status": "ok",
      "operations": [
        {"action": "create", "path": "...", "lines": 30}
      ]
    },
    {
      "recipe": "step2/recipe.yaml",
      "status": "skipped",
      "reason": "when condition evaluated to false"
    },
    {
      "recipe": "step3/recipe.yaml",
      "status": "error",
      "error": {
        "what": "...",
        "where": "...",
        "why": "...",
        "hint": "...",
        "rendered_content": "..."
      }
    }
  ]
}
```

Each step reports its own status. On error, `rendered_content` is included for manual fallback.

## Example: Full endpoint scaffold

```yaml
name: create-endpoint
description: Create schema, handler, import, and route registration

variables:
  route_name:
    type: string
    required: true
  route_path:
    type: string
    required: true
  handler_name:
    type: string
    required: true
  request_schema_name:
    type: string
    required: true
  success_message:
    type: string
    default: "ok"

steps:
  - recipe: schema/recipe.yaml
  - recipe: handler/recipe.yaml
  - recipe: import/recipe.yaml
  - recipe: register/recipe.yaml
```

Invoked as:

```bash
jig workflow create-endpoint/workflow.yaml --vars '{
  "route_name": "projects",
  "route_path": "/projects",
  "handler_name": "createProjectHandler",
  "request_schema_name": "createProjectSchema",
  "success_message": "project created"
}'
```
