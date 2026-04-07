---
name: add-field
description: Add a new field to a Django model and propagate to admin, serializer, and factory. Use when adding model fields or database columns.
---

# Add Field

Add a field to a Django model and propagate it to admin, serializer, and factory.

## Inputs

Prefer structured arguments from `$ARGUMENTS` when available.

Accepted forms:

1. JSON object with keys:
   - `model_name`
   - `field_name`
   - `field_type`
   - `field_args`
   - `factory_default`
2. Positional args:
   - `<model_name> <field_name> <field_type> [field_args] [factory_default]`

Normalization rules:

- `field_args` should be raw Django field args without wrapping parentheses.
  Example: `max_length=20, default="bronze"`
- `factory_default` should be a Python literal rendered into the factory line.
  For string defaults, include quotes in the value (example: `"bronze"`).
- If optional args are omitted, use:
  - `field_args: ""`
  - `factory_default: "'test'"`

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "<Model>", "field_name": "<name>", "field_type": "<Type>", "field_args": "<args>", "factory_default": "<default>"}'
```

If invoked as `/add-field <model_name> <field_name> <field_type>`:

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "$1", "field_name": "$2", "field_type": "$3", "field_args": "", "factory_default": "\"test\""}'
```

## Variables

- **model_name**: Model class name (e.g. `Reservation`)
- **field_name**: New field name (e.g. `loyalty_tier`)
- **field_type**: Django field type (e.g. `CharField`)
- **field_args**: Field arguments (e.g. `max_length=20, default="bronze"`)
- **factory_default**: Factory default value (e.g. `"bronze"`)

Extract these from the user's request.

## Gotchas

- If unsure about required variables, run:
  - `jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml`
- If jig exits non-zero, it prints rendered output to stderr. Use that output for one manual correction pass.
