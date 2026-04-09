# view-contract-enforcer

## What This Does

Adds a request/response contract view plus schema, URL, and test wiring for a stable API boundary.

## When To Use It

Use this pattern when a new endpoint should follow an established contract shape instead of open-coded request handling.

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

- updates `views.py`
- updates `schemas.py`
- updates `urls.py`
- updates `tests/test_views.py`

## Before / After

- input state lives in `before/`
- expected output state lives in `after/`
