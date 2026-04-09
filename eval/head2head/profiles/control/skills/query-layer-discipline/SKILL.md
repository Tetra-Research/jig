---
name: query-layer-discipline
description: Keep reads in selectors/querysets and keep write logic in services without using jig.
---

# Query Layer Discipline (Control)

Use this skill when introducing new read patterns.
Implement the file edits directly. Do not return a checklist-only response.

## Required Variables

- `model_name`
- `queryset_name`
- `manager_name`
- `selector_name`
- `selector_file`
- `view_name`

## Required Output Contract

1. Modify `models.py`, `selectors.py`, and `views.py`.
2. In `models.py`, define `class EntityQuerySet(models.QuerySet)` with an `active(self)` method returning `self.filter(status="active")`.
3. In `models.py`, define `class EntityManager(models.Manager)` with:
- `get_queryset(self)` returning `EntityQuerySet(self.model, using=self._db)`
- `active(self)` returning `self.get_queryset().active()`
4. In `class Entity(models.Model)`, assign `objects = EntityManager()` before the field declarations.
5. In `selectors.py`, import `Entity` from `.models`.
6. In `selectors.py`, define `def select_active_entities():` returning a parenthesized chained expression:
- `Entity.objects.active()`
- `.select_related()`
7. In `views.py`, import `select_active_entities` from `.selectors`.
8. In `entity_list`, assign `records = select_active_entities()` and preserve `return {"results": []}`.

## Guardrails

- Avoid query logic scattered in views.
- Use the exact identifiers provided in the required variables.
- Do not introduce additional selector helpers or unrelated view changes.
