Use this when a TypeScript route already exists and you need to add another standard endpoint without hand-editing three files.

Default flow:
1. Fill in the update schema, handler, and route path variables.
2. Run `jig workflow ${CLAUDE_SKILL_DIR}/workflow.yaml --vars '{...}'`.
3. Verify the schema, handler, and router diffs.
