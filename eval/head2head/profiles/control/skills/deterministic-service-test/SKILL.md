---
name: deterministic-service-test
description: Write deterministic service tests with stable fixtures and boundary mocks without using jig.
---

# Deterministic Service Test (Control)

Use this skill when creating tests for service modules.
Implement the file edits directly. Do not return a checklist-only response.

## Required Variables

- `service_symbol`
- `module_path`
- `create_method`
- `cancel_method`

## Required Output Contract

1. Write `tests/test_core_service.py`.
2. Import `datetime` from `datetime`, `create_autospec` from `unittest.mock`, `pytest`, and `CoreService` from `services.core_service`.
3. Define a `fixed_check_in` fixture that returns `datetime(2024, 1, 1, 12, 0, 0)`.
4. Define `test_create_record_returns_confirmed_status(fixed_check_in)`.
5. In that test:
- instantiate `service = CoreService()`
- instantiate `notifier = create_autospec(object, spec_set=True)`
- include a visible `# Act` comment immediately before the service call
- call `service.create_record(display_name="Alice", check_in=fixed_check_in, check_out=datetime(2024, 1, 3, 12, 0, 0))`
- assert `result["status"] == "confirmed"`
- assert `notifier.mock_calls == []`
6. Define `test_cancel_record_returns_cancelled_status()`.
7. In that test:
- instantiate `service = CoreService()`
- include a visible `# Act` comment immediately before the service call
- call `service.cancel_record(record_id="abc-123")`
- assert `result["status"] == "cancelled"`
8. Preserve the exact test names above and keep one behavior per test.

## Guardrails

- No `unittest.TestCase` in pytest-style tasks.
- No real clock/random dependencies in deterministic tests.
- Do not add extra helper abstractions or additional tests beyond the required contract.
