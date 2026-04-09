# schema-migration-safety

## What This Does

Adds a model field plus a rollout-safe two-step migration sequence with a backfill and a follow-up enforcement migration.

## When To Use It

Use this pattern when a new field must be introduced without a risky one-shot not-null migration.

## Run

```bash
mkdir workdir
cp -R before/. workdir/
(
  cd workdir &&
  jig run ../recipe.yaml --vars "$(cat ../vars.json)"
)
```

## Expected Changes

- updates `models.py`
- creates `migrations/0008_add_entity_classification.py`
- creates `migrations/0009_enforce_entity_classification.py`

## Before / After

- input state lives in `before/`
- expected output state lives in `after/`
