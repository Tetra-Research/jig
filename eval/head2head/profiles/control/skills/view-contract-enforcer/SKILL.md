---
name: view-contract-enforcer
description: Enforce request-validation, permission, service-handoff, and response-contract boundaries in views without using jig.
---

# View Contract Enforcer (Control)

Use this skill when adding or updating API views.
Implement the file edits directly. Do not return a checklist-only response.

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

## Required Output Contract

1. Modify `schemas.py`, `views.py`, `urls.py`, and `tests/test_views.py`.
2. In `schemas.py`:
- add the request serializer class using the provided `request_schema_name`, with `correlation_id = serializers.CharField(required=False, allow_blank=True)`
- add the response serializer class using the provided `response_schema_name`, with `id = serializers.IntegerField()` and `status = serializers.CharField()`
3. In `views.py`:
- import the provided request and response schema names from `.schemas`
- import the provided `service_symbol` from `.services`
- define `@api_view([http_method])` using the provided method literal
- define the view function using the provided `view_name`
- assign `request_contract = RequestSchemaName(data=request.data)` using the provided request schema class
- call `request_contract.is_valid(raise_exception=True)`
- assign `payload = request_contract.validated_data`
- assign `result = service_symbol(pk=pk, payload=payload)` using the provided service symbol
- assign `response_contract = ResponseSchemaName(result)` using the provided response schema class
- return `Response(response_contract.data, status=200)`
4. In `urls.py`, add a path entry using `api/` plus the provided `url_path`, pointing to the provided `view_name`, with the provided `url_name`.
5. In `tests/test_views.py`, add the provided `test_name` method that posts to the provided `test_url` with `{"correlation_id": "h2h"}` and asserts the status code is in `[200, 400, 401, 404]`.
6. Use the exact variable names `request_contract`, `payload`, and `response_contract`.

## Guardrails

- No domain write logic in view functions.
- No ad hoc payload parsing that bypasses the request contract.
- Do not add extra permission decorators or alternate response-contract wiring unless the prompt explicitly asks for them.
