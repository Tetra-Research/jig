---
name: view-contract-enforcer
description: Scaffold request/response contracts, view function wiring, URL route, and test updates with jig.
---

# View Contract Enforcer (Jig)

Use this skill to keep view boundary behavior consistent and mechanical.

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

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{
  "view_name": "entity_summary",
  "http_method": "POST",
  "request_schema_name": "EntitySummaryRequest",
  "response_schema_name": "EntitySummaryResponse",
  "service_symbol": "build_entity_summary",
  "url_path": "entities/<int:pk>/summary/",
  "url_name": "entity-summary",
  "test_name": "test_entity_summary",
  "test_url": "/api/entities/1/summary/"
}'
```

If jig exits non-zero, apply the rendered snippets manually from stderr output.
