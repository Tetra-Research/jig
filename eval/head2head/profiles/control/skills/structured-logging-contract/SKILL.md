---
name: structured-logging-contract
description: Apply a stable method/step structured logging contract without using jig.
---

# Structured Logging Contract (Control)

Use this skill for code paths that need stable observability.
Implement the file edits directly. Do not return a checklist-only response.

## Required Variables

- `target_file`
- `function_name`
- `event_namespace`
- `step_name`
- `entity_id_expr`

## Required Output Contract

1. Modify only the target module named by `target_file`.
2. Add `import logging` and define `logger = logging.getLogger(__name__)` at module scope.
3. In the target function, emit a start log before the existing body work using the event name `event_namespace + ".start"` with the provided variable value.
4. Emit a completion log before the return using the event name `event_namespace + ".done"` with the provided variable value.
5. Both logs must pass `extra={...}` with exactly these keys:
- `"method": function_name` using the provided function name literal
- `"step": step_name` using the provided step name literal
- `"entity_id": entity_id_expr` using the provided entity id expression
6. Preserve the surrounding function behavior and return shape.
7. Use the exact `.done` suffix, not an alternate phase name such as `.complete`.

## Guardrails

- Avoid unstructured free-text-only logs.
- Avoid PII-heavy payloads in log `extra` data.
- Keep key names and event names consistent across related methods.
