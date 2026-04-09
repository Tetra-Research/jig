---
name: schema-migration-safety
description: Plan and apply backwards-compatible two-step schema changes for Django migrations without using jig.
---

# Schema Migration Safety (Control)

Use this skill for model field changes that need rollout-safe migrations.
Implement the file edits directly. Do not return a checklist-only response.

## Required Variables

- `app_label`
- `model_name`
- `field_name`
- `field_type`
- `add_field_kwargs`
- `final_field_kwargs`
- `previous_migration`
- `add_migration_name`
- `finalize_migration_name`
- `backfill_value`

## Required Output Contract

1. Update `models.py` so `class Entity(models.Model)` contains `classification = models.CharField(max_length=20)` as the final model field shape.
2. Create the first migration file using the provided `add_migration_name`.
3. In the first migration:
- import `migrations` and `models`
- define `backfill_classification(apps, schema_editor)`
- use `model_cls = apps.get_model(app_label, model_name)` with the provided variable values
- use `db_alias = schema_editor.connection.alias`
- iterate `for row in model_cls.objects.using(db_alias).all().only("id", field_name):` with the provided field name literal
- when the field is `None` or `""`, set it to the provided backfill value and call `row.save(update_fields=["{{ field_name }}"])`
- declare `migrations.AddField(...)` with the safe first-state kwargs from the variables
- declare `migrations.RunPython(backfill_classification, migrations.RunPython.noop)`
4. Create the second migration file using the provided `finalize_migration_name`.
5. In the second migration:
- depend on the first migration by its provided `add_migration_name`
- declare `migrations.AlterField(...)` with the final field kwargs from the variables
6. Keep the migration chain linear and use the exact migration filenames provided in the variables.

## Guardrails

- Avoid one-shot `NOT NULL` additions on large existing tables.
- Avoid data backfill in request-time code paths.
- Keep backfill logic idempotent.
- Do not write a custom reverse-backfill function; use `migrations.RunPython.noop`.
