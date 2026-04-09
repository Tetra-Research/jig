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

1. [`add-service-test`](/Users/tylerobriant/code/tetra/jig/examples/add-service-test)
2. [`structured-logging-contract`](/Users/tylerobriant/code/tetra/jig/examples/structured-logging-contract)
3. [`view-contract-enforcer`](/Users/tylerobriant/code/tetra/jig/examples/view-contract-enforcer)
4. [`query-layer-discipline`](/Users/tylerobriant/code/tetra/jig/examples/query-layer-discipline)
5. [`schema-migration-safety`](/Users/tylerobriant/code/tetra/jig/examples/schema-migration-safety)

## Common Conventions

- `before/` contains the input file state
- `after/` contains the expected output file state
- `vars.json` contains one runnable variable set
- `recipe.yaml` is the main recipe entrypoint
- `templates/` holds the rendered fragments or files used by the recipe

These examples are product examples, not eval fixtures.

