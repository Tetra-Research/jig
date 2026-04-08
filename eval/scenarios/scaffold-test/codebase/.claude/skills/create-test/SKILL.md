---
name: create-test
description: Create a test file for a service or class. Use when scaffolding tests, adding test coverage, or creating test files.
---

# Create Test

Scaffold a test file for a service class with setup and test methods.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"service_name": "<ServiceName>", "module_path": "<module.path>"}'
```

## Variables

- **service_name**: Name of the service class (e.g. `CoreService`)
- **module_path**: Python module path to the service (e.g. `services.core_service`)

Extract these from the user's request or by inspecting the source file.

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
- The output file is `tests/test_<service_name_snakecase>.py`.
