# typescript-update-endpoint-workflow

Patch an existing TypeScript route stack with a new update endpoint using a multi-step workflow.

## Run

```bash
jig workflow workflow.yaml --vars-file vars.json
```

## Expected Changes

- updates `src/routes/projects/schema.ts`
- updates `src/routes/projects/handler.ts`
- updates `src/routes/index.ts`

## Before / After

See `before/` and `after/`.
