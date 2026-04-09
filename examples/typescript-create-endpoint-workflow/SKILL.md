Use this when a new endpoint needs more than one deterministic move.

Default flow:
1. Fill in route, handler, and schema variables.
2. Run `jig workflow ${CLAUDE_SKILL_DIR}/workflow.yaml --vars '{...}'`.
3. Verify the generated route files and router registration.
