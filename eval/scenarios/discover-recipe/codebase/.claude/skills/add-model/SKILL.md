---
name: add-model
description: Create a new Django model in a module. Use when adding models, database tables, or data classes.
---

# Add Model

Create a new Django model class in a module's models.py.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"module": "<module>", "model_name": "<ModelName>", "fields": "<field_lines>"}'
```

## Variables

- **module**: Module path (e.g. `auth`)
- **model_name**: Model class name (e.g. `Permission`)
- **fields**: Model fields as Python code lines (e.g. `name = models.CharField(max_length=100)`)

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
- The `fields` variable is raw Python — include proper indentation and field definitions.
