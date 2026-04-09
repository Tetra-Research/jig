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
jig run ${CLAUDE_SKILL_DIR:-.claude/skills/structured-logging-contract}/recipe.yaml --vars '{
  "target_file": "services/core_service.py",
  "function_name": "create_record",
  "event_namespace": "core_service.create_record",
  "step_name": "validate_input",
  "entity_id_expr": "record_id if \"record_id\" in locals() else None"
}'
```

Notes:
- The recipe now targets the rendered `function_name` anchor directly.
- The `.done` event is inserted immediately before the `return {` line via a direct `before` selector.
