import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";

export type ToolHandler = (
  toolName: string,
  params: Record<string, unknown>
) => Promise<{
  content: Array<{ type: "text"; text: string }>;
  isError?: boolean;
}>;

export function registerTools(server: McpServer, handler: ToolHandler): void {
  server.tool(
    "jig_run",
    "Execute a jig recipe to create, inject, patch, or replace files from templates. A recipe is a YAML file that declares variables and file operations. Pass structured variables as the 'vars' object. Returns JSON with the list of operations performed (create/inject/patch/replace/skip) and files written. Use 'jig_vars' first to discover what variables a recipe expects.",
    {
      recipe: z.string().describe("Path to recipe.yaml file"),
      vars: z
        .object({})
        .passthrough()
        .optional()
        .describe(
          "Template variables as a JSON object. Use 'jig_vars' to see expected variables."
        ),
      dry_run: z
        .boolean()
        .default(false)
        .describe("Preview operations without writing files"),
      base_dir: z
        .string()
        .optional()
        .describe("Base directory for resolving output paths (default: cwd)"),
      force: z
        .boolean()
        .default(false)
        .describe("Overwrite existing files without error"),
      verbose: z
        .boolean()
        .default(false)
        .describe(
          "Include rendered template content and scope diagnostics in output"
        ),
    },
    async (params) => handler("jig_run", params)
  );

  server.tool(
    "jig_validate",
    "Validate a jig recipe or workflow YAML file. Checks that the YAML is well-formed, all referenced template files exist, and variable declarations are valid. Returns validation status, variable summary, and operation/step listing. Auto-detects whether the file is a recipe (has 'files') or workflow (has 'steps').",
    {
      path: z
        .string()
        .describe("Path to recipe.yaml or workflow.yaml file to validate"),
    },
    async (params) => handler("jig_validate", params)
  );

  server.tool(
    "jig_vars",
    "List the variables a recipe or workflow expects. Returns a JSON object where each key is a variable name and the value describes its type, whether it's required, its default value, and a human-readable description. Use this before 'jig_run' or 'jig_workflow' to discover what variables to pass.",
    {
      path: z
        .string()
        .describe("Path to recipe.yaml or workflow.yaml file"),
    },
    async (params) => handler("jig_vars", params)
  );

  server.tool(
    "jig_render",
    "Render a single Jinja2 template with variables, without a recipe. For one-off template rendering. If 'to' is specified, writes to that file; otherwise returns the rendered content directly. Supports all jig built-in filters (snakecase, camelcase, pascalcase, kebabcase, pluralize, singularize, etc.).",
    {
      template: z.string().describe("Path to a .j2 template file"),
      vars: z
        .object({})
        .passthrough()
        .optional()
        .describe("Template variables as a JSON object"),
      to: z
        .string()
        .optional()
        .describe(
          "Output file path. If omitted, rendered content is returned directly."
        ),
    },
    async (params) => handler("jig_render", params)
  );

  server.tool(
    "jig_workflow",
    "Execute a multi-step jig workflow that chains multiple recipes together. A workflow runs recipes in sequence with conditional steps, variable mapping between steps, and configurable error handling. Returns per-step results (success/skipped/error) with operations detail and aggregate file lists. Use 'jig_vars' first to discover workflow variables.",
    {
      workflow: z.string().describe("Path to workflow.yaml file"),
      vars: z
        .object({})
        .passthrough()
        .optional()
        .describe("Workflow variables as a JSON object"),
      dry_run: z
        .boolean()
        .default(false)
        .describe("Preview operations without writing files"),
      base_dir: z
        .string()
        .optional()
        .describe("Base directory for resolving output paths (default: cwd)"),
      force: z
        .boolean()
        .default(false)
        .describe("Overwrite existing files without error"),
      verbose: z
        .boolean()
        .default(false)
        .describe(
          "Include rendered content and scope diagnostics in output"
        ),
    },
    async (params) => handler("jig_workflow", params)
  );
}
