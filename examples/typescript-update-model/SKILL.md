Use this when a Zod-backed TypeScript model needs a routine field added without reauthoring the schema by hand.

Default flow:
1. Point `target_file` at the model file.
2. Set `schema_symbol`, `field_name`, and `field_schema`.
3. Run `jig run ${CLAUDE_SKILL_DIR}/recipe.yaml --vars '{...}'`.
