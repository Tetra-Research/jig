# typescript-create-endpoint-workflow

Create a new TypeScript endpoint stack with a workflow: schema, handler, import, and route registration.

## Run

```bash
jig workflow workflow.yaml --vars-file vars.json
```

## Expected Changes

- creates `src/routes/projects/schema.ts`
- creates `src/routes/projects/handler.ts`
- updates `src/routes/index.ts`

## Before / After

See `before/` and `after/`.
