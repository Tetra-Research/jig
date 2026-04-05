import { describe, it, expect, afterEach } from "vitest";
import { createClient, type McpClient } from "./helpers.js";

function getToolResult(resp: Record<string, unknown>): { content: Array<{ type: string; text: string }>; isError?: boolean } {
  return resp.result as { content: Array<{ type: string; text: string }>; isError?: boolean };
}

describe("jig_run e2e", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tool-run-success — dry run with valid recipe", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Foo" },
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.dry_run).toBe(true);
    expect(json.operations[0].action).toBe("create");
  });

  it("tool-run-dry-run — flag is passed", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Bar" },
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.dry_run).toBe(true);
  });

  it("tool-run-with-vars — vars serialized correctly", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Baz" },
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    // If vars were double-encoded or wrong, jig would error
    const json = JSON.parse(result.content[0].text);
    expect(json.operations).toBeDefined();
  });

  it("tool-run-no-vars — omitted vars does not error for recipes without required vars", async () => {
    client = createClient();
    await client.initialize();
    // workflow-basic has no required variables
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/workflow-basic/step1/recipe.yaml",
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
  });

  it("tool-run-error — nonexistent recipe", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/nonexistent.yaml",
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("jig exited with code");
  });
});

describe("jig_validate e2e", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tool-validate-recipe", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_validate", {
      path: "tests/fixtures/create-simple/recipe.yaml",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.valid).toBe(true);
  });

  it("tool-validate-workflow", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_validate", {
      path: "tests/fixtures/workflow-basic/workflow.yaml",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.valid).toBe(true);
    expect(json.type).toBe("workflow");
  });
});

describe("jig_vars e2e", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tool-vars-recipe", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_vars", {
      path: "tests/fixtures/create-simple/recipe.yaml",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.class_name).toBeDefined();
  });

  it("tool-vars-workflow", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_vars", {
      path: "tests/fixtures/workflow-basic/workflow.yaml",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    // workflow-basic has no variables, returns {}
    const json = JSON.parse(result.content[0].text);
    expect(json).toEqual({});
  });
});

describe("jig_render e2e", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tool-render-stdout — renders to stdout", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_render", {
      template: "tests/fixtures/create-simple/templates/service.j2",
      vars: { class_name: "Foo" },
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    expect(result.content[0].text).toContain("pub struct Foo;");
  });

  it("tool-render-to-file — writes to file", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_render", {
      template: "tests/fixtures/create-simple/templates/service.j2",
      vars: { class_name: "Bar" },
      to: "/tmp/jig-mcp-test-render-output.rs",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    // jig render --to outputs the rendered content to the file, stdout may be empty or have a confirmation
  });
});

describe("jig_workflow e2e", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("tool-workflow-success — dry run", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_workflow", {
      workflow: "tests/fixtures/workflow-basic/workflow.yaml",
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBeFalsy();
    const json = JSON.parse(result.content[0].text);
    expect(json.status).toBe("success");
    expect(json.steps).toHaveLength(2);
  });

  it("tool-workflow-error — nonexistent workflow", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_workflow", {
      workflow: "tests/fixtures/nonexistent.yaml",
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("jig exited with code");
  });
});

describe("error paths", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("error-missing-recipe — nonexistent recipe", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/nonexistent.yaml",
    });
    const result = getToolResult(resp);
    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("jig exited with code");
  });

  it("error-bad-vars — wrong variable type", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      dry_run: true,
    });
    const result = getToolResult(resp);
    // Missing required var class_name → error
    expect(result.isError).toBe(true);
    expect(result.content[0].text).toContain("jig exited with code");
  });

  it("binary-not-found — jig not on PATH", async () => {
    client = createClient(["--jig-path", "/nonexistent/jig"]);
    await client.initialize();
    const resp = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Foo" },
      dry_run: true,
    });
    const result = getToolResult(resp);
    expect(result.isError).toBe(true);
    // Should get ENOENT spawn error
    expect(result.content[0].text).toMatch(/not found|ENOENT/i);
  });
});

describe("determinism", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("same tool call twice — identical response content", async () => {
    client = createClient();
    await client.initialize();
    const resp1 = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Foo" },
      dry_run: true,
    });
    const resp2 = await client.callTool("jig_run", {
      recipe: "tests/fixtures/create-simple/recipe.yaml",
      vars: { class_name: "Foo" },
      dry_run: true,
    });
    const r1 = getToolResult(resp1);
    const r2 = getToolResult(resp2);
    expect(r1.content[0].text).toBe(r2.content[0].text);
    expect(r1.isError).toBe(r2.isError);
  });
});
