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
jig run ${CLAUDE_SKILL_DIR:-.claude/skills/query-layer-discipline}/recipe.yaml --vars '{
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
- This eval skill currently targets `class Entity` in `models.py` and `def entity_list` in `views.py`.
- Keep the provided symbol values aligned with that contract; `jig 0.1.0` does not template regex anchor fields.
