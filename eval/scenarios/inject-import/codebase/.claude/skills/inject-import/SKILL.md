---
name: inject-import
description: Add a missing import statement to a Python file. Use when fixing missing imports, ImportError, or NameError from missing imports.
---

# Inject Import

Add an import statement to a Python file, placed after existing django imports.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"file": "<target_file>", "import_line": "<import_statement>"}'
```

## Variables

- **file**: Target file to add the import to (e.g. `models.py`)
- **import_line**: Full import line (e.g. `from django.db.models import DateTimeField`)

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
- The recipe skips injection if the import already exists (idempotent).
