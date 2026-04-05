function hasKeys(obj: object | undefined): obj is object {
  return obj !== undefined && Object.keys(obj).length > 0;
}

export function buildRunArgs(params: {
  recipe: string;
  vars?: object;
  dry_run?: boolean;
  base_dir?: string;
  force?: boolean;
  verbose?: boolean;
}): string[] {
  const args = ["run", params.recipe, "--json"];
  if (hasKeys(params.vars)) args.push("--vars", JSON.stringify(params.vars));
  if (params.dry_run) args.push("--dry-run");
  if (params.force) args.push("--force");
  if (params.base_dir) args.push("--base-dir", params.base_dir);
  if (params.verbose) args.push("--verbose");
  return args;
}

export function buildValidateArgs(params: { path: string }): string[] {
  return ["validate", params.path, "--json"];
}

export function buildVarsArgs(params: { path: string }): string[] {
  return ["vars", params.path];
}

export function buildRenderArgs(params: {
  template: string;
  vars?: object;
  to?: string;
}): string[] {
  const args = ["render", params.template];
  if (hasKeys(params.vars)) args.push("--vars", JSON.stringify(params.vars));
  if (params.to) args.push("--to", params.to);
  return args;
}

export function buildWorkflowArgs(params: {
  workflow: string;
  vars?: object;
  dry_run?: boolean;
  base_dir?: string;
  force?: boolean;
  verbose?: boolean;
}): string[] {
  const args = ["workflow", params.workflow, "--json"];
  if (hasKeys(params.vars)) args.push("--vars", JSON.stringify(params.vars));
  if (params.dry_run) args.push("--dry-run");
  if (params.force) args.push("--force");
  if (params.base_dir) args.push("--base-dir", params.base_dir);
  if (params.verbose) args.push("--verbose");
  return args;
}
