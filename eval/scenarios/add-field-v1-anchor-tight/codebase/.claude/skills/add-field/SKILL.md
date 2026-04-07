---
name: add-field
description: Add a new field to a Django model and propagate to admin, serializer, and factory. Use when adding model fields or database columns.
---

# Add Field

Add a field to a Django model and propagate it to admin, serializer, and factory.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "<Model>", "field_name": "<name>", "field_type": "<Type>", "field_args": "<args>", "factory_default": "<default>"}'
```

## Variables

- **model_name**: Model class name (e.g. `Reservation`)
- **field_name**: New field name (e.g. `loyalty_tier`)
- **field_type**: Django field type (e.g. `CharField`)
- **field_args**: Field arguments (e.g. `max_length=20, default="bronze"`)
- **factory_default**: Factory default value (e.g. `"bronze"`)

Extract these from the user's request.

## Gotchas

- If jig exits non-zero, it prints the rendered templates to stderr. Use that output to apply changes manually.
- The recipe uses anchor-based patching. If the anchor pattern doesn't match the actual class definition, jig will fail. Fall back to manual editing using the stderr output.
