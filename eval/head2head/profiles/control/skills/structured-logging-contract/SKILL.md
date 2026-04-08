---
name: structured-logging-contract
description: Apply a stable method/step structured logging contract without using jig.
---

# Structured Logging Contract (Control)

Use this skill for code paths that need stable observability.

## Required Variables

- `target_file`
- `function_name`
- `event_namespace`
- `step_name`
- `entity_id_expr`

## Execution Checklist

1. Ensure logger setup is present and local to module conventions.
2. Emit a start event at function entry.
3. Emit a completion event before return.
4. Use stable keys: `method`, `step`, `entity_id`.
5. Keep event names stable as `<event_namespace>.<phase>`.

## Guardrails

- Avoid unstructured free-text-only logs.
- Avoid PII-heavy payloads in log `extra` data.
- Keep key names and event names consistent across related methods.
