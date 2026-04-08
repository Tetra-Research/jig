---
name: schema-migration-safety
description: Generate two-step rollout-safe Django migrations plus model-field patching using jig.
---

# Schema Migration Safety (Jig)

Use this skill for backwards-compatible model field rollout.

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

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{
  "app_label": "core",
  "model_name": "Entity",
  "field_name": "classification",
  "field_type": "models.CharField",
  "add_field_kwargs": "max_length=20, null=True",
  "final_field_kwargs": "max_length=20",
  "previous_migration": "0007_auto_20260407_1200",
  "add_migration_name": "0008_add_entity_classification",
  "finalize_migration_name": "0009_enforce_entity_classification",
  "backfill_value": "\"standard\""
}'
```

## Notes

- Migration 1 adds nullable field + backfill.
- Migration 2 enforces final constraints.
- `model_file` patching targets the first class in the file. Keep one primary model per file for this recipe.
- If jig fails, use rendered output from stderr and apply manually.
