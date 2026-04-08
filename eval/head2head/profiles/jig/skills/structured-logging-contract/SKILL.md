---
name: structured-logging-contract
description: Insert consistent method/step structured logging lines into target functions using jig.
---

# Structured Logging Contract (Jig)

Use this skill to enforce stable observability signals.

## Required Variables

- `target_file`
- `function_name`
- `event_namespace`
- `step_name`
- `entity_id_expr`

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{
  "target_file": "services/core_service.py",
  "function_name": "create_record",
  "event_namespace": "core_service.create_record",
  "step_name": "validate_input",
  "entity_id_expr": "record_id if \"record_id\" in locals() else None"
}'
```

Notes:
- Patch anchors target the first function in `target_file`.
- Keep one primary function per target file when using this recipe.
