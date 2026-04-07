---
name: add-field
description: Add a new field to a Django model and propagate to admin, serializer, and factory. Use when adding model fields or database columns.
---

# Add Field

Add a field to a Django model and propagate it to admin, serializer, and factory.

## Template-First Mode

This variant is template-first. Start by using template files as the source of truth:

- `templates/model_field.j2`
- `templates/admin_field.j2`
- `templates/serializer_field.j2`
- `templates/factory_field.j2`
- `templates/vars.tmpl.json`

Map the user request into the vars shape, then apply the corresponding edits directly.

## Variables

- **model_name**: Model class name (e.g. `Reservation`)
- **field_name**: New field name (e.g. `loyalty_tier`)
- **field_type**: Django field type (e.g. `CharField`)
- **field_args**: Field arguments (e.g. `max_length=20, default=\"bronze\"`)
- **factory_default**: Factory default value (e.g. `\"bronze\"`)

## Rendered Example For This Task

Given:

- `model_name=Reservation`
- `field_name=loyalty_tier`
- `field_type=CharField`
- `field_args=max_length=20, default="bronze"`
- `factory_default="bronze"`

Render to these exact line shapes:

- model line: `    loyalty_tier = models.CharField(max_length=20, default="bronze")`
- admin list_display entry: `        "loyalty_tier",`
- serializer fields entry: `            "loyalty_tier",`
- factory line: `    loyalty_tier = "bronze"`

## Gotchas

- Keep variable normalization consistent with `templates/vars.tmpl.json`.
- Keep indentation exactly as shown by each template snippet.
- Add only the required field propagation edits; avoid unrelated refactors.
