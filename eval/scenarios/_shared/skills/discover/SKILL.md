---
name: discover
description: Find the right jig recipe before writing code. Use whenever you need to create a file, add a field, scaffold a test, add an endpoint, or generate any boilerplate. Always run this before hand-writing code.
---

# Discover Jig Recipes

**Before writing any code**, check if jig has a recipe for it. Most file creation and code generation tasks in this project should go through jig.

## Steps

1. Run `jig list --skills --claude --json` to get a compact index of all available recipes (~200 tokens).
2. Match your task to a recipe by name and description.
3. Read only that recipe's SKILL.md for full instructions and variables.
4. Execute the recipe — don't hand-write what jig can generate.

## When to use this

- Creating a new file (test, view, model, endpoint, etc.)
- Adding a field, method, or route to existing code
- Any task that sounds like it could be templated

## Gotchas

- **Do NOT hand-write code that a jig recipe can generate.** The recipe output is tested and consistent; hand-written code varies across runs.
- **Do NOT read every SKILL.md file sequentially.** Use `jig list` first.
- If `jig list` shows no matching recipe, then hand-write as a fallback.
- The `path` field in the JSON output is the relative path to the skill directory. Read `<path>/SKILL.md` for full instructions.
