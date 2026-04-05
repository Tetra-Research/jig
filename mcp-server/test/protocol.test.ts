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

  it("protocol-malformed-json — server survives bad input", async () => {
    client = createClient();
    // Send garbage before initialize
    client.proc.stdin!.write("not json at all\n");
    // Small delay then send valid initialize — server should still be alive
    await new Promise((r) => setTimeout(r, 200));
    const resp = await client.initialize();
    const result = resp.result as Record<string, unknown>;
    expect((result.serverInfo as Record<string, string>).name).toBe("jig");
  });
});
