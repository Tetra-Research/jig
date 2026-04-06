---
name: add-endpoint
description: Add a new REST API endpoint with view, URL route, schema, and test. Use when adding endpoints, routes, or API paths.
---

# Add Endpoint

Add a complete API endpoint: view function, URL route, response schema, and test.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"view_name": "<name>", "url_path": "<path>", "url_name": "<name>", "schema_name": "<Schema>", "model_name": "<Model>"}'
```

If invoked as `/add-endpoint <view_name> <model_name>`:

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"view_name": "$1", "url_path": "<inferred>", "url_name": "<inferred>", "schema_name": "<inferred>", "model_name": "$2"}'
```

## Variables

- **view_name**: View function name (e.g. `reservation_receipt`)
- **url_path**: URL path segment (e.g. `reservations/<int:pk>/receipt/`)
- **url_name**: URL name for reverse lookup (e.g. `reservation-receipt`)
- **schema_name**: Response schema class name (e.g. `ReceiptSchema`)
- **model_name**: Django model to query (e.g. `Reservation`)

Extract these from the user's request. Infer url_path and url_name from view_name if not explicit.

## Gotchas

- If jig exits non-zero, it prints the rendered templates to stderr. Use that output to apply changes manually.
- Run `jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml` to confirm required variables if unsure.
