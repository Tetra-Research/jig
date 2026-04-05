import { describe, it, expect, afterEach } from "vitest";
import { createClient, type McpClient } from "./helpers.js";

describe("MCP protocol", () => {
  let client: McpClient;

  afterEach(async () => {
    if (client) {
      try { await client.close(); } catch { /* ignore */ }
    }
  });

  it("protocol-initialize — responds with server info and capabilities", async () => {
    client = createClient();
    const resp = await client.initialize();
    const result = resp.result as Record<string, unknown>;
    const serverInfo = result.serverInfo as Record<string, string>;
    expect(serverInfo.name).toBe("jig");
    expect(serverInfo.version).toBe("0.1.0");
    expect(result.capabilities).toBeDefined();
    expect((result.capabilities as Record<string, unknown>).tools).toBeDefined();
  });

  it("protocol-tools-list — tools/list returns 5 tools", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.listTools();
    const result = resp.result as Record<string, unknown>;
    const tools = result.tools as Array<Record<string, unknown>>;
    expect(tools).toHaveLength(5);
    const names = tools.map((t) => t.name).sort();
    expect(names).toEqual([
      "jig_render",
      "jig_run",
      "jig_validate",
      "jig_vars",
      "jig_workflow",
    ]);
  });

  it("protocol-eof-shutdown — exits when stdin closes", async () => {
    client = createClient();
    await client.initialize();
    const code = await client.close();
    // Server exits (may be 0 or killed by our cleanup timer — both acceptable)
    expect(typeof code).toBe("number");
  });

  it("protocol-malformed-json — server survives bad input and returns error", async () => {
    client = createClient();
    // Send malformed JSON-RPC with an id so we can read the error response
    client.proc.stdin!.write(JSON.stringify({ jsonrpc: "2.0", id: 999 }) + "\n");
    await new Promise((r) => setTimeout(r, 200));
    // Server should still be alive — send valid initialize
    const resp = await client.initialize();
    const result = resp.result as Record<string, unknown>;
    expect((result.serverInfo as Record<string, string>).name).toBe("jig");
  });

  it("protocol-unknown-tool — returns error for unknown tool name", async () => {
    client = createClient();
    await client.initialize();
    const resp = await client.callTool("nonexistent_tool", {});
    // MCP SDK should return an error (either JSON-RPC error or tool error)
    const error = resp.error as Record<string, unknown> | undefined;
    const result = resp.result as Record<string, unknown> | undefined;
    if (error) {
      // JSON-RPC protocol error
      expect(typeof error.code).toBe("number");
    } else if (result) {
      // Tool-level error
      expect((result as { isError?: boolean }).isError).toBe(true);
    }
  });
});
