---
name: add-field
description: Add a new field to a Django model and propagate to admin, serializer, and factory. Use when adding model fields or database columns.
---

# Add Field

Add a field to a Django model and propagate it to admin, serializer, and factory.

## Inputs

Prefer bash-style argument mapping when arguments are available.

Accepted forms:

1. Positional args:
   - `<model_name> <field_name> <field_type> [field_args] [factory_default]`
2. JSON object keys:
   - `model_name`, `field_name`, `field_type`, `field_args`, `factory_default`

## Usage (Bash Style)

Map values into shell variables, then run `jig run` once:

```bash
MODEL_NAME="$1"
FIELD_NAME="$2"
FIELD_TYPE="$3"
FIELD_ARGS="${4:-max_length=20, default=\"bronze\"}"
FACTORY_DEFAULT="${5:-\"bronze\"}"

jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --json-args "{\"model_name\":\"${MODEL_NAME}\",\"field_name\":\"${FIELD_NAME}\",\"field_type\":\"${FIELD_TYPE}\",\"field_args\":\"${FIELD_ARGS}\",\"factory_default\":\"${FACTORY_DEFAULT}\"}"
```

If values come directly from the prompt, assign them first using the same variable names and run the same command.

## Variables

- **model_name**: Model class name (e.g. `Reservation`)
- **field_name**: New field name (e.g. `loyalty_tier`)
- **field_type**: Django field type (e.g. `CharField`)
- **field_args**: Field arguments (e.g. `max_length=20, default=\"bronze\"`)
- **factory_default**: Factory default value (e.g. `\"bronze\"`)

## Gotchas

- Run exactly one `jig run` command first.
- If unsure about required variables, run: `jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml`
- If jig exits non-zero, use stderr rendered output for one manual correction pass.
