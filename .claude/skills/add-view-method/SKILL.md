---
name: add-view-method
description: Add an HTTP method (GET/POST/PATCH/DELETE) to an existing Django view using request_framework
allowed-tools: Read Bash Grep Glob Edit
argument-hint: <view_file_path> <http_method> <description>
---

## Recipe variables

```!
jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml
```

## Steps

1. Read the existing view file at $0 to understand:
   - The view class name and gatekeeper class
   - Existing PathParams dataclass
   - What methods already exist
   - Import patterns
2. Determine what the new $1 method needs based on: $2
3. Run jig to render the method and body dataclass:
   ```
   jig run ${CLAUDE_SKILL_DIR}/recipe.yaml \
     --vars '{ ... }' --json --dry-run --verbose
   ```
4. Read the `rendered_content` from the JSON output for each operation.
5. If the method needs a request body, use Edit to insert the body dataclass above the view class.
6. Use Edit to insert the rendered method at the end of the view class (after the last existing method).
7. Add any new imports needed at the top of the file.
8. Clean up: delete `_rendered/` if jig created it.

## Notes

- jig renders the templates but does NOT patch the file — you must use Edit to place the code.
  (Anchor pattern rendering is not yet supported; this will use jig's patch operation in a future version.)
- The method template includes the decorator stack (`@readonly_database` / `@no_readonly_database` + `@validate_request`).
- `skip_if` is not automatic here — check manually that the method doesn't already exist before inserting.
