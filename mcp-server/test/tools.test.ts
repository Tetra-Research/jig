import { describe, it, expect, afterEach } from "vitest";
import { createClient, type McpClient } from "./helpers.js";

describe("tool definitions", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tools-list-returns-five", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    expect(tools).toHaveLength(5);
  });

  it("tools-names-correct", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const names = tools.map((t) => t.name).sort();
    expect(names).toEqual(["jig_render", "jig_run", "jig_validate", "jig_vars", "jig_workflow"]);
  });

  it("tools-have-descriptions", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    for (const tool of tools) {
      expect(tool.description).toBeTruthy();
      expect(typeof tool.description).toBe("string");
    }
  });

  it("tools-params-have-descriptions", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    for (const tool of tools) {
      const schema = tool.inputSchema as Record<string, unknown>;
      const props = schema.properties as Record<string, Record<string, unknown>>;
      for (const [, param] of Object.entries(props)) {
        expect(param.description).toBeTruthy();
      }
    }
  });

  it("tool-run-schema — correct required/optional params", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const run = tools.find((t) => t.name === "jig_run")!;
    const schema = run.inputSchema as Record<string, unknown>;
    const required = schema.required as string[];
    expect(required).toContain("recipe");
    const props = Object.keys(schema.properties as object);
    expect(props).toContain("vars");
    expect(props).toContain("dry_run");
    expect(props).toContain("base_dir");
    expect(props).toContain("force");
    expect(props).toContain("verbose");
  });

  it("tool-validate-schema", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const tool = tools.find((t) => t.name === "jig_validate")!;
    const schema = tool.inputSchema as Record<string, unknown>;
    expect((schema.required as string[])).toContain("path");
  });

  it("tool-vars-schema", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const tool = tools.find((t) => t.name === "jig_vars")!;
    const schema = tool.inputSchema as Record<string, unknown>;
    expect((schema.required as string[])).toContain("path");
  });

  it("tool-render-schema", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const tool = tools.find((t) => t.name === "jig_render")!;
    const schema = tool.inputSchema as Record<string, unknown>;
    expect((schema.required as string[])).toContain("template");
    const props = Object.keys(schema.properties as object);
    expect(props).toContain("vars");
    expect(props).toContain("to");
  });

  it("tool-workflow-schema", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const tools = (resp.result as Record<string, unknown>).tools as Array<Record<string, unknown>>;
    const tool = tools.find((t) => t.name === "jig_workflow")!;
    const schema = tool.inputSchema as Record<string, unknown>;
    expect((schema.required as string[])).toContain("workflow");
    const props = Object.keys(schema.properties as object);
    expect(props).toContain("vars");
    expect(props).toContain("dry_run");
    expect(props).toContain("base_dir");
    expect(props).toContain("force");
    expect(props).toContain("verbose");
  });
});
