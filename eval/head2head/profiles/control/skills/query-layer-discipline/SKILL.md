---
name: query-layer-discipline
description: Keep reads in selectors/querysets and keep write logic in services without using jig.
---

# Query Layer Discipline (Control)

Use this skill when introducing new read patterns.

## Required Variables

- `model_name`
- `queryset_name`
- `manager_name`
- `selector_name`
- `selector_file`
- `view_name`

## Execution Checklist

1. Add chainable read helpers to a custom QuerySet.
2. Expose that QuerySet through a custom Manager.
3. Define selector entrypoints for query composition.
4. Update views to consume selectors for reads.
5. Keep writes/transactions in service-layer code.

## Guardrails

- Avoid query logic scattered in views.
- Prefer explicit prefetch/select-related choices for relation-heavy reads.
- Keep selector naming stable and action-oriented.
