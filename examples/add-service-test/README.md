# add-service-test

## What This Does

Generates a deterministic pytest file for a service class with stable act sections and predictable assertions.

## When To Use It

Use this pattern when a service already exists and you want to scaffold repeatable unit tests without hand-writing the same test structure every time.

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

- creates `tests/test_core_service.py`

## Before / After

- input state lives in `before/`
- expected output state lives in `after/`
