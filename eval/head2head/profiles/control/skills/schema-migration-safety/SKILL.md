---
name: schema-migration-safety
description: Plan and apply backwards-compatible two-step schema changes for Django migrations without using jig.
---

# Schema Migration Safety (Control)

Use this skill for model field changes that need rollout-safe migrations.

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

## Execution Checklist

1. Add the new model field in a safe first state (nullable or with safe default).
2. Create migration 1 to add the field and backfill existing rows.
3. Create migration 2 to enforce the final constraint shape.
4. Keep changes narrow: no unrelated refactors.
5. Verify migration dependency chain is linear and explicit.

## Guardrails

- Avoid one-shot `NOT NULL` additions on large existing tables.
- Avoid data backfill in request-time code paths.
- Keep backfill logic idempotent.
- Preserve reversibility with clear migration operations.
