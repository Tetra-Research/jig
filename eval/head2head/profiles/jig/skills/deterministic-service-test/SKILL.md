---
name: deterministic-service-test
description: Generate deterministic pytest service tests with stable inputs and autospec-boundary mocks using jig.
---

# Deterministic Service Test (Jig)

Use this skill for service test scaffolding with deterministic structure.

## Required Variables

- `service_symbol`
- `module_path`
- `create_method`
- `cancel_method`

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{
  "service_symbol": "CoreService",
  "module_path": "services.core_service",
  "create_method": "create_record",
  "cancel_method": "cancel_record"
}'
```
