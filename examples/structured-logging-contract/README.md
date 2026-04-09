# structured-logging-contract

## What This Does

Adds stable start and done structured logs to a function while preserving the surrounding return shape.

## When To Use It

Use this pattern when service-layer observability needs a consistent contract instead of ad hoc logging.

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

- updates `services/core_service.py`

## Before / After

- input state lives in `before/`
- expected output state lives in `after/`
