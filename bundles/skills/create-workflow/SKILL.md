---
name: create-workflow
description: When the user needs to chain multiple jig recipes into a multi-step workflow with conditional execution and shared variables. Use when a pattern spans multiple files that need coordinated creation and modification, like scaffolding an entire endpoint or a full feature slice.
allowed-tools: Read Write Edit Glob Grep Bash
---

# Create a jig workflow

A jig workflow chains multiple recipes into a sequential pipeline. Each step runs one recipe, optionally gated by a condition, with variables mapped from the workflow scope.

## When to use this vs. a single recipe

- **Single recipe**: pattern touches 1-2 files, all operations share the same variables
- **Workflow**: pattern creates or modifies 3+ files in a coordinated way, steps have different variable needs, or some steps are conditional

## Step 1: Decompose the pattern

Break the end-to-end pattern into discrete steps. Each step = one recipe.

Example — scaffolding a new API endpoint:
1. Create the request/response schema file
2. Create the handler file
3. Inject the import into the router index
4. Inject the route registration

Each step is independently valid — it can be run alone with `jig run`.

## Step 2: Design workflow variables

Workflow variables are the **union** of everything all steps need, plus any workflow-level controls (like conditionals).

```yaml
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
  include_tests:
    type: boolean
    default: true
```

Rules:
- Don't duplicate what filters can derive. If every step needs `route_name` in snake_case, use `{{ route_name | snakecase }}` in templates — don't add a `route_name_snake` variable.
- Compute ALL derived values up front in `--vars`. Steps cannot mutate variables for downstream steps.

## Step 3: Write the workflow YAML

```yaml
name: create-endpoint
description: Scaffold a complete API endpoint with schema, handler, and routing

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
  include_tests:
    type: boolean
    default: true

steps:
  - recipe: step1-schema/recipe.yaml

  - recipe: step2-handler/recipe.yaml

  - recipe: step3-import/recipe.yaml
    vars_map:
      route_name: route_module    # maps workflow's route_name → recipe's route_module

  - recipe: step4-register/recipe.yaml

  - recipe: step5-tests/recipe.yaml
    when: "{{ include_tests }}"
    on_error: continue
```

See `${CLAUDE_SKILL_DIR}/references/workflow-schema.md` for the full schema.

## Step 4: Organize files

```
my-workflow/
  workflow.yaml
  step1-schema/
    recipe.yaml
    templates/
  step2-handler/
    recipe.yaml
    templates/
  step3-import/
    recipe.yaml
    templates/
  step4-register/
    recipe.yaml
    templates/
  step5-tests/
    recipe.yaml
    templates/
```

Each step directory is a standalone recipe with its own `recipe.yaml` and `templates/`.

## Step 5: Validate

```bash
# Validate each step recipe independently
jig validate path/to/step1-schema/recipe.yaml
jig validate path/to/step2-handler/recipe.yaml

# Dry-run the full workflow
jig workflow path/to/workflow.yaml --vars '{"...":"..."}' --dry-run --verbose --json
```

## Gotchas

- **Workflows pass the SAME variable dict to every step.** A step cannot compute or mutate a variable for downstream steps. Pre-compute everything.
- **`vars_map` direction:** keys are the step recipe's variable names, values are the workflow variable names. `{recipe_var: workflow_var}`.
- **`when` is a rendered Jinja expression.** Empty string, `"false"`, or `"0"` = skip. Everything else = run.
- **`on_error: stop`** (default) halts the entire workflow on first failure. Use `continue` for optional steps, `report` to log and include errors in output.
- **Step order matters for virtual file state.** If step 1 creates a file and step 2 injects into it, that works within a workflow. But each step is a full `jig run` with its own virtual file state — virtual files do NOT carry across steps. The file must be written to disk by step 1 before step 2 can read it.
- **Each step recipe must independently declare its variables.** Workflow variables are passed through, but the recipe still validates its own `required:` and `type:` constraints.
- **Don't make one big recipe when you mean a workflow.** If a recipe has 8+ operations touching 4+ different files, it should probably be a workflow with focused steps.
