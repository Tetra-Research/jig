---
name: add-view
description: Add a new API view function to a Django views.py file. Use when adding views or API handlers.
---

# Add View

Add a new API view function to views.py.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "<Model>", "view_name": "<name>"}'
```

## Variables

- **model_name**: Django model to query (e.g. `User`)
- **view_name**: View function name (e.g. `user_detail`)

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
