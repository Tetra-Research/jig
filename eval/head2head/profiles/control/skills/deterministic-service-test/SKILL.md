---
name: deterministic-service-test
description: Write deterministic service tests with stable fixtures and boundary mocks without using jig.
---

# Deterministic Service Test (Control)

Use this skill when creating tests for service modules.

## Required Variables

- `service_symbol`
- `module_path`
- `create_method`
- `cancel_method`

## Execution Checklist

1. Use deterministic input values (time/data/randomness controlled).
2. Keep one behavior per test function.
3. Include a visible `# Act` section in each test.
4. Mock only external boundaries and use autospec-capable mocks.
5. Avoid test coupling through shared mutable state.

## Guardrails

- No `unittest.TestCase` in pytest-style tasks.
- No real clock/random dependencies in deterministic tests.
- Keep assertions behavior-focused and minimal.
