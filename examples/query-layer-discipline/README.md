# query-layer-discipline

## What This Does

Introduces a queryset, manager, selector module, and view wiring so read-path logic moves out of the view and into a stable query layer.

## When To Use It

Use this pattern when view-level read logic is starting to drift and you want a consistent queryset plus selector structure.

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
- creates `selectors.py`
- updates `views.py`

## Before / After

- input state lives in `before/`
- expected output state lives in `after/`
