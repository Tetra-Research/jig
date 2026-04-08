---
name: view-contract-enforcer
description: Enforce request-validation, permission, service-handoff, and response-contract boundaries in views without using jig.
---

# View Contract Enforcer (Control)

Use this skill when adding or updating API views.

## Required Variables

- `view_name`
- `http_method`
- `request_schema_name`
- `response_schema_name`
- `service_symbol`
- `url_path`
- `url_name`
- `test_name`
- `test_url`

## Execution Checklist

1. Validate input at the view boundary before business logic.
2. Keep authorization checks explicit and close to entrypoint behavior.
3. Hand domain writes to a service function (`service_symbol`), not the view.
4. Return a stable response contract (`response_schema_name`).
5. Add route and test coverage for the new view path.

## Guardrails

- No domain write logic in view functions.
- No ad hoc payload parsing that bypasses the request contract.
- Include both success and access/validation behavior in tests.
