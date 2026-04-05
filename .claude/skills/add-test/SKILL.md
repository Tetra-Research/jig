---
name: add-test
description: Generate a Rust unit test using jig template rendering
allowed-tools: Read Bash Grep Glob Edit
argument-hint: <module_path> <test_description>
---

## Recipe variables

```!
jig vars ${CLAUDE_SKILL_DIR}/recipe.yaml
```

## Steps

1. Read the source file at $0 to understand existing test patterns (naming, helper functions, imports).
2. Write the test body based on: $1
3. Run jig to render the test:
   ```
   jig run ${CLAUDE_SKILL_DIR}/recipe.yaml \
     --vars '{"test_name": "<name>", "body": "<body>"}' \
     --json --dry-run
   ```
4. Review the rendered output. If it looks right, copy the rendered test function and inject it into the appropriate `#[cfg(test)] mod tests` block in the source file using Edit.
5. Run `cargo test <test_name>` to verify it passes.

## Notes

- Match the naming convention of existing tests in the file (e.g., `snake_case_describing_behavior`)
- The `body` variable is the raw test body — setup, action, assertions. It gets indented 4 spaces automatically.
- Use the `--dry-run` flag first so you can review before committing to a file write.
