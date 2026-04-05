---
name: add-endpoint
description: Create a new Django view using the request_framework pattern with typed dataclass params, gatekeeper auth, and response types
allowed-tools: Read Bash Grep Glob Edit
argument-hint: <app_name> <description_of_endpoint>
---

## Recipe variables

```!
jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml
```

## Steps

1. Identify the app from $0 and the endpoint intent from $1.
2. Read the app's existing views to understand patterns:
   - `$0/views/gatekeeper.py` — which gatekeeper class and what resource lookup methods exist
   - `$0/views/` — existing view files for naming and import conventions
   - `$0/urls.py` — URL pattern style
3. Read relevant models and schemas to determine:
   - Path parameter types (UUID fields, slug fields)
   - Request body fields and types
   - Response data type (existing schema or new dataclass)
   - What service/selector to call
4. Construct the jig variables and run:
   ```
   jig run ${CLAUDE_SKILL_DIR}/recipe.yaml \
     --vars '{ ... }' --json --dry-run
   ```
5. Review the dry-run output. If good, run without `--dry-run`.
6. The URL pattern is rendered to a separate file — read it and inject into `$0/urls.py` using Edit, adding the import for the new view class.
7. Clean up the URL pattern temp file.

## Framework conventions

- **Gatekeeper**: one per app at `<app>/views/gatekeeper.py`, named `<App>AppGatekeeper`
- **Parameter injection**: magic param names `path_params`, `query_params`, `body`, `cookies`, `headers` — each typed with a dataclass
- **Response types**: `DataResponse` wraps under `{"data": ...}`, `SimpleResponse` returns directly
- **Decorators**: `@readonly_database()` for GETs, `@no_readonly_database` for mutations
- **PathParams**: always includes `hotel_slug: str`, plus resource UUIDs as needed
