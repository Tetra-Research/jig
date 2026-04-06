---
name: add-view
description: Add a new API view function or endpoint to a Django views.py file. Use when adding views, endpoints, or API routes.
---

# Add View

Add a new API view function to views.py using jig.

## Usage

Run the jig recipe with `view_name` and `model_name`:

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"view_name": "<name>", "model_name": "<Model>"}'
```

If invoked as `/add-view <view_name> <model_name>`:

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"view_name": "$1", "model_name": "$2"}'
```

## Variables

- **view_name**: The function name (e.g. `reservation_receipt`)
- **model_name**: The Django model to query (e.g. `Reservation`)

Extract these from the user's request. For "add a receipt view for Reservation", that's `view_name: reservation_receipt`, `model_name: Reservation`.

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
- Run `jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml` to confirm required variables if unsure.
