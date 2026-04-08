---
name: query-layer-discipline
description: Scaffold queryset/manager/selector layers and wire view reads through selector entrypoints with jig.
---

# Query Layer Discipline (Jig)

Use this skill to standardize read paths through QuerySet + selector layers.

## Required Variables

- `model_name`
- `queryset_name`
- `manager_name`
- `selector_name`
- `selector_file`
- `view_name`

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{
  "model_name": "Entity",
  "queryset_name": "EntityQuerySet",
  "manager_name": "EntityManager",
  "selector_name": "select_active_entities",
  "selector_file": "selectors.py",
  "view_name": "entity_list"
}'
```

If jig fails, use rendered snippets from stderr as manual patch guidance.

Notes:
- The recipe anchors to the first class in `models_file` and first function in `views_file`.
- Use focused files when running this skill so anchors are deterministic.
