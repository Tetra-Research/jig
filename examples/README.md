# Examples

Self-contained `jig` examples for routine, shape-constrained code generation and patching.

Each example is intended to be understandable on its own and includes:

- `README.md`
- `recipe.yaml`
- `vars.json`
- `before/`
- `after/`
- `templates/`

## Example Index

1. [`add-service-test`](./add-service-test)
2. [`structured-logging-contract`](./structured-logging-contract)
3. [`view-contract-enforcer`](./view-contract-enforcer)
4. [`query-layer-discipline`](./query-layer-discipline)
5. [`schema-migration-safety`](./schema-migration-safety)

## TypeScript Example Index

1. [`typescript-create-model`](./typescript-create-model)
2. [`typescript-update-model`](./typescript-update-model)
3. [`typescript-create-endpoint-workflow`](./typescript-create-endpoint-workflow)
4. [`typescript-update-endpoint-workflow`](./typescript-update-endpoint-workflow)

## Common Conventions

- `before/` contains the input file state
- `after/` contains the expected output file state
- `vars.json` contains one runnable variable set
- `recipe.yaml` is the main recipe entrypoint
- `templates/` holds the rendered fragments or files used by the recipe

These examples are product examples, not eval fixtures.
