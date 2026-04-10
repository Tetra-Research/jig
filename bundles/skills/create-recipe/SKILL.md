---
name: create-recipe
description: When the user wants to codify a repeatable code generation pattern into a jig recipe — design variables, choose operations (create/inject/patch/replace), write Jinja templates, and validate the result. Use when someone says "I keep writing this same boilerplate", "make a recipe for X", or wants to turn a manual edit into an automated pattern.
allowed-tools: Read Write Edit Glob Grep Bash
---

# Create a jig recipe

A jig recipe turns a repeatable code edit into a deterministic, idempotent operation. It declares typed variables and file operations that render Jinja templates against those variables. Same recipe + same variables + same files = same output, every time.

## When to use this

- The user has done the same edit 2+ times and will do it again
- A team wants to standardize how a pattern is implemented across a codebase
- An LLM keeps hand-writing the same boilerplate and getting it slightly wrong each time

## Step 1: Identify the pattern

Ask the user to show you 2-3 examples of the pattern done well. Read them. Extract:

- **What files are created or modified?** Each becomes a file operation.
- **What varies between instances?** These become recipe variables.
- **What stays constant?** This becomes template content.
- **What makes each instance unique?** This informs anchors and `skip_if` strings.

Do not search the codebase for examples on your own — the user curates what "good" looks like.

## Step 2: Design variables

See `${CLAUDE_SKILL_DIR}/references/recipe-schema.md` for the full schema.

Rules:
- Only declare what actually varies. Derive everything else with filters (`snakecase`, `pluralize`, `pascalcase`, etc.)
- Use `required: true` sparingly — provide `default:` for conventional paths (e.g., `models.py`, `views.py`)
- `string` handles 90% of cases, even for code-shaped values like `"max_length=20, null=True"`
- Use `enum` for closed sets of 3-5 valid choices
- Never accept two variables when one + a filter gives you both (e.g., `model_name` + `snakecase` instead of `model_name` + `model_name_snake`)

## Step 3: Choose operations

For each file the pattern touches, pick one:

| Need | Operation | Key fields |
|------|-----------|-----------|
| New file | **create** | `template`, `to`, `skip_if_exists` |
| Insert at a line match | **inject** | `template`, `inject`, `after`/`before`/`append`/`prepend`, `skip_if` |
| Insert into a structural region | **patch** | `template`, `patch`, `anchor: {pattern, scope, position}`, `skip_if` |
| Replace a known block | **replace** | `template`, `replace`, `between: {start, end}` or `pattern`, `fallback` |

If "where to insert" is a line → **inject**. If it's "inside this class, after the last field" → **patch**. If you're swapping a delimited region → **replace**.

See `${CLAUDE_SKILL_DIR}/references/operations.md` for detailed guidance with real examples.

## Step 4: Design anchors (for patch)

This is the hardest part. Bad anchors break silently.

- **Pipe variables through `regex_escape`:** `"^class {{ model_name | regex_escape }}\\("`
- **Use the smallest scope** that contains your insertion point
- **Prefer structural positions** that survive reordering (`after_last_field` over `after` a specific field name)

See `${CLAUDE_SKILL_DIR}/references/anchor-guide.md` for scope types, position types, and real examples from existing recipes.

## Step 5: Write templates

Templates use Jinja2 (minijinja). Prefer simple interpolation + case filters. Avoid complex conditionals — encode complexity in variables, not templates.

Available filters: `snakecase`, `camelcase`, `pascalcase`, `kebabcase`, `upper`, `lower`, `capitalize`, `pluralize`, `singularize`, `quote`, `indent(width)`, `replace(from, to)`, `regex_escape`, `join(sep)`.

Template location: place templates in a `templates/` directory alongside `recipe.yaml`. Reference them as `templates/filename.j2` in the recipe.

## Step 6: Make it idempotent

Every operation must be safe to re-run:

- **create** + `skip_if_exists: true` for scaffolding (header files, base modules)
- **create** + `skip_if_exists: false` for files that should regenerate (tests, migrations)
- **inject/patch** + `skip_if: "<unique substring>"` — pick a string from the rendered output that only exists if the operation already ran. Function names and class names work well. Comments don't (they get edited out).

## Step 7: Validate

```bash
jig validate path/to/recipe.yaml                                    # schema check
jig vars path/to/recipe.yaml                                         # confirm variable surface
jig run path/to/recipe.yaml --vars '...' --dry-run --verbose --json  # preview output
```

In `--verbose` output, check `position_fallback` on patch operations — if it's set, the position couldn't resolve cleanly (e.g., `after_last_field` found no fields and fell back to `before_close`).

## Gotchas

- **Anchor patterns are real regex.** Dots, parens, brackets need manual escaping in the static parts of the pattern (`\\.` for a literal dot). The `regex_escape` filter handles interpolated variables.
- **`skip_if` is a literal substring, NOT regex.** Whitespace and casing count.
- **Templates render once with the variables you pass.** They cannot inspect the target file at render time. Encode file-dependent decisions in variables before invoking.
- **Operation order within a recipe matters.** A `create` followed by an `inject` into the same file works — jig holds virtual file state across operations in one run. Splitting across two `jig run` calls breaks this.
- **Trailing newlines matter.** A template without a trailing newline merges its last line with the next file line on inject.
- **Variable values are raw strings.** `"max_length=20, null=True"` is dropped verbatim into the template. jig does not parse or validate code in variable values — the author is responsible for syntax correctness.
- **`position: after_last_field` falls back to `before_close`** when no field patterns match. Always check verbose output on the first dry-run.
- **Three similar lines > one abstraction.** If a recipe needs 11 enum variables to handle different cases, it's actually 3 recipes in a trench coat. Split it.
