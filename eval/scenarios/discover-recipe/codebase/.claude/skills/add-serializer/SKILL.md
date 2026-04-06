---
name: add-serializer
description: Add a new serializer class to serializers.py. Use when adding serializers or API data schemas.
---

# Add Serializer

Add a new serializer class to serializers.py.

## Usage

```bash
jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{"model_name": "<Model>", "fields": "<field_list>"}'
```

## Variables

- **model_name**: Model class name (e.g. `User`)
- **fields**: Comma-separated field names (e.g. `name, email, is_active`)

## Gotchas

- If jig exits non-zero, it prints the rendered template to stderr. Use that output to apply changes manually.
