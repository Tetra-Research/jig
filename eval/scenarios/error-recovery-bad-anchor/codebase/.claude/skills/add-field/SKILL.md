---
name: add-field
description: Add a new field to a Django model. Use when adding model fields or database columns.
---

# Add Field

Add a field to a Django model using anchor-based patching.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "<Model>", "field_name": "<name>", "field_type": "<Type>", "field_args": "<args>"}'
```

## Variables

- **model_name**: Model class name (e.g. `Contact`)
- **field_name**: New field name (e.g. `phone_number`)
- **field_type**: Django field type (e.g. `CharField`)
- **field_args**: Field arguments (e.g. `max_length=20`)

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
- The recipe expects the model to extend `models.Model`. If it doesn't, the anchor won't match and jig will fail. Use the stderr output to apply manually.
